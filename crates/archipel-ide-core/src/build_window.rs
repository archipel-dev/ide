//! State of the Build tool window: the `Sync` and `Build Output` tabs keep
//! their last finished result, and the console toolbar (soft-wrap, scroll to
//! end, clear) acts on the shown console.

use crate::{CommandError, IdeState, spec};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_runner::{BuildEvents, ConsoleBuffer, TaskKind, TaskPlan, TaskState};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// A tab of the Build tool window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildTab {
    Sync,
    #[default]
    Output,
}

impl BuildTab {
    pub const ALL: [BuildTab; 2] = [BuildTab::Sync, BuildTab::Output];

    pub fn title(self) -> &'static str {
        match self {
            BuildTab::Sync => "Sync",
            BuildTab::Output => "Build Output",
        }
    }

    fn slot(self) -> usize {
        self as usize
    }

    /// The tab a task of `kind` reports to; Run and Test have their own layout.
    pub fn of_kind(kind: TaskKind) -> Option<Self> {
        match kind {
            TaskKind::Sync => Some(BuildTab::Sync),
            TaskKind::Build | TaskKind::Custom => Some(BuildTab::Output),
            TaskKind::Run | TaskKind::Test => None,
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name {
            "sync" => Some(BuildTab::Sync),
            "output" => Some(BuildTab::Output),
            _ => None,
        }
    }
}

/// The last finished run of a tab.
#[derive(Debug, Clone)]
pub struct BuildSnapshot {
    pub plan: TaskPlan,
    pub state: TaskState,
    pub events: BuildEvents,
    pub console: ConsoleBuffer,
    pub elapsed: Option<Duration>,
}

#[derive(Debug, Default)]
pub struct BuildWindowState {
    pub tab: BuildTab,
    pub soft_wrap: bool,
    /// Set by Scroll to End; the console consumes it with [`Self::take_scroll_to_end`].
    scroll_to_end: bool,
    /// Console lines hidden by Clear, per tab.
    cleared: [usize; 2],
    snapshots: [Option<Arc<BuildSnapshot>>; 2],
    captured: [Option<u64>; 2],
    seen_run: u64,
}

impl BuildWindowState {
    pub fn snapshot(&self, tab: BuildTab) -> Option<Arc<BuildSnapshot>> {
        self.snapshots[tab.slot()].clone()
    }

    /// Console lines of `tab` hidden by Clear.
    pub fn cleared(&self, tab: BuildTab) -> usize {
        self.cleared[tab.slot()]
    }

    pub fn take_scroll_to_end(&mut self) -> bool {
        std::mem::take(&mut self.scroll_to_end)
    }
}

impl IdeState {
    /// The tab whose content is the running (or last started) task, if it
    /// is a build or sync.
    pub fn live_build_tab(&self) -> Option<BuildTab> {
        self.runner
            .plan()
            .and_then(|plan| BuildTab::of_kind(plan.kind))
    }

    /// Follows the runner: a new run selects its tab, a finished run is kept.
    pub(crate) fn update_build_window(&mut self) {
        let run = self.runner.run_id();
        let Some(tab) = self.live_build_tab() else {
            self.build_window.seen_run = run;
            return;
        };
        let window = &mut self.build_window;
        if window.seen_run != run {
            window.seen_run = run;
            window.tab = tab;
            window.cleared[tab.slot()] = 0;
        }
        if !self.runner.is_running()
            && self.runner.state() != &TaskState::Idle
            && window.captured[tab.slot()] != Some(run)
            && let Some(plan) = self.runner.plan()
        {
            window.captured[tab.slot()] = Some(run);
            window.snapshots[tab.slot()] = Some(Arc::new(BuildSnapshot {
                plan: plan.clone(),
                state: self.runner.state().clone(),
                events: self.runner.events().clone(),
                console: self.runner.console().clone(),
                elapsed: self.runner.elapsed(),
            }));
        }
    }

    fn shown_console_len(&self) -> usize {
        let tab = self.build_window.tab;
        let console = if self.live_build_tab() == Some(tab) {
            Some(self.runner.console())
        } else {
            self.build_window.snapshots[tab.slot()]
                .as_ref()
                .map(|snapshot| &snapshot.console)
        };
        console.map_or(0, |console| console.dropped() + console.lines().len())
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "build.showTab",
                "Show Build Tool Window Tab",
                "Build",
                None,
                r#"{"type":"object","required":["tab"],"properties":{"tab":{"enum":["sync","output"]}}}"#,
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, params: Value| {
                let tab = params["tab"]
                    .as_str()
                    .and_then(BuildTab::parse)
                    .ok_or_else(|| CommandError::InvalidParams("tab must be sync or output".into()))?;
                state.build_window.tab = tab;
                Ok(json!({ "tab": tab.title() }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "build.console.softWrap",
                "Soft-Wrap Build Console",
                "Build",
                None,
                "{}",
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, _| {
                state.build_window.soft_wrap = !state.build_window.soft_wrap;
                Ok(json!({ "softWrap": state.build_window.soft_wrap }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "build.console.scrollToEnd",
                "Scroll Build Console to the End",
                "Build",
                None,
                "{}",
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, _| {
                state.build_window.scroll_to_end = true;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "build.console.clear",
                "Clear Build Console",
                "Build",
                None,
                "{}",
            ),
            |state: &IdeState| state.shown_console_len() > 0,
            |state: &mut IdeState, _| {
                let hidden = state.shown_console_len();
                let tab = state.build_window.tab;
                state.build_window.cleared[tab.slot()] = hidden;
                Ok(json!({ "cleared": hidden }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn run(state: &mut IdeState, program: &str, args: &[&str], kind: TaskKind) {
        let mut plan = TaskPlan::new("job", program, args.iter().map(|a| a.to_string()), ".");
        plan.kind = kind;
        state.runner.start(plan).expect("start");
        for _ in 0..500 {
            state.poll_runner();
            if !state.runner.is_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        state.poll_runner();
    }

    #[test]
    fn tabs_keep_their_last_result_and_the_console_clears() {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        run(&mut state, "echo", &["syncing"], TaskKind::Sync);
        assert_eq!(state.build_window.tab, BuildTab::Sync);
        run(&mut state, "echo", &["building"], TaskKind::Build);
        assert_eq!(state.build_window.tab, BuildTab::Output);
        let sync = state
            .build_window
            .snapshot(BuildTab::Sync)
            .expect("sync kept");
        assert!(sync.console.lines().iter().any(|l| l.text == "syncing"));
        let output = state
            .build_window
            .snapshot(BuildTab::Output)
            .expect("output kept");
        assert!(output.console.lines().iter().any(|l| l.text == "building"));

        commands
            .execute("build.showTab", &mut state, json!({"tab": "sync"}))
            .unwrap();
        assert_eq!(state.build_window.tab, BuildTab::Sync);
        assert!(
            commands
                .execute("build.showTab", &mut state, json!({"tab": "nope"}))
                .is_err()
        );

        commands
            .execute("build.console.clear", &mut state, json!({}))
            .unwrap();
        assert_eq!(state.build_window.cleared(BuildTab::Sync), 1);
        assert_eq!(state.build_window.cleared(BuildTab::Output), 0);
        commands
            .execute("build.console.softWrap", &mut state, json!({}))
            .unwrap();
        assert!(state.build_window.soft_wrap);
        commands
            .execute("build.console.scrollToEnd", &mut state, json!({}))
            .unwrap();
        assert!(state.build_window.take_scroll_to_end());
        assert!(!state.build_window.take_scroll_to_end());

        // A new run of a tab starts with a fresh console.
        run(&mut state, "echo", &["again"], TaskKind::Sync);
        assert_eq!(state.build_window.cleared(BuildTab::Sync), 0);
    }
}
