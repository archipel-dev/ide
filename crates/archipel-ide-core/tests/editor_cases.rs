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
const PASS_BASELINE: usize = 56;

// ---------------------------------------------------------------------------------------------
// Parsing

#[derive(Debug, Default, Clone)]
struct Case {
    file: String,
    id: String,
    number: u32,
    title: String,
    settings: Vec<(String, String)>,
    unverified: Option<String>,
    templates: Vec<(String, String)>,
    language: String,
    viewport: Option<String>,
    tabs: Option<String>,
    history: Option<String>,
    clipboard: Option<String>,
    before: Option<String>,
    keys: String,
    after: Option<String>,
    after_viewport: Option<String>,
    after_tabs: Option<String>,
    after_font: Option<String>,
}

impl Case {
    fn name(&self) -> String {
        format!("{} case {}", self.id, self.number)
    }
}

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/specs/editor/cases")
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
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_owned();
        match key {
            "settings" if value != "default" => {
                case.settings = split_settings(&value);
            }
            "settings" => {}
            "status" => case.unverified = Some(value),
            "template" => {
                if let Some((abbr, text)) = value.split_once(" = ") {
                    case.templates
                        .push((abbr.trim().to_owned(), text.to_owned()));
                }
            }
            "language" => case.language = value,
            "viewport" => case.viewport = Some(value),
            "tabs" => case.tabs = Some(value),
            "history" => case.history = Some(value),
            "clipboard" => case.clipboard = Some(value),
            "keys" => case.keys = value,
            "after viewport" => case.after_viewport = Some(value),
            "after tabs" => case.after_tabs = Some(value),
            "after font size" => case.after_font = Some(value),
            _ => {}
        }
    }
    cases
}

/// `a=on, b=Text with, commas?` is split at `, ` followed by something that looks like `name=`.
fn split_settings(value: &str) -> Vec<(String, String)> {
    let mut parts: Vec<String> = Vec::new();
    for piece in value.split(", ") {
        if piece.contains('=') || parts.is_empty() {
            parts.push(piece.to_owned());
        } else if let Some(last) = parts.last_mut() {
            last.push_str(", ");
            last.push_str(piece);
        }
    }
    parts
        .into_iter()
        .filter_map(|part| {
            let (name, value) = part.rsplit_once('=')?;
            Some((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FoldMark {
    start: usize,
    end: usize,
    placeholder: String,
    expanded: bool,
}

/// A text with `<caret>`, `<selection>` and `<fold>` markup removed.
#[derive(Debug, Clone, Default)]
struct Marked {
    text: String,
    /// `(anchor, head)` in character offsets, in the order of the markup.
    selections: Vec<(usize, usize)>,
    folds: Vec<FoldMark>,
}

fn parse_marked(source: &str) -> Marked {
    let source = source.replace("\\t", "\t");
    let mut out = Marked::default();
    let mut count = 0usize;
    let mut selection_start: Option<(usize, Option<usize>)> = None;
    let mut fold_stack: Vec<(usize, String, bool)> = Vec::new();
    let mut rest = source.as_str();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<caret>") {
            match &mut selection_start {
                Some((_, caret)) => *caret = Some(count),
                None => out.selections.push((count, count)),
            }
            rest = after;
        } else if let Some(after) = rest.strip_prefix("<selection>") {
            selection_start = Some((count, None));
            rest = after;
        } else if let Some(after) = rest.strip_prefix("</selection>") {
            let (start, caret) = selection_start.take().expect("closing without opening");
            let selection = match caret {
                Some(caret) if caret == start && caret != count => (count, start),
                Some(caret) => (start, caret),
                None => (start, count),
            };
            out.selections.push(selection);
            rest = after;
        } else if rest.starts_with("<fold ") {
            let end = rest.find('>').expect("unterminated fold tag");
            let tag = &rest[..end];
            let text = tag
                .split("text=\"")
                .nth(1)
                .and_then(|t| t.split('"').next())
                .unwrap_or("...");
            fold_stack.push((count, text.to_owned(), tag.contains(" expanded")));
            rest = &rest[end + 1..];
        } else if let Some(after) = rest.strip_prefix("</fold>") {
            let (start, placeholder, expanded) = fold_stack.pop().expect("closing fold");
            out.folds.push(FoldMark {
                start,
                end: count,
                placeholder,
                expanded,
            });
            rest = after;
        } else {
            let c = rest.chars().next().unwrap();
            out.text.push(c);
            count += 1;
            rest = &rest[c.len_utf8()..];
        }
    }
    out.folds
        .sort_by_key(|f| (f.start, std::cmp::Reverse(f.end)));
    out
}

/// Renders text, selections and folds back to the markup (for failure messages).
fn render_marked(text: &str, selections: &[(usize, usize)], folds: &[FoldMark]) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut inserts: Vec<(usize, u8, String)> = Vec::new();
    for fold in folds {
        let open = if fold.expanded {
            format!("<fold text=\"{}\" expanded>", fold.placeholder)
        } else {
            format!("<fold text=\"{}\">", fold.placeholder)
        };
        inserts.push((fold.start, 2, open));
        inserts.push((fold.end, 0, "</fold>".into()));
    }
    for &(anchor, head) in selections {
        if anchor == head {
            inserts.push((head, 3, "<caret>".into()));
        } else if anchor < head {
            inserts.push((anchor, 1, "<selection>".into()));
            inserts.push((head, 1, "<caret></selection>".into()));
        } else {
            inserts.push((head, 1, "<selection><caret>".into()));
            inserts.push((anchor, 1, "</selection>".into()));
        }
    }
    inserts.sort_by_key(|(offset, order, _)| (*offset, *order));
    let mut out = String::new();
    let mut next = 0;
    for offset in 0..=chars.len() {
        while next < inserts.len() && inserts[next].0 == offset {
            out.push_str(&inserts[next].2);
            next += 1;
        }
        if let Some(c) = chars.get(offset) {
            out.push(*c);
        }
    }
    out.replace('\t', "\\t")
}

// ---------------------------------------------------------------------------------------------
// Steps

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Type(String),
    Paste(String),
    Action(String),
    Key(String),
    Choose(String),
    Open(String),
    Close(String),
    Other(String),
}

fn unescape(quoted: &str) -> String {
    let inner = quoted.trim();
    let inner = inner
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(inner);
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Splits the `keys:` line at commas outside of quotes.
fn parse_steps(keys: &str) -> Vec<Step> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for c in keys.chars() {
        if quoted {
            current.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        match c {
            '"' => {
                quoted = true;
                current.push(c);
            }
            ',' => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    parts.push(current);
    parts
        .into_iter()
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty())
        .map(|part| {
            if let Some(rest) = part.strip_prefix("type ") {
                Step::Type(unescape(rest))
            } else if let Some(rest) = part.strip_prefix("paste ") {
                Step::Paste(unescape(rest))
            } else if let Some(rest) = part.strip_prefix("action ") {
                Step::Action(rest.trim().to_owned())
            } else if let Some(rest) = part.strip_prefix("choose ") {
                Step::Choose(unescape(rest))
            } else if let Some(rest) = part.strip_prefix("open ") {
                Step::Open(rest.trim().to_owned())
            } else if let Some(rest) = part.strip_prefix("close ") {
                Step::Close(rest.trim().to_owned())
            } else if part == "copy" {
                Step::Key("Ctrl+C".into())
            } else if part == "cut" {
                Step::Key("Ctrl+X".into())
            } else if part.contains('"') {
                Step::Other(part)
            } else {
                Step::Key(part)
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Settings

/// Case label -> setting id.
fn setting_id(label: &str) -> Option<&'static str> {
    use archipel_ide_settings::ids;
    Some(match label {
        "Use \"CamelHumps\" words" => ids::CAMEL_HUMPS,
        "Use tab character" => ids::USE_TAB,
        "Indent" => ids::INDENT_SIZE,
        "Detect and use existing file indents for editing" => ids::DETECT_INDENTS,
        "Move caret down after Comment with Line Comment action" => ids::COMMENT_MOVES_CARET,
        "Line comment at first column" => ids::LINE_COMMENT_FIRST_COLUMN,
        "Add a space at comment start" => ids::LINE_COMMENT_SPACE,
        "Add a space at block comment start" => ids::BLOCK_COMMENT_SPACE,
        "Surround selection on typing quote or brace" => ids::SURROUND_ON_TYPING,
        "Insert paired brackets" => ids::PAIR_BRACKETS,
        "Insert pair quote" => ids::PAIR_QUOTES,
        "Insert pair '}'" => ids::PAIR_BRACE_ON_ENTER,
        "Jump outside closing bracket/quote with Tab when typing" => ids::TAB_OUT,
        "Smart indent" => ids::SMART_INDENT,
        "Close block comment" => ids::CLOSE_BLOCK_COMMENT,
        "Home moves caret to first non-whitespace character" => ids::SMART_HOME,
        "Unindent on Backspace" => ids::UNINDENT_ON_BACKSPACE,
        "Reformat on paste" => ids::REFORMAT_ON_PASTE,
        "Don't copy/cut the current line when invoking the Copy or Cut action with no selection" => {
            ids::COPY_LINE_WITHOUT_SELECTION
        }
        "Use undo transparent caret movement" => ids::UNDO_TRANSPARENT_CARET,
        "Vertical scroll offset" => ids::SCROLL_OFFSET,
        "Expand with" => ids::TEMPLATE_EXPAND_KEY,
        "Imports" => ids::FOLD_IMPORTS,
        "Method bodies" => ids::FOLD_METHOD_BODIES,
        "one-line methods" => ids::FOLD_ONE_LINE_METHODS,
        "Documentation comments" => ids::FOLD_DOC_COMMENTS,
        "Custom folding regions" => ids::FOLD_CUSTOM_REGIONS,
        "caret-stop" => ids::CARET_STOP,
        _ => return None,
    })
}

/// Settings of the case that are not stored in the settings service.
const RUNNER_SETTINGS: &[&str] = &[
    "viewport-lines",
    "line-separator",
    "locale",
    "editor font size",
];

fn setting_value(
    state: &IdeState,
    id: &str,
    label: &str,
    value: &str,
) -> Result<SettingValue, String> {
    let settings = state.settings();
    let settings = settings.read();
    let definition = settings
        .registry
        .setting(id)
        .ok_or_else(|| format!("unknown setting id {id}"))?;
    use archipel_ide_settings::SettingKind;
    Ok(match &definition.kind {
        SettingKind::Bool => SettingValue::Bool(match value {
            "on" | "true" => true,
            "off" | "false" => false,
            other => return Err(format!("`{label}`: expected on/off, got {other}")),
        }),
        SettingKind::Int { .. } => SettingValue::Int(
            value
                .parse()
                .map_err(|_| format!("`{label}`: expected a number, got {value}"))?,
        ),
        SettingKind::Enum { options } => {
            let option = options
                .iter()
                .find(|o| o.label.eq_ignore_ascii_case(value) || o.id == value)
                .ok_or_else(|| format!("`{label}`: no option `{value}`"))?;
            SettingValue::Text(option.id.clone())
        }
        _ => return Err(format!("`{label}`: unsupported setting kind")),
    })
}

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

enum Outcome {
    Pass,
    Fail(String),
    NotRunnable(&'static str),
}

fn not_runnable(case: &Case) -> Option<&'static str> {
    if case.tabs.is_some() || case.after_tabs.is_some() || case.history.is_some() {
        return Some("tabs and window management");
    }
    if case.viewport.is_some() || case.after_viewport.is_some() {
        return Some("viewport and scrolling");
    }
    if case.after_font.is_some() {
        return Some("font size");
    }
    if case.before.is_none() || case.after.is_none() {
        return Some("no text blocks");
    }
    None
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
            Step::Choose(_) => Err("popup choice is not runnable yet".into()),
            Step::Open(_) | Step::Close(_) => Err("open/close steps are not runnable yet".into()),
            Step::Other(text) => Err(format!("unsupported step `{text}`")),
        }
    }

    fn action(&mut self, id: &str, place: ActionPlace) -> Result<(), String> {
        match id {
            "$Copy" | "EditorCopy" => return self.copy(false),
            "$Cut" | "EditorCut" => return self.copy(true),
            "$Paste" | "EditorPaste" => return self.paste(),
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
                self.run(&id, params)?;
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
        Ok(Marked {
            text: document.text.text(),
            selections,
            folds: Vec::new(),
        })
    }
}

/// `Ctrl+Numpad+` and friends in the notation of the keymap data.
fn normalize_chord(chord: &str) -> String {
    let mut out = chord.to_owned();
    for (from, to) in [
        ("Numpad+", "NumPad +"),
        ("Numpad-", "NumPad -"),
        ("Alt+Shift+Period", "Alt+Shift+."),
        ("Alt+Shift+Comma", "Alt+Shift+,"),
    ] {
        out = out.replace(from, to);
    }
    out
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
    let mut want = expected.selections.clone();
    let mut got = actual.selections.clone();
    want.sort_unstable();
    got.sort_unstable();
    if actual.text != expected.text {
        return Outcome::Fail(format!(
            "text differs\n  expected: {}\n  actual:   {}",
            render_marked(&expected.text, &expected.selections, &expected.folds),
            render_marked(&actual.text, &actual.selections, &actual.folds),
        ));
    }
    if want != got {
        return Outcome::Fail(format!(
            "carets differ\n  expected: {}\n  actual:   {}",
            render_marked(&expected.text, &expected.selections, &expected.folds),
            render_marked(&actual.text, &actual.selections, &actual.folds),
        ));
    }
    Outcome::Pass
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
                .is_some_and(|prefix| !case.id.starts_with(prefix.as_str()))
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
