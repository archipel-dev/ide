//! State of the Build tool window: the `Sync` and `Build Output` tabs keep
//! their last finished result, and the console toolbar (soft-wrap, scroll to
//! end, clear) acts on the shown console.

use crate::services::{ExecutionListener, ExecutionManager};
use crate::{CommandError, IdeState, spec};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_runner::{BuildEvents, ConsoleBuffer, TaskKind, TaskPlan, TaskState};
use serde_json::{Value, json};
use std::sync::{Arc, Weak};
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

    /// Asks the console to scroll to its last line.
    pub fn request_scroll_to_end(&mut self) {
        self.scroll_to_end = true;
    }

    /// Hides the `hidden` console lines currently shown by the selected tab.
    pub fn clear_shown(&mut self, hidden: usize) {
        self.cleared[self.tab.slot()] = hidden;
    }
}

/// Makes the Build tool window follow the runner: a new run selects its tab with a fresh
/// console, a finished run is kept as the tab's snapshot. Subscribed to
/// [`EXECUTION_TOPIC`](crate::services::EXECUTION_TOPIC) next to the manager it reads.
pub(crate) struct BuildWindowFollower(pub(crate) Weak<ExecutionManager>);

impl ExecutionListener for BuildWindowFollower {
    fn process_started(&self, plan: &TaskPlan, _run_id: u64) {
        let (Some(manager), Some(tab)) = (self.0.upgrade(), BuildTab::of_kind(plan.kind)) else {
            return;
        };
        let mut window = manager.build_window_mut();
        window.tab = tab;
        window.cleared[tab.slot()] = 0;
    }

    fn process_terminated(&self, plan: &TaskPlan, state: &TaskState, _run_id: u64) {
        let (Some(manager), Some(tab)) = (self.0.upgrade(), BuildTab::of_kind(plan.kind)) else {
            return;
        };
        let snapshot = {
            let runner = manager.runner();
            BuildSnapshot {
                plan: plan.clone(),
                state: state.clone(),
                events: runner.events().clone(),
                console: runner.console().clone(),
                elapsed: runner.elapsed(),
            }
        };
        manager.build_window_mut().snapshots[tab.slot()] = Some(Arc::new(snapshot));
    }
}

impl IdeState {
    /// The tab whose content is the running (or last started) task, if it
    /// is a build or sync.
    pub fn live_build_tab(&self) -> Option<BuildTab> {
        self.execution()
            .runner()
            .plan()
            .and_then(|plan| BuildTab::of_kind(plan.kind))
    }

    fn shown_console_len(&self) -> usize {
        let len = |console: &ConsoleBuffer| console.dropped() + console.lines().len();
        let execution = self.execution();
        let tab = execution.build_window().tab;
        if self.live_build_tab() == Some(tab) {
            len(execution.runner().console())
        } else {
            let snapshot = execution.build_window().snapshot(tab);
            snapshot.map_or(0, |snapshot| len(&snapshot.console))
        }
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
                state.execution().build_window_mut().tab = tab;
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
                let execution = state.execution();
                let mut window = execution.build_window_mut();
                window.soft_wrap = !window.soft_wrap;
                Ok(json!({ "softWrap": window.soft_wrap }))
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
                state.execution().build_window_mut().request_scroll_to_end();
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
                state.execution().build_window_mut().clear_shown(hidden);
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
        state.execution().start(plan).expect("start");
        for _ in 0..500 {
            state.poll_runner();
            if !state.execution().is_running() {
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
        assert_eq!(state.execution().build_window().tab, BuildTab::Sync);
        run(&mut state, "echo", &["building"], TaskKind::Build);
        assert_eq!(state.execution().build_window().tab, BuildTab::Output);
        let sync = state
            .execution()
            .build_window()
            .snapshot(BuildTab::Sync)
            .expect("sync kept");
        assert!(sync.console.lines().iter().any(|l| l.text == "syncing"));
        let output = state
            .execution()
            .build_window()
            .snapshot(BuildTab::Output)
            .expect("output kept");
        assert!(output.console.lines().iter().any(|l| l.text == "building"));

        commands
            .execute("build.showTab", &mut state, json!({"tab": "sync"}))
            .unwrap();
        assert_eq!(state.execution().build_window().tab, BuildTab::Sync);
        assert!(
            commands
                .execute("build.showTab", &mut state, json!({"tab": "nope"}))
                .is_err()
        );

        commands
            .execute("build.console.clear", &mut state, json!({}))
            .unwrap();
        assert_eq!(state.execution().build_window().cleared(BuildTab::Sync), 1);
        assert_eq!(
            state.execution().build_window().cleared(BuildTab::Output),
            0
        );
        commands
            .execute("build.console.softWrap", &mut state, json!({}))
            .unwrap();
        assert!(state.execution().build_window().soft_wrap);
        commands
            .execute("build.console.scrollToEnd", &mut state, json!({}))
            .unwrap();
        assert!(state.execution().build_window_mut().take_scroll_to_end());
        assert!(!state.execution().build_window_mut().take_scroll_to_end());

        // A new run of a tab starts with a fresh console.
        run(&mut state, "echo", &["again"], TaskKind::Sync);
        assert_eq!(state.execution().build_window().cleared(BuildTab::Sync), 0);
    }
}
