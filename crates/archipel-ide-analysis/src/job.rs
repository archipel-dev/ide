//! Background jobs: a `std::thread` pool with three priority queues and a
//! dedicated interactive lane (completion, hover and typed handlers never
//! queue behind indexing). Results come back through [`JobHandle`]s and
//! completion callbacks drained on the UI thread once per frame
//! ([`Executor::run_completions`]); nothing here blocks the caller.

use crate::cancel::{CancelToken, Cancelled};
use crate::file::FileId;
use parking_lot::{Condvar, Mutex};
use std::collections::VecDeque;
use std::sync::Arc;
use std::thread::JoinHandle;

/// Queue of a job. Interactive work also has a lane of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Interactive,
    Daemon,
    Background,
}

/// Which changes revoke (cancel) a running job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revoke {
    /// Runs to completion (or until its token is cancelled explicitly).
    Never,
    OnAnyChangeOf(RevokeScope),
}

/// What a [`Revoke::OnAnyChangeOf`] watches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevokeScope {
    /// Any change to the analysis world (`expire_when(snapshot changed)`).
    All,
    /// Text or existence of these files.
    Files(Vec<FileId>),
    Settings,
}

/// How to run a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSpec {
    pub priority: Priority,
    pub revoke: Revoke,
    /// Spawning a job with the same key cancels the previous one
    /// (`NonBlockingReadAction.coalesceBy`).
    pub coalesce: Option<(&'static str, u64)>,
}

impl JobSpec {
    pub fn new(priority: Priority, revoke: Revoke) -> Self {
        Self {
            priority,
            revoke,
            coalesce: None,
        }
    }

    pub fn coalesce(mut self, kind: &'static str, key: u64) -> Self {
        self.coalesce = Some((kind, key));
        self
    }
}

type Task = Box<dyn FnOnce() + Send>;
type Completion = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Queues {
    interactive: VecDeque<Task>,
    daemon: VecDeque<Task>,
    background: VecDeque<Task>,
    shutdown: bool,
}

impl Queues {
    fn pop(&mut self, interactive_only: bool) -> Option<Task> {
        if let Some(task) = self.interactive.pop_front() {
            return Some(task);
        }
        if interactive_only {
            return None;
        }
        self.daemon
            .pop_front()
            .or_else(|| self.background.pop_front())
    }
}

struct Shared {
    queues: Mutex<Queues>,
    ready: Condvar,
    completions: Mutex<Vec<Completion>>,
    wake: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

/// The state of a spawned job's result.
pub struct JobHandle<T> {
    result: Arc<Mutex<Option<Result<T, Cancelled>>>>,
    cancel: CancelToken,
}

impl<T> JobHandle<T> {
    /// A handle whose result is already known (inline execution).
    pub fn ready(result: Result<T, Cancelled>, cancel: CancelToken) -> Self {
        Self {
            result: Arc::new(Mutex::new(Some(result))),
            cancel,
        }
    }

    /// The result once the job finished (taken once).
    pub fn try_take(&self) -> Option<Result<T, Cancelled>> {
        self.result.lock().take()
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub fn token(&self) -> &CancelToken {
        &self.cancel
    }

    /// Blocks until the result is there (tests and the CLI only).
    pub fn wait(&self) -> Result<T, Cancelled> {
        loop {
            if let Some(result) = self.try_take() {
                return result;
            }
            std::thread::yield_now();
        }
    }
}

/// See the module docs.
pub struct Executor {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
}

impl std::fmt::Debug for Executor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Executor")
            .field("workers", &self.workers.len())
            .finish()
    }
}

/// `max(2, cores - 1)` general workers.
pub fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map_or(2, |cores| cores.get().saturating_sub(1))
        .max(2)
}

impl Executor {
    /// `threads` general workers plus one interactive-only lane.
    pub fn new(threads: usize) -> Self {
        let shared = Arc::new(Shared {
            queues: Mutex::new(Queues::default()),
            ready: Condvar::new(),
            completions: Mutex::new(Vec::new()),
            wake: Mutex::new(None),
        });
        let mut workers = Vec::new();
        for index in 0..=threads.max(1) {
            let shared = shared.clone();
            let interactive_only = index == 0;
            let name = if interactive_only {
                "analysis-interactive".to_string()
            } else {
                format!("analysis-{index}")
            };
            if let Ok(handle) = std::thread::Builder::new()
                .name(name)
                .spawn(move || worker(&shared, interactive_only))
            {
                workers.push(handle);
            }
        }
        Self { shared, workers }
    }

    /// Called (from a worker) whenever a completion is queued, e.g. to
    /// request a repaint.
    pub fn set_wake(&self, wake: impl Fn() + Send + Sync + 'static) {
        *self.shared.wake.lock() = Some(Arc::new(wake));
    }

    /// Runs `task` on the pool. `on_done` receives the result on the thread
    /// that calls [`Executor::run_completions`] (the UI thread).
    pub fn spawn<T: Send + 'static>(
        &self,
        priority: Priority,
        cancel: CancelToken,
        task: impl FnOnce(&CancelToken) -> Result<T, Cancelled> + Send + 'static,
    ) -> JobHandle<T> {
        let result = Arc::new(Mutex::new(None));
        let slot = result.clone();
        let token = cancel.clone();
        let shared = self.shared.clone();
        let job: Task = Box::new(move || {
            let outcome = match token.check() {
                Ok(()) => task(&token),
                Err(cancelled) => Err(cancelled),
            };
            *slot.lock() = Some(outcome);
            let wake = shared.wake.lock().clone();
            if let Some(wake) = wake {
                wake();
            }
        });
        self.push(priority, job);
        JobHandle { result, cancel }
    }

    /// Like [`Executor::spawn`], delivering the result to `on_done` from
    /// [`Executor::run_completions`].
    pub fn spawn_with<T: Send + 'static>(
        &self,
        priority: Priority,
        cancel: CancelToken,
        task: impl FnOnce(&CancelToken) -> Result<T, Cancelled> + Send + 'static,
        on_done: impl FnOnce(Result<T, Cancelled>) + Send + 'static,
    ) -> CancelToken {
        let token = cancel.clone();
        let shared = self.shared.clone();
        let job: Task = Box::new(move || {
            let outcome = match token.check() {
                Ok(()) => task(&token),
                Err(cancelled) => Err(cancelled),
            };
            shared
                .completions
                .lock()
                .push(Box::new(move || on_done(outcome)));
            let wake = shared.wake.lock().clone();
            if let Some(wake) = wake {
                wake();
            }
        });
        self.push(priority, job);
        cancel
    }

    fn push(&self, priority: Priority, job: Task) {
        let mut queues = self.shared.queues.lock();
        match priority {
            Priority::Interactive => queues.interactive.push_back(job),
            Priority::Daemon => queues.daemon.push_back(job),
            Priority::Background => queues.background.push_back(job),
        }
        drop(queues);
        self.shared.ready.notify_all();
    }

    /// Runs the queued completion callbacks; call once per frame on the UI
    /// thread. Returns how many ran.
    pub fn run_completions(&self) -> usize {
        let completions = std::mem::take(&mut *self.shared.completions.lock());
        let count = completions.len();
        for completion in completions {
            completion();
        }
        count
    }

    /// Jobs waiting in the queues.
    pub fn queued(&self) -> usize {
        let queues = self.shared.queues.lock();
        queues.interactive.len() + queues.daemon.len() + queues.background.len()
    }
}

fn worker(shared: &Shared, interactive_only: bool) {
    loop {
        let task = {
            let mut queues = shared.queues.lock();
            loop {
                if queues.shutdown {
                    return;
                }
                if let Some(task) = queues.pop(interactive_only) {
                    break task;
                }
                shared.ready.wait(&mut queues);
            }
        };
        task();
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        self.shared.queues.lock().shutdown = true;
        self.shared.ready.notify_all();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn jobs_run_and_deliver_results() {
        let executor = Executor::new(2);
        let handle = executor.spawn(Priority::Background, CancelToken::new(), |_| Ok(21 * 2));
        assert_eq!(handle.wait(), Ok(42));
        let seen = Arc::new(AtomicUsize::new(0));
        let sink = seen.clone();
        executor.spawn_with(
            Priority::Daemon,
            CancelToken::new(),
            |_| Ok(7),
            move |result| {
                sink.store(result.unwrap_or(0), Ordering::SeqCst);
            },
        );
        while seen.load(Ordering::SeqCst) == 0 {
            executor.run_completions();
            std::thread::yield_now();
        }
        assert_eq!(seen.load(Ordering::SeqCst), 7);
    }

    #[test]
    fn interactive_lane_is_not_blocked_by_background_work() {
        let executor = Executor::new(1);
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        // Occupy the only general worker.
        let blocker = gate.clone();
        let busy = executor.spawn(Priority::Background, CancelToken::new(), move |_| {
            let (lock, cvar) = &*blocker;
            let mut open = lock.lock();
            while !*open {
                cvar.wait(&mut open);
            }
            Ok(())
        });
        let quick = executor.spawn(Priority::Interactive, CancelToken::new(), |_| Ok("hover"));
        assert_eq!(quick.wait(), Ok("hover"));
        *gate.0.lock() = true;
        gate.1.notify_all();
        assert_eq!(busy.wait(), Ok(()));
    }

    #[test]
    fn cancelled_before_start_never_runs() {
        let executor = Executor::new(1);
        let token = CancelToken::new();
        token.cancel();
        let ran = Arc::new(AtomicUsize::new(0));
        let flag = ran.clone();
        let handle = executor.spawn(Priority::Daemon, token, move |_| {
            flag.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        assert_eq!(handle.wait(), Err(Cancelled));
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }
}
