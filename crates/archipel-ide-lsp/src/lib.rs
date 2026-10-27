//! Language Server Protocol client.
//!
//! - [`transport`]: JSON-RPC framing.
//! - [`client`]: one server process ([`LspService`]).
//! - [`registry`]: which server handles which file ([`ServerRegistry`],
//!   [`LanguageServerProvider`]).
//! - [`manager`]: all servers of a project ([`LspManager`]).
//!
//! Nothing here depends on the UI; editor popups live in `archipel-ide-ui`.

pub mod client;
pub mod completion;
pub mod locations;
pub mod manager;
pub mod markup;
pub mod registry;
pub mod snippet;
pub mod transport;
pub mod workspace_symbol;

pub use client::{
    ApplyEditRequest, ImportPhase, LaunchConfig, LspDiagnostic, LspResponse, LspService,
    ResponseError, ServerMessage, ServerState, WorkProgress,
};
pub use locations::{LspLocation, parse_locations};
pub use lsp_types;
pub use manager::{
    CompletionResult, CompletionTrigger, Feature, LspManager, LspNotice, Publication,
    RequestHandle, ServerInfo, ServerKey, ServerStatus, describe_progress,
};
pub use registry::{
    JAVA_DEBUG_PLUGIN_ENV, LanguageServerProvider, LaunchContext, LspSettings, ProgramResolver,
    ResolvedProgram, ServerRegistry, ServerSettings, ServerSpec,
};
pub use workspace_symbol::{WorkspaceSymbolHit, WorkspaceSymbolRequestHandle};

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LspError {
    #[error("failed to start language server: {0}")]
    Start(#[from] std::io::Error),
    #[error("language server stream is unavailable")]
    MissingStream,
    #[error("language server writer lock is unavailable")]
    Lock,
    #[error("invalid file path for LSP URI: {0}")]
    InvalidPath(PathBuf),
    #[error("language server protocol error: {0}")]
    Protocol(String),
    #[error("{language} language server `{server}` was not found. {hint}")]
    ServerNotFound {
        server: String,
        language: String,
        hint: String,
    },
    #[error("{server} is still starting{}", .detail.as_deref().map(|d| format!(": {d}")).unwrap_or_default())]
    Starting {
        server: String,
        detail: Option<String>,
    },
    #[error("{0} is not ready yet")]
    NotReady(String),
    #[error("language server `{0}` is disabled in lsp.toml")]
    Disabled(String),
    #[error("no language server is registered for {0}")]
    Unsupported(String),
    #[error("language server error {code}: {message}")]
    Server { code: i64, message: String },
    #[error("invalid lsp.toml: {0}")]
    Settings(String),
}

impl From<serde_json::Error> for LspError {
    fn from(error: serde_json::Error) -> Self {
        Self::Protocol(error.to_string())
    }
}
