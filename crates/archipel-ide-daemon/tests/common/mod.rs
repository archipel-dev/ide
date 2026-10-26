//! Shared test harness: a host with the Tier-A set registered.
#![allow(dead_code)]
pub mod daemon_harness;

use archipel_ide_analysis::inspect::{InspectionSuppressor, LocalInspection};
use archipel_ide_analysis::vfs::MemVfs;
use archipel_ide_analysis::{
    AnalysisHost, CancelToken, Change, FileId, FileRev, HostConfig, Snapshot, default_registry,
};
use archipel_ide_daemon::inspection::engine::{Found, LocalRun, ToolConfig, run_local};
use archipel_ide_daemon::inspection::profile::EffectiveProfile;
use archipel_ide_daemon::inspection::tier_a;
use std::path::Path;
use std::sync::Arc;

pub struct Harness {
    pub host: AnalysisHost,
    pub vfs: Arc<MemVfs>,
}

impl Harness {
    pub fn new() -> Self {
        let mut registry = default_registry();
        tier_a::register(&mut registry);
        let vfs = Arc::new(MemVfs::new());
        Self {
            host: AnalysisHost::new(HostConfig::testing(), vfs.clone(), registry),
            vfs,
        }
    }

    pub fn open(&mut self, path: &str, text: &str) -> (FileId, Snapshot) {
        let file = self.host.file_id(Path::new(path));
        self.host.apply(Change::Open {
            file,
            rev: FileRev(1),
            text: text.into(),
        });
        (file, self.host.snapshot())
    }

    /// Tools registered for the file's language, with `force` ids enabled
    /// even when off by default.
    pub fn tools(
        &self,
        snap: &Snapshot,
        file: FileId,
        profile: &EffectiveProfile,
        force: &[&str],
    ) -> Vec<ToolConfig> {
        let lang = snap.language_id(file).expect("language");
        let path = snap.path(file).unwrap().to_owned();
        snap.registry()
            .all::<dyn LocalInspection>(lang)
            .iter()
            .filter_map(|tool| {
                if force.contains(&tool.id()) {
                    let (_, level) = profile.state_of(tool.as_ref(), &path);
                    Some(ToolConfig {
                        tool: tool.clone(),
                        level,
                        options: profile.options_of(tool.as_ref()),
                    })
                } else {
                    profile.configure(tool, &path)
                }
            })
            .collect()
    }

    pub fn inspect_with(
        &mut self,
        path: &str,
        text: &str,
        profile: &EffectiveProfile,
        force: &[&str],
    ) -> Vec<Found> {
        let (file, snap) = self.open(path, text);
        let syntax = snap.syntax(file).expect("syntax");
        let tools = self.tools(&snap, file, profile, force);
        let lang = syntax.lang().language_id();
        let suppressors: Vec<Arc<dyn InspectionSuppressor>> = snap.registry().all(lang);
        let cancel = CancelToken::new();
        let run = LocalRun {
            snap: &snap,
            file,
            syntax: &syntax,
            cancel: &cancel,
            on_the_fly: true,
        };
        run_local(&run, &tools, &suppressors, None).expect("not cancelled")
    }

    /// `(tool id, snippet)` of every problem.
    pub fn inspect(&mut self, path: &str, text: &str) -> Vec<(String, String)> {
        let found = self.inspect_with(path, text, &EffectiveProfile::default(), &[]);
        found
            .into_iter()
            .map(|f| {
                (
                    f.tool.to_owned(),
                    text[f.problem.range.start as usize..f.problem.range.end as usize].to_owned(),
                )
            })
            .collect()
    }
}

pub fn only(found: &[(String, String)], tool: &str) -> Vec<String> {
    found
        .iter()
        .filter(|(t, _)| t == tool)
        .map(|(_, s)| s.clone())
        .collect()
}
