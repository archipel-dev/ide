//! Runs the language servers of a project.
//!
//! Servers start lazily the first time a document of their language is
//! synchronised, one per (server, workspace root). Crashed servers are
//! restarted with exponential backoff and their documents re-opened; missing
//! servers produce one actionable [`LspNotice`] each.

use crate::{
    LspError,
    client::{ApplyEditRequest, LspDiagnostic, LspService, ServerState, WorkProgress},
    registry::{
        LaunchContext, LspSettings, ProgramResolver, ServerRegistry, cache_dir, workspace_root,
    },
};
use lsp_types::{CompletionItem, CompletionResponse, CompletionTriggerKind, request::Completion};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

/// Crashes after which a server is not restarted automatically.
const MAX_RESTARTS: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ServerKey {
    pub server: String,
    pub root: PathBuf,
}

/// Lifecycle as shown in the status bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ServerStatus {
    Starting,
    Ready,
    /// Crashed; restarting at `attempt` after a backoff.
    Restarting {
        attempt: u32,
    },
    /// Gave up after repeated crashes or failed to launch.
    Failed {
        reason: String,
    },
}

/// Snapshot of one server for the status bar widget and `language.servers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerInfo {
    pub id: String,
    pub language: String,
    pub root: PathBuf,
    pub status: ServerStatus,
    pub progress: Vec<WorkProgress>,
    pub message: Option<String>,
}

/// A request sent with [`LspManager::request`] or [`LspManager::send_request`],
/// answered asynchronously.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestHandle {
    pub server: ServerKey,
    pub id: u64,
    pub method: String,
}

/// Optional server features, checked against the advertised capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Definition,
    TypeDefinition,
    Implementation,
    References,
    CodeAction,
    CodeActionResolve,
    ExecuteCommand,
    PrepareRename,
    Rename,
    Formatting,
    RangeFormatting,
    PrepareTypeHierarchy,
    TypeHierarchySupertypes,
    TypeHierarchySubtypes,
    PrepareCallHierarchy,
    CallHierarchyIncoming,
    CallHierarchyOutgoing,
}

impl Feature {
    pub fn method(self) -> &'static str {
        match self {
            Feature::Definition => "textDocument/definition",
            Feature::TypeDefinition => "textDocument/typeDefinition",
            Feature::Implementation => "textDocument/implementation",
            Feature::References => "textDocument/references",
            Feature::CodeAction => "textDocument/codeAction",
            Feature::CodeActionResolve => "codeAction/resolve",
            Feature::ExecuteCommand => "workspace/executeCommand",
            Feature::PrepareRename => "textDocument/prepareRename",
            Feature::Rename => "textDocument/rename",
            Feature::Formatting => "textDocument/formatting",
            Feature::RangeFormatting => "textDocument/rangeFormatting",
            Feature::PrepareTypeHierarchy => "textDocument/prepareTypeHierarchy",
            Feature::TypeHierarchySupertypes => "typeHierarchy/supertypes",
            Feature::TypeHierarchySubtypes => "typeHierarchy/subtypes",
            Feature::PrepareCallHierarchy => "textDocument/prepareCallHierarchy",
            Feature::CallHierarchyIncoming => "callHierarchy/incomingCalls",
            Feature::CallHierarchyOutgoing => "callHierarchy/outgoingCalls",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Feature::Definition => "Go to Declaration",
            Feature::TypeDefinition => "Go to Type Declaration",
            Feature::Implementation => "Go to Implementation",
            Feature::References => "Find Usages",
            Feature::CodeAction | Feature::CodeActionResolve => "code actions",
            Feature::ExecuteCommand => "commands",
            Feature::PrepareRename | Feature::Rename => "Rename",
            Feature::Formatting | Feature::RangeFormatting => "Reformat Code",
            Feature::PrepareTypeHierarchy
            | Feature::TypeHierarchySupertypes
            | Feature::TypeHierarchySubtypes => "Type Hierarchy",
            Feature::PrepareCallHierarchy
            | Feature::CallHierarchyIncoming
            | Feature::CallHierarchyOutgoing => "Call Hierarchy",
        }
    }

    fn supported(self, capabilities: &lsp_types::ServerCapabilities) -> bool {
        use lsp_types::{
            ImplementationProviderCapability as Impl, OneOf,
            TypeDefinitionProviderCapability as TypeDef,
        };
        match self {
            Feature::Definition => !matches!(
                capabilities.definition_provider,
                None | Some(OneOf::Left(false))
            ),
            Feature::TypeDefinition => !matches!(
                capabilities.type_definition_provider,
                None | Some(TypeDef::Simple(false))
            ),
            Feature::Implementation => !matches!(
                capabilities.implementation_provider,
                None | Some(Impl::Simple(false))
            ),
            Feature::References => !matches!(
                capabilities.references_provider,
                None | Some(OneOf::Left(false))
            ),
            Feature::CodeAction => !matches!(
                capabilities.code_action_provider,
                None | Some(lsp_types::CodeActionProviderCapability::Simple(false))
            ),
            Feature::CodeActionResolve => matches!(
                &capabilities.code_action_provider,
                Some(lsp_types::CodeActionProviderCapability::Options(options))
                    if options.resolve_provider == Some(true)
            ),
            Feature::ExecuteCommand => capabilities.execute_command_provider.is_some(),
            Feature::PrepareRename => matches!(
                &capabilities.rename_provider,
                Some(OneOf::Right(options)) if options.prepare_provider == Some(true)
            ),
            Feature::Rename => !matches!(
                capabilities.rename_provider,
                None | Some(OneOf::Left(false))
            ),
            Feature::Formatting => !matches!(
                capabilities.document_formatting_provider,
                None | Some(OneOf::Left(false))
            ),
            Feature::RangeFormatting => !matches!(
                capabilities.document_range_formatting_provider,
                None | Some(OneOf::Left(false))
            ),
            // `lsp-types` has no `typeHierarchyProvider` field: assume a running
            // server answers (it replies "method not found" otherwise).
            Feature::PrepareTypeHierarchy
            | Feature::TypeHierarchySupertypes
            | Feature::TypeHierarchySubtypes => true,
            Feature::PrepareCallHierarchy
            | Feature::CallHierarchyIncoming
            | Feature::CallHierarchyOutgoing => !matches!(
                capabilities.call_hierarchy_provider,
                None | Some(lsp_types::CallHierarchyServerCapability::Simple(false))
            ),
        }
    }
}

/// A notification balloon about a language server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LspNotice {
    /// Stable id used to dismiss the notice (`missing:jdtls`).
    pub id: String,
    pub title: String,
    pub body: String,
    pub error: bool,
    /// Toolchain component that a "Set up" action installs (missing servers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<String>,
}

struct ManagedServer {
    key: ServerKey,
    language: &'static str,
    service: Option<LspService>,
    crashes: u32,
    restart_at: Option<Instant>,
    failure: Option<String>,
}

struct OpenDocument {
    server: ServerKey,
    language_id: &'static str,
}

struct CompletionRequest {
    server: ServerKey,
    id: u64,
}

pub struct LspManager {
    registry: ServerRegistry,
    settings: LspSettings,
    project_root: Option<PathBuf>,
    servers: Vec<ManagedServer>,
    documents: HashMap<PathBuf, OpenDocument>,
    /// Servers whose executable is missing, by id; checked once per project.
    missing: BTreeSet<String>,
    notices: Vec<LspNotice>,
    dismissed: BTreeSet<String>,
    completion: Option<CompletionRequest>,
    completion_result: Option<CompletionResult>,
    /// Diagnostics revisions of the live servers at the last poll.
    diagnostics_seen: Vec<u64>,
    diagnostics_generation: u64,
    resolver: Option<Arc<dyn ProgramResolver>>,
}

/// Why completion was requested (LSP `CompletionTriggerKind`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionTrigger {
    /// Ctrl+Space or typing an identifier character.
    Invoked,
    /// A server trigger character such as `.`.
    Character(String),
    /// Re-query because the previous result was `isIncomplete`.
    Incomplete,
}

/// A completion response.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompletionResult {
    pub items: Vec<CompletionItem>,
    pub incomplete: bool,
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new(ServerRegistry::default())
    }
}

impl LspManager {
    pub fn new(registry: ServerRegistry) -> Self {
        Self {
            registry,
            settings: LspSettings::default(),
            project_root: None,
            servers: Vec::new(),
            documents: HashMap::new(),
            missing: BTreeSet::new(),
            notices: Vec::new(),
            dismissed: BTreeSet::new(),
            completion: None,
            completion_result: None,
            diagnostics_seen: Vec::new(),
            diagnostics_generation: 0,
            resolver: None,
        }
    }

    /// Installs the hook that finds provisioned servers outside `PATH`.
    pub fn set_resolver(&mut self, resolver: Option<Arc<dyn ProgramResolver>>) {
        self.resolver = resolver;
    }

    pub fn registry(&self) -> &ServerRegistry {
        &self.registry
    }

    /// Stops all servers and switches to `root`, reloading `lsp.toml`.
    pub fn set_project(&mut self, root: Option<&Path>) {
        self.shutdown();
        self.missing.clear();
        self.notices.clear();
        self.project_root = root.map(|root| root.canonicalize().unwrap_or_else(|_| root.into()));
        self.settings = match LspSettings::load(self.project_root.as_deref()) {
            Ok(settings) => settings,
            Err(error) => {
                self.push_notice(LspNotice {
                    id: "settings".into(),
                    title: "Invalid language server settings".into(),
                    body: error.to_string(),
                    error: true,
                    setup: None,
                });
                LspSettings::default()
            }
        };
    }

    /// Stops every server.
    pub fn shutdown(&mut self) {
        self.cancel_completion();
        self.documents.clear();
        for server in &mut self.servers {
            if let Some(service) = server.service.as_mut() {
                service.shutdown();
            }
        }
        self.servers.clear();
        self.diagnostics_seen.clear();
        self.diagnostics_generation += 1;
    }

    /// Restarts every server and re-opens its documents on the next sync.
    /// Returns the documents that need to be re-synchronised.
    pub fn restart_all(&mut self) -> Vec<PathBuf> {
        let paths: Vec<PathBuf> = self.documents.keys().cloned().collect();
        self.shutdown();
        self.missing.clear();
        self.notices
            .retain(|notice| !notice.id.starts_with("missing:"));
        self.dismissed.clear();
        paths
    }

    /// Whether some enabled server handles `path`.
    pub fn handles(&self, path: &Path) -> bool {
        self.registry
            .for_path(path)
            .is_some_and(|(provider, _)| self.enabled(provider.spec().id))
    }

    fn enabled(&self, id: &str) -> bool {
        self.settings
            .servers
            .get(id)
            .is_none_or(|settings| settings.enabled)
    }

    /// Opens or updates `path` (absolute) on its server, starting the server if needed.
    pub fn sync_document(&mut self, path: &Path, text: &str, version: i32) -> Result<(), LspError> {
        let Some((provider, language_id)) = self.registry.for_path(path) else {
            return Ok(());
        };
        let spec = provider.spec();
        if !self.enabled(spec.id) || self.missing.contains(spec.id) {
            return Ok(());
        }
        let project_root = self
            .project_root
            .clone()
            .unwrap_or_else(|| path.parent().unwrap_or(path).to_owned());
        let key = ServerKey {
            server: spec.id.into(),
            root: workspace_root(path, &project_root, spec.root_markers),
        };
        let language = spec.language;
        let index = match self.servers.iter().position(|server| server.key == key) {
            Some(index) => index,
            None => {
                let service = match self.launch(&key) {
                    Ok(service) => Some(service),
                    Err(error @ LspError::ServerNotFound { .. }) => {
                        let spec = self.registry.get(&key.server).map(|p| p.spec());
                        let notify = spec.is_some_and(|spec| spec.notify_missing);
                        let setup = spec
                            .zip(self.resolver.as_deref())
                            .and_then(|(spec, resolver)| resolver.setup_component(spec));
                        self.missing.insert(key.server.clone());
                        if notify {
                            self.push_notice(LspNotice {
                                id: format!("missing:{}", key.server),
                                title: format!("{language} language server not found"),
                                body: error.to_string(),
                                error: false,
                                setup,
                            });
                        }
                        return Err(error);
                    }
                    Err(error) => {
                        self.servers.push(ManagedServer {
                            key: key.clone(),
                            language,
                            service: None,
                            crashes: MAX_RESTARTS,
                            restart_at: None,
                            failure: Some(error.to_string()),
                        });
                        return Err(error);
                    }
                };
                self.servers.push(ManagedServer {
                    key: key.clone(),
                    language,
                    service,
                    crashes: 0,
                    restart_at: None,
                    failure: None,
                });
                self.servers.len() - 1
            }
        };
        self.documents.insert(
            path.to_owned(),
            OpenDocument {
                server: key,
                language_id,
            },
        );
        match self.servers[index].service.as_mut() {
            Some(service) if service.state() != ServerState::Exited => {
                match service.sync_document(path, language_id, text, version) {
                    // The server exited after the state check; `poll` notices the exit,
                    // restarts it and re-sends every open document, this one included.
                    Err(LspError::Start(error))
                        if error.kind() == std::io::ErrorKind::BrokenPipe =>
                    {
                        Ok(())
                    }
                    other => other,
                }
            }
            // Restarting: the document is re-sent once the server is back.
            _ => Ok(()),
        }
    }

    /// Notifies the server of `path` that the document was saved.
    pub fn save_document(&mut self, path: &Path) -> Result<(), LspError> {
        let Some(key) = self
            .documents
            .get(path)
            .map(|document| document.server.clone())
        else {
            return Ok(());
        };
        match self.service_mut(&key) {
            Some(service) if service.state() == ServerState::Ready => service.save_document(path),
            _ => Ok(()),
        }
    }

    pub fn close_document(&mut self, path: &Path) -> Result<(), LspError> {
        let Some(document) = self.documents.remove(path) else {
            return Ok(());
        };
        match self.service_mut(&document.server) {
            Some(service) => service.close_document(path),
            None => Ok(()),
        }
    }

    fn launch(&self, key: &ServerKey) -> Result<LspService, LspError> {
        let provider = self
            .registry
            .get(&key.server)
            .ok_or_else(|| LspError::Protocol(format!("unknown server {}", key.server)))?;
        let config = provider.launch(&LaunchContext {
            root: &key.root,
            cache_dir: cache_dir(),
            settings: self.settings.servers.get(&key.server),
            resolver: self.resolver.as_deref(),
        })?;
        LspService::launch(&config, &key.root)
    }

    /// Pumps every server, restarts crashed ones and collects completion
    /// results. Returns documents that must be re-synchronised because their
    /// server restarted.
    pub fn poll(&mut self) -> Vec<PathBuf> {
        let now = Instant::now();
        let mut restarted = Vec::new();
        for index in 0..self.servers.len() {
            let server = &mut self.servers[index];
            if let Some(service) = server.service.as_mut() {
                if let Err(error) = service.poll() {
                    server.failure = Some(error.to_string());
                    service.shutdown();
                }
                if service.state() == ServerState::Exited && server.restart_at.is_none() {
                    let reason = service
                        .exit_reason()
                        .map(str::to_owned)
                        .or_else(|| server.failure.clone())
                        .unwrap_or_else(|| format!("{} exited", server.key.server));
                    server.service = None;
                    server.crashes += 1;
                    if server.crashes > MAX_RESTARTS {
                        server.failure = Some(reason.clone());
                        let notice = LspNotice {
                            id: format!("crashed:{}", server.key.server),
                            title: format!("{} language server keeps crashing", server.language),
                            body: format!(
                                "{reason}\n\nRun \"Restart Language Servers\" after fixing the problem."
                            ),
                            error: true,
                            setup: None,
                        };
                        self.push_notice(notice);
                        continue;
                    }
                    let backoff = Duration::from_secs(1 << (server.crashes - 1).min(5));
                    server.restart_at = Some(now + backoff);
                    server.failure = Some(reason);
                }
            }
            let server = &mut self.servers[index];
            if server.service.is_none()
                && server.restart_at.is_some_and(|at| at <= now)
                && server.crashes <= MAX_RESTARTS
            {
                server.restart_at = None;
                let key = server.key.clone();
                match self.launch(&key) {
                    Ok(service) => {
                        self.servers[index].service = Some(service);
                        restarted.extend(
                            self.documents
                                .iter()
                                .filter(|(_, document)| document.server == key)
                                .map(|(path, _)| path.clone()),
                        );
                    }
                    Err(error) => {
                        let server = &mut self.servers[index];
                        server.crashes = MAX_RESTARTS + 1;
                        server.failure = Some(error.to_string());
                    }
                }
            }
        }
        self.poll_completion();
        let seen: Vec<u64> = self
            .servers
            .iter()
            .map(|server| {
                server
                    .service
                    .as_ref()
                    .map_or(0, LspService::diagnostics_revision)
            })
            .collect();
        if seen != self.diagnostics_seen {
            self.diagnostics_seen = seen;
            self.diagnostics_generation += 1;
        }
        restarted
    }

    /// Changes whenever the published diagnostics of any server may have
    /// changed (after [`poll`](Self::poll)), so callers can cache derived views.
    pub fn diagnostics_generation(&self) -> u64 {
        self.diagnostics_generation
    }

    /// Every file with published diagnostics, across all running servers.
    /// Paths are as the servers reported them (absolute, usually canonical).
    pub fn all_diagnostics(&self) -> impl Iterator<Item = (&Path, &[LspDiagnostic])> {
        self.servers
            .iter()
            .filter_map(|server| server.service.as_ref())
            .flat_map(|service| service.diagnostics().iter())
            .map(|(path, diagnostics)| (path.as_path(), diagnostics.as_slice()))
    }

    /// The running server for `path`, if any.
    pub fn server_for(&self, path: &Path) -> Option<&LspService> {
        let key = &self.documents.get(path)?.server;
        self.servers
            .iter()
            .find(|server| &server.key == key)?
            .service
            .as_ref()
    }

    pub fn server_for_mut(&mut self, path: &Path) -> Option<&mut LspService> {
        let key = self.documents.get(path)?.server.clone();
        self.service_mut(&key)
    }

    fn service_mut(&mut self, key: &ServerKey) -> Option<&mut LspService> {
        self.servers
            .iter_mut()
            .find(|server| &server.key == key)?
            .service
            .as_mut()
    }

    /// A ready server for `path`, or an actionable explanation of why there is none.
    pub fn ready_server_for(&mut self, path: &Path) -> Result<&mut LspService, LspError> {
        let Some((provider, _)) = self.registry.for_path(path) else {
            return Err(LspError::Unsupported(
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ));
        };
        let spec = provider.spec();
        let (id, language) = (spec.id, spec.language);
        if !self.enabled(id) {
            return Err(LspError::Disabled(id.into()));
        }
        if self.missing.contains(id) {
            return Err(LspError::ServerNotFound {
                server: id.into(),
                language: language.into(),
                hint: spec.install_hint.into(),
            });
        }
        let Some(document) = self.documents.get(path) else {
            return Err(LspError::NotReady(id.into()));
        };
        let key = document.server.clone();
        let server = self
            .servers
            .iter_mut()
            .find(|server| server.key == key)
            .ok_or_else(|| LspError::NotReady(id.into()))?;
        match server.service.as_mut() {
            Some(service) if service.state() == ServerState::Ready => Ok(service),
            Some(service) => {
                let detail = service
                    .progress()
                    .values()
                    .next()
                    .map(describe_progress)
                    .or_else(|| service.status_message().map(str::to_owned));
                Err(LspError::Starting {
                    server: id.into(),
                    detail,
                })
            }
            None => Err(match &server.failure {
                Some(reason) if server.crashes > MAX_RESTARTS => LspError::Protocol(format!(
                    "{id} failed: {}",
                    reason.lines().next().unwrap_or_default()
                )),
                _ => LspError::Starting {
                    server: id.into(),
                    detail: Some("restarting after a crash".into()),
                },
            }),
        }
    }

    /// Diagnostics published for `path` by its server.
    pub fn diagnostics_for(&self, path: &Path) -> Vec<LspDiagnostic> {
        self.published_diagnostics(path).unwrap_or_default()
    }

    /// Diagnostics for `path`, or `None` when its server has not checked it
    /// yet (so callers can fall back to syntax errors).
    pub fn published_diagnostics(&self, path: &Path) -> Option<Vec<LspDiagnostic>> {
        let service = self.server_for(path)?;
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_owned());
        service
            .diagnostics()
            .get(&canonical)
            .or_else(|| service.diagnostics().get(path))
            .cloned()
    }

    /// One entry per started server.
    /// Whether any language server has been started for this project.
    pub fn has_servers(&self) -> bool {
        !self.servers.is_empty()
    }

    pub fn servers(&self) -> Vec<ServerInfo> {
        self.servers
            .iter()
            .map(|server| {
                let (status, progress, message) = match &server.service {
                    Some(service) => (
                        match service.state() {
                            ServerState::Ready => ServerStatus::Ready,
                            _ => ServerStatus::Starting,
                        },
                        service.progress().values().cloned().collect(),
                        service.status_message().map(str::to_owned),
                    ),
                    None if server.restart_at.is_some() => (
                        ServerStatus::Restarting {
                            attempt: server.crashes,
                        },
                        Vec::new(),
                        server.failure.clone(),
                    ),
                    None => (
                        ServerStatus::Failed {
                            reason: server.failure.clone().unwrap_or_default(),
                        },
                        Vec::new(),
                        None,
                    ),
                };
                ServerInfo {
                    id: server.key.server.clone(),
                    language: server.language.into(),
                    root: server.key.root.clone(),
                    status,
                    progress,
                    message,
                }
            })
            .collect()
    }

    /// Ids of servers that were looked for but are not installed.
    pub fn missing_servers(&self) -> impl Iterator<Item = &str> {
        self.missing.iter().map(String::as_str)
    }

    /// What the server of `path` is still busy with (starting, importing the
    /// project, indexing), if anything. Completion answers given meanwhile may
    /// be empty or incomplete (jdtls answers nothing while "Building 0%").
    pub fn indexing_status(&self, path: &Path) -> Option<String> {
        let service = self.server_for(path)?;
        if let Some(progress) = service.progress().values().next() {
            return Some(describe_progress(progress));
        }
        (service.state() == ServerState::Starting).then(|| format!("{}: starting", service.name()))
    }

    /// Whether the server of `path` is ready and has no work in progress.
    pub fn is_idle_for(&self, path: &Path) -> bool {
        self.server_for(path).is_some_and(|service| {
            service.state() == ServerState::Ready && service.progress().is_empty()
        })
    }

    /// Whether any server is starting or reporting progress.
    pub fn is_busy(&self) -> bool {
        self.servers.iter().any(|server| match &server.service {
            Some(service) => {
                service.state() == ServerState::Starting || !service.progress().is_empty()
            }
            None => server.restart_at.is_some(),
        }) || self.completion.is_some()
    }

    pub fn notices(&self) -> &[LspNotice] {
        &self.notices
    }

    pub fn dismiss_notice(&mut self, id: &str) {
        self.notices.retain(|notice| notice.id != id);
        self.dismissed.insert(id.into());
    }

    fn push_notice(&mut self, notice: LspNotice) {
        if self.dismissed.contains(&notice.id) {
            return;
        }
        self.notices.retain(|existing| existing.id != notice.id);
        self.notices.push(notice);
    }

    /// Sends `feature`'s request for the UTF-16 position in `path` to its
    /// ready server. `extra` fields are merged into the position params
    /// (e.g. `{"context": {"includeDeclaration": true}}`).
    pub fn request(
        &mut self,
        path: &Path,
        feature: Feature,
        line: usize,
        character: usize,
        extra: serde_json::Value,
    ) -> Result<RequestHandle, LspError> {
        let mut params = json!({
            "textDocument": {"uri": crate::client::file_uri(path)?},
            "position": {"line": line, "character": character},
        });
        if let (Some(params), serde_json::Value::Object(extra)) = (params.as_object_mut(), extra) {
            params.extend(extra);
        }
        self.request_with_params(path, feature, params)
    }

    /// Sends `feature`'s request with complete `params` to the ready server
    /// of `path`, after checking the server advertises the feature.
    pub fn request_with_params(
        &mut self,
        path: &Path,
        feature: Feature,
        params: serde_json::Value,
    ) -> Result<RequestHandle, LspError> {
        let service = self.ready_server_for(path)?;
        if !feature.supported(service.capabilities()) {
            return Err(LspError::Protocol(format!(
                "{} does not support {}",
                service.name(),
                feature.label()
            )));
        }
        let server = ServerKey {
            server: service.name().into(),
            root: service.root().to_owned(),
        };
        let id = service.request_raw(feature.method(), params)?;
        let server = self
            .documents
            .get(path)
            .map(|document| document.server.clone())
            .unwrap_or(server);
        Ok(RequestHandle {
            server,
            id,
            method: feature.method().into(),
        })
    }

    /// Whether the server of `path` is running and advertises `feature`.
    pub fn supports(&self, path: &Path, feature: Feature) -> bool {
        self.server_for(path)
            .is_some_and(|service| feature.supported(service.capabilities()))
    }

    /// `workspace/applyEdit` requests the servers sent since the last call.
    /// Each must be answered with [`respond_apply_edit`](Self::respond_apply_edit).
    pub fn take_apply_edits(&mut self) -> Vec<(ServerKey, ApplyEditRequest)> {
        let mut requests = Vec::new();
        for server in &mut self.servers {
            if let Some(service) = server.service.as_mut() {
                requests.extend(
                    service
                        .take_apply_edits()
                        .into_iter()
                        .map(|request| (server.key.clone(), request)),
                );
            }
        }
        requests
    }

    pub fn respond_apply_edit(
        &mut self,
        server: &ServerKey,
        id: serde_json::Value,
        result: Result<(), String>,
    ) {
        if let Some(service) = self.service_mut(server) {
            let _ = service.respond_apply_edit(id, result);
        }
    }

    /// The result of `handle` once it arrived; an error when the server
    /// answered with one or went away.
    pub fn take_response(
        &mut self,
        handle: &RequestHandle,
    ) -> Option<Result<serde_json::Value, LspError>> {
        let Some(service) = self.service_mut(&handle.server) else {
            return Some(Err(LspError::NotReady(handle.server.server.clone())));
        };
        if let Some(response) = service.take_response(handle.id) {
            return Some(response.result.map_err(|error| LspError::Server {
                code: error.code,
                message: error.message,
            }));
        }
        if !service.is_pending(handle.id) || service.state() == ServerState::Exited {
            return Some(Err(LspError::NotReady(handle.server.server.clone())));
        }
        None
    }

    pub fn cancel_request(&mut self, handle: &RequestHandle) {
        if let Some(service) = self.service_mut(&handle.server) {
            service.cancel(handle.id);
        }
    }

    /// Requests completion at a UTF-16 position, cancelling any older request.
    pub fn request_completion(
        &mut self,
        path: &Path,
        line: usize,
        character: usize,
        trigger: CompletionTrigger,
    ) -> Result<u64, LspError> {
        self.cancel_completion();
        let service = self.ready_server_for(path)?;
        let server = ServerKey {
            server: service.name().into(),
            root: service.root().to_owned(),
        };
        let context = match trigger {
            CompletionTrigger::Invoked => json!({"triggerKind": CompletionTriggerKind::INVOKED}),
            CompletionTrigger::Character(character) => json!({
                "triggerKind": CompletionTriggerKind::TRIGGER_CHARACTER,
                "triggerCharacter": character
            }),
            CompletionTrigger::Incomplete => json!({
                "triggerKind": CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS
            }),
        };
        let id = service.request_raw(
            <Completion as lsp_types::request::Request>::METHOD,
            json!({
                "textDocument": {"uri": crate::client::file_uri(path)?},
                "position": {"line": line, "character": character},
                "context": context
            }),
        )?;
        let key = self
            .documents
            .get(path)
            .map(|document| document.server.clone())
            .unwrap_or(server);
        self.completion = Some(CompletionRequest { server: key, id });
        Ok(id)
    }

    /// Cancels the pending completion request and drops an untaken result.
    pub fn cancel_completion(&mut self) {
        self.completion_result = None;
        if let Some(request) = self.completion.take()
            && let Some(service) = self.service_mut(&request.server)
        {
            service.cancel(request.id);
        }
    }

    fn poll_completion(&mut self) {
        let Some(request) = &self.completion else {
            return;
        };
        let (key, id) = (request.server.clone(), request.id);
        let Some(service) = self.service_mut(&key) else {
            self.completion = None;
            return;
        };
        if !service.is_pending(id) && service.state() == ServerState::Exited {
            self.completion = None;
            return;
        }
        let Some(result) = service.take_typed::<Completion>(id) else {
            return;
        };
        self.completion = None;
        self.completion_result = Some(match result {
            Ok(Some(CompletionResponse::Array(items))) => CompletionResult {
                items,
                incomplete: false,
            },
            Ok(Some(CompletionResponse::List(list))) => CompletionResult {
                items: list.items,
                incomplete: list.is_incomplete,
            },
            Ok(None) | Err(_) => CompletionResult::default(),
        });
    }

    pub fn completion_pending(&self) -> bool {
        self.completion.is_some()
    }

    /// Takes the latest completion response once it has arrived.
    pub fn take_completion(&mut self) -> Option<CompletionResult> {
        self.completion_result.take()
    }

    /// Filters and sorts `items` for `path`'s language with its provider's
    /// relevance hints.
    pub fn rank_completions(
        &self,
        path: &Path,
        items: &[CompletionItem],
        prefix: &str,
        usage: impl Fn(&CompletionItem) -> i32,
    ) -> Vec<crate::completion::RankedItem> {
        match self.registry.for_path(path) {
            Some((provider, _)) => crate::completion::rank_with_usage(
                items,
                prefix,
                |item| provider.completion_relevance(item),
                usage,
            ),
            None => crate::completion::rank_with_usage(items, prefix, |_| 0, usage),
        }
    }

    /// Characters that open completion for `path`'s running server (`.`, `::`, ...).
    pub fn trigger_characters(&self, path: &Path) -> Vec<String> {
        self.server_for(path)
            .and_then(|service| service.capabilities().completion_provider.as_ref())
            .and_then(|provider| provider.trigger_characters.clone())
            .unwrap_or_default()
    }

    /// Characters that accept the selected completion for every item of
    /// `path`'s server (`allCommitCharacters`).
    pub fn all_commit_characters(&self, path: &Path) -> Vec<String> {
        self.server_for(path)
            .and_then(|service| service.capabilities().completion_provider.as_ref())
            .and_then(|provider| provider.all_commit_characters.clone())
            .unwrap_or_default()
    }

    /// Sends a typed request to the ready server of `path`. The handle is used
    /// to collect (`take_response`) or cancel (`cancel_request`) the result.
    pub fn send_request<R: lsp_types::request::Request>(
        &mut self,
        path: &Path,
        params: R::Params,
    ) -> Result<RequestHandle, LspError> {
        let service = self.ready_server_for(path)?;
        let server = ServerKey {
            server: service.name().into(),
            root: service.root().to_owned(),
        };
        let id = service.request::<R>(params)?;
        let server = self
            .documents
            .get(path)
            .map(|document| document.server.clone())
            .unwrap_or(server);
        Ok(RequestHandle {
            server,
            id,
            method: R::METHOD.into(),
        })
    }

    /// The decoded response for a [`send_request`](Self::send_request) handle,
    /// once it has arrived.
    pub fn take_typed_response<R: lsp_types::request::Request>(
        &mut self,
        handle: &RequestHandle,
    ) -> Option<Result<R::Result, LspError>> {
        let service = self.service_mut(&handle.server);
        match service {
            Some(service) => service.take_typed::<R>(handle.id),
            None => Some(Err(LspError::NotReady(handle.server.server.clone()))),
        }
    }

    /// Whether `handle` is still waiting for its response.
    pub fn is_pending(&self, handle: &RequestHandle) -> bool {
        self.servers
            .iter()
            .find(|server| server.key == handle.server)
            .and_then(|server| server.service.as_ref())
            .is_some_and(|service| service.is_pending(handle.id))
    }

    /// Server capabilities for `path`'s running server.
    pub fn capabilities_for(&self, path: &Path) -> Option<&lsp_types::ServerCapabilities> {
        self.server_for(path).map(LspService::capabilities)
    }

    /// The language id the server uses for `path`.
    pub fn language_id(&self, path: &Path) -> Option<&'static str> {
        self.documents
            .get(path)
            .map(|document| document.language_id)
            .or_else(|| self.registry.for_path(path).map(|(_, id)| id))
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// "Indexing 45% (crates/foo)". Servers that repeat the title in the
/// message (jdtls: "Initialize Workspace - 0%") get the message alone.
pub fn describe_progress(progress: &WorkProgress) -> String {
    let message = progress
        .message
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());
    let mut text = progress.title.clone();
    if let Some(message) = message
        && !progress.title.is_empty()
        && message.starts_with(&progress.title)
    {
        return message.to_owned();
    }
    if let Some(percentage) = progress.percentage {
        text.push_str(&format!(" {percentage}%"));
    }
    if let Some(message) = message {
        if text.is_empty() {
            text = message.into();
        } else {
            text.push_str(&format!(" ({message})"));
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_descriptions_avoid_repeating_the_title() {
        let progress = |title: &str, message: Option<&str>, percentage| WorkProgress {
            title: title.into(),
            message: message.map(Into::into),
            percentage,
        };
        assert_eq!(
            describe_progress(&progress("Indexing", Some("crates/foo"), Some(45))),
            "Indexing 45% (crates/foo)"
        );
        assert_eq!(
            describe_progress(&progress(
                "Initialize Workspace",
                Some("Initialize Workspace - 0% "),
                Some(0)
            )),
            "Initialize Workspace - 0%"
        );
        assert_eq!(
            describe_progress(&progress("", Some("Loading"), None)),
            "Loading"
        );
    }
}
