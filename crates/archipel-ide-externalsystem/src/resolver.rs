//! The resolver contract (`ExternalSystemProjectResolver`) and the registry of build systems
//! (`ExternalSystemManager.EP_NAME`).

use crate::data::{DataNode, ProjectSystemId};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{0}")]
    ToolMissing(String),
    #[error("{tool} failed: {message}")]
    ToolFailed { tool: String, message: String },
    #[error("import cancelled")]
    Cancelled,
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Io(String),
}

impl From<std::io::Error> for ImportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// Shared cancellation flag (`ProgressIndicator.cancel`).
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> Result<(), ImportError> {
        if self.is_cancelled() {
            Err(ImportError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// `ExternalSystemTaskNotificationListener`: progress text for the status bar and sync view.
pub trait ProgressListener: Send + Sync {
    fn text(&self, text: &str);
}

/// A listener that drops everything.
pub struct NoProgress;

impl ProgressListener for NoProgress {
    fn text(&self, _: &str) {}
}

/// Settings shared by all resolvers (`ExternalSystemExecutionSettings`).
#[derive(Debug, Clone, Default)]
pub struct ImportSettings {
    /// Resolve without network when the tool supports it; resolvers still fall back to
    /// online resolution when the cache is incomplete, unless this is `true`.
    pub offline_only: bool,
}

/// `resolveProjectInfo`'s inputs.
#[derive(Clone)]
pub struct ResolveRequest {
    pub root: PathBuf,
    pub settings: ImportSettings,
    pub cancel: CancelToken,
    pub listener: Arc<dyn ProgressListener>,
}

impl ResolveRequest {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            settings: ImportSettings::default(),
            cancel: CancelToken::default(),
            listener: Arc::new(NoProgress),
        }
    }

    pub fn progress(&self, text: &str) {
        self.listener.text(text);
    }
}

/// A build system the IDE can import: detection, build inputs and the resolver.
pub trait ProjectResolver: Send + Sync {
    fn system_id(&self) -> ProjectSystemId;

    /// Whether the directory is the root of a build of this system.
    fn is_applicable(&self, root: &Path) -> bool;

    /// Files whose change makes the model stale (auto-import triggers).
    fn is_build_input(&self, path: &Path) -> bool;

    /// `resolveProjectInfo`: runs the tool and describes the build.
    fn resolve(&self, request: &ResolveRequest) -> Result<DataNode, ImportError>;
}

/// The registered build systems.
#[derive(Clone)]
pub struct ExternalSystemRegistry {
    resolvers: Vec<Arc<dyn ProjectResolver>>,
}

impl ExternalSystemRegistry {
    pub fn empty() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    /// Gradle, Maven, Cargo and Bazel.
    pub fn with_defaults() -> Self {
        let mut registry = Self::empty();
        registry.register(Arc::new(crate::gradle::GradleProjectResolver));
        registry.register(Arc::new(crate::maven::MavenProjectResolver));
        registry.register(Arc::new(crate::cargo::CargoProjectResolver));
        registry.register(Arc::new(crate::bazel::BazelProjectResolver));
        registry
    }

    pub fn register(&mut self, resolver: Arc<dyn ProjectResolver>) {
        self.resolvers.push(resolver);
    }

    pub fn resolvers(&self) -> &[Arc<dyn ProjectResolver>] {
        &self.resolvers
    }

    /// Resolvers that apply to the project directory.
    pub fn detect(&self, root: &Path) -> Vec<Arc<dyn ProjectResolver>> {
        self.resolvers
            .iter()
            .filter(|resolver| resolver.is_applicable(root))
            .cloned()
            .collect()
    }

    /// Whether a changed file is an input of any build system that applies to `root`.
    pub fn is_build_input(&self, root: &Path, path: &Path) -> bool {
        self.resolvers
            .iter()
            .any(|r| r.is_applicable(root) && r.is_build_input(path))
    }
}
