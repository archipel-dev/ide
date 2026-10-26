//! Inspection contracts (implemented by WP3 and the language crates).

use crate::cancel::{CancelToken, Cancelled};
use crate::file::{FileId, LanguageId};
use crate::host::Snapshot;
use crate::intention::QuickFix;
use crate::resolve::SearchScope;
use crate::stamps::IndexId;
use archipel_ide_language::{CompletenessTier, SyntaxNode, SyntaxSnapshot, TokenSet};
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

/// Languages a tool applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageFilter {
    Any,
    Only(&'static [&'static str]),
}

impl LanguageFilter {
    pub fn accepts(&self, language: LanguageId) -> bool {
        match self {
            LanguageFilter::Any => true,
            LanguageFilter::Only(ids) => ids.contains(&language.as_str()),
        }
    }
}

/// Default highlight level of an inspection (profile value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    Error,
    Warning,
    WeakWarning,
    Information,
    TextAttributes,
}

/// How a problem is highlighted (`ProblemHighlightType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProblemHighlightType {
    GenericErrorOrWarning,
    LikeUnknownSymbol,
    LikeDeprecated,
    LikeMarkedForRemoval,
    LikeUnusedSymbol,
    Error,
    Warning,
    WeakWarning,
    Information,
    PossibleProblem,
}

/// One option of an inspection (typed schema -> generated settings UI).
#[derive(Debug, Clone, PartialEq)]
pub struct OptionDef {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: OptionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionKind {
    Bool(bool),
    Int {
        default: i64,
        min: i64,
        max: i64,
    },
    Text(&'static str),
    Choice {
        default: &'static str,
        choices: &'static [&'static str],
    },
}

/// Current option values of one tool (profile data).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InspectionOptions(pub HashMap<&'static str, OptionValue>);

#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Bool(bool),
    Int(i64),
    Text(String),
}

/// A problem found by an inspection.
#[derive(Clone)]
pub struct ProblemDescriptor {
    pub range: Range<u32>,
    pub message: String,
    pub highlight: ProblemHighlightType,
    pub fixes: Vec<Arc<dyn QuickFix>>,
    pub on_the_fly_only: bool,
}

impl std::fmt::Debug for ProblemDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProblemDescriptor")
            .field("range", &self.range)
            .field("message", &self.message)
            .field("highlight", &self.highlight)
            .field("fixes", &self.fixes.len())
            .finish()
    }
}

/// Collects problems (`ProblemsHolder`).
#[derive(Debug, Default)]
pub struct ProblemsHolder {
    pub on_the_fly: bool,
    problems: Vec<ProblemDescriptor>,
}

impl ProblemsHolder {
    pub fn new(on_the_fly: bool) -> Self {
        Self {
            on_the_fly,
            problems: Vec::new(),
        }
    }

    pub fn register_problem(
        &mut self,
        range: Range<u32>,
        message: impl Into<String>,
        highlight: ProblemHighlightType,
    ) {
        self.register(ProblemDescriptor {
            range,
            message: message.into(),
            highlight,
            fixes: Vec::new(),
            on_the_fly_only: false,
        });
    }

    pub fn register(&mut self, problem: ProblemDescriptor) {
        self.problems.push(problem);
    }

    pub fn problems(&self) -> &[ProblemDescriptor] {
        &self.problems
    }

    pub fn into_problems(self) -> Vec<ProblemDescriptor> {
        self.problems
    }
}

/// Context of a per-element check.
pub struct InspectCx<'a> {
    pub snap: &'a Snapshot,
    pub syntax: &'a SyntaxSnapshot,
    pub cancel: &'a CancelToken,
    pub options: &'a InspectionOptions,
    pub on_the_fly: bool,
}

/// Context of a whole-file check.
pub struct FileCx<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub syntax: &'a SyntaxSnapshot,
    pub cancel: &'a CancelToken,
    pub options: &'a InspectionOptions,
    pub on_the_fly: bool,
}

/// A local inspection (`LocalInspectionTool`).
pub trait LocalInspection: Send + Sync {
    /// Short name = suppression id = settings key (stable!).
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn group_path(&self) -> &'static [&'static str];
    fn description_md(&self) -> &'static str;
    fn languages(&self) -> LanguageFilter;
    fn default_level(&self) -> Level;
    fn enabled_by_default(&self) -> bool;
    /// D6: muted below this tier for the file's language.
    fn min_tier(&self) -> CompletenessTier;
    /// `None` = whole-file; `Some` = dispatched per matching element.
    fn interesting(&self) -> Option<&TokenSet>;
    fn dumb_aware(&self) -> bool;
    fn needs_indexes(&self) -> &[IndexId];
    fn options(&self) -> &'static [OptionDef];
    fn check_element(
        &self,
        _n: SyntaxNode<'_>,
        _h: &mut ProblemsHolder,
        _cx: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        Ok(())
    }
    fn check_file(&self, _f: &FileCx<'_>, _h: &mut ProblemsHolder) -> Result<(), Cancelled> {
        Ok(())
    }
}

/// What a batch run covers (`AnalysisScope`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisScope {
    Project,
    Files(Vec<FileId>),
    Directory(PathBuf),
    Scope(SearchScope),
}

/// Context of a global inspection.
pub struct GlobalCx<'a> {
    pub snap: &'a Snapshot,
    pub cancel: &'a CancelToken,
}

/// Receives problems of a batch run.
pub trait ProblemSink {
    fn report(&mut self, file: FileId, problem: ProblemDescriptor);
}

impl ProblemSink for Vec<(FileId, ProblemDescriptor)> {
    fn report(&mut self, file: FileId, problem: ProblemDescriptor) {
        self.push((file, problem));
    }
}

/// A whole-project inspection (`GlobalInspectionTool`).
pub trait GlobalInspection: Send + Sync {
    fn id(&self) -> &'static str;
    fn run(
        &self,
        scope: &AnalysisScope,
        g: &GlobalCx<'_>,
        out: &mut dyn ProblemSink,
    ) -> Result<(), Cancelled>;
}

/// Marks declarations as used from outside (`EntryPoint`): mains, tests,
/// framework annotations.
pub trait EntryPoint: Send + Sync {
    fn is_entry_point(&self, declaration: SyntaxNode<'_>) -> bool;
}

/// Recognizes suppression comments/annotations (`InspectionSuppressor`).
pub trait InspectionSuppressor: Send + Sync {
    fn is_suppressed(&self, node: SyntaxNode<'_>, tool_id: &str) -> bool;
}
