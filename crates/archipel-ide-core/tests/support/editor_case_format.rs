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
    fn before_has_folds(&self) -> bool {
        self.before.as_deref().is_some_and(|b| b.contains("<fold "))
    }

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
            ids::NO_COPY_LINE_WITHOUT_SELECTION
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

/// Compares what an editor ended up with against the `after` block of a case: text, folds
/// (where the case names them) and carets (where it names them).
fn judge(case: &Case, expected: &Marked, actual: &Marked) -> Outcome {
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
    // Regions are compared in the folding cases and wherever the markup names one.
    let compare_folds =
        case.file.starts_with("04-") || !expected.folds.is_empty() || case.before_has_folds();
    if compare_folds && actual.folds != expected.folds {
        return Outcome::Fail(format!(
            "folds differ\n  expected: {}\n  actual:   {}",
            render_marked(&expected.text, &expected.selections, &expected.folds),
            render_marked(&actual.text, &actual.selections, &actual.folds),
        ));
    }
    // A case that names no caret does not check them.
    if !want.is_empty() && want != got {
        return Outcome::Fail(format!(
            "carets differ\n  expected: {}\n  actual:   {}",
            render_marked(&expected.text, &expected.selections, &expected.folds),
            render_marked(&actual.text, &actual.selections, &actual.folds),
        ));
    }
    Outcome::Pass
}
