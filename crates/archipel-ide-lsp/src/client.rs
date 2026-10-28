//! A single language server process: lifecycle, document synchronisation,
//! request/response plumbing, progress and published diagnostics.
//!
//! The client never blocks the caller: messages are read on a background
//! thread and consumed by [`LspService::poll`], which the UI calls once per
//! frame. Requests return an id immediately; their results are picked up
//! later with [`LspService::take_response`] or cancelled with
//! [`LspService::cancel`].
//!
//! Diagnostics come two ways and are merged per file: pushed
//! (`textDocument/publishDiagnostics`) and pulled (`textDocument/diagnostic`, LSP 3.17). A
//! server that declares `diagnosticProvider` (rust-analyzer, which then serves its native
//! diagnostics only through the pull request, and asks for a new pull with
//! `workspace/diagnostic/refresh`) is pulled after every document change that reached it, so
//! errors show while typing, without saving. Changes are sent as full text after a short quiet
//! time ([`DEFAULT_CHANGE_DELAY`]); anything that talks to the server first sends the pending
//! change, so a request never sees stale text.

use crate::{LspError, transport};
use lsp_types::{
    ClientCapabilities, ClientInfo, CodeActionCapabilityResolveSupport,
    CodeActionClientCapabilities, CodeActionKindLiteralSupport, CodeActionLiteralSupport,
    CompletionClientCapabilities, CompletionItemCapability, CompletionItemCapabilityResolveSupport,
    CompletionItemKind, CompletionItemKindCapability, DiagnosticClientCapabilities, DiagnosticTag,
    DiagnosticWorkspaceClientCapabilities, GeneralClientCapabilities, HoverClientCapabilities,
    InitializeParams, InitializeResult, MarkupKind, ParameterInformationSettings,
    PositionEncodingKind, PublishDiagnosticsClientCapabilities, ServerCapabilities,
    SignatureHelpClientCapabilities, SignatureInformationSettings, TagSupport,
    TextDocumentClientCapabilities, TextDocumentSyncCapability, TextDocumentSyncClientCapabilities,
    WindowClientCapabilities, WorkspaceClientCapabilities, WorkspaceEditClientCapabilities,
    WorkspaceFolder, request::Request,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};
use url::Url;

/// How to launch a language server process.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchConfig {
    /// Display name used in the status bar and in error messages.
    pub name: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub initialization_options: Option<Value>,
}

impl LaunchConfig {
    pub fn new(name: impl Into<String>, program: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            initialization_options: None,
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Starting,
    Ready,
    Exited,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub message: String,
    /// LSP `DiagnosticSeverity`: 1 error, 2 warning, 3 information, 4 hint.
    pub severity: Option<u8>,
    /// Positions are LSP UTF-16 line/character pairs.
    pub start_line: usize,
    pub start_character: usize,
    pub end_line: usize,
    pub end_character: usize,
    /// Diagnostic code (`E0425`, `unused_variables`, a jdtls problem id).
    pub code: Option<String>,
    /// Which tool produced it (`rustc`, `rust-analyzer`, `Java`).
    pub source: Option<String>,
    /// Code is unused (`DiagnosticTag.Unnecessary`).
    pub unnecessary: bool,
    /// Code is deprecated (`DiagnosticTag.Deprecated`).
    pub deprecated: bool,
    /// The diagnostic as published, sent back in code action requests.
    #[serde(skip)]
    pub raw: Value,
}

/// A `workspace/applyEdit` request from the server, waiting for the IDE.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyEditRequest {
    pub id: Value,
    pub label: Option<String>,
    /// The `WorkspaceEdit`.
    pub edit: Value,
}

/// A `$/progress` work-done report that has begun and not yet ended.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct WorkProgress {
    pub title: String,
    pub message: Option<String>,
    pub percentage: Option<u32>,
}

/// Where a server that reports `language/status` (Eclipse JDT LS) is in
/// loading the project: the build import runs between `Starting` and
/// `ServiceReady`, and diagnostics published meanwhile are unreliable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportPhase {
    /// The server does not report a lifecycle (everything but jdtls).
    NotReported,
    /// Starting up or importing the build (Gradle/Maven) model.
    Importing,
    /// The project is loaded and the language services answer.
    ServiceReady,
    /// The server reported a failure while loading.
    Failed,
}

/// JSON-RPC error object returned by a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseError {
    pub code: i64,
    pub message: String,
}

/// A response to a request sent with [`LspService::request`].
#[derive(Debug, Clone, PartialEq)]
pub struct LspResponse {
    pub method: String,
    pub result: Result<Value, ResponseError>,
    pub latency: Duration,
}

/// A message the server asked the client to show (`window/showMessage`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerMessage {
    /// LSP `MessageType`: 1 error, 2 warning, 3 info, 4 log.
    pub kind: u8,
    pub text: String,
}

struct PendingRequest {
    method: String,
    sent: Instant,
}

struct QueuedDocument {
    language_id: String,
    text: String,
    version: i32,
}

/// The JSON-RPC id of the `initialize` request.
const INITIALIZE_ID: u64 = 1;
/// Lines of stderr kept for crash reports.
const STDERR_LINES: usize = 40;
/// Responses nobody collected are dropped beyond this many.
const MAX_UNCLAIMED_RESPONSES: usize = 256;
/// The pull request for a document's diagnostics.
const PULL_METHOD: &str = "textDocument/diagnostic";
/// How long an edit waits before it is sent to the server as `didChange`, so a burst of typing is
/// one change (the daemon's own restart delay, 300 ms, comes on top). A request, a save or a
/// close sends it at once.
pub const DEFAULT_CHANGE_DELAY: Duration = Duration::from_millis(100);

/// A change of an open document not sent yet.
struct PendingChange {
    text: String,
    version: i32,
    /// When it is sent at the latest (the first edit of the burst plus the delay).
    due: Instant,
}

/// The pull of one document: one request at a time, one more after a change that arrived while
/// it was out.
#[derive(Default)]
struct PullState {
    in_flight: Option<(u64, i32)>,
    again: bool,
}

pub struct LspService {
    name: String,
    root: PathBuf,
    child: Child,
    writer: Arc<Mutex<ChildStdin>>,
    messages: Receiver<Result<Value, LspError>>,
    stderr: Arc<Mutex<VecDeque<String>>>,
    state: ServerState,
    next_id: u64,
    capabilities: ServerCapabilities,
    queued_documents: BTreeMap<PathBuf, QueuedDocument>,
    open_documents: HashMap<PathBuf, i32>,
    /// What the file shows: the pulled and the pushed diagnostics merged.
    diagnostics: HashMap<PathBuf, Vec<LspDiagnostic>>,
    pushed: HashMap<PathBuf, Vec<LspDiagnostic>>,
    /// The document version the latest diagnostics of a file were computed for (the version
    /// the server announced, else the last one sent to it).
    diag_versions: HashMap<PathBuf, i32>,
    pulled: HashMap<PathBuf, Vec<LspDiagnostic>>,
    pulls: HashMap<PathBuf, PullState>,
    pull_requests: HashMap<u64, PathBuf>,
    pending_changes: BTreeMap<PathBuf, PendingChange>,
    change_delay: Duration,
    diagnostics_revision: u64,
    progress: BTreeMap<String, WorkProgress>,
    status_message: Option<String>,
    /// jdtls-style `language/status` lifecycle: `Starting` ... `ServiceReady`.
    import: ImportPhase,
    /// Counts the `ServiceReady` notifications (callers refresh once per change).
    service_ready_epoch: u64,
    /// `settings` of the initialization options: sent as configuration and
    /// answered to `workspace/configuration`.
    settings: Option<Value>,
    shown_messages: Vec<ServerMessage>,
    apply_edits: Vec<ApplyEditRequest>,
    pending: HashMap<u64, PendingRequest>,
    responses: HashMap<u64, LspResponse>,
    latencies: HashMap<String, Duration>,
    exit_reason: Option<String>,
}

impl LspService {
    /// Starts rust-analyzer from `PATH` (kept for callers that do not use the registry).
    pub fn start_rust_analyzer(root: &Path) -> Result<Self, LspError> {
        Self::launch(&LaunchConfig::new("rust-analyzer", "rust-analyzer"), root)
    }

    /// Starts an arbitrary program as a language server.
    pub fn start(program: &str, args: &[&str], root: &Path) -> Result<Self, LspError> {
        Self::launch(
            &LaunchConfig::new(program, program).args(args.iter().copied()),
            root,
        )
    }

    pub fn launch(config: &LaunchConfig, root: &Path) -> Result<Self, LspError> {
        let root = root.canonicalize()?;
        let mut child = Command::new(&config.program)
            .args(&config.args)
            .envs(config.env.iter().map(|(k, v)| (k, v)))
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().ok_or(LspError::MissingStream)?;
        let stdout = child.stdout.take().ok_or(LspError::MissingStream)?;
        let stderr_pipe = child.stderr.take().ok_or(LspError::MissingStream)?;
        let (sender, messages) = mpsc::channel();
        let reader_thread = thread::Builder::new()
            .name(format!("lsp-reader-{}", config.name))
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                loop {
                    let message = transport::read_message(&mut reader);
                    let terminal = message.is_err();
                    if sender.send(message).is_err() || terminal {
                        break;
                    }
                }
            });
        if let Err(error) = reader_thread {
            let _ = child.kill();
            return Err(error.into());
        }
        let stderr = Arc::new(Mutex::new(VecDeque::new()));
        let tail = Arc::clone(&stderr);
        let _ = thread::Builder::new()
            .name(format!("lsp-stderr-{}", config.name))
            .spawn(move || {
                for line in BufReader::new(stderr_pipe).lines() {
                    let Ok(line) = line else { break };
                    if let Ok(mut tail) = tail.lock() {
                        if tail.len() == STDERR_LINES {
                            tail.pop_front();
                        }
                        tail.push_back(line);
                    }
                }
            });
        let mut service = Self {
            name: config.name.clone(),
            root,
            child,
            writer: Arc::new(Mutex::new(stdin)),
            messages,
            stderr,
            state: ServerState::Starting,
            next_id: INITIALIZE_ID + 1,
            capabilities: ServerCapabilities::default(),
            queued_documents: BTreeMap::new(),
            open_documents: HashMap::new(),
            diagnostics: HashMap::new(),
            pushed: HashMap::new(),
            diag_versions: HashMap::new(),
            pulled: HashMap::new(),
            pulls: HashMap::new(),
            pull_requests: HashMap::new(),
            pending_changes: BTreeMap::new(),
            change_delay: DEFAULT_CHANGE_DELAY,
            diagnostics_revision: 0,
            progress: BTreeMap::new(),
            status_message: None,
            import: ImportPhase::NotReported,
            service_ready_epoch: 0,
            settings: config
                .initialization_options
                .as_ref()
                .and_then(|options| options.get("settings"))
                .cloned(),
            shown_messages: Vec::new(),
            apply_edits: Vec::new(),
            pending: HashMap::new(),
            responses: HashMap::new(),
            latencies: HashMap::new(),
            exit_reason: None,
        };
        let params = initialize_params(&service.root, config.initialization_options.clone())?;
        service.pending.insert(
            INITIALIZE_ID,
            PendingRequest {
                method: "initialize".into(),
                sent: Instant::now(),
            },
        );
        service.send(&json!({
            "jsonrpc": "2.0", "id": INITIALIZE_ID, "method": "initialize", "params": params,
        }))?;
        Ok(service)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn state(&self) -> ServerState {
        self.state
    }
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }
    pub fn diagnostics(&self) -> &HashMap<PathBuf, Vec<LspDiagnostic>> {
        &self.diagnostics
    }

    /// The document version `path`'s diagnostics were computed for, when known. A server that
    /// has not seen the latest edit yet answers for an older text: its positions do not fit.
    pub fn diagnostics_version(&self, path: &Path) -> Option<i32> {
        self.diag_versions
            .get(path)
            .or_else(|| self.diag_versions.get(&canonical(path)))
            .copied()
    }
    /// Work-done progress currently running on the server, by token.
    /// Changes whenever the server publishes diagnostics; unique across services.
    pub fn diagnostics_revision(&self) -> u64 {
        self.diagnostics_revision
    }

    pub fn progress(&self) -> &BTreeMap<String, WorkProgress> {
        &self.progress
    }
    /// Latest server status line (`language/status` from jdtls, for example).
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }
    /// The project import state of servers that report `language/status`.
    pub fn import_phase(&self) -> ImportPhase {
        self.import
    }
    /// Whether the server is still importing the project, so its diagnostics
    /// and code actions do not reflect the build yet.
    pub fn import_pending(&self) -> bool {
        self.import == ImportPhase::Importing
    }
    /// Increases every time the server reports `ServiceReady`.
    pub fn service_ready_epoch(&self) -> u64 {
        self.service_ready_epoch
    }
    /// Messages from `window/showMessage` since the last call.
    pub fn take_shown_messages(&mut self) -> Vec<ServerMessage> {
        std::mem::take(&mut self.shown_messages)
    }
    /// Why the server exited, including the tail of its stderr.
    pub fn exit_reason(&self) -> Option<&str> {
        self.exit_reason.as_deref()
    }
    /// Round-trip time of the last completed request for `method`.
    pub fn last_latency(&self, method: &str) -> Option<Duration> {
        self.latencies.get(method).copied()
    }
    pub fn is_open(&self, path: &Path) -> bool {
        self.open_documents.contains_key(path) || self.queued_documents.contains_key(path)
    }

    /// Reads all available messages and sends the changes that are due. Never blocks.
    pub fn poll(&mut self) -> Result<(), LspError> {
        self.flush_due_changes()?;
        while let Ok(message) = self.messages.try_recv() {
            match message {
                Ok(message) => self.handle(message)?,
                Err(error) => {
                    self.exit_reason.get_or_insert_with(|| error.to_string());
                    break;
                }
            }
        }
        if self.state != ServerState::Exited
            && let Some(status) = self.child.try_wait()?
        {
            self.state = ServerState::Exited;
            self.progress.clear();
            let tail = self
                .stderr
                .lock()
                .map(|tail| tail.iter().cloned().collect::<Vec<_>>().join("\n"))
                .unwrap_or_default();
            let mut reason = format!("{} exited ({status})", self.name);
            if !tail.is_empty() {
                reason.push_str(":\n");
                reason.push_str(&tail);
            }
            self.exit_reason = Some(reason);
        }
        Ok(())
    }

    /// Opens a document, or sends its full new text if it is already open.
    pub fn sync_document(
        &mut self,
        path: &Path,
        language_id: &str,
        text: &str,
        version: i32,
    ) -> Result<(), LspError> {
        if self.state != ServerState::Ready {
            self.queued_documents.insert(
                path.to_owned(),
                QueuedDocument {
                    language_id: language_id.into(),
                    text: text.into(),
                    version,
                },
            );
            return Ok(());
        }
        match self.open_documents.get(path) {
            Some(&open) if open == version && !self.pending_changes.contains_key(path) => Ok(()),
            Some(_) => {
                if self
                    .pending_changes
                    .get(path)
                    .is_some_and(|pending| pending.version == version)
                {
                    return Ok(());
                }
                let due = self
                    .pending_changes
                    .get(path)
                    .map_or_else(|| Instant::now() + self.change_delay, |pending| pending.due);
                self.pending_changes.insert(
                    path.to_owned(),
                    PendingChange {
                        text: text.into(),
                        version,
                        due,
                    },
                );
                if self.change_delay.is_zero() {
                    self.flush_changes(Some(path))?;
                }
                Ok(())
            }
            None => self.send_open(path, language_id, text, version),
        }
    }

    /// How long an edit waits before it is sent (see [`DEFAULT_CHANGE_DELAY`]).
    pub fn set_change_delay(&mut self, delay: Duration) {
        self.change_delay = delay;
    }

    /// Whether the server answers `textDocument/diagnostic` (LSP 3.17 pull diagnostics).
    pub fn supports_pull_diagnostics(&self) -> bool {
        self.capabilities.diagnostic_provider.is_some()
    }

    /// Whether an edit waits to be sent or a diagnostics pull is out.
    pub fn has_pending_work(&self) -> bool {
        !self.pending_changes.is_empty()
            || self
                .pulls
                .values()
                .any(|pull| pull.in_flight.is_some() || pull.again)
    }

    /// Sends the pending changes whose time has come.
    fn flush_due_changes(&mut self) -> Result<(), LspError> {
        if self.pending_changes.is_empty() || self.state != ServerState::Ready {
            return Ok(());
        }
        let now = Instant::now();
        let due: Vec<PathBuf> = self
            .pending_changes
            .iter()
            .filter(|(_, change)| change.due <= now)
            .map(|(path, _)| path.clone())
            .collect();
        for path in due {
            self.flush_changes(Some(&path))?;
        }
        Ok(())
    }

    /// Sends the pending change of `only` (every pending change for `None`) as `didChange`
    /// with the full text, then asks a pull-capable server for the document's diagnostics.
    fn flush_changes(&mut self, only: Option<&Path>) -> Result<(), LspError> {
        let paths: Vec<PathBuf> = match only {
            Some(path) => vec![path.to_owned()],
            None => self.pending_changes.keys().cloned().collect(),
        };
        for path in paths {
            let Some(change) = self.pending_changes.remove(&path) else {
                continue;
            };
            self.open_documents.insert(path.clone(), change.version);
            self.send(
                &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
                    "textDocument":{"uri":file_uri(&path)?,"version":change.version},
                    "contentChanges":[{"text":change.text}]
                }}),
            )?;
            self.request_pull(&path);
        }
        Ok(())
    }

    /// Asks for the diagnostics of the open document `path` when the server supports pulling
    /// them. One request is out at a time; a change meanwhile asks again when it answered.
    fn request_pull(&mut self, path: &Path) {
        if !self.supports_pull_diagnostics()
            || self.state != ServerState::Ready
            || self.pending_changes.contains_key(path)
        {
            return;
        }
        let Some(&version) = self.open_documents.get(path) else {
            return;
        };
        let pull = self.pulls.entry(path.to_owned()).or_default();
        if pull.in_flight.is_some() {
            pull.again = true;
            return;
        }
        let Ok(uri) = file_uri(path) else {
            return;
        };
        let id = self.next_id;
        self.next_id += 1;
        let sent = self.send(
            &json!({"jsonrpc":"2.0","id":id,"method":PULL_METHOD,"params":{
                "textDocument":{"uri":uri}
            }}),
        );
        if sent.is_err() {
            return;
        }
        self.pending.insert(
            id,
            PendingRequest {
                method: PULL_METHOD.into(),
                sent: Instant::now(),
            },
        );
        self.pull_requests.insert(id, path.to_owned());
        if let Some(pull) = self.pulls.get_mut(path) {
            pull.in_flight = Some((id, version));
        }
    }

    /// The answer to a pull: the full report replaces what was pulled for the file (and the
    /// files of `relatedDocuments`); an answer for text that has changed since is dropped, the
    /// pull that follows the change replaces it.
    fn handle_pull_response(&mut self, id: u64, result: Result<Value, ResponseError>) {
        let Some(path) = self.pull_requests.remove(&id) else {
            return;
        };
        let sent_version = self
            .pulls
            .get_mut(&path)
            .and_then(|pull| pull.in_flight.take())
            .map(|(_, version)| version);
        let stale = sent_version != self.open_documents.get(&path).copied()
            || self.pending_changes.contains_key(&path);
        match result {
            Ok(report) if !stale => {
                if let Some(version) = sent_version {
                    self.diag_versions.insert(canonical(&path), version);
                }
                self.apply_pull_report(&path, &report);
            }
            Ok(_) => {}
            // ServerCancelled and ContentModified: the result would not match the text.
            Err(error) if matches!(error.code, -32802 | -32801) => {
                if let Some(pull) = self.pulls.get_mut(&path) {
                    pull.again = true;
                }
            }
            Err(_) => {}
        }
        if self
            .pulls
            .get_mut(&path)
            .is_some_and(|pull| std::mem::take(&mut pull.again))
        {
            self.request_pull(&path);
        }
    }

    fn apply_pull_report(&mut self, path: &Path, report: &Value) {
        let mut changed: Vec<(PathBuf, Vec<LspDiagnostic>)> = Vec::new();
        let mut take = |path: &Path, report: &Value| {
            if report.get("kind").and_then(Value::as_str) == Some("full") {
                let items = report
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(parse_diagnostic)
                    .collect::<Vec<_>>();
                changed.push((canonical(path), items));
            }
        };
        take(path, report);
        if let Some(related) = report.get("relatedDocuments").and_then(Value::as_object) {
            for (uri, report) in related {
                if let Some(related_path) = Url::parse(uri).ok().and_then(|u| u.to_file_path().ok())
                {
                    take(&related_path, report);
                }
            }
        }
        for (path, items) in changed {
            self.pulled.insert(path.clone(), items);
            self.merge_diagnostics(&path);
        }
    }

    /// `diagnostics` of `path`: what was pulled and what was pushed, without repeats. An empty
    /// list is kept: it records that the file was checked.
    fn merge_diagnostics(&mut self, path: &Path) {
        let pulled = self.pulled.get(path);
        let pushed = self.pushed.get(path);
        if pulled.is_none() && pushed.is_none() {
            self.diagnostics.remove(path);
        } else {
            let mut merged: Vec<LspDiagnostic> = Vec::new();
            for diagnostic in pulled.into_iter().chain(pushed).flatten() {
                let same = |other: &LspDiagnostic| {
                    other.message == diagnostic.message
                        && other.severity == diagnostic.severity
                        && other.code == diagnostic.code
                        && (
                            other.start_line,
                            other.start_character,
                            other.end_line,
                            other.end_character,
                        ) == (
                            diagnostic.start_line,
                            diagnostic.start_character,
                            diagnostic.end_line,
                            diagnostic.end_character,
                        )
                };
                if !merged.iter().any(same) {
                    merged.push(diagnostic.clone());
                }
            }
            self.diagnostics.insert(path.to_owned(), merged);
        }
        self.diagnostics_revision = next_revision();
    }

    /// Opens a Rust document (compatibility shim for single-language callers).
    pub fn open_document(&mut self, path: &Path, text: &str, version: i32) -> Result<(), LspError> {
        self.sync_document(path, "rust", text, version)
    }

    /// Sends a Rust document change (compatibility shim for single-language callers).
    pub fn change_document(
        &mut self,
        path: &Path,
        text: &str,
        version: i32,
    ) -> Result<(), LspError> {
        self.sync_document(path, "rust", text, version)
    }

    /// `workspace/applyEdit` requests received since the last call.
    pub fn take_apply_edits(&mut self) -> Vec<ApplyEditRequest> {
        std::mem::take(&mut self.apply_edits)
    }

    /// Answers a `workspace/applyEdit` request.
    pub fn respond_apply_edit(
        &mut self,
        id: Value,
        result: Result<(), String>,
    ) -> Result<(), LspError> {
        let result = match result {
            Ok(()) => json!({"applied": true}),
            Err(reason) => json!({"applied": false, "failureReason": reason}),
        };
        self.send(&json!({"jsonrpc":"2.0","id":id,"result":result}))
    }

    /// Sends `textDocument/didSave` for an open document when the server asked
    /// for save notifications (rust-analyzer runs `cargo check` on save).
    pub fn save_document(&mut self, path: &Path) -> Result<(), LspError> {
        let wants_save = match &self.capabilities.text_document_sync {
            Some(TextDocumentSyncCapability::Options(options)) => options.save.is_some(),
            _ => false,
        };
        if !wants_save || !self.open_documents.contains_key(path) {
            return Ok(());
        }
        // The server checks the text that was saved.
        self.flush_changes(Some(path))?;
        self.send(
            &json!({"jsonrpc":"2.0","method":"textDocument/didSave","params":{
                "textDocument":{"uri":file_uri(path)?}
            }}),
        )
    }

    pub fn close_document(&mut self, path: &Path) -> Result<(), LspError> {
        self.queued_documents.remove(path);
        self.pending_changes.remove(path);
        self.pulls.remove(path);
        let pulled = canonical(path);
        if self.pulled.remove(&pulled).is_some() {
            self.merge_diagnostics(&pulled);
        }
        if self.open_documents.remove(path).is_some() {
            self.send(
                &json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{
                    "textDocument":{"uri":file_uri(path)?}
                }}),
            )?;
        }
        Ok(())
    }

    /// Sends a request and returns its id. The response is collected with
    /// [`take_response`](Self::take_response).
    pub fn request_raw(&mut self, method: &str, params: Value) -> Result<u64, LspError> {
        if self.state != ServerState::Ready {
            return Err(LspError::NotReady(self.name.clone()));
        }
        // The server answers for the text the user sees.
        self.flush_changes(None)?;
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        self.pending.insert(
            id,
            PendingRequest {
                method: method.into(),
                sent: Instant::now(),
            },
        );
        Ok(id)
    }

    /// Sends a typed request.
    pub fn request<R: Request>(&mut self, params: R::Params) -> Result<u64, LspError> {
        self.request_raw(R::METHOD, serde_json::to_value(params)?)
    }

    pub fn is_pending(&self, id: u64) -> bool {
        self.pending.contains_key(&id)
    }

    /// Takes the response for `id` if it has arrived.
    pub fn take_response(&mut self, id: u64) -> Option<LspResponse> {
        self.responses.remove(&id)
    }

    /// Takes and decodes the response for a typed request.
    pub fn take_typed<R: Request>(&mut self, id: u64) -> Option<Result<R::Result, LspError>> {
        let response = self.take_response(id)?;
        Some(match response.result {
            Ok(value) => serde_json::from_value(value).map_err(LspError::from),
            Err(error) => Err(LspError::Server {
                code: error.code,
                message: error.message,
            }),
        })
    }

    /// Cancels a pending request; a late response is discarded.
    pub fn cancel(&mut self, id: u64) {
        self.responses.remove(&id);
        if self.pending.remove(&id).is_some() && self.state == ServerState::Ready {
            let _ =
                self.send(&json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":id}}));
        }
    }

    /// Sends a notification.
    pub fn notify(&mut self, method: &str, params: Value) -> Result<(), LspError> {
        self.send(&json!({"jsonrpc":"2.0","method":method,"params":params}))
    }

    /// Asks the server to shut down and exit, then kills it if it lingers.
    pub fn shutdown(&mut self) {
        if self.state == ServerState::Exited {
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        let _ = self.send(&json!({"jsonrpc":"2.0","id":id,"method":"shutdown","params":null}));
        let deadline = Instant::now() + Duration::from_millis(300);
        // Give the server a moment to acknowledge before `exit`.
        while Instant::now() < deadline {
            match self.messages.recv_timeout(Duration::from_millis(20)) {
                Ok(Ok(message)) if message.get("id").and_then(Value::as_u64) == Some(id) => break,
                Ok(Ok(_)) => {}
                _ => break,
            }
        }
        let _ = self.send(&json!({"jsonrpc":"2.0","method":"exit","params":null}));
        let deadline = Instant::now() + Duration::from_millis(200);
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.state = ServerState::Exited;
        self.progress.clear();
    }

    fn handle(&mut self, message: Value) -> Result<(), LspError> {
        let method = message.get("method").and_then(Value::as_str);
        let id = message.get("id").cloned();
        match (method, id) {
            (Some(method), Some(id)) => self.handle_server_request(method, id, &message),
            (Some(method), None) => {
                self.handle_notification(method, message.get("params").unwrap_or(&Value::Null));
                Ok(())
            }
            (None, Some(id)) => self.handle_response(&id, &message),
            (None, None) => Ok(()),
        }
    }

    fn handle_response(&mut self, id: &Value, message: &Value) -> Result<(), LspError> {
        let Some(id) = id.as_u64() else {
            return Ok(());
        };
        let Some(pending) = self.pending.remove(&id) else {
            // Cancelled or unknown: drop it.
            return Ok(());
        };
        let latency = pending.sent.elapsed();
        self.latencies.insert(pending.method.clone(), latency);
        let result = match message.get("error") {
            Some(error) => Err(ResponseError {
                code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
                message: error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .into(),
            }),
            None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
        };
        if id == INITIALIZE_ID {
            return self.finish_initialize(result);
        }
        if pending.method == PULL_METHOD {
            self.handle_pull_response(id, result);
            return Ok(());
        }
        if self.responses.len() >= MAX_UNCLAIMED_RESPONSES {
            self.responses.clear();
        }
        self.responses.insert(
            id,
            LspResponse {
                method: pending.method,
                result,
                latency,
            },
        );
        Ok(())
    }

    fn finish_initialize(&mut self, result: Result<Value, ResponseError>) -> Result<(), LspError> {
        let value = result
            .map_err(|error| LspError::Protocol(format!("initialize failed: {}", error.message)))?;
        if let Ok(result) = serde_json::from_value::<InitializeResult>(value) {
            self.capabilities = result.capabilities;
        }
        self.state = ServerState::Ready;
        self.notify("initialized", json!({}))?;
        // jdtls (and others) read most `settings` only from this notification.
        if let Some(settings) = self.settings.clone() {
            self.notify(
                "workspace/didChangeConfiguration",
                json!({"settings": settings}),
            )?;
        }
        let queued = std::mem::take(&mut self.queued_documents);
        for (path, document) in queued {
            self.send_open(
                &path,
                &document.language_id,
                &document.text,
                document.version,
            )?;
        }
        Ok(())
    }

    fn handle_server_request(
        &mut self,
        method: &str,
        id: Value,
        message: &Value,
    ) -> Result<(), LspError> {
        let params = message.get("params").unwrap_or(&Value::Null);
        let result = match method {
            "window/workDoneProgress/create"
            | "client/registerCapability"
            | "client/unregisterCapability"
            | "window/showMessageRequest"
            | "workspace/semanticTokens/refresh"
            | "workspace/inlayHint/refresh"
            | "workspace/codeLens/refresh" => Value::Null,
            "workspace/configuration" => Value::Array(
                params
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|item| {
                        let section = item.get("section").and_then(Value::as_str);
                        configuration_section(self.settings.as_ref(), section)
                    })
                    .collect(),
            ),
            "workspace/workspaceFolders" => json!([workspace_folder(&self.root)?]),
            // The server's view changed (rust-analyzer after an edit or a check): pull again.
            "workspace/diagnostic/refresh" => {
                self.send(&json!({"jsonrpc":"2.0","id":id,"result":Value::Null}))?;
                let open: Vec<PathBuf> = self.open_documents.keys().cloned().collect();
                for path in open {
                    self.request_pull(&path);
                }
                return Ok(());
            }
            "workspace/applyEdit" => {
                // Answered by the IDE once it applied the edit.
                self.apply_edits.push(ApplyEditRequest {
                    id,
                    label: params.get("label").and_then(Value::as_str).map(Into::into),
                    edit: params.get("edit").cloned().unwrap_or(Value::Null),
                });
                return Ok(());
            }
            _ => {
                return self.send(&json!({"jsonrpc":"2.0","id":id,"error":{
                    "code": -32601, "message": format!("{method} is not supported by Archipel IDE")
                }}));
            }
        };
        self.send(&json!({"jsonrpc":"2.0","id":id,"result":result}))
    }

    fn handle_notification(&mut self, method: &str, params: &Value) {
        match method {
            "textDocument/publishDiagnostics" => {
                let Some(path) = params
                    .get("uri")
                    .and_then(Value::as_str)
                    .and_then(|uri| Url::parse(uri).ok())
                    .and_then(|url| url.to_file_path().ok())
                else {
                    return;
                };
                let diagnostics = params
                    .get("diagnostics")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(parse_diagnostic)
                    .collect::<Vec<_>>();
                let version = params
                    .get("version")
                    .and_then(Value::as_i64)
                    .and_then(|v| i32::try_from(v).ok())
                    .or_else(|| self.open_documents.get(&path).copied());
                match version {
                    Some(version) => {
                        self.diag_versions.insert(path.clone(), version);
                    }
                    None => {
                        self.diag_versions.remove(&path);
                    }
                }
                self.pushed.insert(path.clone(), diagnostics);
                self.merge_diagnostics(&path);
            }
            "$/progress" => self.handle_progress(params),
            "window/showMessage" => {
                let kind = params
                    .get("type")
                    .and_then(Value::as_u64)
                    .and_then(|kind| u8::try_from(kind).ok())
                    .unwrap_or(3);
                if let Some(text) = params.get("message").and_then(Value::as_str) {
                    self.shown_messages.push(ServerMessage {
                        kind,
                        text: text.into(),
                    });
                    self.shown_messages.truncate(32);
                }
            }
            // Eclipse JDT LS reports its startup phases here.
            "language/status" => self.handle_language_status(params),
            _ => {}
        }
    }

    fn handle_language_status(&mut self, params: &Value) {
        let kind = params.get("type").and_then(Value::as_str).unwrap_or("");
        match kind {
            "Starting" => {
                if self.import != ImportPhase::ServiceReady {
                    self.import = ImportPhase::Importing;
                }
            }
            "ServiceReady" => {
                self.import = ImportPhase::ServiceReady;
                self.service_ready_epoch += 1;
            }
            "Error" => self.import = ImportPhase::Failed,
            // `ProjectStatus` ("OK", "WARNING") is not a progress line.
            "ProjectStatus" => return,
            _ => {}
        }
        self.status_message = params
            .get("message")
            .and_then(Value::as_str)
            .map(Into::into);
    }

    fn handle_progress(&mut self, params: &Value) {
        let Some(token) = params.get("token").map(token_key) else {
            return;
        };
        let Some(value) = params.get("value") else {
            return;
        };
        let text = |name: &str| value.get(name).and_then(Value::as_str).map(String::from);
        let percentage = value
            .get("percentage")
            .and_then(Value::as_u64)
            .and_then(|p| u32::try_from(p.min(100)).ok());
        match value.get("kind").and_then(Value::as_str) {
            Some("begin") => {
                self.progress.insert(
                    token,
                    WorkProgress {
                        title: text("title").unwrap_or_default(),
                        message: text("message"),
                        percentage,
                    },
                );
            }
            Some("report") => {
                if let Some(progress) = self.progress.get_mut(&token) {
                    if let Some(message) = text("message") {
                        progress.message = Some(message);
                    }
                    if percentage.is_some() {
                        progress.percentage = percentage;
                    }
                }
            }
            Some("end") => {
                self.progress.remove(&token);
            }
            _ => {}
        }
    }

    fn send_open(
        &mut self,
        path: &Path,
        language_id: &str,
        text: &str,
        version: i32,
    ) -> Result<(), LspError> {
        self.open_documents.insert(path.to_owned(), version);
        self.send(
            &json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{
                "uri":file_uri(path)?,"languageId":language_id,"version":version,"text":text
            }}}),
        )?;
        self.request_pull(path);
        Ok(())
    }

    fn send(&mut self, message: &Value) -> Result<(), LspError> {
        let mut writer = self.writer.lock().map_err(|_| LspError::Lock)?;
        transport::write_message(&mut *writer, message)
    }
}

impl Drop for LspService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn token_key(token: &Value) -> String {
    match token {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn workspace_folder(root: &Path) -> Result<WorkspaceFolder, LspError> {
    Ok(WorkspaceFolder {
        uri: Url::from_directory_path(root).map_err(|_| LspError::InvalidPath(root.to_owned()))?,
        name: root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
            .into(),
    })
}

/// The capabilities Archipel IDE advertises.
pub fn client_capabilities() -> ClientCapabilities {
    let markup = Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]);
    ClientCapabilities {
        general: Some(GeneralClientCapabilities {
            position_encodings: Some(vec![PositionEncodingKind::UTF16]),
            ..Default::default()
        }),
        window: Some(WindowClientCapabilities {
            work_done_progress: Some(true),
            ..Default::default()
        }),
        workspace: Some(WorkspaceClientCapabilities {
            diagnostic: Some(DiagnosticWorkspaceClientCapabilities {
                refresh_support: Some(true),
            }),
            apply_edit: Some(true),
            workspace_edit: Some(WorkspaceEditClientCapabilities {
                document_changes: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }),
        text_document: Some(TextDocumentClientCapabilities {
            synchronization: Some(TextDocumentSyncClientCapabilities {
                did_save: Some(true),
                ..Default::default()
            }),
            // Pull diagnostics: rust-analyzer serves its native diagnostics only to a client
            // that declares it.
            diagnostic: Some(DiagnosticClientCapabilities {
                dynamic_registration: Some(false),
                related_document_support: Some(true),
            }),
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                related_information: Some(true),
                tag_support: Some(TagSupport {
                    value_set: vec![DiagnosticTag::UNNECESSARY, DiagnosticTag::DEPRECATED],
                }),
                code_description_support: Some(true),
                ..Default::default()
            }),
            completion: Some(CompletionClientCapabilities {
                completion_item: Some(CompletionItemCapability {
                    snippet_support: Some(true),
                    insert_replace_support: Some(true),
                    documentation_format: markup.clone(),
                    deprecated_support: Some(true),
                    label_details_support: Some(true),
                    resolve_support: Some(CompletionItemCapabilityResolveSupport {
                        properties: vec![
                            "documentation".into(),
                            "detail".into(),
                            "additionalTextEdits".into(),
                        ],
                    }),
                    ..Default::default()
                }),
                completion_item_kind: Some(CompletionItemKindCapability {
                    value_set: Some(
                        (1..=25)
                            .filter_map(|kind| {
                                serde_json::from_value::<CompletionItemKind>(json!(kind)).ok()
                            })
                            .collect(),
                    ),
                }),
                context_support: Some(true),
                ..Default::default()
            }),
            inlay_hint: Some(lsp_types::InlayHintClientCapabilities::default()),
            code_action: Some(CodeActionClientCapabilities {
                code_action_literal_support: Some(CodeActionLiteralSupport {
                    code_action_kind: CodeActionKindLiteralSupport {
                        value_set: [
                            "",
                            "quickfix",
                            "refactor",
                            "refactor.extract",
                            "refactor.inline",
                            "refactor.rewrite",
                            "source",
                            "source.organizeImports",
                        ]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                    },
                }),
                is_preferred_support: Some(true),
                disabled_support: Some(true),
                data_support: Some(true),
                resolve_support: Some(CodeActionCapabilityResolveSupport {
                    properties: vec!["edit".into()],
                }),
                ..Default::default()
            }),
            hover: Some(HoverClientCapabilities {
                content_format: markup.clone(),
                ..Default::default()
            }),
            signature_help: Some(SignatureHelpClientCapabilities {
                signature_information: Some(SignatureInformationSettings {
                    documentation_format: markup,
                    parameter_information: Some(ParameterInformationSettings {
                        label_offset_support: Some(true),
                    }),
                    active_parameter_support: Some(true),
                }),
                context_support: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[allow(deprecated)] // rootUri/rootPath are still read by several servers.
fn initialize_params(
    root: &Path,
    initialization_options: Option<Value>,
) -> Result<InitializeParams, LspError> {
    let folder = workspace_folder(root)?;
    Ok(InitializeParams {
        process_id: Some(std::process::id()),
        root_path: Some(root.display().to_string()),
        root_uri: Some(folder.uri.clone()),
        initialization_options,
        capabilities: client_capabilities(),
        workspace_folders: Some(vec![folder]),
        client_info: Some(ClientInfo {
            name: "Archipel IDE".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
        ..Default::default()
    })
}

/// The `workspace/configuration` answer for `section` (a dotted path such as
/// `java.completion`) out of `settings`; `null` when not configured.
fn configuration_section(settings: Option<&Value>, section: Option<&str>) -> Value {
    let Some(settings) = settings else {
        return Value::Null;
    };
    let Some(section) = section.filter(|section| !section.is_empty()) else {
        return settings.clone();
    };
    section
        .split('.')
        .try_fold(settings, |value, key| value.get(key))
        .cloned()
        .unwrap_or(Value::Null)
}

/// Revisions come from one counter so a restarted server never repeats one.
/// The path as the server's `file://` URIs name it (they come from [`file_uri`]).
fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_owned())
}

fn next_revision() -> u64 {
    static REVISION: AtomicU64 = AtomicU64::new(1);
    REVISION.fetch_add(1, Ordering::Relaxed)
}

fn parse_diagnostic(value: &Value) -> Option<LspDiagnostic> {
    let range = value.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;
    let tags: Vec<u64> = value
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .collect();
    let code = match value.get("code") {
        Some(Value::String(code)) => Some(code.clone()),
        Some(Value::Number(code)) => Some(code.to_string()),
        _ => None,
    };
    Some(LspDiagnostic {
        raw: value.clone(),
        code,
        source: value.get("source").and_then(Value::as_str).map(Into::into),
        unnecessary: tags.contains(&1),
        deprecated: tags.contains(&2),
        message: value.get("message")?.as_str()?.into(),
        severity: value
            .get("severity")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok()),
        start_line: start.get("line")?.as_u64()? as usize,
        start_character: start.get("character")?.as_u64()? as usize,
        end_line: end.get("line")?.as_u64()? as usize,
        end_character: end.get("character")?.as_u64()? as usize,
    })
}

/// `file://` URI of `path` (canonicalized when it exists).
pub fn file_uri(path: &Path) -> Result<Url, LspError> {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    Url::from_file_path(&absolute).map_err(|_| LspError::InvalidPath(path.to_owned()))
}
