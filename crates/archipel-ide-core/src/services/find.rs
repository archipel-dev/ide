//! Find in Files as a project service (IntelliJ: `FindManager` and `FindInProjectManager`).
//!
//! [`FindInProjectService`] owns the background search, its results and the options of the last
//! run, the scope the Find in Files popup has selected and the user-defined scopes of the
//! project. When a search finishes it announces [`FIND_TOPIC`] on the project bus; the status
//! bar text and the tool window follow from that instead of being called.

use crate::SearchScopeKind;
use crate::search_scopes::CustomScope;
use archipel_ide_platform::{Disposable, MessageBus, ServiceState, Topic};
use archipel_ide_search::{SearchError, SearchEvent, SearchOptions, SearchService, TextMatch};
use parking_lot::Mutex;
use std::path::PathBuf;

/// What a finished search found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchSummary {
    pub results: usize,
    pub files_scanned: usize,
    pub truncated: bool,
}

/// Reports Find in Files progress (IntelliJ: the find model change and usage view events).
pub trait FindListener: Send + Sync {
    /// The last search scanned every file it was going to (or hit the result limit).
    fn search_finished(&self, _summary: &SearchSummary) {}
}

/// On the project's message bus.
pub const FIND_TOPIC: Topic<dyn FindListener> = Topic::new("FindListener");

/// Results, scope and options of Find in Files.
#[derive(Default)]
pub struct FindState {
    pub results: Vec<TextMatch>,
    pub query: String,
    pub scope_kind: SearchScopeKind,
    /// User-defined Find in Files scopes of the project and the selected one.
    pub custom_scopes: Vec<CustomScope>,
    pub custom_scope: Option<String>,
    /// Set while the Find in Files popup drives the search: no tool window opens.
    pub background: bool,
    /// Options of the last Find in Files run.
    pub options: SearchOptions,
    pub files_scanned: usize,
    pub truncated: bool,
}

/// The project service.
pub struct FindInProjectService {
    bus: MessageBus,
    search: Mutex<SearchService>,
    state: ServiceState<FindState>,
}

impl FindInProjectService {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            search: Mutex::new(SearchService::default()),
            state: ServiceState::default(),
        }
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, FindState> {
        self.state.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, FindState> {
        self.state.write()
    }

    pub fn is_running(&self) -> bool {
        self.search.lock().is_running()
    }

    pub fn cancel(&self) {
        self.search.lock().cancel();
    }

    /// Starts a background search over `files` and forgets the previous results.
    pub fn start(
        &self,
        root: PathBuf,
        files: Vec<PathBuf>,
        query: String,
        options: SearchOptions,
    ) -> Result<u64, SearchError> {
        let job = self
            .search
            .lock()
            .start(root, files, query.clone(), &options)?;
        self.state.with_mut(|state| {
            state.results.clear();
            state.files_scanned = 0;
            state.truncated = false;
            state.query = query;
            state.options = options;
        });
        Ok(job)
    }

    /// Takes what the search thread produced since the last call; announces [`FIND_TOPIC`] when
    /// the search finished. Returns the summary in that case.
    pub fn poll(&self) -> Option<SearchSummary> {
        let events = self.search.lock().poll();
        let mut summary = None;
        self.state.with_mut(|state| {
            for event in events {
                match event {
                    SearchEvent::Match(found) => state.results.push(found),
                    SearchEvent::Finished {
                        files_scanned,
                        truncated,
                        ..
                    } => {
                        state.files_scanned = files_scanned;
                        state.truncated = truncated;
                        summary = Some(SearchSummary {
                            results: state.results.len(),
                            files_scanned,
                            truncated,
                        });
                    }
                }
            }
        });
        if let Some(summary) = &summary {
            self.bus.publish(FIND_TOPIC, |l| l.search_finished(summary));
        }
        summary
    }
}

impl Disposable for FindInProjectService {
    /// Stops the search thread's reporting and drops the results.
    fn dispose(&self) {
        self.search.lock().cancel();
        self.state.with_mut(|state| *state = FindState::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager, Project, ServiceDescriptor};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[derive(Default)]
    struct Heard(Mutex<Vec<SearchSummary>>);

    impl FindListener for Heard {
        fn search_finished(&self, summary: &SearchSummary) {
            self.0.lock().push(*summary);
        }
    }

    fn project(dir: &std::path::Path) -> (Application, Project) {
        let app = Application::new();
        app.register_project_service(ServiceDescriptor::disposable(|project: &Project| {
            FindInProjectService::new(project.message_bus().clone())
        }))
        .unwrap();
        let project = app.project_manager().open(dir);
        (app, project)
    }

    #[test]
    fn a_finished_search_is_announced_with_its_summary() {
        let dir =
            std::env::temp_dir().join(format!("archipel-find-service-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "needle\nhay\nneedle\n").unwrap();
        let (_app, project) = project(&dir);
        let heard = Arc::new(Heard::default());
        project
            .message_bus()
            .subscribe(project.disposable(), FIND_TOPIC, heard.clone())
            .unwrap();
        let find = project.service::<FindInProjectService>();
        find.start(
            dir.clone(),
            vec![PathBuf::from("a.txt")],
            "needle".into(),
            SearchOptions::default(),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while find.is_running() && Instant::now() < deadline {
            find.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        find.poll();
        assert_eq!(find.read().query, "needle");
        assert_eq!(find.read().results.len(), 2);
        assert_eq!(
            *heard.0.lock(),
            [SearchSummary {
                results: 2,
                files_scanned: 1,
                truncated: false
            }]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disposing_the_project_forgets_the_results_and_cancels() {
        let dir =
            std::env::temp_dir().join(format!("archipel-find-dispose-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (app, project) = project(&dir);
        let find = project.service::<FindInProjectService>();
        find.write().query = "x".into();
        find.write().results.push(TextMatch {
            path: "a".into(),
            line: 1,
            column: 1,
            length: 1,
            preview: String::new(),
            highlights: Vec::new(),
        });
        app.project_manager().close(&project);
        assert!(find.read().query.is_empty() && find.read().results.is_empty());
        assert!(!find.is_running());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
