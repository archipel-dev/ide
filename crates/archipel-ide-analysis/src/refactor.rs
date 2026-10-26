//! Refactoring contracts (implemented by WP7 and language crates).

use crate::cancel::{CancelToken, Cancelled};
use crate::file::FileId;
use crate::host::Snapshot;
use crate::modcommand::{Conflict, ModCommand};
use crate::resolve::Usage;
use crate::target::Target;
use std::ops::Range;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RefactoringKind {
    Rename,
    SafeDelete,
    Move,
    ExtractVariable,
    ExtractMethod,
    Inline,
    IntroduceField,
    IntroduceParameter,
    ChangeSignature,
}

/// Input of a refactoring.
pub struct RefactorCx<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub caret: u32,
    pub selection: Option<Range<u32>>,
    /// The resolved element under the caret, if any.
    pub target: Option<&'a Target>,
    /// New name (rename, extract, introduce).
    pub new_name: Option<&'a str>,
    pub cancel: &'a CancelToken,
}

/// A prepared refactoring: preview it, show conflicts, then execute the one
/// command (one undo step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefactoringPlan {
    pub label: String,
    pub usages: Vec<Usage>,
    pub conflicts: Vec<Conflict>,
    pub command: ModCommand,
}

/// `RefactoringSupportProvider`.
pub trait RefactoringSupport: Send + Sync {
    fn is_available(&self, kind: RefactoringKind, cx: &RefactorCx<'_>) -> bool;
    fn prepare(
        &self,
        kind: RefactoringKind,
        cx: &RefactorCx<'_>,
    ) -> Result<RefactoringPlan, Cancelled>;
    /// Rejects names the language does not allow.
    fn validate_name(&self, kind: RefactoringKind, name: &str) -> Result<(), String> {
        let _ = kind;
        if name.is_empty() {
            Err("the name is empty".into())
        } else {
            Ok(())
        }
    }
}

/// Run configurations, breakpoints and bookmarks follow refactorings.
pub trait RefactoringListener: Send + Sync {
    fn renamed(&self, _from: &Target, _to: &str) {}
    fn moved(&self, _file: FileId, _to: &Path) {}
    fn deleted(&self, _target: &Target) {}
}
