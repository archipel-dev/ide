//! The editor gutter's content that both frontends show (M1): the run icons with their
//! debounce, the popup of a run icon (Run X, Debug X, Run X with Coverage, Modify Run
//! Configuration), and the line breakpoints with their variants.
//!
//! The GUI paints these marks as vector icons, the terminal as Nerd Font glyphs in its two
//! gutter lanes; neither decides which line has which mark or what the popup says.
//! Specs: EXE-CTX-050..053 (run markers), UI-ED-012 (gutter columns), TUI-GUI-090, TUI-GUI-091.

use crate::ViewCx;
use crate::toolbar::command_row;
use archipel_ide_core::{GutterMark, IdeState, RunnableKind};
use archipel_kit_model::{Glyph, MenuEntry, MenuItem};
use serde_json::json;
use std::path::Path;

/// How long after the last edit the previous run icons stay: finding runnables walks the whole
/// syntax tree, too slow for every keystroke in a large file.
pub const RUN_ICONS_STALE_FOR: f64 = 0.25;

/// The glyph of a run icon: a play triangle for a main or a single test, the "run all" arrow
/// for a test class or module (EXE-CTX-051).
pub fn run_glyph(kind: RunnableKind) -> Glyph {
    match kind {
        RunnableKind::Main | RunnableKind::Test => Glyph::Run,
        RunnableKind::TestContainer => Glyph::Rerun,
    }
}

/// The run icons of the active document, refreshed once typing pauses (the debounce both
/// frontends share).
#[derive(Debug, Clone)]
pub struct RunMarks {
    seen: Option<u64>,
    last_edit: f64,
}

impl Default for RunMarks {
    fn default() -> Self {
        Self {
            seen: None,
            last_edit: f64::NEG_INFINITY,
        }
    }
}

impl RunMarks {
    /// The icons to show at time `now` (seconds, any epoch) and whether they are the
    /// previous ones kept while typing (the caller shows a frame again after
    /// [`RUN_ICONS_STALE_FOR`]).
    pub fn marks(&mut self, state: &mut IdeState, now: f64) -> (Vec<GutterMark>, bool) {
        let revision = state.active_doc().map(|document| document.text.revision());
        if self.seen != revision {
            self.last_edit = now;
            self.seen = revision;
        }
        if now - self.last_edit < RUN_ICONS_STALE_FOR
            && let Some(marks) = state.cached_gutter_marks()
        {
            return (marks, true);
        }
        (state.gutter_marks(), false)
    }
}

/// The mark on `line`.
pub fn mark_at(marks: &[GutterMark], line: usize) -> Option<&GutterMark> {
    marks.iter().find(|mark| mark.line == line)
}

/// The rows of the popup of the run icon on `line` of the active document: the executor
/// actions in registration order, a separator, then Modify Run Configuration (EXE-CTX-053).
/// Empty when the line has no icon. Choosing a row is the command of its id.
pub fn run_popup(cx: &ViewCx, line: usize) -> Vec<MenuEntry> {
    let Some(marks) = cx.state.cached_gutter_marks() else {
        return Vec::new();
    };
    let Some(mark) = mark_at(&marks, line) else {
        return Vec::new();
    };
    let target = mark.label.trim_start_matches("Run ").to_owned();
    let running = cx.state.execution().is_running();
    let debugging = cx.state.is_debugging();
    let shortcut = |action: &str| cx.shortcut(action);

    let mut run = MenuItem::new(
        command_row("run.gutter", &json!({"line": line, "edit": false})),
        mark.label.clone(),
    )
    .icon(Glyph::Run)
    .enabled(!running);
    run.shortcut = shortcut("RunClass");
    if running {
        run.disabled_reason = Some("Something is already running".into());
    }

    let mut debug = MenuItem::new(
        command_row(
            "run.gutter",
            &json!({"line": line, "edit": false, "debug": true}),
        ),
        format!("Debug {target}"),
    )
    .icon(Glyph::Debug)
    .enabled(!debugging && !running);
    // The Debug action's shortcut, as the GUI always showed.
    debug.shortcut = shortcut("Debug");
    if debugging {
        debug.disabled_reason = Some("A debug session is already running".into());
    } else if running {
        debug.disabled_reason = Some("Something is already running".into());
    }

    let mut coverage =
        MenuItem::new("gutter.coverage", format!("Run {target} with Coverage")).enabled(false);
    coverage.disabled_reason = Some("Code coverage is not available yet (IJ-RUN-069)".into());

    let mut modify = MenuItem::new(
        command_row("run.gutter", &json!({"line": line, "edit": true})),
        "Modify Run Configuration…",
    )
    .enabled(!running);
    if running {
        modify.disabled_reason = Some("Something is already running".into());
    }

    vec![
        MenuEntry::Item(run),
        MenuEntry::Item(debug),
        MenuEntry::Item(coverage),
        MenuEntry::separator(),
        MenuEntry::Item(modify),
    ]
}

/// What kind of line breakpoint a mark is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointKind {
    /// Suspends on every hit.
    Line,
    /// Has a condition or a hit count.
    Conditional,
    /// Has a log message: prints instead of suspending.
    Logpoint,
}

/// A breakpoint mark of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakpointMark {
    /// The zero-based line.
    pub line: usize,
    /// Its kind.
    pub kind: BreakpointKind,
}

/// The line breakpoints of the document at project-relative `path`, with their kinds, in line
/// order.
pub fn breakpoint_marks(state: &IdeState, path: &Path) -> Vec<BreakpointMark> {
    let mut lines = state.breakpoint_lines(path);
    lines.sort_unstable();
    lines
        .into_iter()
        .map(|line| {
            let kind = match state.breakpoint_options(path, line) {
                Some(options) if options.log_message.is_some() => BreakpointKind::Logpoint,
                Some(options) if options.condition.is_some() || options.hit_condition.is_some() => {
                    BreakpointKind::Conditional
                }
                _ => BreakpointKind::Line,
            };
            BreakpointMark { line, kind }
        })
        .collect()
}
