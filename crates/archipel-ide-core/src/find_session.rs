//! The model of the editor search bar (spec `docs/specs/search/01-find-replace-in-file.md`,
//! SRCH-FIL): the query and its options, the matches of a text, the counter, the navigation
//! with the "no silent wrap" hint, Replace / Replace All with the Java-style replacement
//! templates, Preserve case, the history and the multi-caret commands.
//!
//! A [`FindSession`] owns a copy of the text and the selection it works on; the frontends feed
//! it the document and apply its edits, so the window and the terminal share one behavior.
//! Regular expressions use the Rust `regex` crate: lookaround and backreferences of the Java
//! flavor are not available (spec section 11, question 1); [`FindSession::counter`] then shows
//! `Bad pattern` and [`FindSession::regex_flavor_gap`] tells why.

use crate::name_match::{MatchMode, NameMatcher};
use regex::{Regex, RegexBuilder};
use std::cell::RefCell;
use std::collections::HashSet;

/// Matches counted at most (SRCH-FIL-010).
pub const MATCH_LIMIT: usize = 10_000;
/// Entries of a search history (SRCH-FIL-070).
const HISTORY_LIMIT: usize = 300;

/// Why a query cannot run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// An invalid regular expression (`Bad pattern`).
    BadPattern(String),
    /// A regex that consists only of `|` (`Vague pattern`).
    Vague,
}

/// The options of the bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Flags {
    pub match_case: bool,
    pub whole_words: bool,
    pub regex: bool,
    pub preserve_case: bool,
    pub in_selection: bool,
}

/// Which field has the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Editor,
    Search,
    Replace,
}

/// A char range `[start, end)` of the text.
pub type Range = (usize, usize);

fn is_identifier_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

fn chars_to_string(chars: &[char]) -> String {
    chars.iter().collect()
}

/// SRCH-FIL-021: whether `[s, e)` is a whole word of `text`.
fn is_whole_word(text: &[char], s: usize, e: usize) -> bool {
    if s >= e {
        return true;
    }
    let first = text[s];
    let start_ok = if s == 0 {
        true
    } else if is_identifier_part(first) {
        let prev = text[s - 1];
        // A previous identifier char that is itself preceded by a backslash does not count.
        !is_identifier_part(prev) || (s >= 2 && text[s - 2] == '\\')
    } else {
        text[s - 1] != first
    };
    let last = text[e - 1];
    let end_ok = if e == text.len() {
        true
    } else if is_identifier_part(last) {
        !is_identifier_part(text[e])
    } else {
        text[e] != last
    };
    start_ok && end_ok
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// All matches of a query in `text` (SRCH-FIL-020 to 023), ignoring the scope.
pub fn find_matches(text: &[char], query: &str, flags: Flags) -> Result<Vec<Range>, QueryError> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    if flags.regex {
        if query.chars().all(|c| c == '|') {
            return Err(QueryError::Vague);
        }
        let regex = compile(query, flags)?;
        let haystack = chars_to_string(text);
        let mut byte_to_char = vec![0usize; haystack.len() + 1];
        for (ci, (bi, c)) in haystack.char_indices().enumerate() {
            for k in 0..c.len_utf8() {
                byte_to_char[bi + k] = ci;
            }
        }
        byte_to_char[haystack.len()] = text.len();
        for m in regex.find_iter(&haystack) {
            if m.is_empty() {
                continue;
            }
            let (s, e) = (byte_to_char[m.start()], byte_to_char[m.end()]);
            if !flags.whole_words || is_whole_word(text, s, e) {
                out.push((s, e));
            }
            if out.len() > MATCH_LIMIT {
                break;
            }
        }
        return Ok(out);
    }
    let needle: Vec<char> = if flags.match_case {
        query.chars().collect()
    } else {
        query.chars().map(fold).collect()
    };
    let hay: Vec<char> = if flags.match_case {
        text.to_vec()
    } else {
        text.iter().map(|c| fold(*c)).collect()
    };
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay[i..i + needle.len()] == needle[..] {
            let e = i + needle.len();
            if !flags.whole_words || is_whole_word(text, i, e) {
                out.push((i, e));
                i = e;
                if out.len() > MATCH_LIMIT {
                    break;
                }
                continue;
            }
        }
        i += 1;
    }
    Ok(out)
}

fn compile(query: &str, flags: Flags) -> Result<Regex, QueryError> {
    RegexBuilder::new(query)
        .case_insensitive(!flags.match_case)
        .multi_line(true)
        .size_limit(1 << 22)
        .build()
        .map_err(|e| QueryError::BadPattern(e.to_string()))
}

/// A pattern the Java flavor supports and the Rust engine does not.
pub fn java_only_feature(query: &str) -> bool {
    query.contains("(?<=")
        || query.contains("(?<!")
        || query.contains("(?=")
        || query.contains("(?!")
        || query.contains("(?>")
        || ["\\1", "\\2", "\\3", "\\k<"]
            .iter()
            .any(|b| query.contains(b))
        || ["*+", "++", "?+"].iter().any(|p| query.contains(p))
}

/// Escapes regex metacharacters (a selection copied into the field in regex mode).
pub fn regex_escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if "\\.+*?()|[]{}^$".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

// ---------------------------------------------------------------- replacement

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaseMode {
    None,
    Upper,
    Lower,
}

struct Builder {
    out: String,
    mode: CaseMode,
    next: Option<bool>,
}

impl Builder {
    fn push(&mut self, c: char) {
        let c = match (self.next.take(), self.mode) {
            (Some(true), _) => c.to_uppercase().next().unwrap_or(c),
            (Some(false), _) => c.to_lowercase().next().unwrap_or(c),
            (None, CaseMode::Upper) => c.to_uppercase().next().unwrap_or(c),
            (None, CaseMode::Lower) => c.to_lowercase().next().unwrap_or(c),
            _ => c,
        };
        self.out.push(c);
    }

    fn push_str(&mut self, s: &str) {
        for c in s.chars() {
            self.push(c);
        }
    }
}

/// Expands a replacement template for one regex match (SRCH-FIL-043, SRCH-FIL-044).
pub fn expand_template(
    template: &str,
    regex: &Regex,
    caps: &regex::Captures,
) -> Result<String, String> {
    let chars: Vec<char> = template.chars().collect();
    let groups = regex.captures_len() - 1;
    let mut b = Builder {
        out: String::new(),
        mode: CaseMode::None,
        next: None,
    };
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1;
                let Some(&c) = chars.get(i) else {
                    return Err("character to be escaped is missing".into());
                };
                match c {
                    'n' => b.push('\n'),
                    'r' => b.push('\r'),
                    't' => b.push('\t'),
                    'f' => b.push('\u{c}'),
                    'b' => b.push('\u{8}'),
                    'x' => {
                        let hex: String = chars.iter().skip(i + 1).take(4).collect();
                        match u32::from_str_radix(&hex, 16)
                            .ok()
                            .filter(|_| hex.len() == 4)
                            .and_then(char::from_u32)
                        {
                            Some(ch) => {
                                b.push(ch);
                                i += 4;
                            }
                            None => b.push('x'),
                        }
                    }
                    'U' => {
                        b.mode = CaseMode::Upper;
                        b.next = None;
                    }
                    'L' => {
                        b.mode = CaseMode::Lower;
                        b.next = None;
                    }
                    'E' => {
                        b.mode = CaseMode::None;
                        b.next = None;
                    }
                    'u' => b.next = Some(true),
                    'l' => b.next = Some(false),
                    other => b.push(other),
                }
                i += 1;
            }
            '$' => {
                i += 1;
                let Some(&c) = chars.get(i) else {
                    return Err("Illegal group reference: group index is missing".into());
                };
                if c == '{' {
                    let rest: String = chars[i + 1..].iter().collect();
                    let Some(end) = rest.find('}') else {
                        return Err("named capturing group is missing trailing '}'".into());
                    };
                    let name = &rest[..end];
                    if name.is_empty() {
                        return Err("named capturing group has 0 length name".into());
                    }
                    if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                        return Err(format!(
                            "capturing group name {{{name}}} starts with digit character"
                        ));
                    }
                    match caps.name(name) {
                        Some(m) => b.push_str(m.as_str()),
                        None if regex.capture_names().flatten().any(|n| n == name) => {}
                        None => return Err(format!("No group with name {{{name}}}")),
                    }
                    i += 1 + rest[..end].chars().count() + 1;
                } else if let Some(d) = c.to_digit(10) {
                    let mut n = d as usize;
                    if n > groups {
                        return Err(format!("No group {n}"));
                    }
                    i += 1;
                    while let Some(next) = chars.get(i).and_then(|c| c.to_digit(10)) {
                        let candidate = n * 10 + next as usize;
                        if candidate > groups {
                            break;
                        }
                        n = candidate;
                        i += 1;
                    }
                    if let Some(m) = caps.get(n) {
                        b.push_str(m.as_str());
                    }
                } else {
                    return Err("Illegal group reference".into());
                }
            }
            c => {
                b.push(c);
                i += 1;
            }
        }
    }
    Ok(b.out)
}

/// SRCH-FIL-045: gives `replacement` the capitalisation of `found`.
pub fn preserve_case(found: &str, replacement: &str) -> String {
    fn words(s: &str) -> Vec<(usize, usize)> {
        let chars: Vec<char> = s.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].is_alphanumeric() {
                let start = i;
                while i < chars.len() && chars[i].is_alphanumeric() {
                    i += 1;
                }
                out.push((start, i));
            } else {
                i += 1;
            }
        }
        out
    }
    let found_chars: Vec<char> = found.chars().collect();
    let found_words: Vec<String> = words(found)
        .into_iter()
        .map(|(s, e)| found_chars[s..e].iter().collect())
        .collect();
    if found_words.is_empty() {
        return replacement.to_owned();
    }
    let mut chars: Vec<char> = replacement.chars().collect();
    for (index, (s, e)) in words(replacement).into_iter().enumerate() {
        let pattern = &found_words[index.min(found_words.len() - 1)];
        let has_letter = pattern.chars().any(char::is_alphabetic);
        if !has_letter {
            continue;
        }
        let upper = !pattern.chars().any(char::is_lowercase);
        let lower = !pattern.chars().any(char::is_uppercase);
        let word: String = chars[s..e].iter().collect();
        let new: String = if upper {
            word.to_uppercase()
        } else if lower {
            word.to_lowercase()
        } else {
            let mut it = word.chars();
            let first: String = it
                .next()
                .map(|c| c.to_uppercase().collect())
                .unwrap_or_default();
            let rest: String = it.collect();
            let rest = if rest.chars().any(char::is_lowercase) {
                rest
            } else {
                rest.to_lowercase()
            };
            format!("{first}{rest}")
        };
        for (k, c) in new.chars().enumerate() {
            if s + k < e {
                chars[s + k] = c;
            }
        }
    }
    chars.into_iter().collect()
}

// -------------------------------------------------------------------- session

/// The editor search bar of one editor.
#[derive(Debug, Clone)]
pub struct FindSession {
    text: Vec<char>,
    /// `(anchor, head)` of every selection, the last one is the primary.
    selections: Vec<Range>,
    open: bool,
    replace_mode: bool,
    focus: Focus,
    query: String,
    replacement: String,
    field_selected: bool,
    flags: Flags,
    /// Scroll to results during typing (SRCH-FIL-013).
    follow_typing: bool,
    /// The range searched with search in selection.
    scope: Option<Range>,
    /// The selection of the editor when the bar opened.
    initial_selection: Option<Range>,
    wrapped: bool,
    hint: Option<String>,
    hints: Vec<String>,
    status: Option<String>,
    dialog: Option<(String, String)>,
    undo: Vec<(Vec<char>, Vec<Range>)>,
    excluded: HashSet<Range>,
    history: Vec<String>,
    replace_history: Vec<String>,
    last_action: Option<String>,
    /// Replacements made since the frontend last took them, in the coordinates of the text
    /// before the replacement (non-overlapping).
    pending_edits: Vec<(Range, String)>,
    /// Changes whenever the text does; keys the match cache.
    generation: u64,
    cache: RefCell<Option<MatchCache>>,
}

/// The matches of one (text, query, options, scope) combination.
#[derive(Debug, Clone)]
struct MatchCache {
    key: (u64, String, Flags, Option<Range>),
    matches: Vec<Range>,
}

impl FindSession {
    /// A session over `text` with the carets given as `(anchor, head)`.
    pub fn new(text: &str, selections: Vec<Range>) -> Self {
        Self {
            text: text.chars().collect(),
            selections: if selections.is_empty() {
                vec![(0, 0)]
            } else {
                selections
            },
            open: false,
            replace_mode: false,
            focus: Focus::Editor,
            query: String::new(),
            replacement: String::new(),
            field_selected: false,
            flags: Flags::default(),
            follow_typing: true,
            scope: None,
            initial_selection: None,
            wrapped: false,
            hint: None,
            hints: Vec::new(),
            status: None,
            dialog: None,
            undo: Vec::new(),
            excluded: HashSet::new(),
            history: Vec::new(),
            replace_history: Vec::new(),
            last_action: None,
            pending_edits: Vec::new(),
            generation: 0,
            cache: RefCell::new(None),
        }
    }

    /// The editor's text changed: the session works on `text` from now on, keeping the query,
    /// the options and the history. Undo steps and excluded matches refer to the old text and
    /// are dropped. Costs a copy of the text: call it when the document changed, not per frame.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.chars().collect();
        self.generation += 1;
        self.undo.clear();
        self.excluded.clear();
        self.pending_edits.clear();
        self.wrapped = false;
        let len = self.text.len();
        if let Some((s, e)) = self.scope {
            self.scope = Some((s.min(len), e.min(len)));
        }
        for (anchor, head) in &mut self.selections {
            *anchor = (*anchor).min(len);
            *head = (*head).min(len);
        }
    }

    /// The editor's selections moved (`(anchor, head)` per caret, the primary last).
    pub fn set_selections(&mut self, selections: &[Range]) {
        let selections = if selections.is_empty() {
            vec![(0, 0)]
        } else {
            selections.to_vec()
        };
        if self.selections != selections {
            self.selections = selections;
            self.wrapped = false;
        }
    }

    /// The replacements made since the last call, to be applied to the editor's document in one
    /// step. Offsets refer to the text before the replacements.
    pub fn take_edits(&mut self) -> Vec<(Range, String)> {
        std::mem::take(&mut self.pending_edits)
    }

    /// The text of the replace field.
    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    /// The search field was edited to `query` (as typing does: the search runs again).
    pub fn set_query(&mut self, query: &str) {
        if self.query == query {
            return;
        }
        self.query = query.to_owned();
        self.field_selected = false;
        self.on_query_changed();
    }

    /// The replace field was edited to `replacement`.
    pub fn set_replacement(&mut self, replacement: &str) {
        self.replacement = replacement.to_owned();
        self.field_selected = false;
    }

    pub fn text(&self) -> String {
        chars_to_string(&self.text)
    }

    pub fn selections(&self) -> &[Range] {
        &self.selections
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn replace_mode(&self) -> bool {
        self.replace_mode
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn field_selected(&self) -> bool {
        self.field_selected
    }

    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn set_flags(&mut self, flags: Flags) {
        self.flags = flags;
    }

    pub fn set_follow_typing(&mut self, on: bool) {
        self.follow_typing = on;
    }

    pub fn hint(&self) -> Option<&str> {
        self.hint.as_deref()
    }

    /// Every hint shown so far.
    pub fn hints(&self) -> &[String] {
        &self.hints
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn dialog(&self) -> Option<(&str, &str)> {
        self.dialog.as_ref().map(|(t, m)| (t.as_str(), m.as_str()))
    }

    pub fn last_action_label(&self) -> Option<&str> {
        self.last_action.as_deref()
    }

    /// Search history, newest first (SRCH-FIL-070).
    pub fn history(&self) -> Vec<String> {
        self.history.iter().rev().cloned().collect()
    }

    pub fn exclude_label(&self) -> &'static str {
        match self.current_index_range() {
            Some(r) if self.excluded.contains(&r) => "Include",
            _ => "Exclude",
        }
    }

    fn primary(&self) -> Range {
        *self.selections.last().unwrap_or(&(0, 0))
    }

    fn primary_span(&self) -> Range {
        let (a, h) = self.primary();
        (a.min(h), a.max(h))
    }

    fn set_selection(&mut self, start: usize, end: usize) {
        self.selections = vec![(start, end)];
    }

    fn note_hint(&mut self, hint: Option<String>) {
        if let Some(h) = &hint {
            self.hints.push(h.clone());
        }
        self.hint = hint;
    }

    /// Why the query cannot run, if it cannot.
    pub fn error(&self) -> Option<QueryError> {
        if !self.flags.regex || self.query.is_empty() {
            return None;
        }
        if self.query.chars().all(|c| c == '|') {
            return Some(QueryError::Vague);
        }
        compile(&self.query, self.flags).err()
    }

    /// Whether the query fails only because it uses a Java regex feature.
    pub fn regex_flavor_gap(&self) -> bool {
        matches!(self.error(), Some(QueryError::BadPattern(_))) && java_only_feature(&self.query)
    }

    /// The matches inside the scope of the search (SRCH-FIL-025).
    pub fn matches(&self) -> Vec<Range> {
        let key = (self.generation, self.query.clone(), self.flags, self.scope);
        if let Some(cache) = self.cache.borrow().as_ref()
            && cache.key == key
        {
            return cache.matches.clone();
        }
        let all = find_matches(&self.text, &self.query, self.flags).unwrap_or_default();
        let matches = match (self.flags.in_selection, self.scope) {
            (true, Some((s, e))) => all
                .into_iter()
                .filter(|(ms, me)| *ms >= s && *me <= e)
                .collect(),
            (true, None) => Vec::new(),
            _ => all,
        };
        *self.cache.borrow_mut() = Some(MatchCache {
            key,
            matches: matches.clone(),
        });
        matches
    }

    fn current_index_range(&self) -> Option<Range> {
        let span = self.primary_span();
        self.matches().into_iter().find(|m| *m == span)
    }

    /// The text of the result counter (SRCH-FIL-010).
    pub fn counter(&self) -> String {
        if self.query.is_empty() {
            return "0 results".into();
        }
        match self.error() {
            Some(QueryError::Vague) => return "Vague pattern".into(),
            Some(QueryError::BadPattern(_)) => return "Bad pattern".into(),
            None => {}
        }
        let matches = self.matches();
        if matches.is_empty() {
            return if self.flags.in_selection && self.scope.is_none() {
                "No selection".into()
            } else {
                "0 results".into()
            };
        }
        if matches.len() > MATCH_LIMIT {
            return "10000+".into();
        }
        let span = self.primary_span();
        match matches.iter().position(|m| *m == span) {
            Some(i) => format!("{}/{}", i + 1, matches.len()),
            None if matches.len() == 1 => "1 result".into(),
            None => format!("{} results", matches.len()),
        }
    }

    /// The search field shows the not-found state (SRCH-FIL-011).
    pub fn field_in_error_state(&self) -> bool {
        !self.query.is_empty() && (self.error().is_some() || self.matches().is_empty())
    }

    // ---------------------------------------------------------------- opening

    /// `Ctrl+F` / `Ctrl+R` (SRCH-FIL-002, SRCH-FIL-014).
    pub fn open_bar(&mut self, replace: bool) {
        let was_open = self.open;
        self.open = true;
        self.replace_mode = replace;
        self.focus = Focus::Search;
        self.field_selected = true;
        self.hint = None;
        self.wrapped = false;
        if was_open {
            return;
        }
        let (s, e) = self.primary_span();
        self.initial_selection = (s != e).then_some((s, e));
        if s != e {
            let selected: String = self.text[s..e].iter().collect();
            if selected.contains('\n') {
                self.flags.in_selection = true;
                self.scope = Some((s, e));
                self.query.clear();
            } else if selected.chars().count() <= 10_000 {
                self.query = if self.flags.regex {
                    regex_escape(&selected)
                } else {
                    selected
                };
                self.flags.in_selection = false;
                self.scope = None;
                self.search_from_selection();
            }
        } else if !self.query.is_empty() {
            self.search_from_selection();
        }
    }

    /// `Esc`: closes the bar; the selection stays.
    pub fn close(&mut self) {
        self.open = false;
        self.focus = Focus::Editor;
        self.hint = None;
    }

    /// `Ctrl+Enter` (SRCH-FIL-003).
    pub fn focus_editor(&mut self) {
        if self.query.is_empty() {
            self.close();
        } else {
            self.focus = Focus::Editor;
            self.push_history();
        }
    }

    /// `Tab`: moves between the search and the replace field.
    pub fn tab(&mut self) {
        if self.replace_mode {
            self.focus = if self.focus == Focus::Search {
                Focus::Replace
            } else {
                Focus::Search
            };
            self.field_selected = false;
        }
    }

    // ----------------------------------------------------------------- typing

    /// Typed or pasted characters go to the focused field.
    pub fn type_text(&mut self, typed: &str) {
        match self.focus {
            Focus::Search => {
                if self.field_selected {
                    self.query.clear();
                }
                self.query.push_str(typed);
                self.field_selected = false;
                self.on_query_changed();
            }
            Focus::Replace => {
                if self.field_selected {
                    self.replacement.clear();
                }
                self.replacement.push_str(typed);
                self.field_selected = false;
            }
            Focus::Editor => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focus {
            Focus::Search => {
                if self.field_selected {
                    self.query.clear();
                } else {
                    self.query.pop();
                }
                self.field_selected = false;
                self.on_query_changed();
            }
            Focus::Replace => {
                self.replacement.pop();
            }
            Focus::Editor => {}
        }
    }

    /// `Ctrl+A` in the field.
    pub fn select_field(&mut self) {
        self.field_selected = true;
    }

    fn on_query_changed(&mut self) {
        self.wrapped = false;
        self.hint = None;
        self.excluded.retain(|r| self.text.get(r.0..r.1).is_some());
        self.search_from_selection();
    }

    /// The first search of a query selects the first match at or after the caret (SRCH-FIL-032).
    fn search_from_selection(&mut self) {
        if !self.follow_typing || self.flags.in_selection {
            return;
        }
        let matches = self.matches();
        if matches.is_empty() {
            return;
        }
        let (from, _) = self.primary_span();
        let found = matches
            .iter()
            .find(|m| m.1 >= from)
            .or(matches.first())
            .copied();
        if let Some((s, e)) = found {
            self.set_selection(s, e);
        }
    }

    /// Toggles `Match Case`, `Words`, `Regex` and the like; the search runs again.
    pub fn toggle(&mut self, option: &str) {
        match option {
            "case" => self.flags.match_case = !self.flags.match_case,
            "words" => self.flags.whole_words = !self.flags.whole_words,
            "regex" => self.flags.regex = !self.flags.regex,
            "preserve" => self.flags.preserve_case = !self.flags.preserve_case,
            "selection" => self.toggle_in_selection(),
            _ => {}
        }
        if option != "preserve" && option != "selection" {
            self.on_query_changed();
        }
    }

    fn toggle_in_selection(&mut self) {
        if self.flags.in_selection {
            self.flags.in_selection = false;
            self.scope = None;
            if self.query.is_empty() {
                let (s, e) = self.primary_span();
                if s != e {
                    self.query = self.text[s..e].iter().collect();
                }
            }
        } else {
            self.flags.in_selection = true;
            let (s, e) = self
                .initial_selection
                .unwrap_or_else(|| self.primary_span());
            if let Some((is, ie)) = self.initial_selection {
                self.set_selection(is, ie);
                let initial: String = self.text[is..ie].iter().collect();
                if initial == self.query {
                    self.query.clear();
                }
            }
            self.scope = (s != e).then_some((s, e));
        }
    }

    // -------------------------------------------------------------- navigation

    fn push_history(&mut self) {
        if self.query.is_empty() {
            return;
        }
        self.history.retain(|h| h != &self.query);
        self.history.push(self.query.clone());
        if self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }
    }

    /// `Enter`, `F3`, `Down`, the next button (SRCH-FIL-030, SRCH-FIL-031).
    pub fn find_next(&mut self) {
        self.navigate(true);
    }

    /// `Shift+Enter`, `Shift+F3`.
    pub fn find_previous(&mut self) {
        self.navigate(false);
    }

    fn navigate(&mut self, forward: bool) {
        self.push_history();
        self.hint = None;
        let matches = self.matches();
        if matches.is_empty() || self.error().is_some() {
            return;
        }
        let (s, e) = self.primary_span();
        let caret = self.primary().1;
        let target = if forward {
            matches
                .iter()
                .find(|m| m.0 >= caret.max(if s == e { s } else { e }))
                .copied()
        } else {
            matches.iter().rev().find(|m| m.1 <= s).copied()
        };
        match target {
            Some((ms, me)) => {
                self.wrapped = false;
                self.set_selection(ms, me);
            }
            None if self.wrapped => {
                self.wrapped = false;
                let (ms, me) = if forward {
                    matches[0]
                } else {
                    matches[matches.len() - 1]
                };
                self.set_selection(ms, me);
            }
            None => {
                self.wrapped = true;
                let query = self.query.clone();
                self.note_hint(Some(if forward {
                    format!("\"{query}\" not found, press F3 to search from the top")
                } else {
                    format!("\"{query}\" not found, press Shift+F3 to search from the bottom")
                }));
            }
        }
    }

    /// `Ctrl+F3` / `Ctrl+Shift+F3` (SRCH-FIL-034).
    pub fn find_word_at_caret(&mut self, forward: bool) {
        let (s, e) = self.primary_span();
        let (ws, we) = if s != e {
            (s, e)
        } else {
            let caret = s;
            let mut a = caret;
            while a > 0 && is_identifier_part(self.text[a - 1]) {
                a -= 1;
            }
            let mut b = caret;
            while b < self.text.len() && is_identifier_part(self.text[b]) {
                b += 1;
            }
            (a, b)
        };
        if ws == we {
            return;
        }
        let word: String = self.text[ws..we].iter().collect();
        self.flags.match_case = true;
        self.flags.whole_words = false;
        self.flags.regex = false;
        self.flags.in_selection = false;
        self.scope = None;
        self.query = word;
        self.set_selection(ws, we);
        self.push_history();
        self.navigate(forward);
    }

    // ------------------------------------------------------------------ replace

    fn replacement_for(&self, range: Range) -> Result<String, String> {
        let found: String = self.text[range.0..range.1].iter().collect();
        let mut out = if self.flags.regex {
            let regex = compile(&self.query, self.flags).map_err(|e| match e {
                QueryError::BadPattern(m) => m,
                QueryError::Vague => "Vague pattern".into(),
            })?;
            let haystack = chars_to_string(&self.text);
            let byte = |ci: usize| {
                haystack
                    .char_indices()
                    .nth(ci)
                    .map_or(haystack.len(), |(b, _)| b)
            };
            let (bs, be) = (byte(range.0), byte(range.1));
            let caps = regex
                .captures_iter(&haystack)
                .find(|c| c.get(0).is_some_and(|m| m.start() == bs && m.end() == be))
                .ok_or_else(|| "no match".to_owned())?;
            expand_template(&self.replacement, &regex, &caps)?
        } else {
            self.replacement.clone()
        };
        if self.flags.preserve_case {
            out = preserve_case(&found, &out);
        }
        Ok(out)
    }

    fn apply(&mut self, edits: &[(Range, String)]) {
        self.undo.push((self.text.clone(), self.selections.clone()));
        let mut edits: Vec<&(Range, String)> = edits.iter().collect();
        self.pending_edits
            .extend(edits.iter().map(|edit| (*edit).clone()));
        self.generation += 1;
        edits.sort_by_key(|e| std::cmp::Reverse(e.0.0));
        for ((s, e), repl) in edits {
            let repl: Vec<char> = repl.chars().collect();
            self.text.splice(*s..*e, repl);
        }
    }

    /// `Ctrl+Z`: undoes the last replacement.
    pub fn undo(&mut self) {
        if let Some((text, selections)) = self.undo.pop() {
            self.generation += 1;
            self.pending_edits.clear();
            self.text = text;
            self.selections = selections;
        }
    }

    /// Replace the current match and select the next one (SRCH-FIL-040).
    pub fn replace_current(&mut self) {
        self.dialog = None;
        let Some(range) = self.current_index_range().or_else(|| {
            let from = self.primary().1;
            self.matches().into_iter().find(|m| m.0 >= from)
        }) else {
            return;
        };
        let new = match self.replacement_for(range) {
            Ok(n) => n,
            Err(message) => {
                self.dialog = Some((
                    "Replace Error".into(),
                    format!("Malformed replacement string: {message}"),
                ));
                return;
            }
        };
        self.push_history();
        self.replace_history.push(self.replacement.clone());
        let new_len = new.chars().count();
        self.apply(&[(range, new)]);
        let after = range.0 + new_len;
        let next = self
            .matches()
            .into_iter()
            .find(|m| m.0 >= after)
            .or_else(|| self.matches().first().copied());
        match next {
            Some((s, e)) => self.set_selection(s, e),
            None => self.set_selection(after, after),
        }
    }

    /// Replace All (SRCH-FIL-041): left to right on the original text, one undo step.
    pub fn replace_all(&mut self) {
        self.dialog = None;
        let matches: Vec<Range> = self
            .matches()
            .into_iter()
            .filter(|m| !self.excluded.contains(m))
            .collect();
        if matches.is_empty() {
            return;
        }
        let mut edits = Vec::new();
        for m in &matches {
            match self.replacement_for(*m) {
                Ok(n) => edits.push((*m, n)),
                Err(message) => {
                    self.dialog = Some((
                        "Replace Error".into(),
                        format!("Malformed replacement string: {message}"),
                    ));
                    return;
                }
            }
        }
        self.push_history();
        let caret = self.primary().1;
        self.apply(&edits);
        self.excluded.clear();
        let n = matches.len();
        self.status = Some(if n == 1 {
            "1 occurrence replaced".into()
        } else {
            format!("{n} occurrences replaced")
        });
        self.last_action = Some(format!(
            "Replace all '{}' in file with '{}'",
            self.query, self.replacement
        ));
        let caret = caret.min(self.text.len());
        self.set_selection(caret, caret);
    }

    /// Exclude / Include the current match and move on (SRCH-FIL-042).
    pub fn toggle_exclude(&mut self) {
        if let Some(r) = self.current_index_range() {
            if !self.excluded.remove(&r) {
                self.excluded.insert(r);
            }
            self.navigate(true);
        }
    }

    // --------------------------------------------------------------- multi-caret

    /// `Ctrl+Alt+Shift+J`: a selection on every match, then the bar closes (SRCH-FIL-060).
    pub fn select_all_occurrences(&mut self) {
        let matches = self.matches();
        if matches.is_empty() {
            return;
        }
        self.push_history();
        self.selections = matches;
        self.close();
    }

    /// `Alt+J`: adds the next match to the selections (SRCH-FIL-061).
    pub fn add_next_occurrence(&mut self) {
        let matches = self.matches();
        let last_end = self.primary_span().1;
        if let Some(next) = matches.iter().find(|m| m.0 >= last_end).copied() {
            self.selections.push(next);
        }
    }

    // -------------------------------------------------------------- history, completion

    /// `Alt+Down`: the history popup entries (newest first).
    pub fn show_history(&self) -> Vec<String> {
        self.history()
    }

    /// Chooses a history entry: the field gets it and the search runs.
    pub fn choose_history(&mut self, entry: &str) {
        self.query = entry.to_owned();
        self.field_selected = false;
        self.on_query_changed();
    }

    /// `Ctrl+Space`: the words of the text matching the typed prefix as a camel-hump pattern
    /// (SRCH-FIL-071), ascending.
    pub fn completions(&self) -> Vec<String> {
        let matcher = NameMatcher::new(&self.query, MatchMode::IgnoreCase);
        let mut words: Vec<String> = Vec::new();
        let mut cur = String::new();
        for c in self.text.iter().chain(std::iter::once(&' ')) {
            if is_identifier_part(*c) {
                cur.push(*c);
            } else if !cur.is_empty() {
                if !words.contains(&cur) {
                    words.push(std::mem::take(&mut cur));
                } else {
                    cur.clear();
                }
            }
        }
        // Prefix semantics: the pattern is anchored at the start of the word.
        let mut found: Vec<String> = words
            .into_iter()
            .filter(|w| {
                matcher
                    .match_name(w)
                    .is_some_and(|f| f.first().is_some_and(|r| r.start == 0))
            })
            .collect();
        found.sort();
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_selects_the_first_match_after_the_caret() {
        // SRCH-FIL-032
        let mut s = FindSession::new("foo one\ntwo three\nfoo four", vec![(12, 12)]);
        s.open_bar(false);
        s.type_text("foo");
        assert_eq!(s.counter(), "2/2");
    }

    #[test]
    fn replace_all_does_not_rescan_the_replacement() {
        // SRCH-FIL-041
        let mut s = FindSession::new("aaa", vec![(0, 0)]);
        s.open_bar(true);
        s.type_text("a");
        s.tab();
        s.type_text("aa");
        s.replace_all();
        assert_eq!(s.text(), "aaaaaa");
        assert_eq!(s.status(), Some("3 occurrences replaced"));
    }

    #[test]
    fn templates_follow_java_rules() {
        // SRCH-FIL-043
        let re = Regex::new("(a)(b)").unwrap();
        let caps = re.captures("ab").unwrap();
        assert_eq!(expand_template("$12", &re, &caps).unwrap(), "a2");
        assert_eq!(expand_template("$3", &re, &caps).unwrap_err(), "No group 3");
        assert_eq!(preserve_case("Foo", "bar"), "Bar");
    }

    #[test]
    fn a_frontend_hands_over_text_and_selections_and_takes_the_edits() {
        let mut s = FindSession::new("one two one", vec![(0, 0)]);
        s.open_bar(true);
        s.set_query("one");
        s.set_replacement("1");
        assert_eq!(s.counter(), "1/2");
        assert_eq!(s.replacement(), "1");
        s.replace_current();
        assert_eq!(s.take_edits(), vec![((0, 3), "1".to_owned())]);
        assert!(s.take_edits().is_empty(), "taken once");
        // The editor applied them; the session is told the new text and the caret.
        s.set_text("1 two one");
        s.set_selections(&[(6, 9)]);
        assert_eq!(s.counter(), "1/1");
        s.replace_all();
        assert_eq!(s.take_edits(), vec![((6, 9), "1".to_owned())]);
        assert_eq!(s.status(), Some("1 occurrence replaced"));
    }

    #[test]
    fn matches_are_cached_until_the_text_or_the_query_changes() {
        let mut s = FindSession::new("a a a", vec![(0, 0)]);
        s.set_query("a");
        assert_eq!(s.matches().len(), 3);
        assert_eq!(s.matches().len(), 3);
        s.set_text("a a");
        assert_eq!(s.matches().len(), 2);
        s.toggle("words");
        s.set_query("a a");
        assert_eq!(s.matches().len(), 1);
    }
}
