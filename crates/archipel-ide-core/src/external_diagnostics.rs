//! Language-server diagnostics on their way into the daemon.
//!
//! The LSP client owns the transport; the daemon owns what the editor shows.
//! After every [`IdeState::poll_lsp`] this module mirrors what the servers
//! published into the daemon's [`ExternalDiagnostics`] feed (positions
//! resolved to byte ranges of the open documents' current text) and tells the
//! daemon ([`archipel_ide_daemon::Daemon::diagnostics_published`]), which runs
//! its external pass and deduplicates the result against the native
//! highlights. Nothing else reads [`LspManager::publications`] for display.
//!
//! A server that is still importing its build (jdtls before `ServiceReady`)
//! reports against an incomplete classpath: its diagnostics are held back
//! until the import finished, then published in one go.

use crate::{Document, IdeState};
use archipel_ide_analysis::FileRev;
use archipel_ide_daemon::{ExternalDiagnostic, ExternalFile};
use archipel_ide_lsp::LspDiagnostic;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

/// The mirror's bookkeeping: what the last sync looked at.
#[derive(Debug, Default)]
pub(crate) struct ExternalSync {
    key: Option<u64>,
}

/// An LSP diagnostic as the feed stores it. Ranges are byte offsets in the
/// text of `document` (an empty range is widened to the neighbouring
/// character, so it can be drawn).
fn resolve(document: &Document, diagnostic: &LspDiagnostic) -> ExternalDiagnostic {
    let text = &document.text;
    let start_char = text.char_from_lsp_position(diagnostic.start_line, diagnostic.start_character);
    let end_char = text
        .char_from_lsp_position(diagnostic.end_line, diagnostic.end_character)
        .max(start_char);
    let (mut start, mut end) = (text.char_to_byte(start_char), text.char_to_byte(end_char));
    if start == end {
        let widen = |index: usize| {
            text.char_at(index)
                .filter(|c| !matches!(c, '\n' | '\r'))
                .is_some()
        };
        if widen(end_char) {
            end = text.char_to_byte(end_char + 1);
        } else if start_char > 0 && widen(start_char - 1) {
            start = text.char_to_byte(start_char - 1);
        }
    }
    external(diagnostic, start as u32..end as u32)
}

fn external(diagnostic: &LspDiagnostic, range: std::ops::Range<u32>) -> ExternalDiagnostic {
    ExternalDiagnostic {
        range,
        start: (
            diagnostic.start_line as u32,
            diagnostic.start_character as u32,
        ),
        end: (diagnostic.end_line as u32, diagnostic.end_character as u32),
        severity: diagnostic.severity.unwrap_or(1),
        message: diagnostic.message.clone(),
        code: diagnostic.code.clone(),
        source: diagnostic.source.clone(),
        unnecessary: diagnostic.unnecessary,
        deprecated: diagnostic.deprecated,
    }
}

impl IdeState {
    /// Mirrors the servers' diagnostics into the daemon's feed and triggers
    /// its external pass for the open files whose entry changed. Cheap when
    /// nothing was published since the last call.
    pub(crate) fn sync_external_diagnostics(&mut self) {
        let Some(service) = self.daemon_service() else {
            return;
        };
        let key = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.lsp.diagnostics_generation().hash(&mut hasher);
            for document in &self.documents {
                document.path.hash(&mut hasher);
            }
            hasher.finish()
        };
        if self.external_sync.key == Some(key) {
            return;
        }
        self.external_sync.key = Some(key);
        let feed = service.shared().external.clone();
        let Some(project) = &self.project else {
            feed.retain(|_| false);
            return;
        };
        let root = project.root().to_owned();
        let canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
        let mut keep: HashSet<PathBuf> = HashSet::new();
        let mut changed: Vec<PathBuf> = Vec::new();
        for document in &self.documents {
            let absolute = root.join(&document.path);
            let published = if self.lsp.import_pending_for(&absolute) {
                None
            } else {
                self.lsp.published_diagnostics(&absolute)
            };
            let server = self.lsp.server_id_for(&absolute);
            let (Some(diagnostics), Some(server)) = (published, server) else {
                continue;
            };
            let entry = ExternalFile {
                server: server.into(),
                rev: Some(FileRev(document.text.revision())),
                diagnostics: Arc::new(
                    diagnostics
                        .iter()
                        .map(|diagnostic| resolve(document, diagnostic))
                        .collect(),
                ),
            };
            keep.insert(document.path.clone());
            if feed.publish(&document.path, entry) {
                changed.push(document.path.clone());
            }
        }
        // Files that are not open have no markup; the Problems view lists them
        // from the feed (positions as reported).
        let open: HashSet<&PathBuf> = self.documents.iter().map(|d| &d.path).collect();
        for publication in self.lsp.publications() {
            if publication.importing {
                continue;
            }
            let Ok(relative) = publication.path.strip_prefix(&canonical) else {
                continue;
            };
            if open.contains(&relative.to_path_buf()) {
                continue;
            }
            let entry = ExternalFile {
                server: publication.server.into(),
                rev: None,
                diagnostics: Arc::new(
                    publication
                        .diagnostics
                        .iter()
                        .map(|diagnostic| external(diagnostic, 0..0))
                        .collect(),
                ),
            };
            keep.insert(relative.to_path_buf());
            feed.publish(relative, entry);
        }
        changed.extend(
            feed.retain(|path| keep.contains(path))
                .into_iter()
                .filter(|path| open.contains(path)),
        );
        if changed.is_empty() {
            return;
        }
        let mut daemon = service.lock();
        for path in changed {
            if let Some(file) = self.analysis.host.lookup(&path) {
                daemon.diagnostics_published(file);
            }
        }
    }
}
