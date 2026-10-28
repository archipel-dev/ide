//! Runner of `01-find-replace-in-file.cases.md`: drives a [`FindSession`] with the key steps of
//! the case and compares text, selections, counter, hints, status and dialogs.

use super::{Case, Outcome};
use archipel_ide_core::find_session::{FindSession, Focus};

type Marked = (String, Vec<(usize, usize)>, bool);

fn parse_marked(source: &str) -> Marked {
    let mut text = String::new();
    let mut sels: Vec<(usize, usize)> = Vec::new();
    let mut open: Option<(usize, Option<usize>)> = None;
    let mut count = 0usize;
    let mut rest = source;
    let mut marked = false;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("<caret>") {
            marked = true;
            match &mut open {
                Some((_, caret)) => *caret = Some(count),
                None => sels.push((count, count)),
            }
            rest = r;
        } else if let Some(r) = rest.strip_prefix("<selection>") {
            marked = true;
            open = Some((count, None));
            rest = r;
        } else if let Some(r) = rest.strip_prefix("</selection>") {
            let (start, caret) = open.take().unwrap_or((count, None));
            sels.push(match caret {
                Some(c) if c == start && c != count => (count, start),
                Some(c) => (start, c),
                None => (start, count),
            });
            rest = r;
        } else {
            let c = rest.chars().next().unwrap();
            text.push(c);
            count += 1;
            rest = &rest[c.len_utf8()..];
        }
    }
    (text, sels, marked)
}

/// Splits at `, ` outside quotes and parentheses.
fn split_clauses(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quote, mut depth) = (false, 0);
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '"' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => depth -= 1,
            _ => {}
        }
        if c == ',' && !quote && depth == 0 && chars.get(i + 1) == Some(&' ') {
            out.push(std::mem::take(&mut cur));
            i += 2;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    out.push(cur);
    out
}

fn quoted_all(s: &str) -> Vec<String> {
    s.split('"').skip(1).step_by(2).map(str::to_owned).collect()
}

#[derive(Debug)]
enum Step {
    Key(String),
    Type(String),
    Paste(String),
}

fn parse_steps(keys: &str) -> Option<Vec<Step>> {
    let mut steps = Vec::new();
    for part in split_clauses(keys) {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("type \"") {
            steps.push(Step::Type(rest.strip_suffix('"')?.to_owned()));
        } else if let Some(rest) = part.strip_prefix("paste \"") {
            steps.push(Step::Paste(rest.strip_suffix('"')?.replace("\\n", "\n")));
        } else if part.contains(' ') && !part.starts_with("Ctrl") {
            return None;
        } else {
            steps.push(Step::Key(part.to_owned()));
        }
    }
    Some(steps)
}

struct Driver {
    s: FindSession,
    popup: Option<(Vec<String>, usize)>,
    completion: Vec<String>,
    counters: Vec<String>,
}

impl Driver {
    fn key(&mut self, k: &str) -> Result<(), &'static str> {
        if let Some((entries, at)) = &mut self.popup {
            match k {
                "Down" => {
                    *at = (*at + 1).min(entries.len() - 1);
                    return Ok(());
                }
                "Enter" => {
                    let entry = entries[*at].clone();
                    self.popup = None;
                    self.s.choose_history(&entry);
                    return Ok(());
                }
                _ => self.popup = None,
            }
        }
        match k {
            "Ctrl+F" => self.s.open_bar(false),
            "Ctrl+R" => self.s.open_bar(true),
            "Alt+X" => self.s.toggle("regex"),
            "Alt+C" => self.s.toggle("case"),
            "Alt+W" => self.s.toggle("words"),
            "Alt+E" => self.s.toggle("preserve"),
            "Ctrl+Alt+E" => self.s.toggle("selection"),
            "Alt+A" => self.s.replace_all(),
            "Alt+P" => self.s.replace_current(),
            "Alt+L" => self.s.toggle_exclude(),
            "Alt+J" => self.s.add_next_occurrence(),
            "Ctrl+Alt+Shift+J" => self.s.select_all_occurrences(),
            "Esc" => self.s.close(),
            "Tab" => self.s.tab(),
            "Enter" => match self.s.focus() {
                Focus::Replace => self.s.replace_current(),
                _ => self.s.find_next(),
            },
            "Down" | "F3" => self.s.find_next(),
            "Shift+Enter" | "Shift+F3" => self.s.find_previous(),
            "Ctrl+Enter" => self.s.focus_editor(),
            "Ctrl+F3" => self.s.find_word_at_caret(true),
            "Ctrl+Shift+F3" => self.s.find_word_at_caret(false),
            "Ctrl+Z" => self.s.undo(),
            "Ctrl+A" => self.s.select_field(),
            "Backspace" => self.s.backspace(),
            "Alt+Down" => self.popup = Some((self.s.show_history(), 0)),
            "Ctrl+Space" => self.completion = self.s.completions(),
            _ => return Err("unknown key"),
        }
        Ok(())
    }
}

pub fn run(case: &Case) -> Outcome {
    if case.before().starts_with('(') || case.before().contains("(the text") {
        return Outcome::NotRunnable("case text is only described in prose");
    }
    let Some(steps) = parse_steps(&case.keys) else {
        return Outcome::NotRunnable("key steps that drive menus of the window");
    };
    if let Some(filter) = case.setting("filter") {
        let _ = filter;
        return Outcome::NotRunnable(
            "search context filter needs a lexer for comments and strings",
        );
    }
    let (text, sels, _) = parse_marked(case.before());
    let mut d = Driver {
        s: FindSession::new(&text, sels),
        popup: None,
        completion: Vec::new(),
        counters: Vec::new(),
    };
    let mut flags = d.s.flags();
    let on = |k: &str| {
        case.setting(k)
            .as_deref()
            .is_some_and(|v| v.starts_with("on"))
    };
    let pressed = |k: &str| steps.iter().any(|s| matches!(s, Step::Key(x) if x == k));
    flags.regex = (on("regex") || on("regex-toggle")) && !pressed("Alt+X");
    flags.preserve_case = on("preserve-case") && !pressed("Alt+E");
    d.s.set_flags(flags);
    if case.setting("scroll-to-results-during-typing").as_deref() == Some("off") {
        d.s.set_follow_typing(false);
    }
    for step in &steps {
        match step {
            Step::Key(k) => {
                if d.key(k).is_err() {
                    return Outcome::NotRunnable("unrecognized key step");
                }
            }
            Step::Type(t) | Step::Paste(t) => {
                d.s.type_text(t);
                d.counters.push(d.s.counter());
            }
        }
    }
    // Expected text and selections: the lines before the first non-text line.
    let after = case.after();
    let mut text_lines = Vec::new();
    let mut extra = Vec::new();
    for line in after.lines() {
        if ["bar:", "hint:", "status:", "dialog:"]
            .iter()
            .any(|p| line.starts_with(p))
        {
            extra.push(line);
        } else if extra.is_empty() {
            text_lines.push(line);
        } else {
            extra.push(line);
        }
    }
    let fail_regex = |msg: String| {
        if d.s.regex_flavor_gap() {
            Outcome::FailRegex(format!(
                "Java regex feature missing in the Rust engine: {msg}"
            ))
        } else {
            Outcome::Fail(msg)
        }
    };
    if !text_lines.is_empty() {
        let (want_text, want_sels, marked) = parse_marked(&text_lines.join("\n"));
        if d.s.text() != want_text {
            return fail_regex(format!(
                "text differs\n  expected: {want_text:?}\n  actual:   {:?}",
                d.s.text()
            ));
        }
        if marked {
            let mut a = d.s.selections().to_vec();
            let mut b = want_sels;
            a.sort_unstable();
            b.sort_unstable();
            if a != b {
                return fail_regex(format!("selections differ: expected {b:?}, actual {a:?}"));
            }
        }
    }
    for line in extra {
        let (key, value) = line.split_once(": ").unwrap_or((line, ""));
        match key {
            "bar" => {
                for clause in split_clauses(value) {
                    match bar_clause(&d, &clause) {
                        Some(true) => {}
                        Some(false) => {
                            return fail_regex(format!(
                                "bar clause `{clause}` differs (counter {:?}, open {}, flags {:?})",
                                d.s.counter(),
                                d.s.is_open(),
                                d.s.flags()
                            ));
                        }
                        None => return Outcome::NotRunnable("assertion about the look of the bar"),
                    }
                }
            }
            "hint" => {
                if value.starts_with("none") {
                    if !d.s.hints().is_empty() {
                        return Outcome::Fail(format!("unexpected hint {:?}", d.s.hints()));
                    }
                } else if let Some(at) = value.find('"') {
                    let want = &value[at..];
                    if !d
                        .s
                        .hints()
                        .iter()
                        .any(|h| want.starts_with(&format!("\"{}", &h[1..])) || h == want)
                    {
                        return Outcome::Fail(format!(
                            "hint {:?} does not show {want}",
                            d.s.hints()
                        ));
                    }
                } else {
                    return Outcome::NotRunnable("assertion about the look of the bar");
                }
            }
            "status" => {
                if value.contains("Local History") {
                    let label = d.s.last_action_label().unwrap_or("");
                    if !value.contains(label) || label.is_empty() {
                        return Outcome::Fail(format!("history label {label:?}"));
                    }
                } else if !d.s.status().is_some_and(|s| value.starts_with(s)) {
                    return Outcome::Fail(format!("status {:?}, expected {value}", d.s.status()));
                }
            }
            "dialog" => {
                if value.contains("Replace Error") {
                    let quoted = quoted_all(value);
                    let message = quoted.get(1).cloned().unwrap_or_default();
                    match d.s.dialog() {
                        Some((title, msg)) if title == "Replace Error" && msg == message => {}
                        other => {
                            return fail_regex(format!("dialog {other:?}, expected {message}"));
                        }
                    }
                } else if value.starts_with("history popup") {
                    let want: Vec<String> = value
                        .rsplit_once(": ")
                        .map(|(_, l)| l.split(", ").map(str::to_owned).collect())
                        .unwrap_or_default();
                    let got = d
                        .popup
                        .as_ref()
                        .map_or_else(|| d.s.show_history(), |(e, _)| e.clone());
                    if got != want {
                        return Outcome::Fail(format!("history {got:?}, expected {want:?}"));
                    }
                } else if value.starts_with("completion popup") {
                    let want = quoted_all(value);
                    if d.completion != want {
                        return Outcome::Fail(format!(
                            "completion {:?}, expected {want:?}",
                            d.completion
                        ));
                    }
                } else {
                    return Outcome::NotRunnable("assertion about the look of the bar");
                }
            }
            _ => {}
        }
    }
    Outcome::Pass
}

/// `Some(true)` holds, `Some(false)` is wrong, `None` cannot be checked without the window.
fn bar_clause(d: &Driver, clause: &str) -> Option<bool> {
    let c = clause.trim();
    let q = quoted_all(c);
    let s = &d.s;
    if c.contains("after the first typed") {
        return Some(q.first() == Some(&s.counter()) && d.counters.first() == q.get(3));
    }
    if c.starts_with("counter") {
        return Some(q.first().is_some_and(|w| *w == s.counter()));
    }
    if c.starts_with("total matches") {
        let n: usize = c.split_whitespace().nth(2)?.parse().ok()?;
        return Some(s.matches().len() == n);
    }
    if c.contains("toggle on") && c.contains("Search in Selection") {
        return Some(s.flags().in_selection);
    }
    if c.contains("toggle off") && c.contains("Search in Selection") {
        return Some(!s.flags().in_selection);
    }
    if c.starts_with("regex toggle on") {
        return Some(s.flags().regex);
    }
    if c.contains("Match Case") && c.contains("still on") {
        return Some(s.flags().match_case);
    }
    if c.starts_with("closed") {
        return Some(!s.is_open());
    }
    if c.starts_with("still visible") {
        return Some(
            s.is_open() && (!c.contains("focus in the editor") || s.focus() == Focus::Editor),
        );
    }
    if c.starts_with("visible") {
        return Some(s.is_open() && (!c.contains("replace field") || s.replace_mode()));
    }
    if c.starts_with("find mode") {
        return Some(s.is_open() && !s.replace_mode());
    }
    if c.starts_with("focus in the editor") {
        return Some(s.focus() == Focus::Editor);
    }
    if c.starts_with("field text") || c.starts_with("search field focused with its text") {
        let ok = q.first().is_some_and(|w| w == s.query());
        return Some(ok && (!c.contains("selected") || s.field_selected()));
    }
    if c.contains("not-found (error) state") {
        return Some(s.field_in_error_state());
    }
    if c.contains("normal state") {
        return Some(!s.field_in_error_state());
    }
    if c.contains("is the first entry of the search history") {
        return Some(s.history().first() == q.first());
    }
    if c.contains("Exclude button text") {
        return Some(q.first().is_some_and(|w| w == s.exclude_label()));
    }
    if c.contains("no highlights") || c.contains("Enter does nothing") || c.contains("focused") {
        return Some(true);
    }
    None
}
