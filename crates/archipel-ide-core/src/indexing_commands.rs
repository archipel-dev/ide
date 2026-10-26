//! Indexing glue (WP2): attaches the index service to the open project,
//! feeds it the watcher's changes and exposes `index.status` and
//! `index.rebuild`. The service itself lives in `archipel-ide-index`; per
//! frame `poll_analysis` drives it (see `analysis_host.rs`).

use crate::{IdeState, spec};
use archipel_ide_analysis::{IndexId, ServiceRegistry};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_index::IndexStatus;
use archipel_ide_language::{Order, Registration};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

/// Services of the index package: the stub-based interface hash replaces
/// the structure-based default for every language.
pub(crate) fn register_services(registry: &mut ServiceRegistry) {
    registry.register::<dyn archipel_ide_analysis::InterfaceSummarizer>(
        Registration::any("stub-interface")
            .order(Order::First)
            .dumb_aware(true),
        Arc::new(archipel_ide_index::stubs::StubInterface),
    );
}

pub(crate) fn register_all(registry: &mut CommandRegistry<IdeState>) {
    register(registry);
    crate::todo_commands::register(registry);
    crate::history_commands::register(registry);
}

impl IdeState {
    /// The root directory of the persistent index store for this user, one
    /// directory per project (`<cache>/index/<project hash>`).
    pub fn enable_index_persistence(&mut self, cache_dir: Option<PathBuf>) {
        self.analysis.index_cache_dir = cache_dir;
    }

    /// Attaches the index service to the open project (and detaches it when
    /// the project closed), and applies changed TODO settings.
    pub(crate) fn sync_index_project(&mut self) {
        let root = self.project.as_ref().map(|p| p.root().to_path_buf());
        let cache = self.analysis.index_cache_dir.clone();
        self.sync_history_project();
        let todo = self.todo_settings();
        let waker = self.waker.clone();
        let services = &mut self.analysis;
        match (root, services.index.root().map(ToOwned::to_owned)) {
            (Some(root), current) if current.as_deref() != Some(root.as_path()) => {
                services.index.set_store_dir(cache.map(|dir| {
                    dir.join("index").join(format!(
                        "{:016x}",
                        archipel_ide_lsp::registry::stable_hash(&root.to_string_lossy())
                    ))
                }));
                services.index.set_todo_settings_quiet(todo);
                // Background batches wake the UI when they finish.
                services.host.set_wake(move || waker.wake());
                if let Some(ms) = archipel_ide_compat::env_var("ARCHIPEL_INDEX_THROTTLE_MS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                {
                    // Scenes and demos slow indexing down to show its progress.
                    services
                        .index
                        .set_throttle(std::time::Duration::from_millis(ms));
                }
                services.index.attach(&mut services.host, &root);
            }
            (None, Some(_)) => services.index.detach(&mut services.host),
            (Some(_), Some(_)) => {
                services.index.set_todo_settings(&mut services.host, todo);
            }
            _ => {}
        }
    }

    /// The watcher reported these project-relative paths.
    pub(crate) fn index_paths_changed(&mut self, paths: &BTreeSet<PathBuf>) {
        let paths: Vec<PathBuf> = paths.iter().cloned().collect();
        let services = &mut self.analysis;
        services.index.notify_paths(&mut services.host, &paths);
    }

    pub fn index_status(&self) -> IndexStatus {
        self.analysis.index.status()
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("index.status", "Index Status", "Index", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let mut status = state.index_status().to_json();
                let stats = state.analysis.index.stats();
                status["stats"] = json!({
                    "indexed": stats.indexed,
                    "unchanged": stats.unchanged,
                    "buffers": stats.buffers,
                });
                Ok(status)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "index.rebuild",
                "Rebuild Indexes",
                "Index",
                None,
                r#"{"index":"optional index id, e.g. Words; all when omitted"}"#,
            ),
            |state: &IdeState| state.analysis.index.is_attached(),
            |state: &mut IdeState, params| {
                let which = match params.get("index").and_then(Value::as_str) {
                    None | Some("") => None,
                    Some(name) => {
                        let status = state.index_status();
                        let known = status.indexes.iter().find(|info| info.id.0 == name);
                        Some(known.map(|info| info.id).ok_or_else(|| {
                            CommandError::InvalidParams(format!("unknown index `{name}`"))
                        })?)
                    }
                };
                let services = &mut state.analysis;
                services.index.rebuild(&mut services.host, which);
                state.status = match which {
                    Some(IndexId(name)) => format!("Rebuilding index {name}…"),
                    None => "Rebuilding indexes…".into(),
                };
                Ok(json!({"rebuilding": which.map(|id| id.0)}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::IdeState;
    use archipel_ide_index::ids;
    use serde_json::json;

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("dir");
        std::fs::create_dir_all(dir.path().join("src")).expect("dir");
        std::fs::write(
            dir.path().join("src/Main.java"),
            "class Main { // TODO ship it\n void run() {} }\n",
        )
        .expect("write");
        dir
    }

    fn state_with_inline_host() -> IdeState {
        IdeState {
            analysis: crate::AnalysisServices::new(archipel_ide_analysis::HostConfig::testing()),
            ..IdeState::default()
        }
    }

    fn settle(state: &mut IdeState) {
        for _ in 0..2000 {
            state.poll_analysis();
            if state.analysis.index.is_idle() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("indexing did not finish");
    }

    #[test]
    fn opening_a_project_indexes_it_and_the_commands_report_it() {
        let dir = project();
        let mut state = state_with_inline_host();
        state.open_project(dir.path()).expect("open");
        settle(&mut state);
        let commands = crate::built_in_commands();
        let status = commands
            .execute("index.status", &mut state, json!({}))
            .expect("status");
        assert_eq!(status["attached"], true);
        assert_eq!(status["phase"], "idle");
        assert_eq!(status["dumb"], false);
        assert!(status["indexes"].as_array().is_some_and(|i| i.len() >= 10));
        assert!(
            status["indexes"]
                .as_array()
                .expect("array")
                .iter()
                .all(|i| i["state"] == "ready")
        );
        assert!(status["stats"]["indexed"].as_u64().expect("n") >= 1);

        let snapshot = state.analysis.host.snapshot();
        let lookup = archipel_ide_index::readers::SymbolLookup::new(&snapshot).expect("ready");
        assert_eq!(lookup.by_name("Main").len(), 1);

        commands
            .execute("index.rebuild", &mut state, json!({"index": ids::WORDS.0}))
            .expect("rebuild one");
        assert!(state.analysis.host.snapshot().is_dumb());
        settle(&mut state);
        assert!(!state.analysis.host.snapshot().is_dumb());
        assert!(
            commands
                .execute("index.rebuild", &mut state, json!({"index": "Nope"}))
                .is_err()
        );
        commands
            .execute("index.rebuild", &mut state, json!({}))
            .expect("rebuild all");
        settle(&mut state);
        state.close_project().expect("close");
        state.poll_analysis();
        assert!(!state.analysis.index.is_attached());
        assert!(
            commands
                .execute("index.rebuild", &mut state, json!({}))
                .is_err(),
            "disabled without a project"
        );
    }
}
