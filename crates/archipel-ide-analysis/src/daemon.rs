//! Highlighting-pass contracts (implemented by WP3's `archipel-ide-daemon`):
//! passes, annotators and external annotators.

use crate::cancel::{CancelToken, Cancelled};
use crate::file::{FileId, FileRev};
use crate::highlight::{FixDescriptor, HighlightInfo, HighlightKind, Severity, ToolId};
use crate::host::Snapshot;
use archipel_ide_language::{CompletenessTier, SyntaxNode, SyntaxSnapshot, TokenSet};
use std::any::Any;
use std::ops::Range;
use std::sync::Arc;

/// Stable id of a pass (`"syntax"`, `"general"`, `"local-inspections"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PassId(pub &'static str);

/// Input of one pass run (on a background thread, over a snapshot).
pub struct PassCx<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub syntax: &'a SyntaxSnapshot,
    /// The range this run covers (visible range first, then the rest).
    pub range: Range<u32>,
    pub visible: bool,
    pub cancel: &'a CancelToken,
}

/// Output of one pass run: replaces the tool's highlights in `range`.
#[derive(Debug, Clone, PartialEq)]
pub struct PassOutput {
    pub file: FileId,
    pub rev: FileRev,
    pub tool: ToolId,
    pub range: Range<u32>,
    pub infos: Vec<HighlightInfo>,
}

/// A highlighting pass (`TextEditorHighlightingPass`).
pub trait Pass: Send + Sync {
    fn id(&self) -> PassId;
    /// Prerequisites.
    fn after(&self) -> &[PassId];
    fn dumb_aware(&self) -> bool;
    fn visible_range_first(&self) -> bool;
    /// Background, on a snapshot.
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled>;
}

/// The markup model (`MarkupModel`) passes write into; WP3 owns the real
/// range store. Replace semantics: the `(tool, range)` slice is replaced
/// atomically.
pub trait HighlightMarkup {
    fn replace(&mut self, tool: &ToolId, range: Range<u32>, infos: Vec<HighlightInfo>);
    /// Highlights intersecting `range`.
    fn query(&self, range: Range<u32>) -> Vec<&HighlightInfo>;
}

/// UI-thread half of a pass (cheap).
pub trait PassApply {
    fn apply(&self, out: PassOutput, markup: &mut dyn HighlightMarkup);
}

/// Default apply: replace the tool's slice.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReplaceSlice;

impl PassApply for ReplaceSlice {
    fn apply(&self, out: PassOutput, markup: &mut dyn HighlightMarkup) {
        markup.replace(&out.tool, out.range, out.infos);
    }
}

/// Context of an annotator call.
pub struct AnnotateCx<'a> {
    pub snap: &'a Snapshot,
    pub syntax: &'a SyntaxSnapshot,
    pub cancel: &'a CancelToken,
    pub tier: CompletenessTier,
    pub dumb: bool,
}

/// Collects annotations (`AnnotationHolder`).
#[derive(Debug)]
pub struct AnnotationHolder {
    tool: ToolId,
    infos: Vec<HighlightInfo>,
}

impl AnnotationHolder {
    pub fn new(tool: ToolId) -> Self {
        Self {
            tool,
            infos: Vec::new(),
        }
    }

    /// Starts an annotation; finish it with [`AnnotationBuilder::create`].
    pub fn new_annotation(
        &mut self,
        severity: Severity,
        message: impl Into<Arc<str>>,
    ) -> AnnotationBuilder<'_> {
        let kind = match severity.value {
            v if v >= Severity::ERROR.value => HighlightKind::Error,
            v if v >= Severity::WARNING.value => HighlightKind::Warning,
            v if v >= Severity::WEAK_WARNING.value => HighlightKind::WeakWarning,
            _ => HighlightKind::Information,
        };
        AnnotationBuilder {
            info: HighlightInfo::new(0..0, severity, kind, self.tool.clone()).message(message),
            holder: self,
        }
    }

    pub fn infos(&self) -> &[HighlightInfo] {
        &self.infos
    }

    pub fn into_infos(self) -> Vec<HighlightInfo> {
        self.infos
    }
}

/// See [`AnnotationHolder::new_annotation`].
pub struct AnnotationBuilder<'h> {
    holder: &'h mut AnnotationHolder,
    info: HighlightInfo,
}

impl AnnotationBuilder<'_> {
    pub fn range(mut self, range: Range<u32>) -> Self {
        self.info.range = range;
        self
    }

    pub fn kind(mut self, kind: HighlightKind) -> Self {
        self.info.kind = kind;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<Arc<str>>) -> Self {
        self.info.tooltip = Some(tooltip.into());
        self
    }

    pub fn with_fix(mut self, fix: FixDescriptor) -> Self {
        self.info.fixes.push(fix);
        self
    }

    pub fn file_level(mut self) -> Self {
        self.info.file_level = true;
        self
    }

    pub fn create(self) {
        self.holder.infos.push(self.info);
    }
}

/// Per-element annotator (`Annotator`).
pub trait Annotator: Send + Sync {
    /// Element types to visit.
    fn interesting(&self) -> &TokenSet;
    fn dumb_aware(&self) -> bool;
    fn annotate(
        &self,
        n: SyntaxNode<'_>,
        h: &mut AnnotationHolder,
        cx: &AnnotateCx<'_>,
    ) -> Result<(), Cancelled>;
}

/// Three-phase external tool (`ExternalAnnotator`): fast `collect`, slow
/// off-thread `run` (LSP, cargo check, javac, linters), then `apply` to the
/// revision it was computed for (results are range-mapped through the edit
/// journal when the file changed meanwhile).
pub trait ExternalAnnotator: Send + Sync {
    type Info: Send;
    type Result: Send;
    fn collect(&self, snap: &Snapshot, file: FileId) -> Option<Self::Info>;
    fn run(&self, info: Self::Info, cancel: &CancelToken) -> Option<Self::Result>;
    fn apply(&self, file: FileId, rev: FileRev, r: Self::Result, h: &mut AnnotationHolder);
}

/// Object-safe form of [`ExternalAnnotator`] (what the registry stores);
/// implemented for every `ExternalAnnotator`.
pub trait DynExternalAnnotator: Send + Sync {
    fn collect_dyn(&self, snap: &Snapshot, file: FileId) -> Option<Box<dyn Any + Send>>;
    fn run_dyn(
        &self,
        info: Box<dyn Any + Send>,
        cancel: &CancelToken,
    ) -> Option<Box<dyn Any + Send>>;
    fn apply_dyn(
        &self,
        file: FileId,
        rev: FileRev,
        result: Box<dyn Any + Send>,
        h: &mut AnnotationHolder,
    );
}

impl<A> DynExternalAnnotator for A
where
    A: ExternalAnnotator,
    A::Info: 'static,
    A::Result: 'static,
{
    fn collect_dyn(&self, snap: &Snapshot, file: FileId) -> Option<Box<dyn Any + Send>> {
        self.collect(snap, file)
            .map(|info| Box::new(info) as Box<dyn Any + Send>)
    }

    fn run_dyn(
        &self,
        info: Box<dyn Any + Send>,
        cancel: &CancelToken,
    ) -> Option<Box<dyn Any + Send>> {
        let info = info.downcast::<A::Info>().ok()?;
        self.run(*info, cancel)
            .map(|result| Box::new(result) as Box<dyn Any + Send>)
    }

    fn apply_dyn(
        &self,
        file: FileId,
        rev: FileRev,
        result: Box<dyn Any + Send>,
        h: &mut AnnotationHolder,
    ) {
        if let Ok(result) = result.downcast::<A::Result>() {
            self.apply(file, rev, *result, h);
        }
    }
}
