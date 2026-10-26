//! Batch inspection ("Inspect Code"): runs the enabled local tools
//! (`on_the_fly = false`) and the global tools over an [`AnalysisScope`],
//! streaming problems to a sink. Files are parsed on demand through the
//! host's closed-tree LRU, so a project-wide run holds a bounded number of
//! trees.

use super::engine::{LocalRun, run_local};
use super::profile::{EffectiveProfile, parse_level};
use archipel_ide_analysis::inspect::{
    AnalysisScope, GlobalCx, GlobalInspection, InspectionSuppressor, Level, LocalInspection,
    ProblemDescriptor, ProblemSink,
};
use archipel_ide_analysis::job::{JobSpec, Priority, Revoke};
use archipel_ide_analysis::{AnalysisHost, CancelToken, Cancelled, FileId, LanguageId, Snapshot};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// A problem of a batch run.
#[derive(Debug, Clone)]
pub struct BatchProblem {
    pub file: FileId,
    pub tool: &'static str,
    pub level: Level,
    pub problem: ProblemDescriptor,
}

/// Counters of a finished run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BatchSummary {
    pub files: usize,
    pub problems: usize,
    /// Files without a grammar or unreadable.
    pub skipped: usize,
}

/// The files `scope` covers (sorted).
pub fn files_in_scope(snap: &Snapshot, scope: &AnalysisScope) -> Vec<FileId> {
    let mut files: Vec<FileId> = match scope {
        AnalysisScope::Files(files) => files.clone(),
        AnalysisScope::Project => snap.files().iter().map(|(id, _)| id).collect(),
        AnalysisScope::Directory(dir) => snap
            .files()
            .iter()
            .filter(|(_, path)| path.starts_with(dir))
            .map(|(id, _)| id)
            .collect(),
        AnalysisScope::Scope(search) => snap
            .files()
            .iter()
            .map(|(id, _)| id)
            .filter(|id| search.contains(*id))
            .collect(),
    };
    files.sort();
    files.dedup();
    files
}

struct GlobalCollector<'a> {
    tool: &'static str,
    level: Level,
    out: &'a mut dyn FnMut(BatchProblem),
    count: usize,
}

impl ProblemSink for GlobalCollector<'_> {
    fn report(&mut self, file: FileId, problem: ProblemDescriptor) {
        self.count += 1;
        (self.out)(BatchProblem {
            file,
            tool: self.tool,
            level: self.level,
            problem,
        });
    }
}

/// Runs the batch synchronously on `snap`, polling `cancel`.
pub fn run_batch(
    snap: &Snapshot,
    scope: &AnalysisScope,
    profile: &EffectiveProfile,
    cancel: &CancelToken,
    sink: &mut dyn FnMut(BatchProblem),
) -> Result<BatchSummary, Cancelled> {
    let mut summary = BatchSummary::default();
    for file in files_in_scope(snap, scope) {
        cancel.check()?;
        let Some(syntax) = snap.syntax(file) else {
            summary.skipped += 1;
            continue;
        };
        let lang = syntax.lang();
        let path = snap.path(file).map(|p| p.to_owned()).unwrap_or_default();
        let tools: Vec<_> = snap
            .registry()
            .all::<dyn LocalInspection>(lang.language_id())
            .iter()
            .filter(|tool| lang.tier >= tool.min_tier())
            .filter_map(|tool| profile.configure(tool, &path))
            .collect();
        let suppressors: Vec<Arc<dyn InspectionSuppressor>> =
            snap.registry().all(lang.language_id());
        let run = LocalRun {
            snap,
            file,
            syntax: &syntax,
            cancel,
            on_the_fly: false,
        };
        for found in run_local(&run, &tools, &suppressors, None)? {
            summary.problems += 1;
            sink(BatchProblem {
                file,
                tool: found.tool,
                level: found.level,
                problem: found.problem,
            });
        }
        summary.files += 1;
    }
    let globals: Vec<Arc<dyn GlobalInspection>> = snap.registry().all(LanguageId::ANY);
    for global in globals {
        cancel.check()?;
        let entry = profile.entry(global.id());
        if entry.and_then(|e| e.enabled) == Some(false) {
            continue;
        }
        let level = entry
            .and_then(|e| e.level.as_deref())
            .and_then(parse_level)
            .unwrap_or(Level::Warning);
        let mut collector = GlobalCollector {
            tool: global.id(),
            level,
            out: sink,
            count: 0,
        };
        global.run(scope, &GlobalCx { snap, cancel }, &mut collector)?;
        summary.problems += collector.count;
    }
    Ok(summary)
}

/// Progress of a background run.
#[derive(Debug, Clone)]
pub enum BatchEvent {
    Problem(Box<BatchProblem>),
    Finished(BatchSummary),
    Cancelled,
}

/// A running batch: poll [`BatchRun::drain`] once per frame.
pub struct BatchRun {
    rx: Receiver<BatchEvent>,
    cancel: CancelToken,
}

impl BatchRun {
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Events since the last call.
    pub fn drain(&self) -> Vec<BatchEvent> {
        self.rx.try_iter().collect()
    }
}

/// Starts a batch as a `Background` job over a snapshot (editing during the
/// run never cancels it: [`Revoke::Never`]).
pub fn start_batch(
    host: &mut AnalysisHost,
    scope: AnalysisScope,
    profile: Arc<EffectiveProfile>,
) -> BatchRun {
    let (tx, rx): (Sender<BatchEvent>, Receiver<BatchEvent>) = channel();
    let done = tx.clone();
    let token = host.spawn_with(
        JobSpec::new(Priority::Background, Revoke::Never),
        move |snap, cancel| {
            let mut sink = |problem: BatchProblem| {
                let _ = tx.send(BatchEvent::Problem(Box::new(problem)));
            };
            run_batch(snap, &scope, &profile, cancel, &mut sink)
        },
        move |result| {
            let _ = done.send(match result {
                Ok(summary) => BatchEvent::Finished(summary),
                Err(Cancelled) => BatchEvent::Cancelled,
            });
        },
    );
    BatchRun { rx, cancel: token }
}
