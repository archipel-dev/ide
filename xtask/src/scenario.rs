//! `cargo xtask scenario`: replays recorded IDE behavior scenarios against the real
//! workbench and reports how close it is to IntelliJ IDEA.
//!
//! A scenario (`docs/parity/scenarios/<area>/<name>/scenario.json` plus a small
//! `project/` fixture) types real keystrokes into a freshly started IDE over the control
//! socket, observes what the editor shows (markup, Alt+Enter items, completion rows,
//! navigation targets, file text) and compares it with what IntelliJ shows. Expected
//! results are recorded by hand in IntelliJ and committed as text only. The format is
//! described in `docs/parity/scenarios/README.md`.
//!
//! The runner reuses the scenes infrastructure (`scenes.rs`): Xvfb, the glow renderer, one
//! IDE process per scenario with its own cache, and the control socket. It never injects
//! results: input is keys and text, reads are queries.

use crate::scenes::{self, AppProcess, Display, Step, build_app, connect, git_init, send};
use anyhow::{Context, Result, anyhow, bail};
use archipel_ide_control::{ControlClient, Request, WaitCondition};
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SCENARIOS_DIR: &str = "docs/parity/scenarios";
const DEFAULT_REPORT: &str = "target/scenarios/report.md";
const DEFAULT_WORK: &str = "target/scenarios/work";
/// How long a step that waits for the IDE may take unless the step says otherwise.
const DEFAULT_WAIT: Duration = Duration::from_secs(120);

// ---------------------------------------------------------------------------
// Format
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScenarioSpec {
    pub title: String,
    /// Toolchain component ids (`jdk`, `jdtls`, `gradle`, `maven`, `kotlin-lsp`,
    /// `rust-analyzer`, `bazelisk`) or `network`. The scenario is `skipped` when one is
    /// missing.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Project-relative files opened on the command line.
    #[serde(default)]
    pub open: OneOrMany,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub steps: Vec<Value>,
    #[serde(default)]
    pub expect: Expect,
    /// How the expected results were observed in IntelliJ.
    pub intellij: IntellijNote,
    /// Why the scenario is expected to differ today; shown in the report.
    #[serde(default)]
    pub known_gap: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn paths(&self) -> Vec<String> {
        match self {
            OneOrMany::None => Vec::new(),
            OneOrMany::One(path) => vec![path.clone()],
            OneOrMany::Many(paths) => paths.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntellijNote {
    /// Product and version the result was observed in (`IntelliJ IDEA 2025.2 Ultimate`).
    pub version: String,
    /// The action that produced it (`Alt+Enter on the unresolved call`).
    pub action: String,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Expect {
    #[serde(default)]
    pub markup: Vec<MarkupExpect>,
    #[serde(default)]
    pub no_markup: Vec<NoMarkupExpect>,
    #[serde(default)]
    pub intentions: Vec<IntentionsExpect>,
    #[serde(default)]
    pub file: Vec<FileExpect>,
    #[serde(default)]
    pub navigation: Vec<NavigationExpect>,
    #[serde(default)]
    pub usages: Vec<UsagesExpect>,
    #[serde(default)]
    pub completion: Vec<CompletionExpect>,
    #[serde(default)]
    pub state: Vec<StateExpect>,
    #[serde(default)]
    pub query: Vec<QueryExpect>,
}

/// Markup at a range: `range` is `line:column-line:column` (1-based, end exclusive, the
/// positions of the status bar) or `line:column-column` on one line.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkupExpect {
    #[serde(default)]
    pub when: Option<String>,
    pub range: String,
    /// `error`, `warning`, `weak_warning` (or `hint`).
    pub severity: String,
    /// IntelliJ's message; without it only range and severity are compared.
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
}

/// No markup of `severity` (default `error`) anywhere in the file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoMarkupExpect {
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentionsExpect {
    #[serde(default)]
    pub when: Option<String>,
    /// `contains` (default): every item is in the list; `ordered`: the items appear in this
    /// order (not necessarily adjacent); `exact`: the list is exactly these items.
    #[serde(default)]
    pub mode: Option<String>,
    /// Item texts; `re:<regex>` matches by regular expression.
    pub items: Vec<String>,
    #[serde(default)]
    pub confidence: Option<String>,
}

/// The text of a project file at the end of the steps (the open file by default).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FileExpect {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub equals: Option<String>,
    #[serde(default)]
    pub contains: Vec<String>,
    #[serde(default)]
    pub not_contains: Vec<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NavigationExpect {
    #[serde(default)]
    pub when: Option<String>,
    /// Exact project-relative path of the file the caret lands in.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub path_contains: Option<String>,
    #[serde(default)]
    pub line: Option<u64>,
    #[serde(default)]
    pub column: Option<u64>,
    #[serde(default)]
    pub confidence: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UsagesExpect {
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub count: Option<usize>,
    #[serde(default)]
    pub at_least: Option<usize>,
    /// Each path must appear in at least one usage.
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub confidence: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionExpect {
    #[serde(default)]
    pub when: Option<String>,
    /// Labels (or label prefixes up to `(`) the popup must list.
    #[serde(default)]
    pub contains: Vec<String>,
    /// The first row.
    #[serde(default)]
    pub first: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
}

/// A JSON pointer into the `state` snapshot at the end of the steps.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateExpect {
    pub pointer: String,
    #[serde(default)]
    pub equals: Option<Value>,
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
}

/// Runs a command at the end of the steps and checks a JSON pointer into its result.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct QueryExpect {
    pub command: String,
    #[serde(default)]
    pub args: Value,
    pub pointer: String,
    #[serde(default)]
    pub equals: Option<Value>,
    #[serde(default)]
    pub contains: Option<String>,
    #[serde(default)]
    pub at_least: Option<f64>,
    #[serde(default)]
    pub confidence: Option<String>,
}

fn is_unverified(confidence: &Option<String>) -> bool {
    confidence.as_deref() == Some("unverified")
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

/// One step of a scenario: IDE-level operations (typed as a person would) or a generic
/// scene step (`key`, `type`, `sleep`, `writeFile`, `screenshot`, `execute`, `wait`, …).
#[derive(Debug, PartialEq)]
pub enum ScenarioStep {
    Scene(Step),
    /// Moves the caret (1-based line and column).
    Caret {
        line: u64,
        column: u64,
    },
    /// Moves the caret, then types `text`; `\n` is the Enter key.
    TypeAt {
        line: u64,
        column: u64,
        text: String,
    },
    /// Types `text` at the caret; `\n` is the Enter key.
    Text(String),
    /// Waits until the analysis of the active file settled and records its markup.
    WaitAnalysis {
        name: Option<String>,
        timeout: Duration,
    },
    /// Alt+Enter; records the popup rows.
    AltEnter {
        name: Option<String>,
    },
    /// Chooses an Alt+Enter row with Down and Enter.
    ApplyIntention {
        title: Option<String>,
        index: Option<usize>,
    },
    /// Ctrl+Space; records the completion rows.
    Complete {
        name: Option<String>,
    },
    /// Ctrl+B; records where the caret lands.
    Declaration {
        name: Option<String>,
    },
    /// Alt+F7; records the usages.
    FindUsages {
        name: Option<String>,
    },
    /// Shift+F6, types the new name, Enter.
    Rename {
        to: String,
    },
    /// Ctrl+S.
    Save,
    /// Opens another project file (what a double click in the Project view does).
    OpenFile {
        path: String,
    },
    /// Runs a build-system sync command (`gradle.sync`, `bazel.sync`) and waits for it.
    Sync {
        command: String,
        timeout: Duration,
    },
    /// Waits until a language server finished starting and importing the project.
    WaitServer {
        timeout: Duration,
    },
}

impl ScenarioStep {
    pub fn parse(value: &Value) -> Result<Self> {
        let op = value
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("step without \"op\": {value}"))?;
        let number = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_u64)
                .filter(|n| *n >= 1)
                .ok_or_else(|| anyhow!("{op} step needs \"{key}\" (1-based): {value}"))
        };
        let string = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("{op} step needs string \"{key}\": {value}"))
        };
        let name = || value.get("name").and_then(Value::as_str).map(str::to_owned);
        let timeout = || {
            value
                .get("timeoutMs")
                .and_then(Value::as_u64)
                .map_or(DEFAULT_WAIT, Duration::from_millis)
        };
        Ok(match op {
            "caret" => Self::Caret {
                line: number("line")?,
                column: number("column")?,
            },
            "typeAt" => Self::TypeAt {
                line: number("line")?,
                column: number("column")?,
                text: string("text")?,
            },
            "text" => Self::Text(string("text")?),
            "waitAnalysis" => Self::WaitAnalysis {
                name: name(),
                timeout: timeout(),
            },
            "altEnter" => Self::AltEnter { name: name() },
            "applyIntention" => {
                let title = value
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let index = value
                    .get("index")
                    .and_then(Value::as_u64)
                    .map(|n| n as usize);
                if title.is_none() == index.is_none() {
                    bail!("applyIntention needs exactly one of \"title\" or \"index\": {value}");
                }
                Self::ApplyIntention { title, index }
            }
            "complete" => Self::Complete { name: name() },
            "declaration" => Self::Declaration { name: name() },
            "findUsages" => Self::FindUsages { name: name() },
            "rename" => Self::Rename { to: string("to")? },
            "save" => Self::Save,
            "openFile" => Self::OpenFile {
                path: string("path")?,
            },
            "sync" => Self::Sync {
                command: string("command")?,
                timeout: timeout(),
            },
            "waitServer" => Self::WaitServer { timeout: timeout() },
            _ => Self::Scene(Step::parse(value)?),
        })
    }
}

/// A scenario on disk.
pub struct Scenario {
    /// `area/name`.
    pub id: String,
    pub dir: PathBuf,
    pub spec: ScenarioSpec,
    pub steps: Vec<ScenarioStep>,
}

pub fn load_scenarios(root: &Path) -> Result<Vec<Scenario>> {
    let mut found = Vec::new();
    let mut areas: Vec<PathBuf> = fs::read_dir(root)
        .with_context(|| format!("read {}", root.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    areas.sort();
    for area in areas {
        let mut names: Vec<PathBuf> = fs::read_dir(&area)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.join("scenario.json").is_file())
            .collect();
        names.sort();
        for dir in names {
            let id = format!(
                "{}/{}",
                area.file_name().unwrap_or_default().to_string_lossy(),
                dir.file_name().unwrap_or_default().to_string_lossy()
            );
            let path = dir.join("scenario.json");
            let text = fs::read_to_string(&path)?;
            let spec: ScenarioSpec =
                serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
            let steps = spec
                .steps
                .iter()
                .map(ScenarioStep::parse)
                .collect::<Result<Vec<_>>>()
                .with_context(|| format!("in {}", path.display()))?;
            if !dir.join("project").is_dir() {
                bail!("{id} has no project/ fixture");
            }
            found.push(Scenario {
                id,
                dir,
                spec,
                steps,
            });
        }
    }
    Ok(found)
}

// ---------------------------------------------------------------------------
// Results
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Pass,
    Partial,
    Fail,
    Skipped,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Partial => "partial",
            Status::Fail => "fail",
            Status::Skipped => "skipped",
        }
    }

    /// The worse of two results (`Skipped` never combines).
    fn worst(self, other: Status) -> Status {
        match (self, other) {
            (Status::Fail, _) | (_, Status::Fail) => Status::Fail,
            (Status::Partial, _) | (_, Status::Partial) => Status::Partial,
            _ => Status::Pass,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub status: Status,
    pub what: String,
    pub detail: String,
    /// The expectation was not confirmed in IntelliJ.
    pub unverified: bool,
}

impl CheckResult {
    fn new(status: Status, what: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status,
            what: what.into(),
            detail: detail.into(),
            unverified: false,
        }
    }

    fn unverified(mut self, flag: bool) -> Self {
        self.unverified = flag;
        self
    }
}

pub struct ScenarioResult {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub checks: Vec<CheckResult>,
    /// Harness notes: waits that timed out, steps that failed, why it was skipped.
    pub notes: Vec<String>,
    pub duration: Duration,
    pub known_gap: Option<String>,
    pub intellij: String,
}

fn combine(checks: &[CheckResult]) -> Status {
    checks
        .iter()
        .fold(Status::Pass, |status, check| status.worst(check.status))
}

// ---------------------------------------------------------------------------
// Expectation evaluation (pure; unit tested)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub line: u64,
    pub column: u64,
    pub end_line: u64,
    pub end_column: u64,
}

impl Range {
    /// `9:9-9:19` or `9:9-19`.
    pub fn parse(text: &str) -> Result<Self> {
        let invalid = || anyhow!("invalid range {text:?}; expected line:column-line:column");
        let (from, to) = text.split_once('-').ok_or_else(invalid)?;
        let pair = |part: &str| -> Result<(u64, u64)> {
            let (line, column) = part.split_once(':').ok_or_else(invalid)?;
            Ok((
                line.trim().parse().map_err(|_| invalid())?,
                column.trim().parse().map_err(|_| invalid())?,
            ))
        };
        let (line, column) = pair(from)?;
        let (end_line, end_column) = if to.contains(':') {
            pair(to)?
        } else {
            (line, to.trim().parse().map_err(|_| invalid())?)
        };
        Ok(Self {
            line,
            column,
            end_line,
            end_column,
        })
    }

    fn of(value: &Value) -> Option<Self> {
        let get = |key: &str| value.get(key).and_then(Value::as_u64);
        Some(Self {
            line: get("line")?,
            column: get("column")?,
            end_line: get("endLine")?,
            end_column: get("endColumn")?,
        })
    }
}

impl std::fmt::Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}-{}:{}",
            self.line, self.column, self.end_line, self.end_column
        )
    }
}

fn normalize_severity(severity: &str) -> String {
    severity
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
}

fn describe_markup(markup: &[Value]) -> String {
    if markup.is_empty() {
        return "no markup".into();
    }
    let rows: Vec<String> = markup
        .iter()
        .take(6)
        .map(|item| {
            let range = Range::of(&item["range"])
                .map(|r| r.to_string())
                .unwrap_or_default();
            format!(
                "{} {} \"{}\"",
                item["severity"].as_str().unwrap_or("?"),
                range,
                item["message"].as_str().unwrap_or("")
            )
        })
        .collect();
    let more = if markup.len() > rows.len() {
        format!(" (+{} more)", markup.len() - rows.len())
    } else {
        String::new()
    };
    format!("markup seen: {}{more}", rows.join("; "))
}

pub fn check_markup(expect: &MarkupExpect, observed: Option<&Value>) -> CheckResult {
    let what = format!(
        "markup {} {} {}",
        expect.severity,
        expect.range,
        expect
            .message
            .as_deref()
            .map(|m| format!("\"{m}\""))
            .unwrap_or_default()
    )
    .trim_end()
    .to_owned();
    let unverified = is_unverified(&expect.confidence);
    let Some(observed) = observed else {
        return CheckResult::new(Status::Fail, what, "markup was never observed")
            .unverified(unverified);
    };
    let range = match Range::parse(&expect.range) {
        Ok(range) => range,
        Err(error) => return CheckResult::new(Status::Fail, what, error.to_string()),
    };
    let markup: Vec<Value> = observed["markup"].as_array().cloned().unwrap_or_default();
    let wanted = normalize_severity(&expect.severity);
    let at_range: Vec<&Value> = markup
        .iter()
        .filter(|item| Range::of(&item["range"]) == Some(range))
        .collect();
    if at_range.is_empty() {
        return CheckResult::new(
            Status::Fail,
            what,
            format!("no markup at {range}; {}", describe_markup(&markup)),
        )
        .unverified(unverified);
    }
    let mut best: Option<CheckResult> = None;
    for item in at_range {
        let severity = item["severity"].as_str().unwrap_or("");
        let message = item["message"].as_str().unwrap_or("");
        // Unused code is drawn greyed out; IntelliJ reports it as a warning.
        let unused_hint = wanted == "warning"
            && severity == "hint"
            && item["unnecessary"].as_bool() == Some(true);
        let result = if severity != wanted && !unused_hint {
            CheckResult::new(
                Status::Fail,
                what.clone(),
                format!("at {range} the severity is {severity}, expected {wanted}: \"{message}\""),
            )
        } else if unused_hint {
            CheckResult::new(
                Status::Partial,
                what.clone(),
                format!(
                    "at {range} the code is greyed out (hint) instead of a warning: \"{message}\""
                ),
            )
        } else {
            match expect.message.as_deref() {
                Some(expected) if expected != message => CheckResult::new(
                    Status::Partial,
                    what.clone(),
                    format!("right range and severity, message is \"{message}\""),
                ),
                _ => CheckResult::new(Status::Pass, what.clone(), ""),
            }
        };
        if best.as_ref().is_none_or(|b| result.status < b.status) {
            best = Some(result);
        }
    }
    best.expect("at least one candidate").unverified(unverified)
}

pub fn check_no_markup(expect: &NoMarkupExpect, observed: Option<&Value>) -> CheckResult {
    let severity = normalize_severity(expect.severity.as_deref().unwrap_or("error"));
    let what = format!("no {severity} markup");
    let unverified = is_unverified(&expect.confidence);
    let Some(observed) = observed else {
        return CheckResult::new(Status::Fail, what, "markup was never observed")
            .unverified(unverified);
    };
    let markup: Vec<Value> = observed["markup"].as_array().cloned().unwrap_or_default();
    let offending: Vec<Value> = markup
        .into_iter()
        .filter(|item| item["severity"].as_str() == Some(severity.as_str()))
        .collect();
    if offending.is_empty() {
        CheckResult::new(Status::Pass, what, "").unverified(unverified)
    } else {
        CheckResult::new(Status::Fail, what, describe_markup(&offending)).unverified(unverified)
    }
}

fn item_matches(pattern: &str, text: &str) -> bool {
    match pattern.strip_prefix("re:") {
        Some(regex) => Regex::new(regex).is_ok_and(|re| re.is_match(text)),
        None => pattern == text,
    }
}

pub fn check_intentions(expect: &IntentionsExpect, observed: Option<&[String]>) -> CheckResult {
    let mode = expect.mode.as_deref().unwrap_or("contains");
    let what = format!("Alt+Enter {mode}: {}", expect.items.join(" | "));
    let unverified = is_unverified(&expect.confidence);
    let Some(actual) = observed else {
        return CheckResult::new(Status::Fail, what, "Alt+Enter was never pressed")
            .unverified(unverified);
    };
    let shown = if actual.is_empty() {
        "no popup or no items".to_owned()
    } else {
        format!("popup: {}", actual.join(" | "))
    };
    let position = |pattern: &String| actual.iter().position(|text| item_matches(pattern, text));
    let found: Vec<Option<usize>> = expect.items.iter().map(position).collect();
    let present = found.iter().flatten().count();
    let result = match mode {
        "exact" => {
            let same = actual.len() == expect.items.len()
                && expect
                    .items
                    .iter()
                    .zip(actual)
                    .all(|(pattern, text)| item_matches(pattern, text));
            if same {
                CheckResult::new(Status::Pass, what, "")
            } else if present == expect.items.len() {
                CheckResult::new(
                    Status::Partial,
                    what,
                    format!("all items present but the list differs; {shown}"),
                )
            } else if present > 0 {
                CheckResult::new(
                    Status::Partial,
                    what,
                    format!("{present} of {} items; {shown}", expect.items.len()),
                )
            } else {
                CheckResult::new(Status::Fail, what, shown)
            }
        }
        "ordered" => {
            let ordered =
                found.iter().all(Option::is_some) && found.windows(2).all(|pair| pair[0] < pair[1]);
            if ordered {
                CheckResult::new(Status::Pass, what, "")
            } else if present == expect.items.len() {
                CheckResult::new(
                    Status::Partial,
                    what,
                    format!("all items present in a different order; {shown}"),
                )
            } else if present > 0 {
                CheckResult::new(
                    Status::Partial,
                    what,
                    format!("{present} of {} items; {shown}", expect.items.len()),
                )
            } else {
                CheckResult::new(Status::Fail, what, shown)
            }
        }
        "contains" => {
            if present == expect.items.len() {
                CheckResult::new(Status::Pass, what, "")
            } else if present > 0 {
                CheckResult::new(
                    Status::Partial,
                    what,
                    format!("{present} of {} items; {shown}", expect.items.len()),
                )
            } else {
                CheckResult::new(Status::Fail, what, shown)
            }
        }
        other => CheckResult::new(Status::Fail, what, format!("unknown mode {other:?}")),
    };
    result.unverified(unverified)
}

pub fn check_file(expect: &FileExpect, path: &str, text: Result<&str, &str>) -> CheckResult {
    let what = format!("file {path}");
    let unverified = is_unverified(&expect.confidence);
    let text = match text {
        Ok(text) => text,
        Err(error) => {
            return CheckResult::new(Status::Fail, what, format!("cannot read: {error}"))
                .unverified(unverified);
        }
    };
    let mut problems = Vec::new();
    if let Some(equals) = &expect.equals
        && equals != text
    {
        problems.push("content differs from the expected text".to_owned());
    }
    for needle in &expect.contains {
        if !text.contains(needle.as_str()) {
            problems.push(format!("missing {needle:?}"));
        }
    }
    for needle in &expect.not_contains {
        if text.contains(needle.as_str()) {
            problems.push(format!("still contains {needle:?}"));
        }
    }
    if let Some(pattern) = &expect.regex {
        match Regex::new(pattern) {
            Ok(re) if re.is_match(text) => {}
            Ok(_) => problems.push(format!("does not match /{pattern}/")),
            Err(error) => problems.push(format!("bad regex: {error}")),
        }
    }
    if problems.is_empty() {
        CheckResult::new(Status::Pass, what, "").unverified(unverified)
    } else {
        CheckResult::new(Status::Fail, what, problems.join("; ")).unverified(unverified)
    }
}

pub fn check_navigation(expect: &NavigationExpect, observed: Option<&Value>) -> CheckResult {
    let what = format!(
        "go to declaration {}",
        expect
            .path
            .as_deref()
            .or(expect.path_contains.as_deref())
            .unwrap_or("")
    )
    .trim_end()
    .to_owned();
    let unverified = is_unverified(&expect.confidence);
    let Some(observed) = observed else {
        return CheckResult::new(Status::Fail, what, "navigation was never invoked")
            .unverified(unverified);
    };
    let path = observed["path"].as_str().unwrap_or("");
    let landed = format!(
        "landed in {path} at {}:{}",
        observed["line"], observed["column"]
    );
    let mut wrong = Vec::new();
    if let Some(expected) = &expect.path
        && expected != path
    {
        wrong.push("file");
    }
    if let Some(part) = &expect.path_contains
        && !path.contains(part.as_str())
    {
        wrong.push("file");
    }
    if let Some(line) = expect.line
        && observed["line"].as_u64() != Some(line)
    {
        wrong.push("line");
    }
    if let Some(column) = expect.column
        && observed["column"].as_u64() != Some(column)
    {
        wrong.push("column");
    }
    if wrong.is_empty() {
        return CheckResult::new(Status::Pass, what, "").unverified(unverified);
    }
    // The right file at a different position is a partial result.
    let status = if wrong.iter().all(|w| *w != "file") {
        Status::Partial
    } else {
        Status::Fail
    };
    CheckResult::new(
        status,
        what,
        format!("{landed}; wrong {}", wrong.join(", ")),
    )
    .unverified(unverified)
}

pub fn check_usages(expect: &UsagesExpect, observed: Option<&Value>) -> CheckResult {
    let what = "find usages".to_owned();
    let unverified = is_unverified(&expect.confidence);
    let Some(observed) = observed else {
        return CheckResult::new(Status::Fail, what, "find usages was never invoked")
            .unverified(unverified);
    };
    let items: Vec<&Value> = observed["items"]
        .as_array()
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    let paths: Vec<&str> = items.iter().filter_map(|i| i["path"].as_str()).collect();
    let mut problems = Vec::new();
    if let Some(count) = expect.count
        && items.len() != count
    {
        problems.push(format!("{} usages, expected {count}", items.len()));
    }
    if let Some(least) = expect.at_least
        && items.len() < least
    {
        problems.push(format!("{} usages, expected at least {least}", items.len()));
    }
    for path in &expect.paths {
        if !paths.iter().any(|p| p.contains(path.as_str())) {
            problems.push(format!("no usage in {path}"));
        }
    }
    if problems.is_empty() {
        CheckResult::new(Status::Pass, what, "").unverified(unverified)
    } else {
        let partial = !items.is_empty();
        CheckResult::new(
            if partial {
                Status::Partial
            } else {
                Status::Fail
            },
            what,
            problems.join("; "),
        )
        .unverified(unverified)
    }
}

pub fn check_completion(expect: &CompletionExpect, observed: Option<&Value>) -> CheckResult {
    let what = format!("completion {}", expect.contains.join(", "));
    let unverified = is_unverified(&expect.confidence);
    let Some(observed) = observed else {
        return CheckResult::new(Status::Fail, what, "completion was never invoked")
            .unverified(unverified);
    };
    let labels: Vec<String> = observed["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["label"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let has = |wanted: &str| {
        labels
            .iter()
            .any(|label| label == wanted || label.split('(').next() == Some(wanted))
    };
    let shown = if labels.is_empty() {
        "the popup listed nothing".to_owned()
    } else {
        format!(
            "popup: {}{}",
            labels
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            if labels.len() > 8 { ", …" } else { "" }
        )
    };
    let present = expect.contains.iter().filter(|wanted| has(wanted)).count();
    let first_ok = expect.first.as_deref().is_none_or(|wanted| {
        labels
            .first()
            .is_some_and(|l| l == wanted || l.split('(').next() == Some(wanted))
    });
    if present == expect.contains.len() && first_ok {
        CheckResult::new(Status::Pass, what, "").unverified(unverified)
    } else if present > 0 || (present == expect.contains.len() && !first_ok) {
        CheckResult::new(Status::Partial, what, shown).unverified(unverified)
    } else {
        CheckResult::new(Status::Fail, what, shown).unverified(unverified)
    }
}

pub fn check_pointer(
    what: String,
    value: &Value,
    pointer: &str,
    equals: Option<&Value>,
    contains: Option<&str>,
    at_least: Option<f64>,
    unverified: bool,
) -> CheckResult {
    let actual = value.pointer(pointer).unwrap_or(&Value::Null);
    let mut problems = Vec::new();
    if let Some(expected) = equals
        && actual != expected
    {
        problems.push(format!("is {actual}, expected {expected}"));
    }
    if let Some(needle) = contains {
        let haystack = match actual {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        if !haystack.contains(needle) {
            problems.push(format!("{actual} does not contain {needle:?}"));
        }
    }
    if let Some(least) = at_least
        && actual.as_f64().is_none_or(|n| n < least)
    {
        problems.push(format!("is {actual}, expected at least {least}"));
    }
    if problems.is_empty() {
        CheckResult::new(Status::Pass, what, "").unverified(unverified)
    } else {
        CheckResult::new(Status::Fail, what, problems.join("; ")).unverified(unverified)
    }
}

// ---------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------

/// What the steps observed, by observation name (`when` in the expectations).
#[derive(Default)]
struct Observed {
    markup: Vec<(Option<String>, Value)>,
    intentions: Vec<(Option<String>, Vec<Value>)>,
    completion: Vec<(Option<String>, Value)>,
    navigation: Vec<(Option<String>, Value)>,
    usages: Vec<(Option<String>, Value)>,
    notes: Vec<String>,
}

fn pick<'a, T>(list: &'a [(Option<String>, T)], when: Option<&str>) -> Option<&'a T> {
    match when {
        None => list.first().map(|(_, value)| value),
        Some(name) => list
            .iter()
            .find(|(n, _)| n.as_deref() == Some(name))
            .map(|(_, value)| value),
    }
}

struct Session<'a> {
    client: ControlClient,
    project: PathBuf,
    out: &'a Path,
    observed: Observed,
}

impl Session<'_> {
    fn execute(&mut self, command: &str, args: Value) -> Result<Value> {
        send(
            &mut self.client,
            &Request::Execute {
                command: command.into(),
                args,
            },
        )
    }

    fn state(&mut self) -> Result<Value> {
        send(&mut self.client, &Request::State)
    }

    fn key(&mut self, chord: &str) -> Result<()> {
        send(
            &mut self.client,
            &Request::Key {
                chord: chord.into(),
            },
        )?;
        Ok(())
    }

    fn type_text(&mut self, text: &str) -> Result<()> {
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                self.key("Enter")?;
            }
            if !line.is_empty() {
                send(&mut self.client, &Request::Type { text: line.into() })?;
            }
        }
        Ok(())
    }

    fn wait(&mut self, until: WaitCondition, timeout: Duration) -> Result<()> {
        send(
            &mut self.client,
            &Request::Wait {
                until,
                text: None,
                timeout_ms: Some(timeout.as_millis() as u64),
            },
        )?;
        Ok(())
    }

    /// Polls `condition` on the state snapshot every 150 ms.
    fn poll_state(
        &mut self,
        timeout: Duration,
        condition: impl Fn(&Value) -> bool,
    ) -> Result<Option<Value>> {
        let started = Instant::now();
        loop {
            let state = self.state()?;
            if condition(&state) {
                return Ok(Some(state));
            }
            if started.elapsed() > timeout {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(150));
        }
    }

    fn caret(&mut self, line: u64, column: u64) -> Result<()> {
        self.execute("editor.gotoLine", json!({"line": line, "column": column}))?;
        Ok(())
    }

    fn run(&mut self, step: &ScenarioStep) -> Result<()> {
        match step {
            ScenarioStep::Scene(step) => {
                scenes::run_step(&mut self.client, step, Some(&self.project), self.out)?
            }
            ScenarioStep::Caret { line, column } => self.caret(*line, *column)?,
            ScenarioStep::TypeAt { line, column, text } => {
                self.caret(*line, *column)?;
                self.type_text(text)?;
            }
            ScenarioStep::Text(text) => self.type_text(text)?,
            ScenarioStep::WaitAnalysis { name, timeout } => {
                if let Err(error) = self.wait(WaitCondition::Analysis, *timeout) {
                    self.observed
                        .notes
                        .push(format!("analysis did not settle: {error}"));
                }
                let markup = self.execute("automation.markup", Value::Null)?;
                self.observed.markup.push((name.clone(), markup));
            }
            ScenarioStep::AltEnter { name } => {
                self.key("Alt+Enter")?;
                let opened = self.poll_state(Duration::from_secs(20), |state| {
                    state["popups"]["intentions"] == json!(true)
                })?;
                let rows = if opened.is_some() {
                    self.execute("code.alerts", Value::Null)?["actions"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                self.observed.intentions.push((name.clone(), rows));
            }
            ScenarioStep::ApplyIntention { title, index } => {
                let rows = self
                    .observed
                    .intentions
                    .last()
                    .map(|(_, rows)| rows.clone())
                    .ok_or_else(|| anyhow!("applyIntention without a preceding altEnter"))?;
                // Rows of submenus are collapsed under a header at the end of the popup.
                let visible: Vec<&Value> = rows.iter().filter(|r| r["submenu"].is_null()).collect();
                let position = match (title, index) {
                    (Some(title), _) => visible
                        .iter()
                        .position(|row| item_matches(title, row["text"].as_str().unwrap_or("")))
                        .ok_or_else(|| anyhow!("no Alt+Enter item {title:?} to apply"))?,
                    (None, Some(index)) => *index,
                    (None, None) => unreachable!("checked at parse time"),
                };
                if position >= visible.len() {
                    bail!(
                        "Alt+Enter has only {} items, cannot apply item {position}",
                        visible.len()
                    );
                }
                let before = self.state()?["editor"]["text"].clone();
                for _ in 0..position {
                    self.key("ArrowDown")?;
                }
                self.key("Enter")?;
                // Language-server actions resolve their edit asynchronously: wait for the
                // text to change (an action may change nothing, or another file).
                self.poll_state(Duration::from_secs(20), |state| {
                    state["editor"]["text"] != before
                })?;
                self.wait(WaitCondition::Idle, Duration::from_secs(60))?;
            }
            ScenarioStep::Complete { name } => {
                self.key("Ctrl+Space")?;
                let state = self.poll_state(Duration::from_secs(30), |state| {
                    state["popups"]["completion"] == json!(true)
                })?;
                let rows = if state.is_some() {
                    self.execute("automation.completion", Value::Null)?
                } else {
                    json!({"open": false, "items": []})
                };
                self.observed.completion.push((name.clone(), rows));
            }
            ScenarioStep::Declaration { name } => {
                let before = self.state()?;
                let at = |state: &Value| {
                    (
                        state["activeTab"].clone(),
                        state["editor"]["caret"]["offset"].clone(),
                    )
                };
                let start = at(&before);
                self.key("Ctrl+B")?;
                let after =
                    match self.poll_state(Duration::from_secs(45), |state| at(state) != start)? {
                        Some(state) => state,
                        None => self.state()?,
                    };
                let results = self.execute("navigate.results", Value::Null)?;
                self.observed.navigation.push((
                    name.clone(),
                    json!({
                        "path": after["activeTab"],
                        "line": after["editor"]["caret"]["line"],
                        "column": after["editor"]["caret"]["column"],
                        "moved": at(&after) != start,
                        "choices": results["choices"],
                        "status": after["status"],
                    }),
                ));
            }
            ScenarioStep::FindUsages { name } => {
                self.key("Alt+F7")?;
                let started = Instant::now();
                let results = loop {
                    let results = self.execute("navigate.results", Value::Null)?;
                    let done = results["pending"] == json!(false) && !results["usages"].is_null();
                    if done || started.elapsed() > Duration::from_secs(45) {
                        break results;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                };
                self.observed
                    .usages
                    .push((name.clone(), results["usages"].clone()));
            }
            ScenarioStep::Rename { to } => {
                self.key("Shift+F6")?;
                let opened = self.poll_state(Duration::from_secs(30), |state| {
                    state["popups"]["rename"] == json!(true)
                })?;
                if opened.is_none() {
                    bail!("Shift+F6 did not start a rename");
                }
                self.type_text(to)?;
                self.key("Enter")?;
                // The server applies the rename asynchronously; the session ends with it.
                let finished = self.poll_state(Duration::from_secs(60), |state| {
                    state["popups"]["rename"] != json!(true)
                })?;
                if finished.is_none() {
                    self.observed
                        .notes
                        .push("the rename did not finish within 60s".into());
                }
                self.wait(WaitCondition::Idle, Duration::from_secs(60))?;
            }
            ScenarioStep::Save => self.key("Ctrl+S")?,
            ScenarioStep::OpenFile { path } => {
                self.execute("editor.openFile", json!({"path": path}))?;
            }
            ScenarioStep::Sync { command, timeout } => {
                self.execute(command, Value::Null)?;
                // The task starts asynchronously: wait for it to run, then to finish.
                let started = Instant::now();
                let mut saw_running = false;
                while started.elapsed() < *timeout {
                    let state = self.state()?;
                    let running = state["run"]["running"] == json!(true);
                    saw_running |= running;
                    if !running && (saw_running || started.elapsed() > Duration::from_secs(5)) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(300));
                }
            }
            ScenarioStep::WaitServer { timeout } => {
                // Servers report progress in bursts (importing, indexing, checking): ready
                // means ready and silent for a few seconds.
                const SILENT: Duration = Duration::from_secs(4);
                let started = Instant::now();
                let mut quiet_since: Option<Instant> = None;
                loop {
                    let servers = self.execute("language.servers", Value::Null)?;
                    let list = servers["servers"].as_array().cloned().unwrap_or_default();
                    let ready = !list.is_empty()
                        && list.iter().all(|s| {
                            s["status"]["state"] == json!("ready")
                                && s["progress"].as_array().is_none_or(|p| p.is_empty())
                        });
                    if ready {
                        let since = *quiet_since.get_or_insert_with(Instant::now);
                        if since.elapsed() >= SILENT {
                            break;
                        }
                    } else {
                        quiet_since = None;
                    }
                    if started.elapsed() > *timeout {
                        self.observed.notes.push(format!(
                            "language server not ready after {}s: {}",
                            timeout.as_secs(),
                            list.iter()
                                .map(|s| format!("{} {}", s["id"], s["status"]["state"]))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
        Ok(())
    }
}

fn evaluate(
    scenario: &Scenario,
    session: &mut Session<'_>,
    observed: &Observed,
    open_file: Option<&str>,
) -> Result<Vec<CheckResult>> {
    let expect = &scenario.spec.expect;
    let mut checks = Vec::new();
    for e in &expect.markup {
        checks.push(check_markup(e, pick(&observed.markup, e.when.as_deref())));
    }
    for e in &expect.no_markup {
        checks.push(check_no_markup(
            e,
            pick(&observed.markup, e.when.as_deref()),
        ));
    }
    for e in &expect.intentions {
        let rows: Option<Vec<String>> = pick(&observed.intentions, e.when.as_deref()).map(|rows| {
            rows.iter()
                .filter(|row| row["submenu"].is_null())
                .filter_map(|row| row["text"].as_str().map(str::to_owned))
                .collect()
        });
        checks.push(check_intentions(e, rows.as_deref()));
    }
    for e in &expect.navigation {
        checks.push(check_navigation(
            e,
            pick(&observed.navigation, e.when.as_deref()),
        ));
    }
    for e in &expect.usages {
        checks.push(check_usages(e, pick(&observed.usages, e.when.as_deref())));
    }
    for e in &expect.completion {
        checks.push(check_completion(
            e,
            pick(&observed.completion, e.when.as_deref()),
        ));
    }
    for e in &expect.file {
        let path = e
            .path
            .as_deref()
            .or(open_file)
            .ok_or_else(|| anyhow!("file expectation without a path and no open file"))?
            .to_owned();
        let outcome: std::result::Result<String, String> = session
            .execute("automation.documentText", json!({"path": path}))
            .map(|value| value["text"].as_str().unwrap_or("").to_owned())
            .map_err(|error| format!("{error:#}"));
        checks.push(check_file(
            e,
            &path,
            match &outcome {
                Ok(text) => Ok(text.as_str()),
                Err(error) => Err(error.as_str()),
            },
        ));
    }
    if !expect.state.is_empty() {
        let state = session.state()?;
        for e in &expect.state {
            checks.push(check_pointer(
                format!("state {}", e.pointer),
                &state,
                &e.pointer,
                e.equals.as_ref(),
                e.contains.as_deref(),
                None,
                is_unverified(&e.confidence),
            ));
        }
    }
    for e in &expect.query {
        let what = format!("{} {}", e.command, e.pointer);
        match session.execute(&e.command, e.args.clone()) {
            Ok(value) => checks.push(check_pointer(
                what,
                &value,
                &e.pointer,
                e.equals.as_ref(),
                e.contains.as_deref(),
                e.at_least,
                is_unverified(&e.confidence),
            )),
            Err(error) => checks.push(
                CheckResult::new(Status::Fail, what, format!("{error:#}"))
                    .unverified(is_unverified(&e.confidence)),
            ),
        }
    }
    Ok(checks)
}

/// Copies the fixture into `dst` and commits it to a fresh Git repository.
fn copy_project(src: &Path, dst: &Path) -> Result<()> {
    fn copy(src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let target = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy(&entry.path(), &target)?;
            } else {
                fs::copy(entry.path(), &target)?;
            }
        }
        Ok(())
    }
    copy(src, dst)?;
    git_init(dst)
}

struct Environment {
    app: PathBuf,
    display: Display,
    work: PathBuf,
    out: PathBuf,
    java_home: Option<PathBuf>,
}

fn launch(
    env: &Environment,
    dir: &Path,
    project: Option<&Path>,
    open: &[String],
    extra: &BTreeMap<String, String>,
    tag: &str,
) -> Result<(AppProcess, PathBuf)> {
    // Unix socket paths are limited to ~100 bytes, so keep them short.
    let socket = std::env::temp_dir().join(format!("jt-scn-{}-{tag}.sock", std::process::id()));
    let log = fs::File::create(dir.join("app.log"))?;
    let mut command = Command::new(&env.app);
    if let Some(project) = project {
        command.arg(project);
        command.args(open);
    }
    command
        .env(archipel_ide_control::SOCKET_ENV, &socket)
        .env("ARCHIPEL_NO_UPDATE_CHECK", "1")
        .env("ARCHIPEL_NO_TOOLCHAIN_SETUP", "1")
        // Indexes, language-server workspaces and settings of a scenario stay in its
        // directory; toolchains (XDG_DATA_HOME) are shared so they are installed once.
        .env("ARCHIPEL_INDEX_DIR", dir.join("cache"))
        .env("XDG_CACHE_HOME", dir.join("xdg-cache"))
        .env("XDG_CONFIG_HOME", dir.join("xdg-config"))
        .envs(extra)
        .stdout(log.try_clone()?)
        .stderr(log);
    if let Some(java_home) = &env.java_home {
        command.env("JAVA_HOME", java_home);
    }
    if archipel_ide_compat::env_var_os("ARCHIPEL_RENDERER").is_none() && cfg!(target_os = "linux") {
        command.env("ARCHIPEL_RENDERER", "glow");
    }
    if let Some(display) = &env.display.name {
        command.env("DISPLAY", display);
    }
    let process = AppProcess(
        command
            .spawn()
            .with_context(|| format!("launch {}", env.app.display()))?,
    );
    Ok((process, socket))
}

fn run_scenario(env: &Environment, scenario: &Scenario) -> ScenarioResult {
    let started = Instant::now();
    let mut result = ScenarioResult {
        id: scenario.id.clone(),
        title: scenario.spec.title.clone(),
        status: Status::Fail,
        checks: Vec::new(),
        notes: Vec::new(),
        duration: Duration::ZERO,
        known_gap: scenario.spec.known_gap.clone(),
        intellij: format!(
            "{}: {}{}",
            scenario.spec.intellij.version,
            scenario.spec.intellij.action,
            scenario
                .spec
                .intellij
                .notes
                .as_deref()
                .map(|n| format!(" ({n})"))
                .unwrap_or_default()
        ),
    };
    match run_scenario_inner(env, scenario) {
        Ok((checks, notes)) => {
            result.status = combine(&checks);
            result.checks = checks;
            result.notes = notes;
        }
        Err((error, notes)) => {
            result.notes = notes;
            result.notes.push(format!("{error:#}"));
            result.checks = vec![CheckResult::new(
                Status::Fail,
                "scenario ran",
                format!("{error:#}"),
            )];
        }
    }
    result.duration = started.elapsed();
    result
}

type RunOutcome =
    std::result::Result<(Vec<CheckResult>, Vec<String>), (anyhow::Error, Vec<String>)>;

fn run_scenario_inner(env: &Environment, scenario: &Scenario) -> RunOutcome {
    let slug = scenario.id.replace('/', "-");
    let dir = env.work.join(&slug);
    let setup = || -> Result<(PathBuf, AppProcess, ControlClient)> {
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
        let project = dir.join("project");
        copy_project(&scenario.dir.join("project"), &project)?;
        let project = fs::canonicalize(&project)?;
        let open = scenario.spec.open.paths();
        let (mut process, socket) =
            launch(env, &dir, Some(&project), &open, &scenario.spec.env, &slug)?;
        let client = connect(&socket, &mut process.0, &dir)?;
        Ok((project, process, client))
    };
    let (project, process, client) = setup().map_err(|e| (e, Vec::new()))?;
    let mut session = Session {
        client,
        project,
        out: &env.out,
        observed: Observed::default(),
    };
    let outcome = play(scenario, &mut session);
    // Debugging aid: keep the IDE alive to inspect it with `archipel-cli ui --socket …`.
    if let Some(seconds) = std::env::var("ARCHIPEL_SCENARIO_HOLD_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        eprintln!(
            "\nholding {} for {seconds}s: control socket {}",
            scenario.id,
            std::env::temp_dir()
                .join(format!("jt-scn-{}-{slug}.sock", std::process::id()))
                .display()
        );
        std::thread::sleep(Duration::from_secs(seconds));
    }
    let observed = std::mem::take(&mut session.observed);
    let _ = fs::write(
        dir.join("observed.json"),
        serde_json::to_string_pretty(&json!({
            "markup": observed.markup,
            "intentions": observed.intentions,
            "completion": observed.completion,
            "navigation": observed.navigation,
            "usages": observed.usages,
            "notes": observed.notes,
        }))
        .unwrap_or_default(),
    );
    let open = scenario.spec.open.paths();
    // Whatever was observed before a step failed still counts: the failed step is one more
    // failing check next to the ones that passed.
    let mut checks = evaluate(
        scenario,
        &mut session,
        &observed,
        open.first().map(String::as_str),
    )
    .map_err(|e| (e, observed.notes.clone()))?;
    drop(session);
    drop(process);
    // Language-server workspaces and build output are large; the log and what was
    // observed are what a report needs.
    if std::env::var_os("ARCHIPEL_SCENARIO_KEEP_WORK").is_none() {
        for heavy in ["project", "cache", "xdg-cache", "xdg-config"] {
            let _ = fs::remove_dir_all(dir.join(heavy));
        }
    }
    let mut notes = observed.notes;
    if let Err(error) = outcome {
        notes.push(format!("{error:#}"));
        checks.push(CheckResult::new(
            Status::Fail,
            "scenario steps ran",
            format!("{error:#}"),
        ));
    }
    Ok((checks, notes))
}

fn play(scenario: &Scenario, session: &mut Session<'_>) -> Result<()> {
    // The modal first-run wizard would swallow the scenario's keys.
    let state = session.state()?;
    if state["popups"]["customize"] == json!(true) {
        session.execute("app.skipCustomize", Value::Null)?;
    }
    session.wait(WaitCondition::Idle, Duration::from_secs(60))?;
    for (index, step) in scenario.steps.iter().enumerate() {
        session
            .run(step)
            .with_context(|| format!("step {} ({step:?})", index + 1))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Requirements
// ---------------------------------------------------------------------------

/// Which of `needs` are available on this machine; asks a bare IDE for toolchains.
fn probe(
    env: &Environment,
    needs: &BTreeSet<String>,
    install: bool,
) -> Result<BTreeMap<String, bool>> {
    let dir = env.work.join("probe");
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    let (mut process, socket) = launch(env, &dir, None, &[], &BTreeMap::new(), "probe")?;
    let mut client = connect(&socket, &mut process.0, &dir)?;
    // A binary built from another checkout (shared target directory) lacks `wait analysis`.
    send(
        &mut client,
        &Request::Wait {
            until: WaitCondition::Analysis,
            text: None,
            timeout_ms: Some(10_000),
        },
    )
    .context(
        "the IDE binary does not support `wait analysis`; touch the changed crates and rebuild",
    )?;
    let mut available = BTreeMap::new();
    for need in needs {
        let have = if need == "network" {
            network_available()
        } else {
            let locate = |client: &mut ControlClient| {
                send(
                    client,
                    &Request::Execute {
                        command: "toolchain.locate".into(),
                        args: json!({"component": need}),
                    },
                )
            };
            match locate(&mut client) {
                Ok(value) if !value.is_null() && runs(need, &value) => true,
                Ok(_) if install => {
                    println!("installing toolchain component {need} …");
                    let installed = send(
                        &mut client,
                        &Request::Execute {
                            command: "toolchain.install".into(),
                            args: json!({"component": need, "wait": true}),
                        },
                    );
                    match installed {
                        Ok(_) => true,
                        Err(error) => {
                            println!("  could not install {need}: {error:#}");
                            false
                        }
                    }
                }
                Ok(_) => false,
                Err(error) => {
                    println!("  cannot ask for {need}: {error:#}");
                    false
                }
            }
        };
        available.insert(need.clone(), have);
    }
    // A current jdtls needs a newer JDK than most machines have, and the IDE prefers
    // `JAVA_HOME` over its managed JDK: make sure the store has one to point `JAVA_HOME` at.
    if install && needs.contains("jdtls") && managed_java_home().is_none() {
        println!("installing a managed JDK for jdtls …");
        let installed = send(
            &mut client,
            &Request::Execute {
                command: "toolchain.install".into(),
                args: json!({"component": "jdk", "wait": true}),
            },
        );
        if let Err(error) = installed {
            println!("  could not install a JDK: {error:#}");
        }
    }
    Ok(available)
}

/// A located component is usable: rustup's `rust-analyzer` shim exists even when the
/// component is not installed, so it has to answer `--version`.
fn runs(need: &str, located: &Value) -> bool {
    if need != "rust-analyzer" {
        return true;
    }
    located["path"].as_str().is_some_and(|path| {
        Command::new(path)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

fn network_available() -> bool {
    Command::new("curl")
        .args([
            "-sSI",
            "-m",
            "15",
            "-o",
            "/dev/null",
            "https://repo.maven.apache.org/maven2/",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The newest JDK the toolchain store installed. The IDE prefers `JAVA_HOME` over its
/// managed JDK, and a language server release may need a newer JDK than the machine's.
fn managed_java_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("ARCHIPEL_SCENARIO_JAVA_HOME") {
        return Some(PathBuf::from(home));
    }
    let root = archipel_ide_toolchains::default_root().join("jdk");
    let mut versions: Vec<PathBuf> = fs::read_dir(root)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("bin").is_dir())
        .collect();
    versions.sort_by_key(|path| {
        path.file_name()
            .map(|n| {
                n.to_string_lossy()
                    .split(['.', '+'])
                    .filter_map(|p| p.parse::<u64>().ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    versions.pop()
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub pass: usize,
    pub partial: usize,
    pub fail: usize,
    pub skipped: usize,
}

impl Totals {
    pub fn of(results: &[ScenarioResult]) -> Self {
        let mut totals = Self::default();
        for result in results {
            match result.status {
                Status::Pass => totals.pass += 1,
                Status::Partial => totals.partial += 1,
                Status::Fail => totals.fail += 1,
                Status::Skipped => totals.skipped += 1,
            }
        }
        totals
    }

    pub fn run(&self) -> usize {
        self.pass + self.partial + self.fail
    }

    /// Pass counts 1, partial one half, over the scenarios that ran.
    pub fn score(&self) -> f64 {
        if self.run() == 0 {
            return 0.0;
        }
        (self.pass as f64 + self.partial as f64 / 2.0) / self.run() as f64 * 100.0
    }
}

fn icon(status: Status) -> &'static str {
    match status {
        Status::Pass => "PASS",
        Status::Partial => "PARTIAL",
        Status::Fail => "FAIL",
        Status::Skipped => "skipped",
    }
}

pub fn render_report(results: &[ScenarioResult], context: &str) -> String {
    let totals = Totals::of(results);
    let mut out = String::new();
    let _ = writeln!(out, "# Scenario report\n");
    let _ = writeln!(out, "{context}\n");
    let _ = writeln!(
        out,
        "**{} scenarios: {} pass, {} partial, {} fail, {} skipped.** Parity score {:.0}% \
         (pass = 1, partial = ½, over the {} that ran).\n",
        results.len(),
        totals.pass,
        totals.partial,
        totals.fail,
        totals.skipped,
        totals.score(),
        totals.run()
    );
    let unverified: usize = results
        .iter()
        .filter(|r| r.status != Status::Skipped)
        .flat_map(|r| &r.checks)
        .filter(|c| c.unverified)
        .count();
    if unverified > 0 {
        let _ = writeln!(
            out,
            "{unverified} expectations are marked `unverified` (not yet confirmed in IntelliJ); \
             they are listed with `(unverified)`.\n"
        );
    }
    let _ = writeln!(out, "| Scenario | Status | Time | Result |");
    let _ = writeln!(out, "|---|---|---|---|");
    for result in results {
        let summary = match result.status {
            Status::Pass => format!("{} checks", result.checks.len()),
            Status::Skipped => result.notes.first().cloned().unwrap_or_default(),
            _ => {
                let bad = result
                    .checks
                    .iter()
                    .filter(|c| c.status != Status::Pass)
                    .count();
                format!("{bad} of {} checks", result.checks.len())
            }
        };
        let _ = writeln!(
            out,
            "| `{}` {} | {} | {}s | {} |",
            result.id,
            result.title,
            icon(result.status),
            result.duration.as_secs(),
            summary.replace('|', "\\|")
        );
    }
    let details: Vec<&ScenarioResult> = results
        .iter()
        .filter(|r| matches!(r.status, Status::Partial | Status::Fail))
        .collect();
    if !details.is_empty() {
        let _ = writeln!(out, "\n## Differences from IntelliJ\n");
        for result in details {
            let _ = writeln!(out, "### `{}` ({})\n", result.id, result.status.label());
            let _ = writeln!(out, "IntelliJ: {}\n", result.intellij);
            if let Some(gap) = &result.known_gap {
                let _ = writeln!(out, "Known gap: {gap}\n");
            }
            for check in result.checks.iter().filter(|c| c.status != Status::Pass) {
                let _ = writeln!(
                    out,
                    "- **{}** {}{}: {}",
                    check.status.label(),
                    check.what,
                    if check.unverified {
                        " (unverified)"
                    } else {
                        ""
                    },
                    check.detail
                );
            }
            for note in &result.notes {
                let _ = writeln!(out, "- note: {}", note.replace('\n', " "));
            }
            let _ = writeln!(out);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Command line
// ---------------------------------------------------------------------------

struct Options {
    only: Vec<String>,
    report: PathBuf,
    work: PathBuf,
    release: bool,
    list: bool,
    install: bool,
    strict: bool,
}

fn parse_options(args: impl Iterator<Item = String>) -> Result<Options> {
    let mut options = Options {
        only: Vec::new(),
        report: PathBuf::from(DEFAULT_REPORT),
        work: PathBuf::from(DEFAULT_WORK),
        release: false,
        list: false,
        install: false,
        strict: false,
    };
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--only" => {
                let list = args.next().ok_or_else(|| anyhow!("--only needs names"))?;
                options.only = list.split(',').map(|id| id.trim().to_owned()).collect();
            }
            "--report" => {
                options.report = args
                    .next()
                    .ok_or_else(|| anyhow!("--report needs a file"))?
                    .into()
            }
            "--work" => {
                options.work = args
                    .next()
                    .ok_or_else(|| anyhow!("--work needs a directory"))?
                    .into()
            }
            "--release" => options.release = true,
            "--list" => options.list = true,
            "--install-toolchains" => options.install = true,
            "--strict" => options.strict = true,
            other => bail!("unknown scenario option {other:?}"),
        }
    }
    Ok(options)
}

/// `--only` accepts `area/name`, `name` or a whole `area`.
fn selected(scenario: &Scenario, only: &[String]) -> bool {
    only.is_empty()
        || only.iter().any(|name| {
            scenario.id == *name
                || scenario.id.rsplit('/').next() == Some(name.as_str())
                || scenario.id.split('/').next() == Some(name.as_str())
        })
}

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let options = parse_options(args)?;
    let all = load_scenarios(Path::new(SCENARIOS_DIR))?;
    let scenarios: Vec<&Scenario> = all
        .iter()
        .filter(|scenario| selected(scenario, &options.only))
        .collect();
    if options.list {
        for scenario in &scenarios {
            println!("{:<40} {}", scenario.id, scenario.spec.title);
        }
        return Ok(());
    }
    if scenarios.is_empty() {
        bail!("no scenarios selected");
    }
    let built = build_app(options.release)?;
    let display = Display::acquire()?;
    let cwd = std::env::current_dir()?;
    let work = cwd.join(&options.work);
    fs::create_dir_all(&work)?;
    // The target directory may be shared with other checkouts that overwrite the binary
    // while the scenarios run: run a private copy.
    let app = work.join(format!("archipel-ide{}", std::env::consts::EXE_SUFFIX));
    fs::copy(&built, &app).with_context(|| format!("copy {}", built.display()))?;
    let mut env = Environment {
        app,
        display,
        out: work.join("out"),
        work,
        java_home: None,
    };
    fs::create_dir_all(&env.out)?;

    let needs: BTreeSet<String> = scenarios
        .iter()
        .flat_map(|s| s.spec.requires.iter().cloned())
        .collect();
    let available = if needs.is_empty() {
        BTreeMap::new()
    } else {
        probe(&env, &needs, options.install)?
    };
    if needs.contains("jdtls") {
        env.java_home = managed_java_home();
    }

    let mut results = Vec::new();
    for scenario in scenarios {
        let missing: Vec<&String> = scenario
            .spec
            .requires
            .iter()
            .filter(|need| !available.get(*need).copied().unwrap_or(false))
            .collect();
        print!("{} … ", scenario.id);
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let result = if missing.is_empty() {
            run_scenario(&env, scenario)
        } else {
            ScenarioResult {
                id: scenario.id.clone(),
                title: scenario.spec.title.clone(),
                status: Status::Skipped,
                checks: Vec::new(),
                notes: vec![format!(
                    "requires {} (not available)",
                    missing
                        .iter()
                        .map(|m| m.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )],
                duration: Duration::ZERO,
                known_gap: None,
                intellij: String::new(),
            }
        };
        println!("{} ({}s)", result.status.label(), result.duration.as_secs());
        results.push(result);
    }

    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    let context = format!(
        "Archipel IDE commit `{}`, headless (Xvfb, glow). Generated by `cargo xtask scenario`.",
        commit.trim()
    );
    let report = render_report(&results, &context);
    if let Some(parent) = options
        .report
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&options.report, &report)?;
    let json = results
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "status": r.status.label(),
                "seconds": r.duration.as_secs(),
                "checks": r.checks.iter().map(|c| json!({
                    "status": c.status.label(), "what": c.what, "detail": c.detail, "unverified": c.unverified,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        options.report.with_extension("json"),
        serde_json::to_string_pretty(&json!({"scenarios": json, "totals": {
            "pass": Totals::of(&results).pass, "partial": Totals::of(&results).partial,
            "fail": Totals::of(&results).fail, "skipped": Totals::of(&results).skipped,
        }}))?,
    )?;
    println!("\n{report}");
    println!("report written to {}", options.report.display());
    let totals = Totals::of(&results);
    if options.strict && (totals.fail > 0 || totals.partial > 0) {
        bail!(
            "{} scenario(s) fail and {} differ partially from IntelliJ",
            totals.fail,
            totals.partial
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markup(items: Value) -> Value {
        json!({"markup": items})
    }

    fn item(severity: &str, range: (u64, u64, u64, u64), message: &str) -> Value {
        json!({"severity": severity, "message": message, "unnecessary": false,
            "range": {"line": range.0, "column": range.1, "endLine": range.2, "endColumn": range.3}})
    }

    fn expect(range: &str, severity: &str, message: Option<&str>) -> MarkupExpect {
        MarkupExpect {
            when: None,
            range: range.into(),
            severity: severity.into(),
            message: message.map(Into::into),
            confidence: None,
        }
    }

    #[test]
    fn ranges_parse_in_both_forms() {
        assert_eq!(
            Range::parse("9:9-9:19").unwrap(),
            Range {
                line: 9,
                column: 9,
                end_line: 9,
                end_column: 19
            }
        );
        assert_eq!(Range::parse("3:1-12").unwrap().end_line, 3);
        assert!(Range::parse("3-4").is_err());
        assert!(Range::parse("a:b-c:d").is_err());
    }

    #[test]
    fn markup_passes_partially_or_fails() {
        let observed = markup(json!([item(
            "error",
            (9, 9, 9, 19),
            "The method is undefined"
        )]));
        let exact = check_markup(
            &expect("9:9-9:19", "error", Some("The method is undefined")),
            Some(&observed),
        );
        assert_eq!(exact.status, Status::Pass);
        let other_message = check_markup(
            &expect("9:9-9:19", "error", Some("Cannot resolve method")),
            Some(&observed),
        );
        assert_eq!(other_message.status, Status::Partial);
        assert!(other_message.detail.contains("The method is undefined"));
        let no_message = check_markup(&expect("9:9-9:19", "error", None), Some(&observed));
        assert_eq!(no_message.status, Status::Pass);
        let wrong_range = check_markup(&expect("9:9-9:20", "error", None), Some(&observed));
        assert_eq!(wrong_range.status, Status::Fail);
        let wrong_severity = check_markup(&expect("9:9-9:19", "warning", None), Some(&observed));
        assert_eq!(wrong_severity.status, Status::Fail);
        assert_eq!(
            check_markup(&expect("9:9-9:19", "error", None), None).status,
            Status::Fail
        );
    }

    #[test]
    fn unused_code_is_a_partial_warning() {
        let mut unused = item("hint", (3, 1, 3, 20), "The import is never used");
        unused["unnecessary"] = json!(true);
        let result = check_markup(
            &expect("3:1-3:20", "warning", Some("Unused import statement")),
            Some(&markup(json!([unused]))),
        );
        assert_eq!(result.status, Status::Partial);
    }

    #[test]
    fn no_markup_lists_offenders() {
        let observed = markup(json!([
            item("error", (1, 1, 1, 2), "bad"),
            item("warning", (2, 1, 2, 2), "meh")
        ]));
        let none = NoMarkupExpect {
            when: None,
            severity: None,
            confidence: None,
        };
        assert_eq!(check_no_markup(&none, Some(&observed)).status, Status::Fail);
        let weak = NoMarkupExpect {
            when: None,
            severity: Some("weak warning".into()),
            confidence: None,
        };
        assert_eq!(check_no_markup(&weak, Some(&observed)).status, Status::Pass);
    }

    fn intentions(mode: &str, items: &[&str]) -> IntentionsExpect {
        IntentionsExpect {
            when: None,
            mode: Some(mode.into()),
            items: items.iter().map(|s| s.to_string()).collect(),
            confidence: None,
        }
    }

    #[test]
    fn intentions_contains_ordered_exact() {
        let popup: Vec<String> = ["Import class", "Create class 'List'", "Rename reference"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            check_intentions(&intentions("contains", &["Import class"]), Some(&popup)).status,
            Status::Pass
        );
        assert_eq!(
            check_intentions(
                &intentions("contains", &["Import class", "Add library"]),
                Some(&popup)
            )
            .status,
            Status::Partial
        );
        assert_eq!(
            check_intentions(&intentions("contains", &["Add library"]), Some(&popup)).status,
            Status::Fail
        );
        assert_eq!(
            check_intentions(
                &intentions("ordered", &["Import class", "Rename reference"]),
                Some(&popup)
            )
            .status,
            Status::Pass
        );
        assert_eq!(
            check_intentions(
                &intentions("ordered", &["Rename reference", "Import class"]),
                Some(&popup)
            )
            .status,
            Status::Partial
        );
        assert_eq!(
            check_intentions(&intentions("exact", &["Import class"]), Some(&popup)).status,
            Status::Partial
        );
        assert_eq!(
            check_intentions(&intentions("contains", &["re:^Create class"]), Some(&popup)).status,
            Status::Pass
        );
        assert_eq!(
            check_intentions(&intentions("contains", &["x"]), None).status,
            Status::Fail
        );
    }

    #[test]
    fn file_checks_combine_contains_regex_and_negatives() {
        let expect = FileExpect {
            path: None,
            equals: None,
            contains: vec!["import static a.B.c;".into()],
            not_contains: vec!["TODO".into()],
            regex: Some(r"(?m)^import static".into()),
            confidence: None,
        };
        assert_eq!(
            check_file(&expect, "A.java", Ok("import static a.B.c;\n")).status,
            Status::Pass
        );
        let failed = check_file(&expect, "A.java", Ok("TODO\n"));
        assert_eq!(failed.status, Status::Fail);
        assert!(failed.detail.contains("missing"));
        assert_eq!(
            check_file(&expect, "A.java", Err("gone")).status,
            Status::Fail
        );
    }

    #[test]
    fn navigation_is_partial_when_only_the_position_differs() {
        let expect = NavigationExpect {
            when: None,
            path: Some("A.java".into()),
            path_contains: None,
            line: Some(3),
            column: None,
            confidence: None,
        };
        let right = json!({"path": "A.java", "line": 3, "column": 5});
        assert_eq!(check_navigation(&expect, Some(&right)).status, Status::Pass);
        let line = json!({"path": "A.java", "line": 4, "column": 5});
        assert_eq!(
            check_navigation(&expect, Some(&line)).status,
            Status::Partial
        );
        let file = json!({"path": "B.java", "line": 3, "column": 5});
        assert_eq!(check_navigation(&expect, Some(&file)).status, Status::Fail);
    }

    #[test]
    fn usages_and_completion() {
        let usages = json!({"items": [{"path": "src/A.java"}, {"path": "src/B.java"}]});
        let ok = UsagesExpect {
            when: None,
            count: Some(2),
            at_least: None,
            paths: vec!["B.java".into()],
            confidence: None,
        };
        assert_eq!(check_usages(&ok, Some(&usages)).status, Status::Pass);
        let few = UsagesExpect {
            when: None,
            count: Some(3),
            at_least: None,
            paths: vec![],
            confidence: None,
        };
        assert_eq!(check_usages(&few, Some(&usages)).status, Status::Partial);
        let popup = json!({"items": [{"label": "toUpperCase()"}, {"label": "toLowerCase()"}]});
        let wanted = CompletionExpect {
            when: None,
            contains: vec!["toUpperCase".into()],
            first: None,
            confidence: None,
        };
        assert_eq!(check_completion(&wanted, Some(&popup)).status, Status::Pass);
        let missing = CompletionExpect {
            when: None,
            contains: vec!["isEmpty".into()],
            first: None,
            confidence: None,
        };
        assert_eq!(
            check_completion(&missing, Some(&popup)).status,
            Status::Fail
        );
    }

    #[test]
    fn steps_parse_ide_operations_and_scene_steps() {
        assert_eq!(
            ScenarioStep::parse(&json!({"op": "typeAt", "line": 9, "column": 9, "text": "x"}))
                .unwrap(),
            ScenarioStep::TypeAt {
                line: 9,
                column: 9,
                text: "x".into()
            }
        );
        assert!(matches!(
            ScenarioStep::parse(&json!({"op": "key", "chord": "Escape"})).unwrap(),
            ScenarioStep::Scene(_)
        ));
        assert!(ScenarioStep::parse(&json!({"op": "caret", "line": 0, "column": 1})).is_err());
        assert!(ScenarioStep::parse(&json!({"op": "applyIntention"})).is_err());
        assert!(
            ScenarioStep::parse(&json!({"op": "applyIntention", "title": "a", "index": 1}))
                .is_err()
        );
    }

    #[test]
    fn report_scores_pass_and_partial() {
        let result = |status| ScenarioResult {
            id: "a/b".into(),
            title: "t".into(),
            status,
            checks: vec![CheckResult::new(status, "markup", "detail")],
            notes: Vec::new(),
            duration: Duration::from_secs(2),
            known_gap: None,
            intellij: "IntelliJ".into(),
        };
        let results = vec![
            result(Status::Pass),
            result(Status::Partial),
            result(Status::Fail),
            result(Status::Skipped),
        ];
        let totals = Totals::of(&results);
        assert_eq!(
            (totals.pass, totals.partial, totals.fail, totals.skipped),
            (1, 1, 1, 1)
        );
        assert!((totals.score() - 50.0).abs() < 1e-9);
        let report = render_report(&results, "context");
        assert!(report.contains("| `a/b` t | PARTIAL |"));
        assert!(report.contains("## Differences from IntelliJ"));
    }

    #[test]
    fn selection_matches_id_name_and_area() {
        let scenario = Scenario {
            id: "java/unresolved-static-import".into(),
            dir: PathBuf::new(),
            spec: serde_json::from_value(json!({
                "title": "t", "steps": [], "intellij": {"version": "v", "action": "a"}
            }))
            .unwrap(),
            steps: Vec::new(),
        };
        assert!(selected(&scenario, &[]));
        assert!(selected(&scenario, &["java".into()]));
        assert!(selected(&scenario, &["unresolved-static-import".into()]));
        assert!(selected(
            &scenario,
            &["java/unresolved-static-import".into()]
        ));
        assert!(!selected(&scenario, &["kotlin".into()]));
    }

    #[test]
    fn checked_in_scenarios_are_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(SCENARIOS_DIR);
        let scenarios = load_scenarios(&root).unwrap();
        assert!(scenarios.len() >= 30, "{} scenarios", scenarios.len());
        let mut ids = BTreeSet::new();
        for scenario in &scenarios {
            assert!(ids.insert(scenario.id.clone()), "duplicate {}", scenario.id);
            let expect = &scenario.spec.expect;
            let checks = expect.markup.len()
                + expect.no_markup.len()
                + expect.intentions.len()
                + expect.file.len()
                + expect.navigation.len()
                + expect.usages.len()
                + expect.completion.len()
                + expect.state.len()
                + expect.query.len();
            assert!(checks > 0, "{} expects nothing", scenario.id);
            for e in &expect.markup {
                Range::parse(&e.range).unwrap_or_else(|err| panic!("{}: {err}", scenario.id));
            }
            for e in &expect.file {
                if let Some(path) = &e.path {
                    assert!(
                        scenario.dir.join("project").join(path).exists()
                            || e.contains.len() + e.not_contains.len() > 0,
                        "{}: {path}",
                        scenario.id
                    );
                }
            }
            for need in &scenario.spec.requires {
                assert!(
                    [
                        "jdk",
                        "jdtls",
                        "gradle",
                        "maven",
                        "kotlin-lsp",
                        "rust-analyzer",
                        "bazelisk",
                        "network"
                    ]
                    .contains(&need.as_str()),
                    "{}: unknown requirement {need}",
                    scenario.id
                );
            }
        }
    }
}
