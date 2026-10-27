//! LSP diagnostics as an external pass (IntelliJ's `ExternalAnnotator`
//! contract: *collect* information on the UI side, *run* the slow part off
//! thread, *apply* the result to the file).
//!
//! The language server client lives in `archipel-ide-core` (it owns the
//! transport). It *publishes* what the server reported into the
//! [`ExternalDiagnostics`] feed - positions already resolved to byte ranges of
//! the revision the editor had at that moment - and tells the daemon
//! ([`crate::Daemon::diagnostics_published`]). [`ExternalDiagnosticsPass`] then
//! turns the feed entry of a file into highlights owned by `lsp:<server>`,
//! and [`FileMarkup::reconcile`](crate::FileMarkup::reconcile) deduplicates
//! them against the native results (spec 1.10).
//!
//! Nothing here knows the LSP types; any external provider (a linter, a
//! build) can publish the same way.

use crate::passes::EXTERNAL_DIAGNOSTICS;
use archipel_ide_analysis::daemon::{
    AnnotationHolder, ExternalAnnotator, Pass, PassCx, PassId, PassOutput,
};
use archipel_ide_analysis::{
    CancelToken, Cancelled, FileId, FileRev, HighlightKind, Severity, Snapshot, ToolId,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// One diagnostic of an external provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalDiagnostic {
    /// Byte range in the text of the revision the file entry belongs to;
    /// meaningful only for open files ([`ExternalFile::rev`] is `Some`).
    pub range: Range<u32>,
    /// Start line (0-based) and column as the provider reported them (LSP:
    /// UTF-16 units). Used for files that are not open.
    pub start: (u32, u32),
    pub end: (u32, u32),
    /// LSP `DiagnosticSeverity`: 1 error, 2 warning, 3 information, 4 hint.
    pub severity: u8,
    pub message: String,
    pub code: Option<String>,
    /// The tool behind the diagnostic (`Java`, `rustc`).
    pub source: Option<String>,
    /// `DiagnosticTag.Unnecessary`: unused code, drawn greyed out.
    pub unnecessary: bool,
    /// `DiagnosticTag.Deprecated`: drawn struck through.
    pub deprecated: bool,
}

/// What a provider published for one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalFile {
    /// Provider id (`jdtls`): the markup owner is `lsp:<server>`.
    pub server: Arc<str>,
    /// The revision `ExternalDiagnostic::range`s refer to (`None` for files
    /// that are not open: no ranges).
    pub rev: Option<FileRev>,
    pub diagnostics: Arc<Vec<ExternalDiagnostic>>,
}

#[derive(Debug, Default)]
struct Feed {
    files: HashMap<PathBuf, ExternalFile>,
    version: u64,
}

/// The diagnostics external providers published, by project-relative path.
/// An absent entry means "the provider has not checked the file (yet)"; an
/// entry with no diagnostics means "checked, clean".
#[derive(Debug, Default)]
pub struct ExternalDiagnostics {
    feed: Mutex<Feed>,
    /// What the publisher looked at last (it skips a sync when nothing moved).
    sync_key: Mutex<Option<u64>>,
}

impl ExternalDiagnostics {
    /// Records the publisher's input key; false when it is the one seen last.
    pub fn changed_since_last_sync(&self, key: u64) -> bool {
        let mut last = self.sync_key.lock();
        let changed = *last != Some(key);
        *last = Some(key);
        changed
    }

    /// Replaces the entry of `path`; true when it changed.
    pub fn publish(&self, path: &Path, file: ExternalFile) -> bool {
        let mut feed = self.feed.lock();
        if feed.files.get(path) == Some(&file) {
            return false;
        }
        feed.files.insert(path.to_owned(), file);
        feed.version += 1;
        true
    }

    /// Forgets `path` (not checked any more); true when it was there.
    pub fn withdraw(&self, path: &Path) -> bool {
        let mut feed = self.feed.lock();
        let removed = feed.files.remove(path).is_some();
        if removed {
            feed.version += 1;
        }
        removed
    }

    /// Drops the entries `keep` rejects (paths that are no longer published).
    pub fn retain(&self, mut keep: impl FnMut(&Path) -> bool) -> Vec<PathBuf> {
        let mut feed = self.feed.lock();
        let gone: Vec<PathBuf> = feed
            .files
            .keys()
            .filter(|path| !keep(path))
            .cloned()
            .collect();
        for path in &gone {
            feed.files.remove(path);
        }
        if !gone.is_empty() {
            feed.version += 1;
        }
        gone
    }

    pub fn get(&self, path: &Path) -> Option<ExternalFile> {
        self.feed.lock().files.get(path).cloned()
    }

    /// Whether a provider has published for `path` (even an empty list).
    pub fn is_published(&self, path: &Path) -> bool {
        self.feed.lock().files.contains_key(path)
    }

    /// Every entry, sorted by path.
    pub fn all(&self) -> Vec<(PathBuf, ExternalFile)> {
        let feed = self.feed.lock();
        let mut all: Vec<_> = feed
            .files
            .iter()
            .map(|(path, file)| (path.clone(), file.clone()))
            .collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all
    }

    /// Changes whenever an entry changes (cache key).
    pub fn version(&self) -> u64 {
        self.feed.lock().version
    }
}

/// Severity and kind of an LSP diagnostic. Hints without a tag are plain
/// information (not a problem); the tags select the grey and struck-through
/// text attributes.
pub fn lsp_severity(diagnostic: &ExternalDiagnostic) -> (Severity, HighlightKind) {
    let severity = match diagnostic.severity {
        1 => Severity::ERROR,
        2 => Severity::WARNING,
        3 => Severity::WEAK_WARNING,
        _ => Severity::INFORMATION,
    };
    let kind = if diagnostic.unnecessary {
        HighlightKind::Unused
    } else if diagnostic.deprecated {
        HighlightKind::Deprecated
    } else {
        match diagnostic.severity {
            1 => HighlightKind::Error,
            2 => HighlightKind::Warning,
            3 => HighlightKind::WeakWarning,
            _ => HighlightKind::Information,
        }
    };
    (severity, kind)
}

/// `collect` result: what the provider published, for the revision the
/// ranges belong to.
#[derive(Debug)]
pub struct ExternalInfo {
    server: Arc<str>,
    rev: FileRev,
    diagnostics: Arc<Vec<ExternalDiagnostic>>,
}

/// `run` result: the highlights' source data (nothing slow to compute for
/// the LSP, the server did the work; the step exists for the contract).
#[derive(Debug)]
pub struct ExternalResult {
    diagnostics: Vec<ExternalDiagnostic>,
}

/// The `ExternalAnnotator` of published diagnostics.
#[derive(Debug)]
pub struct DiagnosticsAnnotator {
    feed: Arc<ExternalDiagnostics>,
}

impl DiagnosticsAnnotator {
    pub fn new(feed: Arc<ExternalDiagnostics>) -> Self {
        Self { feed }
    }

    /// The owner of the highlights of `server`.
    pub fn tool(server: &str) -> ToolId {
        ToolId::lsp(server)
    }
}

impl ExternalAnnotator for DiagnosticsAnnotator {
    type Info = ExternalInfo;
    type Result = ExternalResult;

    fn collect(&self, snap: &Snapshot, file: FileId) -> Option<ExternalInfo> {
        let path = snap.path(file)?;
        let entry = self.feed.get(path)?;
        Some(ExternalInfo {
            server: entry.server,
            rev: entry.rev?,
            diagnostics: entry.diagnostics,
        })
    }

    fn run(&self, info: ExternalInfo, _cancel: &CancelToken) -> Option<ExternalResult> {
        Some(ExternalResult {
            diagnostics: info.diagnostics.iter().cloned().collect(),
        })
    }

    fn apply(
        &self,
        _file: FileId,
        _rev: FileRev,
        result: ExternalResult,
        h: &mut AnnotationHolder,
    ) {
        for diagnostic in result.diagnostics {
            let (severity, kind) = lsp_severity(&diagnostic);
            let mut annotation = h
                .new_annotation(severity, diagnostic.message.clone())
                .range(diagnostic.range.clone())
                .kind(kind);
            if let Some(code) = &diagnostic.code {
                annotation = annotation.code(code.as_str());
            }
            if let Some(source) = &diagnostic.source {
                annotation = annotation.source(source.as_str());
            }
            annotation.create();
        }
    }
}

/// Runs the [`DiagnosticsAnnotator`]; the `ExternalAnnotators` pass for the
/// `lsp:` tool namespace. Listens to `DiagnosticsPublished` only: an edit
/// does not re-map published positions (the markup shifts them through the
/// edit journal until the provider republishes).
#[derive(Debug)]
pub struct ExternalDiagnosticsPass {
    annotator: DiagnosticsAnnotator,
}

impl ExternalDiagnosticsPass {
    pub fn new(feed: Arc<ExternalDiagnostics>) -> Self {
        Self {
            annotator: DiagnosticsAnnotator::new(feed),
        }
    }
}

impl Pass for ExternalDiagnosticsPass {
    fn id(&self) -> PassId {
        EXTERNAL_DIAGNOSTICS
    }
    fn after(&self) -> &[PassId] {
        &[]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        false
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        cx.cancel.check()?;
        let Some(info) = self.annotator.collect(cx.snap, cx.file) else {
            // Nothing published: clear what an earlier entry left.
            return Ok(PassOutput {
                file: cx.file,
                rev: cx.syntax.rev(),
                tool: ToolId::lsp(STALE_OWNER),
                range: 0..u32::MAX,
                infos: Vec::new(),
            });
        };
        let (server, rev) = (info.server.clone(), info.rev);
        cx.cancel.check()?;
        let Some(result) = self.annotator.run(info, cx.cancel) else {
            return Err(Cancelled);
        };
        cx.cancel.check()?;
        let tool = DiagnosticsAnnotator::tool(&server);
        let mut holder = AnnotationHolder::new(tool.clone());
        self.annotator.apply(cx.file, rev, result, &mut holder);
        // The file may be shorter than the revision the ranges were made for;
        // the daemon maps them forward, a bad one is dropped here.
        Ok(PassOutput {
            file: cx.file,
            rev,
            tool,
            range: 0..u32::MAX,
            infos: holder.into_infos(),
        })
    }
}

/// Owner name of the output that clears every `lsp:` highlight of a file (an
/// entry was withdrawn, so no provider has published for it); the daemon
/// recognizes it when it applies the output.
pub const STALE_OWNER: &str = "*";
