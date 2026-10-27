//! The daemon: glues the [`Scheduler`], the passes, the analysis host and the
//! [`MarkupStore`]. UI-thread object; passes run in host jobs on a snapshot.
//!
//! Per frame the app calls [`Daemon::tick`] with the host and the events the
//! host published. The daemon
//! 1. discovers opened/closed files and file revisions (an edit triggers
//!    `SyntaxChanged`, which cancels the file's running passes),
//! 2. shifts the markup of edited files through the host's edit journal, so
//!    highlights stay on their code until a pass replaces them,
//! 3. applies finished runs (results of an older revision are mapped through
//!    the journal; results older than what is already shown are dropped),
//! 4. launches the runs the scheduler says are due (inline ones - `Syntax` -
//!    synchronously, the rest as `Daemon`-priority host jobs).
//!
//! Nothing here blocks on a pass: the work is in jobs and `tick` only moves
//! data.

use crate::clock::Clock;
use crate::external::STALE_OWNER;
use crate::markup::{Coexistence, FileMarkup, MarkupLimits, MarkupStore, shift_infos};
use crate::passes::{Shared, spec_of};
use crate::problem_files::ProblemFiles;
use crate::scheduler::{
    PassStats, Phase, RunId, RunOutcome, RunRequest, Scheduler, SchedulerConfig, Trigger,
};
use archipel_ide_analysis::daemon::{Pass, PassCx, PassId, PassOutput};
use archipel_ide_analysis::job::{JobSpec, Priority, Revoke, RevokeScope};
use archipel_ide_analysis::{
    AnalysisEvent, AnalysisHost, CancelToken, Cancelled, FileId, FileRev, LanguageId, Snapshot,
    ToolId,
};
use archipel_ide_language::{CompletenessTier, EditMap};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

/// User-facing knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonSettings {
    /// Quiet time before non-syntax passes run (IntelliJ: 300 ms).
    pub debounce: Duration,
    pub limits: MarkupLimits,
    /// When false nothing but the syntax pass runs.
    pub enabled: bool,
}

impl Default for DaemonSettings {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(300),
            limits: MarkupLimits::default(),
            enabled: true,
        }
    }
}

struct TrackedFile {
    lang: LanguageId,
    dumb: bool,
    seen_rev: FileRev,
    passes: HashMap<PassId, Arc<dyn Pass>>,
}

struct Done {
    run: RunId,
    file: FileId,
    phase: Phase,
    result: Result<PassOutput, Cancelled>,
}

/// See the module docs.
pub struct Daemon {
    scheduler: Scheduler,
    store: MarkupStore,
    shared: Arc<Shared>,
    inbox: Arc<Mutex<Vec<Done>>>,
    jobs: HashMap<RunId, CancelToken>,
    tracked: HashMap<FileId, TrackedFile>,
    untracked: HashSet<FileId>,
    /// Visible ranges reported by the editor (also before a file is tracked).
    visible: HashMap<FileId, Range<u32>>,
    problem_files: ProblemFiles,
    settings: DaemonSettings,
    /// Newest file revision whose result of an owner is shown.
    applied: HashMap<(FileId, ToolId), FileRev>,
    applied_count: u64,
}

impl std::fmt::Debug for Daemon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Daemon")
            .field("tracked", &self.tracked.len())
            .field("running", &self.jobs.len())
            .finish()
    }
}

impl Daemon {
    pub fn new(clock: Arc<dyn Clock>, shared: Arc<Shared>) -> Self {
        let settings = DaemonSettings::default();
        Self {
            scheduler: Scheduler::new(clock, scheduler_config(settings)),
            store: MarkupStore::with_limits(settings.limits),
            shared,
            inbox: Arc::default(),
            jobs: HashMap::new(),
            tracked: HashMap::new(),
            untracked: HashSet::new(),
            visible: HashMap::new(),
            problem_files: ProblemFiles::new(),
            settings,
            applied: HashMap::new(),
            applied_count: 0,
        }
    }

    pub fn settings(&self) -> DaemonSettings {
        self.settings
    }

    pub fn set_settings(&mut self, settings: DaemonSettings) {
        self.settings = settings;
        self.scheduler.set_config(scheduler_config(settings));
    }

    pub fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    // ---- queries (UI thread)

    pub fn markup(&self, file: FileId) -> Option<&FileMarkup> {
        self.store.get(file)
    }

    pub fn store(&self) -> &MarkupStore {
        &self.store
    }

    pub fn problem_files(&self) -> &ProblemFiles {
        &self.problem_files
    }

    pub fn is_tracking(&self, file: FileId) -> bool {
        self.tracked.contains_key(&file)
    }

    /// Nothing pending or running for `file`.
    pub fn is_idle(&self, file: FileId) -> bool {
        self.scheduler.is_idle(file)
    }

    /// Every tracked file is idle and no job is in flight.
    pub fn all_idle(&self) -> bool {
        self.jobs.is_empty()
            && self.inbox.lock().is_empty()
            && self
                .tracked
                .keys()
                .all(|file| self.scheduler.is_idle(*file))
    }

    pub fn pass_stats(&self, pass: PassId) -> PassStats {
        self.scheduler.stats(pass)
    }

    /// Time until the next pass becomes due (for repaint scheduling).
    pub fn next_deadline(&self) -> Option<Duration> {
        self.scheduler.next_deadline()
    }

    /// How many pass results were applied so far (diagnostics, tests).
    pub fn applied_results(&self) -> u64 {
        self.applied_count
    }

    // ---- inputs (UI thread)

    /// The editor's visible byte range of `file`.
    pub fn set_visible(&mut self, file: FileId, range: Range<u32>) {
        self.visible.insert(file, range.clone());
        self.scheduler.set_visible(file, range);
    }

    /// The caret moved (only caret-driven passes react).
    pub fn set_caret(&mut self, file: FileId, offset: u32) {
        let changed = self.shared.carets.lock().insert(file, offset) != Some(offset);
        if changed {
            let cancelled = self.scheduler.trigger(file, Trigger::CaretChanged);
            self.cancel_jobs(&cancelled);
        }
    }

    /// An external provider published new diagnostics for `file`.
    pub fn diagnostics_published(&mut self, file: FileId) {
        let cancelled = self.scheduler.trigger(file, Trigger::DiagnosticsPublished);
        self.cancel_jobs(&cancelled);
    }

    /// Settings (profile, options) changed: every file is re-analyzed.
    pub fn settings_changed(&mut self) {
        self.trigger_all(Trigger::SettingsChanged);
    }

    fn trigger_all(&mut self, trigger: Trigger) {
        let files: Vec<FileId> = self.tracked.keys().copied().collect();
        for file in files {
            let cancelled = self.scheduler.trigger(file, trigger);
            self.cancel_jobs(&cancelled);
        }
    }

    fn cancel_jobs(&mut self, runs: &[RunId]) {
        for run in runs {
            if let Some(token) = self.jobs.remove(run) {
                token.cancel();
            }
        }
    }

    // ---- the frame tick

    /// Advances the daemon; returns true when the markup changed.
    pub fn tick(&mut self, host: &mut AnalysisHost, events: &[AnalysisEvent]) -> bool {
        let mut changed = false;
        let snap = host.snapshot();
        changed |= self.sync_files(&snap);
        for event in events {
            match event {
                AnalysisEvent::IndexUpdated(_) => self.trigger_all(Trigger::IndexUpdated),
                AnalysisEvent::SettingsChanged => self.trigger_all(Trigger::SettingsChanged),
                AnalysisEvent::DumbModeChanged(_) => {
                    self.refresh_pass_lists(&snap);
                    self.trigger_all(Trigger::IndexUpdated);
                }
                AnalysisEvent::InterfaceChanged { file, .. } => {
                    let cancelled = self.scheduler.trigger(*file, Trigger::InterfaceChanged);
                    self.cancel_jobs(&cancelled);
                }
                _ => {}
            }
        }
        changed |= self.sync_revisions(&snap);
        changed |= self.drain(&snap);
        for request in self.scheduler.poll() {
            changed |= self.launch(host, &snap, request);
        }
        if changed {
            self.store.touch();
        }
        changed
    }

    fn passes_for(snap: &Snapshot, lang: LanguageId) -> Vec<Arc<dyn Pass>> {
        snap.registry()
            .all_available::<dyn Pass>(lang, snap.is_dumb())
    }

    fn specs(&self, passes: &[Arc<dyn Pass>]) -> Vec<crate::scheduler::PassSpec> {
        passes
            .iter()
            .filter(|p| self.settings.enabled || p.id() == crate::passes::SYNTAX)
            .map(|p| spec_of(p.as_ref()))
            .collect()
    }

    /// Starts/stops tracking files as they open and close.
    fn sync_files(&mut self, snap: &Snapshot) -> bool {
        let mut changed = false;
        let mut open: Vec<FileId> = snap.open_files().collect();
        open.sort();
        let open_set: HashSet<FileId> = open.iter().copied().collect();
        self.untracked.retain(|file| open_set.contains(file));
        for file in &open {
            let lang = snap.language_id(*file);
            let known = self.tracked.get(file).map(|t| t.lang);
            if known.is_some() && known == lang {
                continue;
            }
            if known.is_some() {
                self.stop_tracking(*file);
                changed = true;
            }
            let Some(lang) = lang else {
                self.untracked.insert(*file);
                continue;
            };
            if self.untracked.contains(file) || snap.syntax(*file).is_none() {
                self.untracked.insert(*file);
                continue;
            }
            let Some(rev) = snap.file_rev(*file) else {
                continue;
            };
            let passes = Self::passes_for(snap, lang);
            let specs = self.specs(&passes);
            let cancelled = self.scheduler.open_file(*file, specs);
            self.cancel_jobs(&cancelled);
            if let Some(range) = self.visible.get(file) {
                self.scheduler.set_visible(*file, range.clone());
            }
            self.store.ensure(*file, rev);
            self.tracked.insert(
                *file,
                TrackedFile {
                    lang,
                    dumb: snap.is_dumb(),
                    seen_rev: rev,
                    passes: passes.into_iter().map(|p| (p.id(), p)).collect(),
                },
            );
        }
        let gone: Vec<FileId> = self
            .tracked
            .keys()
            .filter(|file| !open_set.contains(file))
            .copied()
            .collect();
        for file in gone {
            self.stop_tracking(file);
            changed = true;
        }
        changed
    }

    fn stop_tracking(&mut self, file: FileId) {
        let cancelled = self.scheduler.close_file(file);
        self.cancel_jobs(&cancelled);
        self.tracked.remove(&file);
        self.store.remove(file);
        self.problem_files.update(file, None);
        self.shared.carets.lock().remove(&file);
        self.visible.remove(&file);
        self.applied.retain(|(f, _), _| *f != file);
    }

    fn refresh_pass_lists(&mut self, snap: &Snapshot) {
        let files: Vec<FileId> = self.tracked.keys().copied().collect();
        for file in files {
            let Some(tracked) = self.tracked.get(&file) else {
                continue;
            };
            let passes = Self::passes_for(snap, tracked.lang);
            let specs = self.specs(&passes);
            let cancelled = self.scheduler.set_passes(file, specs);
            self.cancel_jobs(&cancelled);
            if let Some(tracked) = self.tracked.get_mut(&file) {
                tracked.dumb = snap.is_dumb();
                tracked.passes = passes.into_iter().map(|p| (p.id(), p)).collect();
            }
        }
    }

    /// Edits: trigger `SyntaxChanged` and shift the markup to the new text.
    fn sync_revisions(&mut self, snap: &Snapshot) -> bool {
        let mut changed = false;
        let files: Vec<FileId> = self.tracked.keys().copied().collect();
        for file in files {
            let Some(rev) = snap.file_rev(file) else {
                continue;
            };
            let seen = self.tracked.get(&file).map(|t| t.seen_rev);
            if seen != Some(rev) {
                if let Some(tracked) = self.tracked.get_mut(&file) {
                    tracked.seen_rev = rev;
                }
                let cancelled = self.scheduler.trigger(file, Trigger::SyntaxChanged);
                self.cancel_jobs(&cancelled);
            }
            changed |= self.shift_markup(snap, file, rev);
        }
        changed
    }

    fn shift_markup(&mut self, snap: &Snapshot, file: FileId, rev: FileRev) -> bool {
        let Some(markup) = self.store.get(file) else {
            return false;
        };
        let old = markup.rev();
        if old == rev {
            return false;
        }
        let text = snap.file_text(file).ok();
        let map = snap.edits_since(file, old);
        let Some(markup) = self.store.get_mut(file) else {
            return false;
        };
        match (map, text) {
            (Some(map), Some(text)) => markup.shift(&map, rev, &text),
            _ => {
                // The journal does not reach back (reload): nothing to keep.
                markup.retain(|_, _| false);
                markup.set_rev(rev);
            }
        }
        let markup = self.store.get(file);
        self.problem_files.update(file, markup);
        true
    }

    /// Applies finished runs.
    fn drain(&mut self, snap: &Snapshot) -> bool {
        let done: Vec<Done> = std::mem::take(&mut *self.inbox.lock());
        let mut changed = false;
        for done in done {
            changed |= self.finish(snap, done);
        }
        changed
    }

    fn finish(&mut self, snap: &Snapshot, done: Done) -> bool {
        self.jobs.remove(&done.run);
        let outcome = if done.result.is_ok() {
            RunOutcome::Completed
        } else {
            RunOutcome::Cancelled
        };
        self.scheduler.complete(done.run, outcome);
        match done.result {
            Ok(out) => self.apply(snap, done.file, done.phase, out),
            Err(Cancelled) => false,
        }
    }

    fn apply(&mut self, snap: &Snapshot, file: FileId, phase: Phase, out: PassOutput) -> bool {
        if !self.tracked.contains_key(&file) {
            return false;
        }
        let Some(current) = snap.file_rev(file) else {
            return false;
        };
        let key = (file, out.tool.clone());
        if self.applied.get(&key).is_some_and(|shown| *shown > out.rev) {
            return false;
        }
        let Ok(text) = snap.file_text(file) else {
            return false;
        };
        let map = if out.rev == current {
            EditMap::new()
        } else {
            match snap.edits_since(file, out.rev) {
                Some(map) => map,
                None => return false,
            }
        };
        let mut range = if phase == Phase::Visible {
            let mapped = map.map_range(out.range.start as usize..out.range.end as usize);
            mapped.start as u32..mapped.end as u32
        } else {
            0..u32::MAX
        };
        range.end = range.end.max(range.start);
        let infos = shift_infos(out.infos, &map, &text);
        let markup = self.store.ensure(file, current);
        if markup.rev() != current {
            return false;
        }
        if out.tool.is_lsp() && out.tool.as_str() == format!("lsp:{STALE_OWNER}") {
            // Nothing is published for the file any more.
            markup.retain(|owner, _| !owner.is_lsp());
        } else {
            markup.replace(&out.tool, range, infos);
        }
        self.applied.insert(key, out.rev);
        self.applied_count += 1;
        self.reconcile(snap, file);
        let markup = self.store.get(file);
        self.problem_files.update(file, markup);
        true
    }

    /// Deduplicates the native and language-server highlights of `file` (see
    /// [`FileMarkup::reconcile`]); the language's tier decides who wins.
    fn reconcile(&mut self, snap: &Snapshot, file: FileId) {
        let graduated = snap
            .syntax(file)
            .is_some_and(|syntax| syntax.lang().tier >= CompletenessTier::G2);
        let external_published = snap
            .path(file)
            .is_some_and(|path| self.shared.external.is_published(path));
        if let Some(markup) = self.store.get_mut(file) {
            markup.reconcile(Coexistence {
                graduated,
                external_published,
            });
        }
    }

    fn launch(&mut self, host: &mut AnalysisHost, snap: &Snapshot, request: RunRequest) -> bool {
        let Some(pass) = self
            .tracked
            .get(&request.file)
            .and_then(|t| t.passes.get(&request.pass))
            .cloned()
        else {
            self.scheduler.complete(request.id, RunOutcome::Failed);
            return false;
        };
        let (file, run, phase) = (request.file, request.id, request.phase);
        if request.inline {
            // Incremental and cheap: on the edit thread, no job.
            let result = match snap.syntax(file) {
                Some(syntax) => {
                    let len = syntax.text().len() as u32;
                    pass.collect(&PassCx {
                        snap,
                        file,
                        syntax: &syntax,
                        range: 0..len,
                        visible: false,
                        cancel: &request.token,
                    })
                }
                None => Err(Cancelled),
            };
            return self.finish(
                snap,
                Done {
                    run,
                    file,
                    phase,
                    result,
                },
            );
        }
        let visible = request.visible.clone();
        let inbox = self.inbox.clone();
        let spec = JobSpec::new(
            Priority::Daemon,
            Revoke::OnAnyChangeOf(RevokeScope::Files(vec![file])),
        );
        let token = host.spawn_with(
            spec,
            move |snap, cancel| {
                let syntax = snap.syntax(file).ok_or(Cancelled)?;
                let len = syntax.text().len() as u32;
                let (range, is_visible) = match (phase, visible) {
                    (Phase::Visible, Some(v)) => (v.start.min(len)..v.end.min(len), true),
                    _ => (0..len, false),
                };
                cancel.check()?;
                pass.collect(&PassCx {
                    snap,
                    file,
                    syntax: &syntax,
                    range,
                    visible: is_visible,
                    cancel,
                })
            },
            move |result| {
                inbox.lock().push(Done {
                    run,
                    file,
                    phase,
                    result,
                })
            },
        );
        self.jobs.insert(run, token);
        false
    }
}

fn scheduler_config(settings: DaemonSettings) -> SchedulerConfig {
    SchedulerConfig {
        debounce: settings.debounce,
        ..SchedulerConfig::default()
    }
}
