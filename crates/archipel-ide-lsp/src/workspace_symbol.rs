//! `workspace/symbol` behind a small request API for Search Everywhere.
//!
//! The caller starts a request with [`LspManager::request_workspace_symbols`]
//! (the anchor file only selects which server to ask) and later collects it
//! with [`LspManager::take_workspace_symbols`] after [`LspManager::poll`].

use crate::{LspError, LspManager};
use lsp_types::request::{Request, WorkspaceSymbolRequest};
use lsp_types::{SymbolKind, WorkspaceSymbolResponse};
use serde_json::json;
use std::path::{Path, PathBuf};

/// An in-flight `workspace/symbol` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbolRequestHandle {
    anchor: PathBuf,
    id: u64,
}

/// One symbol reported by the server, resolved to a file position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbolHit {
    pub name: String,
    pub kind: SymbolKind,
    pub container: Option<String>,
    pub path: PathBuf,
    /// Zero-based line of the symbol.
    pub line: usize,
}

impl LspManager {
    /// Asks the server of `anchor` for symbols matching `query`.
    pub fn request_workspace_symbols(
        &mut self,
        anchor: &Path,
        query: &str,
    ) -> Result<WorkspaceSymbolRequestHandle, LspError> {
        let service = self.ready_server_for(anchor)?;
        if service.capabilities().workspace_symbol_provider.is_none() {
            return Err(LspError::Protocol(format!(
                "{} does not support workspace symbols",
                service.name()
            )));
        }
        let id = service.request_raw(
            <WorkspaceSymbolRequest as Request>::METHOD,
            json!({ "query": query }),
        )?;
        Ok(WorkspaceSymbolRequestHandle {
            anchor: anchor.to_owned(),
            id,
        })
    }

    /// The decoded result once it has arrived; `None` while pending.
    pub fn take_workspace_symbols(
        &mut self,
        request: &WorkspaceSymbolRequestHandle,
    ) -> Option<Result<Vec<WorkspaceSymbolHit>, LspError>> {
        let service = self.server_for_mut(&request.anchor)?;
        let result = service.take_typed::<WorkspaceSymbolRequest>(request.id)?;
        Some(result.map(|response| hits(response.unwrap_or(WorkspaceSymbolResponse::Flat(vec![])))))
    }

    /// Cancels a request whose answer is no longer wanted.
    pub fn cancel_workspace_symbols(&mut self, request: &WorkspaceSymbolRequestHandle) {
        if let Some(service) = self.server_for_mut(&request.anchor) {
            service.cancel(request.id);
        }
    }
}

fn hits(response: WorkspaceSymbolResponse) -> Vec<WorkspaceSymbolHit> {
    let mut out = Vec::new();
    match response {
        WorkspaceSymbolResponse::Flat(symbols) => {
            for symbol in symbols {
                let Ok(path) = symbol.location.uri.to_file_path() else {
                    continue;
                };
                out.push(WorkspaceSymbolHit {
                    name: symbol.name,
                    kind: symbol.kind,
                    container: symbol.container_name,
                    path,
                    line: symbol.location.range.start.line as usize,
                });
            }
        }
        WorkspaceSymbolResponse::Nested(symbols) => {
            for symbol in symbols {
                let (uri, line) = match symbol.location {
                    lsp_types::OneOf::Left(location) => {
                        (location.uri, location.range.start.line as usize)
                    }
                    lsp_types::OneOf::Right(location) => (location.uri, 0),
                };
                let Ok(path) = uri.to_file_path() else {
                    continue;
                };
                out.push(WorkspaceSymbolHit {
                    name: symbol.name,
                    kind: symbol.kind,
                    container: symbol.container_name,
                    path,
                    line,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Location, Position, Range, SymbolInformation, Url};

    #[test]
    #[allow(deprecated)]
    fn flat_response_resolves_file_positions() {
        let uri = Url::parse("file:///project/src/lib.rs").unwrap();
        let symbol = SymbolInformation {
            name: "Greeter".into(),
            kind: SymbolKind::STRUCT,
            tags: None,
            deprecated: None,
            location: Location::new(uri, Range::new(Position::new(4, 0), Position::new(9, 1))),
            container_name: Some("greet".into()),
        };
        let found = hits(WorkspaceSymbolResponse::Flat(vec![symbol]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, PathBuf::from("/project/src/lib.rs"));
        assert_eq!(found[0].line, 4);
        assert_eq!(found[0].container.as_deref(), Some("greet"));
    }
}
