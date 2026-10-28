//! The pass scheduler: a deterministic state machine (no threads, no system
//! clock) that decides which pass runs on which file and when.
//!
//! Rules (spec 1.7):
//! 1. A *trigger* (`SyntaxChanged`, `IndexUpdated`, ..., caret, visible range)
//!    marks the passes that listen to it *pending* and cancels their running
//!    runs (cancel-on-typing). Passes that do not listen are untouched.
//! 2. Pending passes become due after the debounce (default 300 ms); *inline*
//!    passes (`Syntax`) have none. Every new trigger restarts the debounce.
//! 3. A pass runs only when every prerequisite (`Pass::after`) of the same
//!    file is settled (neither pending, running nor half done).
//! 4. Visible-range first: a `visible_first` pass is requested twice, first
//!    for the visible range, then (as soon as the first run completed) for
//!    the whole file; the first result is applied before the second starts.
//! 5. Runs carry the pass's generation; a completion for an older generation
//!    is reported as not current and changes no state.
//!
//! The [`crate::Daemon`] feeds it events, executes the [`RunRequest`]s and
//! reports completions.

use crate::clock::Clock;
use archipel_ide_analysis::daemon::PassId;
use archipel_ide_analysis::{CancelToken, FileId};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

/// Why passes are restarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trigger {
    SyntaxChanged,
    IndexUpdated,
    InterfaceChanged,
    SettingsChanged,
    /// Fresh diagnostics of an external provider (LSP, linters).
    DiagnosticsPublished,
    /// The editor scrolled.
    VisibleRangeChanged,
    /// The caret moved (only caret-driven passes react).
    CaretChanged,
}

/// A set of [`Trigger`]s a pass listens to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Triggers(u8);

impl Triggers {
    pub const NONE: Triggers = Triggers(0);
    pub const SYNTAX: Triggers = Triggers(1);
    pub const INDEX: Triggers = Triggers(2);
    pub const INTERFACE: Triggers = Triggers(4);
    pub const SETTINGS: Triggers = Triggers(8);
    pub const DIAGNOSTICS: Triggers = Triggers(16);
    pub const VISIBLE: Triggers = Triggers(32);
    pub const CARET: Triggers = Triggers(64);
    /// What ordinary content passes listen to.
    pub const CONTENT: Triggers = Triggers(1 | 2 | 4 | 8);

    pub const fn union(self, other: Triggers) -> Triggers {
        Triggers(self.0 | other.0)
    }

    pub const fn contains(self, trigger: Trigger) -> bool {
        self.0 & Triggers::of(trigger).0 != 0
    }

    pub const fn of(trigger: Trigger) -> Triggers {
        match trigger {
            Trigger::SyntaxChanged => Triggers::SYNTAX,
            Trigger::IndexUpdated => Triggers::INDEX,
            Trigger::InterfaceChanged => Triggers::INTERFACE,
            Trigger::SettingsChanged => Triggers::SETTINGS,
            Trigger::DiagnosticsPublished => Triggers::DIAGNOSTICS,
            Trigger::VisibleRangeChanged => Triggers::VISIBLE,
            Trigger::CaretChanged => Triggers::CARET,
        }
    }
}

/// What the scheduler needs to know about a pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassSpec {
    pub id: PassId,
    pub after: Vec<PassId>,
    pub visible_first: bool,
    /// Runs synchronously on the edit thread, without debounce.
    pub inline: bool,
    pub triggers: Triggers,
    /// Quiet time of this pass instead of [`SchedulerConfig::debounce`].
    pub debounce: Option<Duration>,
}

impl PassSpec {
    pub fn new(id: PassId) -> Self {
        Self {
            id,
            after: Vec::new(),
            visible_first: false,
            inline: false,
            triggers: Triggers::CONTENT,
            debounce: None,
        }
    }

    pub fn after(mut self, after: &[PassId]) -> Self {
        self.after = after.to_vec();
        self
    }

    pub fn visible_first(mut self) -> Self {
        self.visible_first = true;
        self
    }

    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    pub fn debounce(mut self, debounce: Option<Duration>) -> Self {
        self.debounce = debounce;
        self
    }

    pub fn listens(mut self, triggers: Triggers) -> Self {
        self.triggers = triggers;
        self
    }
}

/// Timing knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    /// Quiet time after the last change before non-inline passes run.
    pub debounce: Duration,
    /// Quiet time after a caret move (caret-driven passes).
    pub caret_debounce: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(300),
            caret_debounce: Duration::from_millis(50),
        }
    }
}

/// Which part of the file a run covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// The visible range of a `visible_first` pass.
    Visible,
    /// The whole file after the visible run.
    Rest,
    /// The whole file, no visible run before (single-run passes).
    Whole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunId(pub u64);

/// One run to execute.
#[derive(Debug, Clone)]
pub struct RunRequest {
    pub id: RunId,
    pub file: FileId,
    pub pass: PassId,
    pub phase: Phase,
    /// Generation of the pass when the run started.
    pub generation: u64,
    pub inline: bool,
    /// The visible range known when the run was requested.
    pub visible: Option<Range<u32>>,
    pub token: CancelToken,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Completed,
    /// Stopped by cancellation; the pass is retried if the run was current.
    Cancelled,
    /// The pass failed (not retried until the next trigger).
    Failed,
}

/// Answer to [`Scheduler::complete`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Completion {
    /// The run belonged to the pass's current generation: its result may be
    /// applied as is. Otherwise it is stale (apply only if range-mappable).
    pub current: bool,
}

/// Per-pass counters for the debug panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PassStats {
    pub runs: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub total_time: Duration,
    pub last_time: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Pending {
        due: Duration,
    },
    Running {
        run: RunId,
        phase: Phase,
    },
    /// The visible half finished; the rest is requested by the next poll.
    VisibleDone,
    Done,
}

#[derive(Debug)]
struct PassState {
    spec: PassSpec,
    state: State,
    generation: u64,
    token: CancelToken,
}

#[derive(Debug, Default)]
struct FileState {
    passes: Vec<PassState>,
    visible: Option<Range<u32>>,
}

#[derive(Debug, Clone, Copy)]
struct RunInfo {
    file: FileId,
    pass: PassId,
    generation: u64,
    started: Duration,
}

/// See the module docs.
pub struct Scheduler {
    clock: Arc<dyn Clock>,
    config: SchedulerConfig,
    files: HashMap<FileId, FileState>,
    runs: HashMap<RunId, RunInfo>,
    next_run: u64,
    stats: HashMap<PassId, PassStats>,
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduler")
            .field("files", &self.files.len())
            .field("runs", &self.runs.len())
            .finish()
    }
}

/// Orders `specs` so prerequisites come first (stable; cycles and unknown
/// prerequisites keep registration order).
pub fn order_passes(mut specs: Vec<PassSpec>) -> Vec<PassSpec> {
    let mut ordered: Vec<PassSpec> = Vec::with_capacity(specs.len());
    let known: Vec<PassId> = specs.iter().map(|s| s.id).collect();
    while !specs.is_empty() {
        let placed: Vec<PassId> = ordered.iter().map(|s| s.id).collect();
        let next = specs
            .iter()
            .position(|s| {
                s.after
                    .iter()
                    .all(|a| placed.contains(a) || !known.contains(a))
            })
            .unwrap_or(0);
        ordered.push(specs.remove(next));
    }
    ordered
}

impl Scheduler {
    pub fn new(clock: Arc<dyn Clock>, config: SchedulerConfig) -> Self {
        Self {
            clock,
            config,
            files: HashMap::new(),
            runs: HashMap::new(),
            next_run: 1,
            stats: HashMap::new(),
        }
    }

    pub fn config(&self) -> SchedulerConfig {
        self.config
    }

    pub fn set_config(&mut self, config: SchedulerConfig) {
        self.config = config;
    }

    /// Starts tracking `file`; every pass is pending immediately (the first
    /// analysis of an opened file has no debounce).
    pub fn open_file(&mut self, file: FileId, specs: Vec<PassSpec>) -> Vec<RunId> {
        let cancelled = self.close_file(file);
        let now = self.clock.now();
        let passes = order_passes(specs)
            .into_iter()
            .map(|spec| PassState {
                spec,
                state: State::Pending { due: now },
                generation: 1,
                token: CancelToken::new(),
            })
            .collect();
        self.files.insert(
            file,
            FileState {
                passes,
                visible: None,
            },
        );
        cancelled
    }

    /// Replaces the pass list (language or dumb mode changed). Known passes
    /// keep their state; new ones start pending; dropped ones are cancelled.
    pub fn set_passes(&mut self, file: FileId, specs: Vec<PassSpec>) -> Vec<RunId> {
        let now = self.clock.now();
        let Some(state) = self.files.get_mut(&file) else {
            return Vec::new();
        };
        let mut old: Vec<PassState> = std::mem::take(&mut state.passes);
        let mut cancelled = Vec::new();
        let mut passes = Vec::new();
        for spec in order_passes(specs) {
            match old.iter().position(|p| p.spec.id == spec.id) {
                Some(at) => {
                    let mut kept = old.remove(at);
                    kept.spec = spec;
                    passes.push(kept);
                }
                None => passes.push(PassState {
                    spec,
                    state: State::Pending { due: now },
                    generation: 1,
                    token: CancelToken::new(),
                }),
            }
        }
        for dropped in old {
            dropped.token.cancel();
            if let State::Running { run, .. } = dropped.state {
                cancelled.push(run);
            }
        }
        state.passes = passes;
        for run in &cancelled {
            self.runs.remove(run);
        }
        cancelled
    }

    /// Stops tracking `file`; returns the runs that were cancelled.
    pub fn close_file(&mut self, file: FileId) -> Vec<RunId> {
        let mut cancelled = Vec::new();
        if let Some(state) = self.files.remove(&file) {
            for pass in state.passes {
                pass.token.cancel();
                if let State::Running { run, .. } = pass.state {
                    cancelled.push(run);
                    self.runs.remove(&run);
                }
            }
        }
        cancelled
    }

    pub fn is_tracking(&self, file: FileId) -> bool {
        self.files.contains_key(&file)
    }

    /// The visible range of the editor showing `file` (no trigger).
    pub fn set_visible(&mut self, file: FileId, range: Range<u32>) {
        if let Some(state) = self.files.get_mut(&file) {
            state.visible = Some(range);
        }
    }

    /// Applies `trigger` to `file`; returns the runs that were cancelled
    /// (the caller cancels their jobs).
    pub fn trigger(&mut self, file: FileId, trigger: Trigger) -> Vec<RunId> {
        let now = self.clock.now();
        let config = self.config;
        let Some(state) = self.files.get_mut(&file) else {
            return Vec::new();
        };
        let mut cancelled = Vec::new();
        for pass in &mut state.passes {
            if !pass.spec.triggers.contains(trigger) {
                continue;
            }
            // Cancel-on-typing: flip the token of the running run.
            pass.token.cancel();
            pass.token = CancelToken::new();
            pass.generation += 1;
            if let State::Running { run, .. } = pass.state {
                cancelled.push(run);
                if let Some(info) = self.runs.remove(&run) {
                    let stats = self.stats.entry(info.pass).or_default();
                    stats.cancelled += 1;
                }
            }
            let delay = if pass.spec.inline {
                Duration::ZERO
            } else if trigger == Trigger::CaretChanged {
                config.caret_debounce
            } else if trigger == Trigger::VisibleRangeChanged {
                Duration::ZERO
            } else {
                pass.spec.debounce.unwrap_or(config.debounce)
            };
            pass.state = State::Pending { due: now + delay };
        }
        cancelled
    }

    fn settled(state: State) -> bool {
        matches!(state, State::Done)
    }

    /// Requests that are due now.
    pub fn poll(&mut self) -> Vec<RunRequest> {
        let now = self.clock.now();
        let mut requests = Vec::new();
        let files: Vec<FileId> = {
            let mut files: Vec<FileId> = self.files.keys().copied().collect();
            files.sort();
            files
        };
        for file in files {
            let Some(state) = self.files.get_mut(&file) else {
                continue;
            };
            let visible = state.visible.clone();
            for index in 0..state.passes.len() {
                let launch = match state.passes[index].state {
                    State::VisibleDone => Some(Phase::Rest),
                    State::Pending { due } if due <= now => {
                        let blocked = state.passes[index].spec.after.iter().any(|dep| {
                            state
                                .passes
                                .iter()
                                .any(|p| p.spec.id == *dep && !Self::settled(p.state))
                        });
                        if blocked {
                            None
                        } else if state.passes[index].spec.visible_first && visible.is_some() {
                            Some(Phase::Visible)
                        } else {
                            Some(Phase::Whole)
                        }
                    }
                    _ => None,
                };
                let Some(phase) = launch else { continue };
                let id = RunId(self.next_run);
                self.next_run += 1;
                let pass = &mut state.passes[index];
                pass.state = State::Running { run: id, phase };
                self.runs.insert(
                    id,
                    RunInfo {
                        file,
                        pass: pass.spec.id,
                        generation: pass.generation,
                        started: now,
                    },
                );
                self.stats.entry(pass.spec.id).or_default().runs += 1;
                requests.push(RunRequest {
                    id,
                    file,
                    pass: pass.spec.id,
                    phase,
                    generation: pass.generation,
                    inline: pass.spec.inline,
                    visible: visible.clone(),
                    token: pass.token.child(),
                });
            }
        }
        requests
    }

    /// Reports the end of a run.
    pub fn complete(&mut self, run: RunId, outcome: RunOutcome) -> Completion {
        let now = self.clock.now();
        let Some(info) = self.runs.remove(&run) else {
            return Completion { current: false };
        };
        let elapsed = now.saturating_sub(info.started);
        let stats = self.stats.entry(info.pass).or_default();
        stats.last_time = elapsed;
        stats.total_time += elapsed;
        let Some(pass) = self
            .files
            .get_mut(&info.file)
            .and_then(|f| f.passes.iter_mut().find(|p| p.spec.id == info.pass))
        else {
            return Completion { current: false };
        };
        let current = pass.generation == info.generation
            && matches!(pass.state, State::Running { run: r, .. } if r == run);
        if !current {
            return Completion { current: false };
        }
        let phase = match pass.state {
            State::Running { phase, .. } => phase,
            _ => Phase::Whole,
        };
        match outcome {
            RunOutcome::Completed => {
                stats.completed += 1;
                pass.state = if phase == Phase::Visible {
                    State::VisibleDone
                } else {
                    State::Done
                };
            }
            RunOutcome::Cancelled => {
                stats.cancelled += 1;
                pass.token = CancelToken::new();
                pass.generation += 1;
                pass.state = State::Pending { due: now };
            }
            RunOutcome::Failed => pass.state = State::Done,
        }
        Completion { current: true }
    }

    /// When the next pending pass becomes due (`Duration::ZERO` = now).
    pub fn next_deadline(&self) -> Option<Duration> {
        let now = self.clock.now();
        self.files
            .values()
            .flat_map(|f| f.passes.iter())
            .filter_map(|p| match p.state {
                State::Pending { due } => Some(due.saturating_sub(now)),
                State::VisibleDone => Some(Duration::ZERO),
                _ => None,
            })
            .min()
    }

    /// Nothing pending or running for `file`.
    pub fn is_idle(&self, file: FileId) -> bool {
        self.files
            .get(&file)
            .is_none_or(|f| f.passes.iter().all(|p| Self::settled(p.state)))
    }

    /// Passes of `file` that have not settled (diagnostics).
    pub fn unsettled(&self, file: FileId) -> Vec<PassId> {
        self.files.get(&file).map_or_else(Vec::new, |f| {
            f.passes
                .iter()
                .filter(|p| !Self::settled(p.state))
                .map(|p| p.spec.id)
                .collect()
        })
    }

    pub fn stats(&self, pass: PassId) -> PassStats {
        self.stats.get(&pass).copied().unwrap_or_default()
    }

    pub fn running(&self) -> usize {
        self.runs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::ManualClock;

    const SYNTAX: PassId = PassId("syntax");
    const GENERAL: PassId = PassId("general");
    const INSPECT: PassId = PassId("inspect");
    const CARET: PassId = PassId("caret");
    const F: FileId = FileId(1);

    fn specs() -> Vec<PassSpec> {
        vec![
            PassSpec::new(INSPECT).after(&[GENERAL]).visible_first(),
            PassSpec::new(GENERAL).visible_first(),
            PassSpec::new(SYNTAX).inline(),
            PassSpec::new(CARET).listens(Triggers::CARET.union(Triggers::SYNTAX)),
        ]
    }

    fn setup() -> (Arc<ManualClock>, Scheduler) {
        let clock = Arc::new(ManualClock::new());
        let mut scheduler = Scheduler::new(clock.clone(), SchedulerConfig::default());
        scheduler.open_file(F, specs());
        scheduler.set_visible(F, 0..100);
        (clock, scheduler)
    }

    fn ids(requests: &[RunRequest]) -> Vec<(&'static str, Phase)> {
        requests.iter().map(|r| (r.pass.0, r.phase)).collect()
    }

    fn finish(scheduler: &mut Scheduler, requests: &[RunRequest]) {
        for request in requests {
            scheduler.complete(request.id, RunOutcome::Completed);
        }
    }

    fn settle(scheduler: &mut Scheduler) {
        for _ in 0..10 {
            let requests = scheduler.poll();
            if requests.is_empty() {
                break;
            }
            finish(scheduler, &requests);
        }
    }

    #[test]
    fn passes_are_ordered_by_prerequisites() {
        let ordered: Vec<_> = order_passes(specs()).into_iter().map(|s| s.id.0).collect();
        assert_eq!(ordered, ["general", "inspect", "syntax", "caret"]);
        // A cycle keeps registration order instead of looping.
        let cyclic = vec![
            PassSpec::new(SYNTAX).after(&[GENERAL]),
            PassSpec::new(GENERAL).after(&[SYNTAX]),
        ];
        assert_eq!(order_passes(cyclic).len(), 2);
    }

    #[test]
    fn opening_a_file_runs_visible_first_then_the_rest_in_dependency_order() {
        let (_, mut scheduler) = setup();
        // First poll: general (visible), syntax (inline), caret; inspect waits on general.
        let first = scheduler.poll();
        assert_eq!(
            ids(&first),
            [
                ("general", Phase::Visible),
                ("syntax", Phase::Whole),
                ("caret", Phase::Whole)
            ]
        );
        assert!(first[1].inline);
        finish(&mut scheduler, &first);
        // general's rest run starts before inspect (it is not settled yet).
        let second = scheduler.poll();
        assert_eq!(ids(&second), [("general", Phase::Rest)]);
        finish(&mut scheduler, &second);
        let third = scheduler.poll();
        assert_eq!(ids(&third), [("inspect", Phase::Visible)]);
        finish(&mut scheduler, &third);
        let fourth = scheduler.poll();
        assert_eq!(ids(&fourth), [("inspect", Phase::Rest)]);
        finish(&mut scheduler, &fourth);
        assert!(scheduler.is_idle(F));
        assert!(scheduler.poll().is_empty());
    }

    #[test]
    fn without_a_visible_range_passes_run_once_over_the_whole_file() {
        let clock = Arc::new(ManualClock::new());
        let mut scheduler = Scheduler::new(clock, SchedulerConfig::default());
        scheduler.open_file(F, specs());
        let first = scheduler.poll();
        assert_eq!(ids(&first)[0], ("general", Phase::Whole));
    }

    #[test]
    fn edits_debounce_by_300ms_and_every_edit_restarts_the_timer() {
        let (clock, mut scheduler) = setup();
        settle(&mut scheduler);
        scheduler.trigger(F, Trigger::SyntaxChanged);
        // Inline syntax is immediate, the rest is not.
        assert_eq!(ids(&scheduler.poll()), [("syntax", Phase::Whole)]);
        clock.advance(Duration::from_millis(299));
        assert!(scheduler.poll().is_empty());
        // Another keystroke restarts the debounce.
        scheduler.trigger(F, Trigger::SyntaxChanged);
        clock.advance(Duration::from_millis(299));
        let early = scheduler.poll();
        assert_eq!(ids(&early), [("syntax", Phase::Whole)]);
        assert_eq!(scheduler.next_deadline(), Some(Duration::from_millis(1)));
        clock.advance(Duration::from_millis(1));
        let due = scheduler.poll();
        assert_eq!(
            ids(&due),
            [("general", Phase::Visible), ("caret", Phase::Whole)]
        );
    }

    #[test]
    fn typing_cancels_running_passes_and_ignores_their_stale_results() {
        let (clock, mut scheduler) = setup();
        let first = scheduler.poll();
        let general = first.iter().find(|r| r.pass == GENERAL).unwrap().clone();
        assert!(!general.token.is_cancelled());
        let cancelled = scheduler.trigger(F, Trigger::SyntaxChanged);
        assert!(cancelled.contains(&general.id));
        assert!(general.token.is_cancelled(), "the run's token flips");
        // Its late completion is stale and changes nothing.
        assert_eq!(
            scheduler.complete(general.id, RunOutcome::Completed),
            Completion { current: false }
        );
        clock.advance(Duration::from_millis(300));
        let again = scheduler.poll();
        assert!(
            again
                .iter()
                .any(|r| r.pass == GENERAL && r.generation > general.generation)
        );
        assert_eq!(scheduler.stats(GENERAL).cancelled, 1);
    }

    #[test]
    fn a_cancelled_current_run_is_retried() {
        let (_, mut scheduler) = setup();
        let first = scheduler.poll();
        let general = first.iter().find(|r| r.pass == GENERAL).unwrap();
        assert!(
            scheduler
                .complete(general.id, RunOutcome::Cancelled)
                .current
        );
        let retry = scheduler.poll();
        assert!(retry.iter().any(|r| r.pass == GENERAL));
    }

    #[test]
    fn caret_moves_only_restart_caret_passes_with_a_short_delay() {
        let (clock, mut scheduler) = setup();
        settle(&mut scheduler);
        scheduler.trigger(F, Trigger::CaretChanged);
        assert!(scheduler.poll().is_empty());
        clock.advance(Duration::from_millis(50));
        assert_eq!(ids(&scheduler.poll()), [("caret", Phase::Whole)]);
    }

    #[test]
    fn closing_a_file_cancels_its_runs() {
        let (_, mut scheduler) = setup();
        let first = scheduler.poll();
        let cancelled = scheduler.close_file(F);
        assert_eq!(cancelled.len(), first.len());
        assert!(first.iter().all(|r| r.token.is_cancelled()));
        assert!(scheduler.is_idle(F) && !scheduler.is_tracking(F));
    }

    #[test]
    fn set_passes_keeps_known_state_and_starts_new_passes() {
        let (_, mut scheduler) = setup();
        settle(&mut scheduler);
        assert!(scheduler.is_idle(F));
        let mut more = specs();
        more.push(PassSpec::new(PassId("extra")));
        scheduler.set_passes(F, more);
        assert_eq!(scheduler.unsettled(F), [PassId("extra")]);
        scheduler.set_passes(F, vec![PassSpec::new(SYNTAX).inline()]);
        assert!(scheduler.is_idle(F));
    }

    mod props {
        use super::*;
        use proptest::prelude::*;

        #[derive(Debug, Clone)]
        enum Op {
            Trigger(u8),
            Advance(u16),
            Poll,
            CompleteOne(u8, bool),
        }

        fn op() -> impl Strategy<Value = Op> {
            prop_oneof![
                (0u8..7).prop_map(Op::Trigger),
                (0u16..400).prop_map(Op::Advance),
                Just(Op::Poll),
                (0u8..8, any::<bool>()).prop_map(|(i, ok)| Op::CompleteOne(i, ok)),
            ]
        }

        fn trigger_of(i: u8) -> Trigger {
            [
                Trigger::SyntaxChanged,
                Trigger::IndexUpdated,
                Trigger::InterfaceChanged,
                Trigger::SettingsChanged,
                Trigger::DiagnosticsPublished,
                Trigger::VisibleRangeChanged,
                Trigger::CaretChanged,
            ][i as usize % 7]
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(128))]
            #[test]
            fn scheduling_invariants_hold_for_any_event_sequence(ops in proptest::collection::vec(op(), 1..80)) {
                let clock = Arc::new(ManualClock::new());
                let mut scheduler = Scheduler::new(clock.clone(), SchedulerConfig::default());
                scheduler.open_file(F, specs());
                scheduler.set_visible(F, 0..100);
                let mut live: Vec<RunRequest> = Vec::new();
                // Per pass: did the Visible run complete before a Rest run started?
                let mut visible_done: HashMap<(PassId, u64), bool> = HashMap::new();
                for op in ops {
                    match op {
                        Op::Trigger(i) => {
                            let cancelled = scheduler.trigger(F, trigger_of(i));
                            for run in cancelled {
                                if let Some(at) = live.iter().position(|r| r.id == run) {
                                    prop_assert!(live[at].token.is_cancelled());
                                    live.remove(at);
                                }
                            }
                        }
                        Op::Advance(ms) => clock.advance(Duration::from_millis(ms as u64)),
                        Op::Poll => {
                            for request in scheduler.poll() {
                                // never two live runs of one pass
                                prop_assert!(!live.iter().any(|r| r.pass == request.pass));
                                // prerequisites are settled
                                for spec in specs().iter().find(|s| s.id == request.pass).map(|s| s.after.clone()).unwrap_or_default() {
                                    prop_assert!(!live.iter().any(|r| r.pass == spec), "prerequisite still running");
                                    prop_assert!(!scheduler.unsettled(F).contains(&spec) || request.phase == Phase::Rest, "prerequisite unsettled for {:?}", request.pass);
                                }
                                if request.phase == Phase::Rest {
                                    prop_assert!(visible_done.get(&(request.pass, request.generation)).copied().unwrap_or(false), "rest before visible");
                                }
                                live.push(request);
                            }
                        }
                        Op::CompleteOne(i, ok) => {
                            if live.is_empty() { continue; }
                            let request = live.remove(i as usize % live.len());
                            let outcome = if ok { RunOutcome::Completed } else { RunOutcome::Cancelled };
                            let c = scheduler.complete(request.id, outcome);
                            if c.current && ok && request.phase == Phase::Visible {
                                visible_done.insert((request.pass, request.generation), true);
                            }
                        }
                    }
                }
            }
        }
    }
}
