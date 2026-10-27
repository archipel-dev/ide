//! Logcat: `adb logcat -v threadtime` parsing, the Logcat filter query
//! language, and a streaming session with a pid → process-name map.
//!
//! Filter syntax (subset of the reference IDE's): `key:value` (contains),
//! `key=:value` (exact), `key~:regex`, a leading `-` negates, values may be
//! quoted. Keys: `tag`, `message`, `package` (`package:mine` = the project's
//! application IDs), `process`, `level` (that level and above). Bare words
//! match tag, message or package. Positive terms on the same key are ORed;
//! different keys and negated terms are ANDed.

use crate::adb::Adb;
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use std::io::Read;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Assert,
}

impl LogLevel {
    pub fn from_letter(letter: char) -> Option<Self> {
        Some(match letter {
            'V' => LogLevel::Verbose,
            'D' => LogLevel::Debug,
            'I' => LogLevel::Info,
            'W' => LogLevel::Warn,
            'E' => LogLevel::Error,
            'F' | 'A' => LogLevel::Assert,
            _ => return None,
        })
    }

    /// Parses `level:` filter values: `WARN`, `w`, `error`, `fatal`, …
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "v" | "verbose" => Some(LogLevel::Verbose),
            "d" | "debug" => Some(LogLevel::Debug),
            "i" | "info" => Some(LogLevel::Info),
            "w" | "warn" | "warning" => Some(LogLevel::Warn),
            "e" | "error" => Some(LogLevel::Error),
            "a" | "f" | "assert" | "fatal" => Some(LogLevel::Assert),
            _ => None,
        }
    }

    pub fn letter(self) -> char {
        match self {
            LogLevel::Verbose => 'V',
            LogLevel::Debug => 'D',
            LogLevel::Info => 'I',
            LogLevel::Warn => 'W',
            LogLevel::Error => 'E',
            LogLevel::Assert => 'A',
        }
    }
}

/// One logcat line in `threadtime` format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// `MM-DD HH:MM:SS.mmm` as printed by logcat.
    pub timestamp: String,
    pub pid: u32,
    pub tid: u32,
    pub level: LogLevel,
    pub tag: String,
    pub message: String,
}

/// Parses `10-10 14:01:02.345  1234  1250 I ActivityManager: Start proc …`.
pub fn parse_threadtime(line: &str) -> Option<LogEntry> {
    let line = line.trim_end_matches(['\r', '\n']);
    let mut rest = line.trim_start();
    let mut field = || {
        let trimmed = rest.trim_start();
        let end = trimmed.find(char::is_whitespace)?;
        let (value, tail) = trimmed.split_at(end);
        rest = tail;
        Some(value)
    };
    let date = field()?;
    let time = field()?;
    let pid = field()?.parse().ok()?;
    let tid = field()?.parse().ok()?;
    let level_text = field()?;
    let mut level_chars = level_text.chars();
    let level = LogLevel::from_letter(level_chars.next()?)?;
    if level_chars.next().is_some() || date.len() != 5 || !time.contains(':') {
        return None;
    }
    // Tag runs to the first ": "; tags may contain spaces and colons.
    let body = rest.strip_prefix(' ').unwrap_or(rest);
    let (tag, message) = match body.find(": ") {
        Some(index) => (&body[..index], &body[index + 2..]),
        None => (body.strip_suffix(':').unwrap_or(body), ""),
    };
    Some(LogEntry {
        timestamp: format!("{date} {time}"),
        pid,
        tid,
        level,
        tag: tag.trim_end().to_owned(),
        message: message.to_owned(),
    })
}

/// Turns raw logcat output into entries, buffering partial lines.
#[derive(Debug, Default)]
pub struct LogcatParser {
    partial: Vec<u8>,
}

impl LogcatParser {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<LogEntry> {
        self.partial.extend_from_slice(bytes);
        let Some(last_newline) = self.partial.iter().rposition(|b| *b == b'\n') else {
            return Vec::new();
        };
        let complete: Vec<u8> = self.partial.drain(..=last_newline).collect();
        String::from_utf8_lossy(&complete)
            .lines()
            .filter(|line| !line.starts_with("--------- "))
            .filter_map(parse_threadtime)
            .collect()
    }
}

/// Process started by ActivityManager: `Start proc 1234:com.example/u0a190 …`.
pub fn started_process(entry: &LogEntry) -> Option<(u32, String)> {
    let rest = entry.message.strip_prefix("Start proc ")?;
    let (pid, rest) = rest.split_once(':')?;
    let name: String = rest
        .chars()
        .take_while(|c| *c != '/' && !c.is_whitespace())
        .collect();
    Some((pid.parse().ok()?, name))
}

/// Parses `ps -A -o PID,NAME` output.
pub fn parse_ps(text: &str) -> Vec<(u32, String)> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let name = fields.next()?.to_owned();
            Some((pid, name))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Tag,
    Message,
    Package,
    Level,
    Text,
}

#[derive(Debug, Clone)]
enum Matcher {
    Contains(String),
    Exact(String),
    Regex(Regex),
    Mine,
    AtLeast(LogLevel),
}

#[derive(Debug, Clone)]
struct Term {
    field: Field,
    negated: bool,
    matcher: Matcher,
}

/// What a filter needs to know beyond the entry itself.
pub struct FilterContext<'a> {
    /// Application IDs of the open project (`package:mine`).
    pub mine: &'a [String],
    pub process_name: &'a dyn Fn(u32) -> Option<&'a str>,
}

/// A parsed Logcat filter query.
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    terms: Vec<Term>,
    match_case: bool,
}

fn tokenize(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = query.chars();
    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), '\\') => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            (Some(_), c) => current.push(c),
            (None, '"' | '\'') => quote = Some(ch),
            (None, c) if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            (None, c) => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

impl LogFilter {
    /// Parses a query. Invalid regular expressions and unknown levels are errors.
    pub fn parse(query: &str, match_case: bool) -> Result<Self, String> {
        let mut terms = Vec::new();
        for token in tokenize(query) {
            let (negated, body) = match token.strip_prefix('-') {
                Some(body) if body.contains(':') => (true, body),
                _ => (false, token.as_str()),
            };
            let parsed = body.split_once(':').and_then(|(key, value)| {
                let (key, mode) = if let Some(key) = key.strip_suffix('~') {
                    (key, '~')
                } else if let Some(key) = key.strip_suffix('=') {
                    (key, '=')
                } else {
                    (key, ' ')
                };
                let field = match key {
                    "tag" => Field::Tag,
                    "message" | "msg" => Field::Message,
                    "package" | "process" => Field::Package,
                    "level" => Field::Level,
                    _ => return None,
                };
                Some((field, mode, value.to_owned()))
            });
            let term = match parsed {
                None => Term {
                    field: Field::Text,
                    negated: false,
                    matcher: Matcher::Contains(token.clone()),
                },
                Some((Field::Level, _, value)) => Term {
                    field: Field::Level,
                    negated,
                    matcher: Matcher::AtLeast(
                        LogLevel::from_name(&value)
                            .ok_or_else(|| format!("unknown level `{value}`"))?,
                    ),
                },
                Some((Field::Package, ' ', value)) if value == "mine" => Term {
                    field: Field::Package,
                    negated,
                    matcher: Matcher::Mine,
                },
                Some((field, '~', value)) => Term {
                    field,
                    negated,
                    matcher: Matcher::Regex(
                        RegexBuilder::new(&value)
                            .case_insensitive(!match_case)
                            .size_limit(1 << 20)
                            .build()
                            .map_err(|e| format!("invalid regex `{value}`: {e}"))?,
                    ),
                },
                Some((field, '=', value)) => Term {
                    field,
                    negated,
                    matcher: Matcher::Exact(value),
                },
                Some((field, _, value)) => Term {
                    field,
                    negated,
                    matcher: Matcher::Contains(value),
                },
            };
            terms.push(term);
        }
        Ok(Self { terms, match_case })
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Whether the query refers to process names (results change as the
    /// pid → name map fills in).
    pub fn uses_packages(&self) -> bool {
        self.terms
            .iter()
            .any(|t| matches!(t.field, Field::Package | Field::Text))
    }

    fn text_matches(&self, matcher: &Matcher, text: &str) -> bool {
        match matcher {
            Matcher::Contains(needle) if self.match_case => text.contains(needle.as_str()),
            Matcher::Contains(needle) => text.to_lowercase().contains(&needle.to_lowercase()),
            Matcher::Exact(value) if self.match_case => text == value,
            Matcher::Exact(value) => text.eq_ignore_ascii_case(value),
            Matcher::Regex(regex) => regex.is_match(text),
            Matcher::Mine | Matcher::AtLeast(_) => false,
        }
    }

    fn term_matches(&self, term: &Term, entry: &LogEntry, ctx: &FilterContext) -> bool {
        let package = (ctx.process_name)(entry.pid);
        match (&term.field, &term.matcher) {
            (Field::Level, Matcher::AtLeast(level)) => entry.level >= *level,
            (Field::Package, Matcher::Mine) => package.is_some_and(|name| {
                ctx.mine.iter().any(|mine| {
                    name == mine
                        || name
                            .strip_prefix(mine.as_str())
                            .is_some_and(|rest| rest.starts_with(':'))
                })
            }),
            (Field::Package, matcher) => {
                package.is_some_and(|name| self.text_matches(matcher, name))
            }
            (Field::Tag, matcher) => self.text_matches(matcher, &entry.tag),
            (Field::Message, matcher) => self.text_matches(matcher, &entry.message),
            (Field::Text, matcher) => {
                self.text_matches(matcher, &entry.tag)
                    || self.text_matches(matcher, &entry.message)
                    || package.is_some_and(|name| self.text_matches(matcher, name))
            }
            _ => false,
        }
    }

    pub fn matches(&self, entry: &LogEntry, ctx: &FilterContext) -> bool {
        let mut groups: Vec<(Field, bool)> = Vec::new();
        for term in &self.terms {
            let hit = self.term_matches(term, entry, ctx);
            if term.negated {
                if hit {
                    return false;
                }
                continue;
            }
            match groups.iter_mut().find(|(field, _)| *field == term.field) {
                Some((_, any)) => *any |= hit,
                None => groups.push((term.field, hit)),
            }
        }
        groups.iter().all(|(_, any)| *any)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogcatEvent {
    Entries(Vec<LogEntry>),
    /// A full `ps` snapshot of the device's processes.
    Processes(Vec<(u32, String)>),
    Ended(String),
}

/// A running `adb -s <serial> logcat -v threadtime`. Dropping it stops logcat.
pub struct LogcatSession {
    pub serial: String,
    child: Arc<Mutex<Child>>,
    stop: Arc<AtomicBool>,
    events: crate::Inbox<LogcatEvent>,
}

impl LogcatSession {
    pub fn start(adb: &Adb, serial: &str) -> Result<Self, String> {
        let mut child = crate::adb::spawn(
            std::process::Command::new(&adb.program)
                .args(["-s", serial, "logcat", "-v", "threadtime"])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .map_err(|e| format!("cannot run {}: {e}", adb.program.display()))?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child = Arc::new(Mutex::new(child));
        let stop = Arc::new(AtomicBool::new(false));
        let (sender, events) = mpsc::channel();
        {
            let sender = sender.clone();
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("archipel-ide-logcat".into())
                .spawn(move || read_logcat(stdout, stderr, sender, stop))
                .map_err(|e| e.to_string())?;
        }
        {
            let adb = adb.clone();
            let serial = serial.to_owned();
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("archipel-ide-logcat-ps".into())
                .spawn(move || poll_processes(adb, serial, sender, stop))
                .map_err(|e| e.to_string())?;
        }
        Ok(Self {
            serial: serial.to_owned(),
            child,
            stop,
            events: crate::Inbox::new(events),
        })
    }

    pub fn poll(&self) -> Vec<LogcatEvent> {
        self.events.try_iter()
    }

    pub fn wait(&self, timeout: Duration) -> Option<LogcatEvent> {
        self.events.recv_timeout(timeout).ok()
    }
}

impl Drop for LogcatSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn read_logcat(
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
    sender: Sender<LogcatEvent>,
    stop: Arc<AtomicBool>,
) {
    let mut parser = LogcatParser::default();
    if let Some(mut stdout) = stdout {
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let read = match stdout.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => read,
            };
            let entries = parser.push(&buffer[..read]);
            if !entries.is_empty() && sender.send(LogcatEvent::Entries(entries)).is_err() {
                return;
            }
        }
    }
    if stop.load(Ordering::Relaxed) {
        return;
    }
    let mut message = String::new();
    if let Some(mut stderr) = stderr {
        let _ = stderr.read_to_string(&mut message);
    }
    let message = message.trim();
    let _ = sender.send(LogcatEvent::Ended(if message.is_empty() {
        "logcat ended (device disconnected?)".into()
    } else {
        message.to_owned()
    }));
}

fn poll_processes(adb: Adb, serial: String, sender: Sender<LogcatEvent>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        if let Ok(text) = adb.run(&["-s", &serial, "shell", "ps", "-A", "-o", "PID,NAME"]) {
            let processes = parse_ps(&text);
            if !processes.is_empty() && sender.send(LogcatEvent::Processes(processes)).is_err() {
                return;
            }
        }
        for _ in 0..20 {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const SAMPLE: &str = "--------- beginning of main\n\
10-10 14:01:02.345  1234  1250 I ActivityManager: Start proc 4321:com.example.myapplication/u0a190 for next-top-activity {com.example.myapplication/com.example.myapplication.MainActivity}\n\
10-10 14:01:02.500  4321  4321 D MainActivity: onCreate\n\
10-10 14:01:02.501  4321  4330 W OpenGLRenderer: Failed to choose config with EGL_SWAP_BEHAVIOR_PRESERVED\n\
10-10 14:01:03.000  4321  4321 E AndroidRuntime: FATAL EXCEPTION: main\n\
10-10 14:01:03.001  4321  4321 E AndroidRuntime: Process: com.example.myapplication, PID: 4321\n\
10-10 14:01:03.100   777   777 V chatty  : uid=1000(system) identical 2 lines\n\
10-10 14:01:03.200  5555  5555 I Tag With Spaces: a: b\n\
garbage line\n";

    fn entries() -> Vec<LogEntry> {
        LogcatParser::default().push(SAMPLE.as_bytes())
    }

    #[test]
    fn parses_threadtime_lines() {
        let entries = entries();
        assert_eq!(entries.len(), 7);
        assert_eq!(entries[0].timestamp, "10-10 14:01:02.345");
        assert_eq!((entries[0].pid, entries[0].tid), (1234, 1250));
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[0].tag, "ActivityManager");
        assert_eq!(entries[5].tag, "chatty");
        assert_eq!(entries[6].tag, "Tag With Spaces");
        assert_eq!(entries[6].message, "a: b");
        assert_eq!(
            started_process(&entries[0]),
            Some((4321, "com.example.myapplication".into()))
        );
    }

    #[test]
    fn parser_buffers_partial_lines() {
        let mut parser = LogcatParser::default();
        let (a, b) = SAMPLE.as_bytes().split_at(200);
        let mut all = parser.push(a);
        all.extend(parser.push(b));
        assert_eq!(all, entries());
        assert!(
            parser
                .push(b"10-10 14:01:02.345  1 1 I x: no newline")
                .is_empty()
        );
    }

    fn filtered(query: &str, match_case: bool) -> Vec<String> {
        let names: HashMap<u32, &str> = HashMap::from([
            (1234, "system_server"),
            (4321, "com.example.myapplication"),
            (5555, "com.example.myapplication:remote"),
        ]);
        let lookup = |pid: u32| names.get(&pid).copied();
        let mine = vec!["com.example.myapplication".to_owned()];
        let ctx = FilterContext {
            mine: &mine,
            process_name: &lookup,
        };
        let filter = LogFilter::parse(query, match_case).unwrap();
        entries()
            .into_iter()
            .filter(|e| filter.matches(e, &ctx))
            .map(|e| format!("{}/{}", e.level.letter(), e.tag))
            .collect()
    }

    #[test]
    fn filter_query_language() {
        assert_eq!(filtered("", false).len(), 7);
        assert_eq!(
            filtered("package:mine", false),
            [
                "D/MainActivity",
                "W/OpenGLRenderer",
                "E/AndroidRuntime",
                "E/AndroidRuntime",
                "I/Tag With Spaces"
            ]
        );
        assert_eq!(
            filtered("package:mine level:warn", false),
            ["W/OpenGLRenderer", "E/AndroidRuntime", "E/AndroidRuntime"]
        );
        assert_eq!(
            filtered("tag:MainActivity tag:chatty", false),
            ["D/MainActivity", "V/chatty"]
        );
        assert_eq!(filtered("tag=:mainactivity", false), ["D/MainActivity"]);
        assert!(filtered("tag=:mainactivity", true).is_empty());
        assert_eq!(
            filtered("-tag:AndroidRuntime level:E", false),
            Vec::<String>::new()
        );
        assert_eq!(
            filtered("message~:\"^Process: .*PID\"", false),
            ["E/AndroidRuntime"]
        );
        assert_eq!(filtered("\"Tag With\"", false), ["I/Tag With Spaces"]);
        assert_eq!(filtered("process:system", false), ["I/ActivityManager"]);
        assert_eq!(filtered("fatal", false), ["E/AndroidRuntime"]);
        assert_eq!(filtered("FATAL", true), ["E/AndroidRuntime"]);
        assert_eq!(
            filtered("is:crash", false),
            Vec::<String>::new(),
            "unknown keys are text"
        );
    }

    #[test]
    fn filter_errors_are_reported() {
        assert!(
            LogFilter::parse("message~:(", false)
                .unwrap_err()
                .contains("invalid regex")
        );
        assert!(
            LogFilter::parse("level:loud", false)
                .unwrap_err()
                .contains("unknown level")
        );
    }

    #[test]
    fn parses_ps_output() {
        let ps = parse_ps("  PID NAME\n    1 init\n 4321 com.example.myapplication\n");
        assert_eq!(
            ps,
            [
                (1, "init".into()),
                (4321, "com.example.myapplication".into())
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn streams_logcat_from_fake_adb() {
        use std::os::unix::fs::PermissionsExt;
        let dir = crate::adb::tests::temp("logcat");
        let script = dir.join("adb");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\ncase \"$*\" in\n  \"-s emulator-5554 logcat -v threadtime\") printf '%s' '{}'; exit 0 ;;\n  \"-s emulator-5554 shell ps -A -o PID,NAME\") printf '  PID NAME\\n 4321 com.example.myapplication\\n' ;;\n  *) exit 1 ;;\nesac\n",
                SAMPLE
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let session = LogcatSession::start(&Adb::new(&script), "emulator-5554").unwrap();
        let mut count = 0;
        let mut processes = false;
        let mut ended = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while (!ended || !processes) && std::time::Instant::now() < deadline {
            match session.wait(Duration::from_millis(200)) {
                Some(LogcatEvent::Entries(entries)) => count += entries.len(),
                Some(LogcatEvent::Processes(list)) => {
                    processes = list.iter().any(|(pid, _)| *pid == 4321)
                }
                Some(LogcatEvent::Ended(_)) => ended = true,
                None => {}
            }
        }
        assert_eq!(count, 7);
        assert!(processes && ended);
        drop(session);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
