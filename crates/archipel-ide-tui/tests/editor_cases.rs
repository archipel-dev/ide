//! The editor acceptance cases of `docs/specs/editor/cases/` replayed through the terminal
//! frontend: the same case files, parser and judgement as the core's runner
//! (`archipel-ide-core/tests/editor_cases.rs`), but every step is a key press, a typed
//! character or a paste event of the TUI (`TuiApp::event`), resolved through the IDE keymap
//! and the core's input layer, and the result is read from the core document.
//!
//! The TUI has no own edit logic, so what passes in the core runner passes here. The test fails
//! when fewer cases pass than [`PASS_BASELINE`] (raise it, never lower it) and when one of the
//! cases of [`MUST_PASS`] does not.
#![allow(dead_code)]

mod common;

use archipel_ide_actions::ActionPlace;
use archipel_ide_core::IdeState;
use archipel_ide_settings::SettingValue;
use archipel_kit_tui::input::{Event, KeyCode, KeyModifiers, key_with};
use common::{Project, Tui};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Number of cases that must pass through the TUI. Never lower it; raise it with every fix.
const PASS_BASELINE: usize = 341;

/// Cases that must pass whatever the baseline says: typing pairs, Enter in braces, Backspace
/// unindent, duplicate line, multiple carets.
const MUST_PASS: &[&str] = &[
    // Typing: pairs, overtype, a pair's closer removed by Backspace.
    "ED-TYPE-010 case 1",
    "ED-TYPE-020 case 5",
    "ED-TYPE-043 case 1",
    // Enter between braces, Backspace snapping to the proper indent.
    "ED-TYPE-064 case 1",
    "ED-TYPE-046 case 2",
    // Duplicate line, with one caret and with two.
    "ED-LINE-001 case 1",
    "ED-LINE-001 case 4",
    // Several carets: typing at two, Add Selection for Next Occurrence.
    "ED-TYPE-001 case 2",
    "ED-CARET-074 case 2",
];

include!("../../archipel-ide-core/tests/support/editor_case_format.rs");

/// The chords a terminal cannot send (Ctrl+W is the window leader, the numeric keypad is
/// not reported) as the fallbacks `docs/tui.md` lists: Alt+Up/Down, or Ctrl+K and a key.
fn terminal_chord(chord: &str) -> String {
    match chord {
        "Ctrl+W" => "Alt+Up",
        "Ctrl+Shift+W" => "Alt+Down",
        "Ctrl+Numpad+" => "Ctrl+K =",
        "Ctrl+Numpad-" => "Ctrl+K -",
        "Ctrl+Alt+Numpad+" => "Ctrl+K Alt+=",
        "Ctrl+Alt+Numpad-" => "Ctrl+K Alt+-",
        "Ctrl+Shift+Numpad+" => "Ctrl+K Shift+=",
        "Ctrl+Shift+Numpad-" => "Ctrl+K Shift+-",
        other => other,
    }
    .to_owned()
}

fn chord_events(chord: &str) -> Result<Vec<Event>, String> {
    let normalized = normalize_chord(&terminal_chord(chord))
        .replace("Page Up", "PageUp")
        .replace("Page Down", "PageDown");
    let mut events = Vec::new();
    for stroke in normalized.split_whitespace() {
        let mut parts: Vec<&str> = stroke.split('+').collect();
        // `Ctrl++` and `Ctrl+Numpad+` end in an empty part.
        let key = if parts.last() == Some(&"") && parts.len() > 1 {
            parts.pop();
            parts.pop();
            "+"
        } else {
            parts.pop().ok_or("empty chord")?
        };
        let mut mods = KeyModifiers::NONE;
        for part in parts {
            mods |= match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => KeyModifiers::CONTROL,
                "alt" => KeyModifiers::ALT,
                "shift" => KeyModifiers::SHIFT,
                "meta" | "cmd" | "super" => KeyModifiers::SUPER,
                other => return Err(format!("unknown modifier `{other}` in `{chord}`")),
            };
        }
        let code = match key {
            "Enter" => KeyCode::Enter,
            "Tab" => KeyCode::Tab,
            "Backspace" => KeyCode::Backspace,
            "Delete" => KeyCode::Delete,
            "Insert" => KeyCode::Insert,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            "PageUp" | "Page Up" => KeyCode::PageUp,
            "PageDown" | "Page Down" => KeyCode::PageDown,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Escape" | "Esc" => KeyCode::Esc,
            "Space" => KeyCode::Char(' '),
            f if f.len() > 1 && f.starts_with('F') && f[1..].parse::<u8>().is_ok() => {
                KeyCode::F(f[1..].parse().unwrap())
            }
            c if c.chars().count() == 1 => {
                KeyCode::Char(c.chars().next().unwrap().to_ascii_lowercase())
            }
            other => return Err(format!("unknown key `{other}` in `{chord}`")),
        };
        events.push(key_with(code, mods));
    }
    Ok(events)
}

struct Replay {
    _project: Project,
    tui: Tui,
}

impl Replay {
    /// The case on a terminal tall enough that the editor shows the `viewport-lines` the case
    /// asks for (the TUI sizes the viewport from the screen).
    fn new(case: &Case, serial: usize) -> Result<Self, String> {
        let Some(lines) = case
            .settings
            .iter()
            .find(|(label, _)| label == "viewport-lines")
            .and_then(|(_, value)| value.parse::<usize>().ok())
        else {
            return Self::with_height(case, serial, 30);
        };
        for height in lines + 1..lines + 12 {
            let replay = Self::with_height(case, serial, height as u16)?;
            let state = replay.tui.app.ide().state();
            let page = state
                .active_document
                .map(|i| state.documents[i].input.page_lines);
            if page == Some(lines) {
                return Ok(replay);
            }
        }
        Err(format!("no terminal height shows {lines} editor lines"))
    }

    fn with_height(case: &Case, serial: usize, height: u16) -> Result<Self, String> {
        let marked = parse_marked(case.before.as_deref().unwrap_or_default());
        let extension = match case.language.as_str() {
            "java" => "java",
            "kotlin" => "kt",
            _ => "txt",
        };
        let path = format!("Case.{extension}");
        let project = Project::new(
            &format!("cases-{serial}"),
            &[(path.as_str(), marked.text.as_str())],
        );
        let mut tui = Tui::open(&project.root, &[], 100, height);
        let mut replay_error = None;
        let set = |tui: &mut Tui, id: &str, value: Value| {
            tui.app
                .ide_mut()
                .dispatch("settings.set", json!({"id": id, "value": value}))
                .map(|_| ())
        };
        set(&mut tui, "keymap.active", json!("$default"))?;
        for (label, value) in &case.settings {
            if RUNNER_SETTINGS.contains(&label.as_str()) {
                continue;
            }
            let id = setting_id(label).ok_or_else(|| format!("unknown setting `{label}`"))?;
            let value = setting_value(tui.app.ide().state(), id, label, value)?;
            if let Err(error) = set(&mut tui, id, value.to_json()) {
                replay_error = Some(error);
            }
        }
        if let Some(error) = replay_error {
            return Err(error);
        }
        for (abbreviation, text) in &case.templates {
            tui.app
                .ide_mut()
                .dispatch(
                    "template.save",
                    json!({"abbreviation": abbreviation, "text": text, "group": "user",
                           "description": ""}),
                )
                .map_err(|e| format!("template: {e}"))?;
        }
        tui.app
            .ide_mut()
            .dispatch("editor.openFile", json!({"path": path}))
            .map_err(|e| format!("open: {e}"))?;
        // A frame first (it creates the editor view); the setup then changes the core document
        // directly, like the core's runner does.
        tui.draw();
        if !marked.selections.is_empty() {
            let selections: Vec<Value> = marked
                .selections
                .iter()
                .map(|&(anchor, head)| json!({"anchor": anchor, "head": head}))
                .collect();
            tui.app
                .ide_mut()
                .dispatch("editor.setSelections", json!({ "selections": selections }))?;
        }
        if !marked.folds.is_empty() {
            tui.app.ide_mut().state_mut().edit_active(|document| {
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
            tui.rt.clipboard.set(unescape(clipboard));
        }
        Ok(Self {
            _project: project,
            tui,
        })
    }

    fn dump(&self, what: &str) {
        if std::env::var("EDITOR_CASES_TRACE").is_ok() {
            let state = self.tui.app.ide().state();
            let d = &state.documents[0];
            eprintln!(
                "  {what}: sel {:?} folds {:?}",
                d.selections.selections(),
                d.fold_regions()
                    .iter()
                    .map(|r| (r.range.start_byte, r.range.end_byte, r.collapsed, r.user))
                    .collect::<Vec<_>>()
            );
        }
    }

    fn press_chord(&mut self, chord: &str) -> Result<(), String> {
        let events = chord_events(chord)?;
        let last = events.len().saturating_sub(1);
        for (index, event) in events.into_iter().enumerate() {
            self.dump("before stroke");
            if index < last {
                self.tui.send_undrawn(event);
            } else {
                self.tui.send(event);
            }
            self.dump("after stroke");
        }
        Ok(())
    }

    fn step(&mut self, step: &Step) -> Result<(), String> {
        match step {
            Step::Type(text) => {
                for c in text.chars() {
                    let code = match c {
                        '\n' => KeyCode::Enter,
                        '\t' => KeyCode::Tab,
                        c => KeyCode::Char(c),
                    };
                    self.tui.press(code);
                }
                Ok(())
            }
            Step::Paste(text) => {
                self.tui.rt.clipboard.set(text.clone());
                self.press_chord("Ctrl+V")
            }
            Step::Action(id) => {
                let Self { tui, .. } = self;
                if !tui.app.ide().knows_action(id) {
                    return Err(format!("action {id} is not available"));
                }
                tui.app.run_action(&mut tui.rt, id, ActionPlace::MainMenu);
                tui.draw();
                Ok(())
            }
            Step::Key(chord) => self.press_chord(chord),
            Step::Choose(text) => {
                let state = self.tui.app.ide_mut().state_mut();
                let row = state
                    .completion
                    .as_ref()
                    .and_then(|session| {
                        session
                            .ranked
                            .iter()
                            .position(|ranked| session.items[ranked.index].label == *text)
                    })
                    .ok_or_else(|| format!("the popup has no entry `{text}`"))?;
                state.select_completion(row);
                self.tui
                    .app
                    .ide_mut()
                    .dispatch("completion.accept", json!({"replace": false}))?;
                self.tui.draw();
                Ok(())
            }
            Step::Open(_) | Step::Close(_) => Err("open/close steps are not runnable yet".into()),
            Step::Other(text) => {
                if let Some(rest) = text.strip_prefix("Ctrl+G \"")
                    && let Some((line, tail)) = rest.split_once('"')
                    && tail.trim() == "Enter"
                    && let Ok(line) = line.parse::<u64>()
                {
                    self.tui
                        .app
                        .ide_mut()
                        .dispatch("editor.gotoLine", json!({"line": line}))?;
                    return Ok(());
                }
                Err(format!("unsupported step `{text}`"))
            }
        }
    }

    /// Text, selections (in markup order) and folds of the document in the core.
    fn observe(&self) -> Result<Marked, String> {
        let state = self.tui.app.ide().state();
        let index = state.active_document.ok_or("no active document")?;
        let document = &state.documents[index];
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

fn run_case(case: &Case, serial: usize) -> Outcome {
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
        let mut replay = Replay::new(case, serial)?;
        for step in &steps {
            if matches!(step, Step::Open(f) if f == "file") {
                continue;
            }
            replay
                .step(step)
                .map_err(|e| format!("step {step:?}: {e}"))?;
        }
        replay.observe()
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

fn all_cases() -> Vec<Case> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(cases_dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".cases.md"))
        .collect();
    paths.sort();
    paths.iter().flat_map(|path| parse_file(path)).collect()
}

#[test]
fn editor_cases_replayed_through_the_tui_key_path() {
    let only = std::env::var("EDITOR_CASES_ONLY").ok();
    let verbose = std::env::var("EDITOR_CASES_VERBOSE").is_ok();
    let mut passing = 0;
    let mut total = 0;
    let mut failed_must: Vec<String> = Vec::new();
    let mut passed_must: Vec<String> = Vec::new();
    for (serial, case) in all_cases().into_iter().enumerate() {
        if only
            .as_ref()
            .is_some_and(|prefixes| !prefixes.split(',').any(|p| case.id.starts_with(p)))
        {
            continue;
        }
        total += 1;
        let outcome = run_case(&case, serial);
        match &outcome {
            Outcome::Pass => {
                passing += 1;
                passed_must.push(case.name());
            }
            Outcome::Fail(reason) => {
                if verbose {
                    eprintln!("FAIL {} ({}): {reason}", case.name(), case.title);
                }
                if MUST_PASS.contains(&case.name().as_str()) {
                    failed_must.push(format!("{}: {reason}", case.name()));
                }
            }
            Outcome::NotRunnable(_) => {}
        }
    }
    assert!(
        failed_must.is_empty(),
        "cases the TUI must pass failed:\n{}",
        failed_must.join("\n")
    );
    if only.is_none() {
        for name in MUST_PASS {
            assert!(
                passed_must.iter().any(|passed| passed == name),
                "{name} is not among the passing cases"
            );
        }
        assert!(total >= 370, "cases were dropped: parsed {total}");
        assert!(
            passing >= PASS_BASELINE,
            "only {passing} editor cases pass through the TUI, the baseline is {PASS_BASELINE}; \
             run `EDITOR_CASES_VERBOSE=1 cargo test -p archipel-ide-tui --test editor_cases`"
        );
        if passing > PASS_BASELINE {
            eprintln!("{passing} cases pass: raise PASS_BASELINE from {PASS_BASELINE}");
        }
    } else {
        eprintln!("{passing} of {total} selected cases pass through the TUI");
    }
}
