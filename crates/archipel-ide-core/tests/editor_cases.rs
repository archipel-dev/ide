//! Executable editor acceptance cases (`docs/specs/editor/cases/*.cases.md`).
//!
//! Every `###` block of the case files is parsed, set up in a real [`IdeState`] (a project with one
//! file, the settings and the language of the case), driven through the path the window uses
//! (default keymap, action registry, typed commands) and compared with the `after` block:
//! text, carets and selections, and collapsed/expanded folding regions.
//!
//! The report `docs/parity/editor-cases.md` is written when `EDITOR_CASES_REPORT=1`
//! (`cargo run -p xtask -- editor-cases`). `EDITOR_CASES_ONLY=ED-TYPE` runs the cases whose id starts
//! with the text; `EDITOR_CASES_VERBOSE=1` prints the failing cases with expected and actual text.
//! The test fails when fewer cases pass than [`PASS_BASELINE`] (raise it when you fix cases, never
//! lower it).

use archipel_ide_actions::{ActionEffect, ActionPlace};
use archipel_ide_command::CommandRegistry;
use archipel_ide_core::ide_actions::{CommandActions, IdeDataProvider};
use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_keymap::{Lookup, Shortcut};
use archipel_ide_platform::{DataContext, data_keys};
use archipel_ide_settings::SettingValue;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Number of cases that must pass. Never lower it; raise it with every fix.
const PASS_BASELINE: usize = 341;

include!("support/editor_case_format.rs");
// ---------------------------------------------------------------------------------------------
// Execution

struct Runner {
    _root: tempfile::TempDir,
    state: IdeState,
    commands: CommandRegistry<IdeState>,
    _actions: CommandActions,
    clipboard: String,
    clipboard_line: Option<String>,
    path: String,
}

impl Runner {
    fn new(case: &Case) -> Result<Self, String> {
        let root = tempfile::tempdir().map_err(|e| e.to_string())?;
        let marked = parse_marked(case.before.as_deref().unwrap_or_default());
        let extension = match case.language.as_str() {
            "java" => "java",
            "kotlin" => "kt",
            _ => "txt",
        };
        let path = format!("Case.{extension}");
        std::fs::write(root.path().join(&path), &marked.text).map_err(|e| e.to_string())?;
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root.path()}))
            .map_err(|e| e.to_string())?;
        let mut actions = CommandActions::default();
        actions
            .sync(state.application().extensions(), &commands)
            .map_err(|e| e.to_string())?;
        let mut pages = None;
        let mut runner = Self {
            _root: root,
            state,
            commands,
            _actions: actions,
            clipboard: String::new(),
            clipboard_line: None,
            path,
        };
        runner.set("keymap.active", json!("$default"))?;
        for (label, value) in &case.settings {
            if label == "viewport-lines" {
                let lines: usize = value.parse().map_err(|_| "bad viewport-lines".to_owned())?;
                pages = Some(lines);
                continue;
            }
            if RUNNER_SETTINGS.contains(&label.as_str()) {
                continue;
            }
            let id = setting_id(label).ok_or_else(|| format!("unknown setting `{label}`"))?;
            let value = setting_value(&runner.state, id, label, value)?;
            runner.set(id, value.to_json())?;
        }
        for (abbreviation, text) in &case.templates {
            runner
                .run(
                    "template.save",
                    json!({"abbreviation": abbreviation, "text": text, "group": "user",
                           "description": ""}),
                )
                .map_err(|e| format!("template: {e}"))?;
        }
        runner
            .run("editor.openFile", json!({"path": runner.path}))
            .map_err(|e| format!("open: {e}"))?;
        runner.state.edit_active(|document| {
            if let Some(lines) = pages {
                document.input.page_lines = lines;
            }
            let len = document.text.len_chars();
            let selections: Vec<archipel_ide_text::Selection> = marked
                .selections
                .iter()
                .map(|&(anchor, head)| archipel_ide_text::Selection {
                    anchor: anchor.min(len),
                    head: head.min(len),
                })
                .collect();
            if !selections.is_empty() {
                document.selections.set_all(selections);
            }
            Ok(())
        })?;
        if !marked.folds.is_empty() {
            runner.state.edit_active(|document| {
                // The markup lists the state of every region: start from none collapsed.
                document.folds.clear();
                let generated = document
                    .analysis
                    .folds_in(0..document.analysis.source.len());
                for mark in marked.folds.iter().filter(|m| !m.expanded) {
                    let start_byte = document.text.char_to_byte(mark.start);
                    let end_byte = document.text.char_to_byte(mark.end);
                    let range = generated
                        .iter()
                        .find(|g| g.start_byte == start_byte && g.end_byte == end_byte)
                        .cloned()
                        .unwrap_or(archipel_ide_language::FoldRange {
                            start_byte,
                            end_byte,
                            start_line: document.text.char_to_line(mark.start),
                            end_line: document.text.char_to_line(mark.end),
                            placeholder: mark.placeholder.clone(),
                            kind: archipel_ide_language::FoldKind::Block,
                            default: archipel_ide_language::FoldDefault::Expanded,
                            group: 0,
                        });
                    document.folds.collapse(&range);
                }
                Ok(())
            })?;
        }
        if let Some(clipboard) = &case.clipboard {
            runner.clipboard = unescape(clipboard);
        }
        Ok(runner)
    }

    fn set(&mut self, id: &str, value: Value) -> Result<(), String> {
        self.run("settings.set", json!({"id": id, "value": value}))
            .map(|_| ())
    }

    fn run(&mut self, command: &str, params: Value) -> Result<Value, String> {
        self.commands
            .execute(command, &mut self.state, params)
            .map_err(|e| e.to_string())
    }

    /// Like [`Runner::run`] for commands the window starts from a key: a command that is
    /// disabled (nothing to redo) does nothing, as in the window.
    fn run_from_key(&mut self, command: &str, params: Value) -> Result<Value, String> {
        match self.commands.execute(command, &mut self.state, params) {
            Err(archipel_ide_command::CommandError::Disabled(_)) => Ok(Value::Null),
            other => other.map_err(|e| e.to_string()),
        }
    }

    fn step(&mut self, step: &Step) -> Result<(), String> {
        match step {
            Step::Type(text) => {
                for c in text.chars() {
                    self.run("editor.type", json!({"text": c.to_string()}))?;
                }
                Ok(())
            }
            Step::Paste(text) => {
                self.clipboard = text.clone();
                self.clipboard_line = None;
                self.key("Ctrl+V")
            }
            Step::Action(id) => self.action(id, ActionPlace::MainMenu),
            Step::Key(chord) => self.key(chord),
            Step::Choose(text) => {
                let row = self
                    .state
                    .completion
                    .as_ref()
                    .and_then(|session| {
                        session
                            .ranked
                            .iter()
                            .position(|ranked| session.items[ranked.index].label == *text)
                    })
                    .ok_or_else(|| format!("the popup has no entry `{text}`"))?;
                self.state.select_completion(row);
                self.run("completion.accept", json!({"replace": false}))?;
                Ok(())
            }
            Step::Open(_) | Step::Close(_) => Err("open/close steps are not runnable yet".into()),
            Step::Other(text) => {
                // `Ctrl+G "6" Enter`: the Go to Line dialog with its input.
                if let Some(rest) = text.strip_prefix("Ctrl+G \"")
                    && let Some((line, tail)) = rest.split_once('"')
                    && tail.trim() == "Enter"
                    && let Ok(line) = line.parse::<u64>()
                {
                    self.run("editor.gotoLine", json!({"line": line}))?;
                    return Ok(());
                }
                Err(format!("unsupported step `{text}`"))
            }
        }
    }

    fn action(&mut self, id: &str, place: ActionPlace) -> Result<(), String> {
        match id {
            "$Copy" | "EditorCopy" => return self.copy(false),
            "$Cut" | "EditorCut" => return self.copy(true),
            "$Paste" | "EditorPaste" => return self.paste(),
            "SaveAll" => {
                self.run_from_key("editor.saveAll", Value::Null)?;
                return Ok(());
            }
            "EditorPasteSimple" => {
                let text = self.clipboard.replace("\r\n", "\n");
                self.run("editor.paste", json!({"text": text, "raw": true}))?;
                return Ok(());
            }
            _ => {}
        }
        let effect = {
            let provider = IdeDataProvider::new(&self.state, &self.commands);
            let data = DataContext::new()
                .with_provider(&provider)
                .with_value(&data_keys::EDITOR_FOCUSED, true);
            self.state.action_registry().perform(id, place, &data)
        };
        match effect {
            Some(ActionEffect::Command { id, params }) => {
                let result = self.run_from_key(&id, params)?;
                if matches!(id.as_str(), "editor.cutLineEnd" | "editor.cutLineBackward") {
                    self.clipboard = result["text"].as_str().unwrap_or_default().to_owned();
                    self.clipboard_line = None;
                }
                Ok(())
            }
            Some(_) => Err(format!("action {id} is a window request")),
            None => Err(format!("action {id} is not available")),
        }
    }

    fn key(&mut self, chord: &str) -> Result<(), String> {
        let normalized = normalize_chord(chord);
        let shortcut: Shortcut = normalized
            .parse()
            .map_err(|e: archipel_ide_keymap::ParseError| e.to_string())?;
        let Shortcut::Keys(strokes) = shortcut else {
            return Err(format!("not a key chord: {chord}"));
        };
        // The window gives a visible completion popup the arrows, Enter and Tab first.
        if self.popup_key(&strokes[0].key)? {
            return Ok(());
        }
        let lookup = self.state.keymap_state().keymap.lookup(&strokes);
        let Lookup::Actions(actions) = lookup else {
            return Err(format!("no action is bound to {chord}"));
        };
        for action in &actions {
            match self.action(action, ActionPlace::Keymap) {
                Ok(()) => return Ok(()),
                Err(error) if error.contains("is not available") => continue,
                Err(error) => return Err(error),
            }
        }
        Err(format!(
            "no available action for {chord}: {}",
            actions.join(", ")
        ))
    }

    fn popup_key(&mut self, key: &str) -> Result<bool, String> {
        let open = self
            .state
            .completion
            .as_ref()
            .is_some_and(|session| session.visible());
        if !open {
            return Ok(false);
        }
        match key {
            "Enter" => {
                self.run("completion.accept", json!({"replace": false}))?;
            }
            "Tab" => {
                self.run("completion.accept", json!({"replace": true}))?;
            }
            "Down" => self.state.move_completion_selection(1),
            "Up" => self.state.move_completion_selection(-1),
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn copy(&mut self, cut: bool) -> Result<(), String> {
        let result = self.run(if cut { "editor.cut" } else { "editor.copy" }, Value::Null)?;
        let text = result["text"].as_str().unwrap_or_default().to_owned();
        let whole_line = result["wholeLine"].as_bool().unwrap_or(false);
        if text.is_empty() && !whole_line && self.clipboard_line.is_none() {
            // Nothing copied (copying without selection switched off).
            return Ok(());
        }
        self.clipboard_line = whole_line.then(|| text.clone());
        self.clipboard = text;
        Ok(())
    }

    fn paste(&mut self) -> Result<(), String> {
        let text = self.clipboard.replace("\r\n", "\n");
        let whole_line = self.clipboard_line.as_deref() == Some(self.clipboard.as_str());
        self.run(
            "editor.paste",
            json!({"text": text, "wholeLine": whole_line}),
        )?;
        Ok(())
    }

    /// Text, selections (in markup order) and folds of the active document.
    fn observe(&self) -> Result<Marked, String> {
        let index = self.state.active_document.ok_or("no active document")?;
        let document = &self.state.documents[index];
        let selections = document
            .selections
            .selections()
            .iter()
            .map(|s| (s.anchor, s.head))
            .collect();
        let mut folds: Vec<FoldMark> = document
            .fold_regions()
            .into_iter()
            .map(|region| FoldMark {
                start: document.text.byte_to_char(region.range.start_byte),
                end: document.text.byte_to_char(region.range.end_byte),
                placeholder: region.range.placeholder.clone(),
                expanded: !region.collapsed,
            })
            .collect();
        folds.sort_by_key(|f| (f.start, std::cmp::Reverse(f.end)));
        Ok(Marked {
            text: document.text.text(),
            selections,
            folds,
        })
    }
}

fn run_case(case: &Case) -> Outcome {
    if let Some(reason) = not_runnable(case) {
        return Outcome::NotRunnable(reason);
    }
    let steps = parse_steps(&case.keys);
    if steps
        .iter()
        .any(|s| matches!(s, Step::Open(_) | Step::Close(_)))
        && !steps
            .iter()
            .all(|s| matches!(s, Step::Open(f) if f == "file"))
    {
        return Outcome::NotRunnable("tabs and window management");
    }
    let expected = parse_marked(case.after.as_deref().unwrap());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut runner = Runner::new(case)?;
        for step in &steps {
            if matches!(step, Step::Open(f) if f == "file") {
                continue;
            }
            runner
                .step(step)
                .map_err(|e| format!("step {step:?}: {e}"))?;
        }
        runner.observe()
    }));
    let actual = match result {
        Ok(Ok(actual)) => actual,
        Ok(Err(error)) => return Outcome::Fail(error),
        Err(panic) => {
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default();
            return Outcome::Fail(format!("panic: {message}"));
        }
    };
    judge(case, &expected, &actual)
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

fn report(rows: &[Row]) -> String {
    let mut files: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for row in rows {
        let entry = files.entry(row.case.file.as_str()).or_default();
        match row.outcome {
            Outcome::Pass => entry[0] += 1,
            Outcome::Fail(_) => entry[1] += 1,
            Outcome::NotRunnable(_) => entry[2] += 1,
        }
    }
    let total = |i: usize| files.values().map(|v| v[i]).sum::<usize>();
    let mut out = String::new();
    out.push_str("# Editor acceptance cases\n\n");
    out.push_str(
        "Generated by `cargo run -p xtask -- editor-cases` (the test `crates/archipel-ide-core/tests/editor_cases.rs`)\n\
         from `docs/specs/editor/cases/*.cases.md`. Do not edit by hand.\n\n",
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
        out.push_str("\nNot runnable yet (needs window state the cases drive):\n\n");
        for (reason, count) in &reasons {
            let _ = writeln!(out, "- {reason}: {count}");
        }
    }
    let mut groups: BTreeMap<String, [usize; 3]> = BTreeMap::new();
    for row in rows {
        let prefix: Vec<&str> = row.case.id.split('-').take(2).collect();
        let entry = groups.entry(prefix.join("-")).or_default();
        match row.outcome {
            Outcome::Pass => entry[0] += 1,
            Outcome::Fail(_) => entry[1] += 1,
            Outcome::NotRunnable(_) => entry[2] += 1,
        }
    }
    out.push_str("\n## By requirement group\n\n| Group | Pass | Fail | Not runnable |\n|---|---:|---:|---:|\n");
    for (group, v) in &groups {
        let _ = writeln!(out, "| {group} | {} | {} | {} |", v[0], v[1], v[2]);
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
            Outcome::NotRunnable(reason) => ("not runnable", (*reason).to_owned()),
        };
        let unverified = if row.case.unverified.is_some() {
            " (unverified)"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "| {}{} | {} | {} |",
            row.case.name(),
            unverified,
            status,
            note
        );
    }
    out
}

#[test]
fn editor_cases_do_not_regress() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(cases_dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".cases.md"))
        .collect();
    paths.sort();
    let only = std::env::var("EDITOR_CASES_ONLY").ok();
    let verbose = std::env::var("EDITOR_CASES_VERBOSE").is_ok();
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
            rows.len() >= 370,
            "cases were dropped: parsed {}",
            rows.len()
        );
        if std::env::var("EDITOR_CASES_REPORT").is_ok() {
            let target =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/parity/editor-cases.md");
            std::fs::write(target, report(&rows)).unwrap();
        }
        assert!(
            passing >= PASS_BASELINE,
            "only {passing} editor cases pass, the baseline is {PASS_BASELINE}; run \
             `EDITOR_CASES_VERBOSE=1 cargo test -p archipel-ide-core --test editor_cases` to see why"
        );
        if passing > PASS_BASELINE {
            eprintln!("{passing} cases pass: raise PASS_BASELINE from {PASS_BASELINE}");
        }
    } else {
        eprintln!("{passing} of {} selected cases pass", rows.len());
    }
}
