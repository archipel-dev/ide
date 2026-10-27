//! Highlight data shared by the daemon (WP3), the LSP provider (WP8) and the
//! editor: severities and [`HighlightInfo`].

use crate::intention::QuickFix;
use smallvec::SmallVec;
use std::ops::Range;
use std::sync::Arc;

/// An ordered severity (IntelliJ `HighlightSeverity`: ordered integers;
/// users may define more). Compare by `value`, then name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Severity {
    pub value: u16,
    pub name: &'static str,
}

impl Severity {
    pub const INFORMATION: Severity = Severity::new(10, "INFORMATION");
    pub const TEXT_ATTRIBUTES: Severity = Severity::new(11, "TEXT ATTRIBUTES");
    pub const SERVER_PROBLEM: Severity = Severity::new(100, "SERVER PROBLEM");
    pub const INFO: Severity = Severity::new(200, "INFO");
    pub const WEAK_WARNING: Severity = Severity::new(200, "WEAK WARNING");
    pub const WARNING: Severity = Severity::new(300, "WARNING");
    pub const ERROR: Severity = Severity::new(400, "ERROR");

    pub const fn new(value: u16, name: &'static str) -> Self {
        Self { value, name }
    }

    /// At least a warning (shown in the error stripe / problems).
    pub fn is_problem(self) -> bool {
        self.value >= Self::WEAK_WARNING.value
    }
}

/// What a highlight means (drives its text attributes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HighlightKind {
    Error,
    Warning,
    WeakWarning,
    Information,
    Unused,
    Deprecated,
    MarkedForRemoval,
    Unresolved,
    Typo,
    /// Semantic coloring with a text-attributes key.
    Semantic(&'static str),
    /// Plain text attributes (no problem).
    TextAttributes(&'static str),
}

/// Owner of a highlight (a pass or inspection id): each tool's slice is
/// replaced atomically. Namespaced: `native:<id>`, `lsp:<server>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolId(pub Arc<str>);

impl ToolId {
    pub fn native(id: &str) -> Self {
        Self(format!("native:{id}").into())
    }

    pub fn lsp(server: &str) -> Self {
        Self(format!("lsp:{server}").into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_lsp(&self) -> bool {
        self.0.starts_with("lsp:")
    }
}

/// A cheap fix description; the action is materialized only when shown or
/// applied.
#[derive(Clone)]
pub struct FixDescriptor {
    pub family: Arc<str>,
    pub text: Arc<str>,
    pub action: Option<Arc<dyn QuickFix>>,
}

impl std::fmt::Debug for FixDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixDescriptor")
            .field("family", &self.family)
            .field("text", &self.text)
            .finish()
    }
}

impl PartialEq for FixDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.family == other.family && self.text == other.text
    }
}

/// Groups problems for "fix all of this kind".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProblemGroup(pub Arc<str>);

/// One highlight (problem or decoration) of a file revision.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightInfo {
    pub range: Range<u32>,
    pub severity: Severity,
    pub kind: HighlightKind,
    pub message: Option<Arc<str>>,
    pub tooltip: Option<Arc<str>>,
    pub fixes: SmallVec<[FixDescriptor; 2]>,
    pub tool: ToolId,
    pub group: Option<ProblemGroup>,
    pub problem_range_for_gutter: Option<Range<u32>>,
    pub file_level: bool,
    /// Problem code of an external tool (`67108964`, `E0425`).
    pub code: Option<Arc<str>>,
    /// Display name of the external tool that reported it (`Java`, `rustc`).
    pub source: Option<Arc<str>>,
}

impl HighlightInfo {
    pub fn new(range: Range<u32>, severity: Severity, kind: HighlightKind, tool: ToolId) -> Self {
        Self {
            range,
            severity,
            kind,
            message: None,
            tooltip: None,
            fixes: SmallVec::new(),
            tool,
            group: None,
            problem_range_for_gutter: None,
            file_level: false,
            code: None,
            source: None,
        }
    }

    pub fn message(mut self, message: impl Into<Arc<str>>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_fix(mut self, fix: FixDescriptor) -> Self {
        self.fixes.push(fix);
        self
    }

    /// Same range, kind and message (the markup recycler's identity).
    pub fn same_as(&self, other: &HighlightInfo) -> bool {
        self.range == other.range && self.kind == other.kind && self.message == other.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severities_are_ordered_like_intellij() {
        assert!(Severity::ERROR > Severity::WARNING);
        assert!(Severity::WARNING > Severity::WEAK_WARNING);
        assert_eq!(Severity::INFO.value, Severity::WEAK_WARNING.value);
        assert!(Severity::WEAK_WARNING > Severity::SERVER_PROBLEM);
        assert!(!Severity::INFORMATION.is_problem());
        let info = HighlightInfo::new(
            0..3,
            Severity::ERROR,
            HighlightKind::Error,
            ToolId::native("syntax"),
        )
        .message("boom");
        assert!(info.same_as(&info.clone()));
        assert!(ToolId::lsp("rust-analyzer").is_lsp());
    }
}
