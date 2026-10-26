//! Structured build events recognised in console output.
//!
//! The Build tool window shows a tree of executed steps (Gradle tasks, Cargo crates,
//! Maven mojos) with durations, and compiler messages that navigate to source. The
//! parser is deliberately textual: it understands the stable, documented console
//! formats of the supported tools and ignores everything else.

use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildMessage {
    pub kind: MessageKind,
    pub text: String,
    /// Source location as printed by the tool (absolute or relative to the task cwd).
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    /// Index of the console line the message was read from.
    pub console_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Running,
    Done,
    UpToDate,
    NoSource,
    FromCache,
    Skipped,
    Failed,
}

impl StepOutcome {
    pub fn label(self) -> Option<&'static str> {
        match self {
            StepOutcome::UpToDate => Some("UP-TO-DATE"),
            StepOutcome::NoSource => Some("NO-SOURCE"),
            StepOutcome::FromCache => Some("FROM-CACHE"),
            StepOutcome::Skipped => Some("SKIPPED"),
            StepOutcome::Failed => Some("FAILED"),
            StepOutcome::Running | StepOutcome::Done => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildStep {
    pub name: String,
    pub started: Duration,
    pub finished: Option<Duration>,
    pub outcome: StepOutcome,
    /// Indices into [`BuildEvents::messages`].
    pub messages: Vec<usize>,
    pub console_line: usize,
}

impl BuildStep {
    pub fn duration(&self) -> Option<Duration> {
        self.finished
            .map(|finished| finished.saturating_sub(self.started))
    }
}

#[derive(Debug, Default, Clone)]
pub struct BuildEvents {
    steps: Vec<BuildStep>,
    messages: Vec<BuildMessage>,
    summary: Option<String>,
    /// Messages seen before any step started.
    orphan_messages: Vec<usize>,
    /// Arrival time of each message.
    arrivals: Vec<Duration>,
    expect_gradle_failure: bool,
}

impl BuildEvents {
    pub fn steps(&self) -> &[BuildStep] {
        &self.steps
    }

    pub fn messages(&self) -> &[BuildMessage] {
        &self.messages
    }

    /// Messages that do not belong to a recognised step.
    pub fn orphan_messages(&self) -> &[usize] {
        &self.orphan_messages
    }

    /// Final summary line reported by the tool, e.g. `BUILD SUCCESSFUL in 4s`.
    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }

    pub fn count(&self, kind: MessageKind) -> usize {
        self.messages
            .iter()
            .filter(|message| message.kind == kind)
            .count()
    }

    /// Feeds one complete console line received `at` after the task started.
    pub fn on_line(&mut self, text: &str, at: Duration, index: usize) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.expect_gradle_failure {
            self.expect_gradle_failure = false;
            self.push_message(MessageKind::Error, trimmed.into(), None, at, index);
            return;
        }
        if trimmed == "* What went wrong:" {
            self.expect_gradle_failure = true;
            return;
        }
        if let Some((name, outcome)) = step_start(trimmed) {
            self.finish_running(at);
            // stdout and stderr are separate pipes: a compiler message read in the same
            // poll as the step header belongs to that step even if it arrived first.
            let adopted: Vec<usize> = if self.steps.is_empty() {
                let (same_poll, earlier) = self
                    .orphan_messages
                    .iter()
                    .partition(|&&id| self.arrivals[id] == at);
                self.orphan_messages = earlier;
                same_poll
            } else {
                Vec::new()
            };
            let failed = adopted
                .iter()
                .any(|&id| self.messages[id].kind == MessageKind::Error);
            self.steps.push(BuildStep {
                name,
                started: at,
                finished: (outcome != StepOutcome::Running).then_some(at),
                outcome: if failed && outcome == StepOutcome::Running {
                    StepOutcome::Failed
                } else {
                    outcome
                },
                messages: adopted,
                console_line: index,
            });
            return;
        }
        if is_summary(trimmed) {
            self.finish_running(at);
            self.summary = Some(trimmed.to_owned());
            return;
        }
        if let Some(location) = trimmed.strip_prefix("--> ") {
            // rustc prints the location on a line after the message.
            if let Some(message) = self.messages.last_mut()
                && message.file.is_none()
                && let Some((file, line, column)) = split_location(location)
            {
                message.file = Some(file.into());
                message.line = Some(line);
                message.column = column;
            }
            return;
        }
        if let Some((kind, text, location)) = message(trimmed) {
            self.push_message(kind, text, location, at, index);
        }
    }

    fn push_message(
        &mut self,
        kind: MessageKind,
        text: String,
        location: Option<(String, u32, Option<u32>)>,
        at: Duration,
        index: usize,
    ) {
        let (file, line, column) = match location {
            Some((file, line, column)) => (Some(PathBuf::from(file)), Some(line), column),
            None => (None, None, None),
        };
        // Gradle repeats compiler errors in its failure summary; keep one node each.
        if self.messages.iter().any(|existing| {
            existing.kind == kind
                && existing.text == text
                && existing.file == file
                && existing.line == line
                && existing.column == column
        }) {
            return;
        }
        let id = self.messages.len();
        self.messages.push(BuildMessage {
            kind,
            text,
            file,
            line,
            column,
            console_line: index,
        });
        self.arrivals.push(at);
        match self.steps.last_mut() {
            Some(step) => {
                step.messages.push(id);
                if kind == MessageKind::Error && step.outcome == StepOutcome::Running {
                    step.outcome = StepOutcome::Failed;
                }
            }
            None => self.orphan_messages.push(id),
        }
    }

    fn finish_running(&mut self, at: Duration) {
        if let Some(step) = self.steps.last_mut()
            && step.finished.is_none()
        {
            step.finished = Some(at);
            if step.outcome == StepOutcome::Running {
                step.outcome = StepOutcome::Done;
            }
        }
    }

    /// Closes the last step when the process ends.
    pub fn finish(&mut self, at: Duration, success: bool) {
        if let Some(step) = self.steps.last_mut()
            && step.finished.is_none()
        {
            step.finished = Some(at);
            step.outcome = if success || step.outcome == StepOutcome::Failed {
                if step.outcome == StepOutcome::Running {
                    StepOutcome::Done
                } else {
                    step.outcome
                }
            } else {
                StepOutcome::Failed
            };
        }
    }
}

fn step_start(line: &str) -> Option<(String, StepOutcome)> {
    // Gradle: `> Task :app:compileJava UP-TO-DATE`
    if let Some(rest) = line.strip_prefix("> Task ") {
        let mut parts = rest.split_whitespace();
        let name = parts.next()?.to_owned();
        let outcome = match parts.next() {
            Some("UP-TO-DATE") => StepOutcome::UpToDate,
            Some("NO-SOURCE") => StepOutcome::NoSource,
            Some("FROM-CACHE") => StepOutcome::FromCache,
            Some("SKIPPED") => StepOutcome::Skipped,
            Some("FAILED") => StepOutcome::Failed,
            _ => StepOutcome::Running,
        };
        return Some((name, outcome));
    }
    // Cargo: `Compiling foo v0.1.0 (/path)`
    for verb in [
        "Compiling",
        "Checking",
        "Documenting",
        "Running",
        "Doc-tests",
    ] {
        if let Some(rest) = line.strip_prefix(verb)
            && rest.starts_with(' ')
        {
            let subject = rest.trim();
            let subject = subject
                .split_once(" (")
                .map_or(subject, |(head, _)| head)
                .trim_matches('`');
            return Some((format!("{verb} {subject}"), StepOutcome::Running));
        }
    }
    // Maven: `[INFO] --- compiler:3.13.0:compile (default-compile) @ app ---`
    if let Some(rest) = line.strip_prefix("[INFO] --- ")
        && let Some(rest) = rest.strip_suffix(" ---")
    {
        let (goal, module) = rest.split_once(" @ ")?;
        let goal = goal.split_whitespace().next()?;
        let mut parts = goal.split(':');
        let plugin = parts.next()?;
        let goal = parts.next_back().unwrap_or(plugin);
        return Some((format!("{plugin}:{goal} @ {module}"), StepOutcome::Running));
    }
    None
}

fn is_summary(line: &str) -> bool {
    line.starts_with("BUILD SUCCESSFUL")
        || line.starts_with("BUILD FAILED")
        || line.starts_with("[INFO] BUILD SUCCESS")
        || line.starts_with("[INFO] BUILD FAILURE")
        || line.starts_with("INFO: Build completed")
        || line.starts_with("FAILED: Build did NOT complete")
        || (line.starts_with("Finished ") && line.contains(" target(s) in "))
        || line.starts_with("Finished `")
}

type Location = (String, u32, Option<u32>);

fn message(line: &str) -> Option<(MessageKind, String, Option<Location>)> {
    // Kotlin: `e: file:///p/A.kt:12:5 Unresolved reference` / `w: …`
    for (prefix, kind) in [("e: ", MessageKind::Error), ("w: ", MessageKind::Warning)] {
        if let Some(rest) = line.strip_prefix(prefix) {
            let rest = rest.strip_prefix("file://").unwrap_or(rest);
            if let Some((location, text)) = rest.split_once(' ')
                && let Some(location) = split_location(location.trim_end_matches(':'))
            {
                return Some((kind, text.trim().to_owned(), Some(location)));
            }
            return Some((kind, rest.trim().to_owned(), None));
        }
    }
    // Maven: `[ERROR] /p/App.java:[12,5] cannot find symbol`
    for (prefix, kind) in [
        ("[ERROR] ", MessageKind::Error),
        ("[WARNING] ", MessageKind::Warning),
    ] {
        if let Some(rest) = line.strip_prefix(prefix) {
            if let Some((file, rest)) = rest.split_once(":[")
                && let Some((position, text)) = rest.split_once("] ")
            {
                let mut numbers = position.split(',').map(|n| n.trim().parse::<u32>().ok());
                if let Some(Some(line_no)) = numbers.next() {
                    let column = numbers.next().flatten();
                    return Some((
                        kind,
                        text.trim().into(),
                        Some((file.into(), line_no, column)),
                    ));
                }
            }
            if rest.trim().is_empty() || rest.starts_with("-> [Help") || rest.starts_with("Re-run")
            {
                return None;
            }
            return Some((kind, rest.trim().into(), None));
        }
    }
    // Bazel: `ERROR: /p/BUILD:3:11: message` / `WARNING: …`
    for (prefix, kind) in [
        ("ERROR: ", MessageKind::Error),
        ("WARNING: ", MessageKind::Warning),
    ] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(match located(rest) {
                Some((location, text)) => (kind, text, Some(location)),
                None => (kind, rest.trim().into(), None),
            });
        }
    }
    // rustc / cargo: `error[E0308]: mismatched types`, `warning: unused variable`
    for (prefix, kind) in [
        ("error", MessageKind::Error),
        ("warning", MessageKind::Warning),
    ] {
        if let Some(rest) = line.strip_prefix(prefix) {
            let rest = match rest.strip_prefix('[') {
                Some(code) => code.split_once(']').map(|(_, rest)| rest)?,
                None => rest,
            };
            if let Some(text) = rest.strip_prefix(": ") {
                if kind == MessageKind::Warning && text.contains(" generated ") {
                    return None;
                }
                return Some((kind, text.trim().into(), None));
            }
        }
    }
    // javac / go / gcc style: `path/File.java:12: error: text`, `./main.go:5:2: text`
    if let Some((location, text)) = located(line) {
        let (kind, text) = if let Some(text) = text.strip_prefix("error: ") {
            (MessageKind::Error, text.to_owned())
        } else if let Some(text) = text.strip_prefix("warning: ") {
            (MessageKind::Warning, text.to_owned())
        } else if location.0.ends_with(".go") {
            (MessageKind::Error, text)
        } else {
            return None;
        };
        return Some((kind, text, Some(location)));
    }
    None
}

/// Splits `path:line[:column]: text`, requiring the path to look like a source file.
fn located(line: &str) -> Option<(Location, String)> {
    let (location, text) = line.split_once(": ")?;
    let location = split_location(location)?;
    Some((location, text.trim().to_owned()))
}

/// Parses `path:line[:column]`; the path must name a file (extension or BUILD file).
fn split_location(location: &str) -> Option<Location> {
    let (head, last) = location.rsplit_once(':')?;
    let last: u32 = last.parse().ok()?;
    let (path, line, column) = match head.rsplit_once(':') {
        Some((path, middle)) if middle.parse::<u32>().is_ok() => {
            (path, middle.parse::<u32>().ok()?, Some(last))
        }
        _ => (head, last, None),
    };
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let looks_like_file = file_name.contains('.') || file_name.starts_with("BUILD");
    (looks_like_file && !path.is_empty() && !path.contains(' ') && line > 0)
        .then(|| (path.to_owned(), line, column))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(lines: &[&str]) -> BuildEvents {
        let mut events = BuildEvents::default();
        for (index, line) in lines.iter().enumerate() {
            events.on_line(line, Duration::from_millis(index as u64 * 100), index);
        }
        events
    }

    #[test]
    fn gradle_tasks_get_outcomes_and_durations() {
        let mut events = feed(&[
            "> Task :compileJava UP-TO-DATE",
            "> Task :processResources NO-SOURCE",
            "> Task :test",
            "> Task :app:run",
            "Hello, Gradle!",
        ]);
        events.finish(Duration::from_millis(900), true);
        let steps = events.steps();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0].name, ":compileJava");
        assert_eq!(steps[0].outcome, StepOutcome::UpToDate);
        assert_eq!(steps[2].outcome, StepOutcome::Done);
        assert_eq!(steps[2].duration(), Some(Duration::from_millis(100)));
        assert_eq!(steps[3].duration(), Some(Duration::from_millis(600)));
    }

    #[test]
    fn javac_errors_and_gradle_failure_reason() {
        let events = feed(&[
            "> Task :compileJava",
            "/w/src/main/java/com/example/App.java:12: error: cannot find symbol",
            "/w/src/main/java/com/example/App.java:3: warning: [deprecation] old",
            "* What went wrong:",
            "Execution failed for task ':compileJava'.",
            "/w/src/main/java/com/example/App.java:12: error: cannot find symbol",
            "BUILD FAILED in 2s",
        ]);
        let messages = events.messages();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].kind, MessageKind::Error);
        assert_eq!(messages[0].text, "cannot find symbol");
        assert_eq!(
            messages[0].file.as_deref(),
            Some(std::path::Path::new(
                "/w/src/main/java/com/example/App.java"
            ))
        );
        assert_eq!(messages[0].line, Some(12));
        assert_eq!(messages[1].kind, MessageKind::Warning);
        assert_eq!(
            messages[2].text,
            "Execution failed for task ':compileJava'."
        );
        assert_eq!(events.steps()[0].outcome, StepOutcome::Failed);
        assert_eq!(events.summary(), Some("BUILD FAILED in 2s"));
    }

    #[test]
    fn stderr_message_read_before_its_step_header_is_adopted() {
        let mut events = BuildEvents::default();
        let at = Duration::from_millis(40);
        events.on_line("Earlier.java:1: error: old", Duration::ZERO, 0);
        events.on_line("/w/App.java:14: error: cannot find symbol", at, 1);
        events.on_line("> Task :compileJava", at, 2);
        assert_eq!(events.orphan_messages(), [0]);
        assert_eq!(events.steps()[0].messages, [1]);
        assert_eq!(events.steps()[0].outcome, StepOutcome::Failed);
    }

    #[test]
    fn kotlin_messages_with_file_urls() {
        let events = feed(&[
            "> Task :app:compileDebugKotlin",
            "e: file:///w/app/src/main/java/Main.kt:12:5 Unresolved reference 'x'.",
            "w: /w/app/Other.kt:3:1 Deprecated",
        ]);
        let messages = events.messages();
        assert_eq!(messages[0].text, "Unresolved reference 'x'.");
        assert_eq!(messages[0].line, Some(12));
        assert_eq!(messages[0].column, Some(5));
        assert_eq!(messages[1].kind, MessageKind::Warning);
        assert_eq!(
            messages[1].file.as_deref(),
            Some(std::path::Path::new("/w/app/Other.kt"))
        );
    }

    #[test]
    fn cargo_crates_and_rustc_locations() {
        let mut events = feed(&[
            "   Compiling demo v0.1.0 (/w/demo)",
            "warning: unused variable: `x`",
            "  --> src/main.rs:2:9",
            "error[E0308]: mismatched types",
            " --> src/lib.rs:7:5",
            "warning: `demo` (bin \"demo\") generated 1 warning",
            "error: could not compile `demo` (bin \"demo\") due to 1 previous error",
        ]);
        events.finish(Duration::from_secs(1), false);
        assert_eq!(events.steps()[0].name, "Compiling demo v0.1.0");
        assert_eq!(events.steps()[0].outcome, StepOutcome::Failed);
        let messages = events.messages();
        assert_eq!(messages.len(), 3);
        assert_eq!(
            messages[0].file.as_deref(),
            Some(std::path::Path::new("src/main.rs"))
        );
        assert_eq!((messages[0].line, messages[0].column), (Some(2), Some(9)));
        assert_eq!(messages[1].text, "mismatched types");
        assert_eq!(messages[1].line, Some(7));
        assert_eq!(events.count(MessageKind::Error), 2);
    }

    #[test]
    fn maven_and_bazel_and_go_formats() {
        let events = feed(&[
            "[INFO] --- compiler:3.13.0:compile (default-compile) @ app ---",
            "[ERROR] /w/src/main/java/App.java:[12,5] cannot find symbol",
            "ERROR: /w/BUILD.bazel:3:11: Compiling Java failed",
            "./main.go:5:2: undefined: x",
            "INFO: Build completed successfully, 3 total actions",
        ]);
        assert_eq!(events.steps()[0].name, "compiler:compile @ app");
        let messages = events.messages();
        assert_eq!((messages[0].line, messages[0].column), (Some(12), Some(5)));
        assert_eq!(
            messages[1].file.as_deref(),
            Some(std::path::Path::new("/w/BUILD.bazel"))
        );
        assert_eq!(messages[1].text, "Compiling Java failed");
        assert_eq!(
            messages[2].file.as_deref(),
            Some(std::path::Path::new("./main.go"))
        );
        assert_eq!(
            events.summary(),
            Some("INFO: Build completed successfully, 3 total actions")
        );
    }

    #[test]
    fn ordinary_program_output_is_not_a_message() {
        let events = feed(&[
            "Hello, Java!",
            "Time: 12:30: lunch",
            "http://localhost:8080: listening",
            "errors are fine",
        ]);
        assert!(events.messages().is_empty());
        assert!(events.steps().is_empty());
    }
}
