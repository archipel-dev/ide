//! Editing-assistance contracts (implemented by WP6): completion,
//! documentation, parameter info, formatting and template contexts.

use crate::cancel::{CancelToken, Cancelled};
use crate::file::{FileId, FileMeta};
use crate::host::Snapshot;
use crate::modcommand::TextEdit;
use crate::resolve::LookupItem;
use crate::target::Target;
use archipel_ide_language::SyntaxSnapshot;
use std::ops::Range;

/// Input of a completion run.
pub struct CompletionParams<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub syntax: &'a SyntaxSnapshot,
    pub offset: u32,
    /// The identifier prefix before the caret.
    pub prefix: &'a str,
    /// 1 for auto-popup/first Ctrl+Space, 2+ for repeated invocations.
    pub invocation_count: u32,
    pub cancel: &'a CancelToken,
}

/// Collects variants; contributors may stop the chain.
#[derive(Debug, Default)]
pub struct CompletionResultSet {
    items: Vec<LookupItem>,
    stopped: bool,
}

impl CompletionResultSet {
    pub fn add(&mut self, item: LookupItem) {
        self.items.push(item);
    }

    /// No later contributor runs.
    pub fn stop_here(&mut self) {
        self.stopped = true;
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn items(&self) -> &[LookupItem] {
        &self.items
    }

    pub fn into_items(self) -> Vec<LookupItem> {
        self.items
    }
}

/// `CompletionContributor`.
pub trait CompletionContributor: Send + Sync {
    fn dumb_aware(&self) -> bool;
    fn fill(
        &self,
        params: &CompletionParams<'_>,
        result: &mut CompletionResultSet,
    ) -> Result<(), Cancelled>;
}

/// One stage of the weigher chain (`CompletionWeigher`); higher sorts first.
pub trait CompletionWeigher: Send + Sync {
    fn id(&self) -> &'static str;
    fn weigh(&self, item: &LookupItem, params: &CompletionParams<'_>) -> i64;
}

/// Rendered documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Documentation {
    /// Declaration signature.
    pub definition: Option<String>,
    pub markdown: String,
}

/// `DocumentationProvider`.
pub trait DocumentationProvider: Send + Sync {
    fn documentation(
        &self,
        snap: &Snapshot,
        target: &Target,
        cancel: &CancelToken,
    ) -> Result<Option<Documentation>, Cancelled>;
}

/// One signature of parameter info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub label: String,
    /// Parameter ranges inside `label`.
    pub parameters: Vec<Range<u32>>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInfo {
    pub signatures: Vec<Signature>,
    pub active_signature: usize,
    pub active_parameter: Option<usize>,
}

/// `ParameterInfoHandler`.
pub trait ParameterInfoProvider: Send + Sync {
    fn parameter_info(
        &self,
        snap: &Snapshot,
        syntax: &SyntaxSnapshot,
        offset: u32,
        cancel: &CancelToken,
    ) -> Result<Option<ParameterInfo>, Cancelled>;
}

/// Code style in effect for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeStyle {
    pub indent: crate::editing::IndentOptions,
    pub right_margin: u32,
    pub insert_final_newline: bool,
}

impl Default for CodeStyle {
    fn default() -> Self {
        Self {
            indent: Default::default(),
            right_margin: 120,
            insert_final_newline: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    #[error("cancelled")]
    Cancelled,
    #[error("the file has syntax errors")]
    SyntaxErrors,
    #[error("{0}")]
    Tool(String),
}

impl From<Cancelled> for FormatError {
    fn from(_: Cancelled) -> Self {
        FormatError::Cancelled
    }
}

/// One formatter in the chain (native model, external tool, LSP): the first
/// that `can_format` wins.
pub trait FormattingService: Send + Sync {
    fn id(&self) -> &'static str;
    fn can_format(&self, file: &FileMeta) -> bool;
    /// Minimal edits formatting `range` (the whole file when `None`).
    fn format(
        &self,
        syntax: &SyntaxSnapshot,
        range: Option<Range<u32>>,
        style: &CodeStyle,
        cancel: &CancelToken,
    ) -> Result<Vec<TextEdit>, FormatError>;
}

/// A live-template context (`TemplateContextType`).
pub trait TemplateContextType: Send + Sync {
    fn id(&self) -> &'static str;
    fn is_in_context(&self, syntax: &SyntaxSnapshot, offset: u32) -> bool;
}
