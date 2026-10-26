//! A host + daemon + manual clock for end-to-end daemon tests.
#![allow(dead_code)]

use archipel_ide_analysis::daemon::Pass;
use archipel_ide_analysis::vfs::MemVfs;
use archipel_ide_analysis::{
    AnalysisHost, Change, FileId, FileRev, HostConfig, ServiceRegistry, default_registry,
};
use archipel_ide_daemon::{DaemonService, ManualClock};
use archipel_ide_language::{ByteEdit, Registration};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub struct DaemonHarness {
    pub host: AnalysisHost,
    pub svc: Arc<DaemonService>,
    pub clock: Arc<ManualClock>,
    rev: u64,
}

impl DaemonHarness {
    pub fn new() -> Self {
        Self::with(HostConfig::testing(), |_| {})
    }

    pub fn with(config: HostConfig, extra: impl FnOnce(&mut ServiceRegistry)) -> Self {
        let clock = Arc::new(ManualClock::new());
        let svc = DaemonService::new(clock.clone());
        let mut registry = default_registry();
        svc.register(&mut registry);
        extra(&mut registry);
        Self {
            host: AnalysisHost::new(config, Arc::new(MemVfs::new()), registry),
            svc,
            clock,
            rev: 1,
        }
    }

    pub fn add_pass(registry: &mut ServiceRegistry, id: &'static str, pass: Arc<dyn Pass>) {
        registry.register::<dyn Pass>(Registration::any(id).dumb_aware(true), pass);
    }

    pub fn open(&mut self, path: &str, text: &str) -> FileId {
        let file = self.host.file_id(Path::new(path));
        self.rev = 1;
        self.host.apply(Change::Open {
            file,
            rev: FileRev(1),
            text: text.into(),
        });
        file
    }

    /// Replaces `range` of `old` with `new`; returns the new text.
    pub fn edit(
        &mut self,
        file: FileId,
        old: &str,
        range: std::ops::Range<usize>,
        new: &str,
    ) -> String {
        let edit: ByteEdit = archipel_ide_analysis::testing::byte_edit(old, range.clone(), new);
        let mut text = old.to_owned();
        text.replace_range(range, new);
        self.rev += 1;
        self.host.apply(Change::Edit {
            file,
            edits: vec![edit],
            new_rev: FileRev(self.rev),
            text: text.as_str().into(),
            syntax: None,
        });
        text
    }

    /// One frame: host housekeeping, then the daemon.
    pub fn tick(&mut self) -> bool {
        self.host.poll();
        let events = self.host.take_events();
        self.svc.lock().tick(&mut self.host, &events)
    }

    /// Ticks (advancing the clock by `step` between frames) until the daemon
    /// is idle; returns the number of frames.
    pub fn settle(&mut self, step: Duration) -> usize {
        for frame in 1..=200 {
            self.tick();
            if self.svc.lock().all_idle() {
                return frame;
            }
            self.clock.advance(step);
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("daemon did not settle");
    }

    /// `(tool, message)` of the problems of `file`, in position order.
    pub fn problems(&self, file: FileId) -> Vec<(String, String)> {
        let daemon = self.svc.lock();
        daemon
            .markup(file)
            .map(|m| {
                m.iter()
                    .filter(|i| i.severity.is_problem())
                    .map(|i| {
                        (
                            i.tool.as_str().to_owned(),
                            i.message.as_deref().unwrap_or("").to_owned(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
