//! In-memory implementations of every extension point and fixtures, so each
//! package can develop and test against the frozen contracts before the real
//! implementations land. They are deliberately small but behave correctly.

use crate::assist::{
    CodeStyle, CompletionContributor, CompletionParams, CompletionResultSet, CompletionWeigher,
    Documentation, DocumentationProvider, FormatError, FormattingService, ParameterInfo,
    ParameterInfoProvider, TemplateContextType,
};
use crate::cancel::{CancelToken, Cancelled, walk};
use crate::daemon::{
    AnnotateCx, AnnotationHolder, Annotator, ExternalAnnotator, HighlightMarkup, Pass, PassCx,
    PassId, PassOutput,
};
use crate::editing::{
    BreadcrumbsProvider, Commenter, Crumb, EditSet, IndentOptions, Indenter, MoveStatementHandler,
    QuoteHandler, SmartEnterProcessor, StickyScope, StickyScopesProvider, TypedDecision,
    TypedHandler,
};
use crate::file::{FileId, FileMeta, FileRev};
use crate::highlight::{HighlightInfo, HighlightKind, Severity, ToolId};
use crate::host::{AnalysisHost, Change, HostConfig, Snapshot, default_registry};
use crate::index::{FileIndex, IndexInput, IndexPriority};
use crate::inspect::{
    AnalysisScope, EntryPoint, GlobalCx, GlobalInspection, InspectCx, InspectionSuppressor,
    LanguageFilter, Level, LocalInspection, OptionDef, ProblemDescriptor, ProblemHighlightType,
    ProblemSink, ProblemsHolder,
};
use crate::intention::{ActionCx, FixCx, IntentionAction, Presentation, QuickFix};
use crate::modcommand::{
    EditError, ExecOutcome, ModCommand, ModCommandExecutor, TextEdit, apply_edits,
};
use crate::provider::{AnalysisProvider, Merger, ProviderMode, merge_dedupe};
use crate::refactor::{
    RefactorCx, RefactoringKind, RefactoringListener, RefactoringPlan, RefactoringSupport,
};
use crate::resolve::{
    LookupItem, ReadWriteAccessDetector, Reference, ReferenceProvider, ResolveCompleteness,
    ResolveCx, ResolveResults, Resolver, Usage, UsageKind, UsageSearcher,
};
use crate::stamps::IndexId;
use crate::target::{Confidence, Provenance, Target};
use crate::vfs::MemVfs;
use archipel_ide_language::structure::StructureItem;
use archipel_ide_language::{
    Anchor, CompletenessTier, LanguageDef, Order, Registration, ServiceRegistry, SyntaxNode,
    SyntaxSnapshot, TokenSet,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ---- fixtures --------------------------------------------------------------

pub const JAVA_PATH: &str = "/fixture/src/App.java";
pub const JAVA_SAMPLE: &str = "package demo;\n\n/** Greets. */\npublic class App {\n    private int count;\n\n    public String greet(String name) {\n        count = count + 1;\n        return \"Hello, \" + name;\n    }\n\n    void empty() {}\n}\n";
pub const RUST_PATH: &str = "/fixture/src/lib.rs";
pub const RUST_SAMPLE: &str = "/// Answer.\npub fn answer(x: u32) -> u32 {\n    let doubled = x * 2;\n    doubled + 42 // FIXME tune\n}\n\npub struct Point {\n    x: i32,\n}\n";
pub const MARKDOWN_PATH: &str = "/fixture/README.md";
pub const MARKDOWN_SAMPLE: &str = "# Fixture\n\n```rust\nfn main() {}\n```\n";

/// A fixture project in memory.
pub struct Fixture {
    pub host: AnalysisHost,
    pub vfs: Arc<MemVfs>,
    pub java: FileId,
    pub rust: FileId,
    pub markdown: FileId,
}

impl Fixture {
    /// The three sample files on a [`MemVfs`], none open, synchronous host
    /// ([`HostConfig::testing`]) with [`default_registry`] plus every mock of
    /// this module ([`register_mocks`]).
    pub fn new() -> Self {
        let vfs = Arc::new(MemVfs::with_files([
            (JAVA_PATH, JAVA_SAMPLE),
            (RUST_PATH, RUST_SAMPLE),
            (MARKDOWN_PATH, MARKDOWN_SAMPLE),
        ]));
        let mut registry = default_registry();
        register_mocks(&mut registry);
        let mut host = AnalysisHost::new(HostConfig::testing(), vfs.clone(), registry);
        let java = host.file_id(Path::new(JAVA_PATH));
        let rust = host.file_id(Path::new(RUST_PATH));
        let markdown = host.file_id(Path::new(MARKDOWN_PATH));
        Self {
            host,
            vfs,
            java,
            rust,
            markdown,
        }
    }

    /// Opens `file` with its disk text at revision 1.
    pub fn open(&mut self, file: FileId) -> Arc<str> {
        let snap = self.host.snapshot();
        let text = snap.file_text(file).unwrap_or_else(|_| "".into());
        self.host.apply(Change::Open {
            file,
            rev: FileRev(1),
            text: text.clone(),
        });
        text
    }
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Byte edit replacing `range` of `old` with `text` (row/column computed).
pub fn byte_edit(old: &str, range: Range<usize>, text: &str) -> archipel_ide_language::ByteEdit {
    use archipel_ide_language::BytePoint;
    let point = |s: &str, at: usize| BytePoint {
        row: s[..at].matches('\n').count(),
        column: at - s[..at].rfind('\n').map_or(0, |i| i + 1),
    };
    let mut new = old.to_string();
    new.replace_range(range.clone(), text);
    archipel_ide_language::ByteEdit {
        start_byte: range.start,
        old_end_byte: range.end,
        new_end_byte: range.start + text.len(),
        start_point: point(old, range.start),
        old_end_point: point(old, range.end),
        new_end_point: point(&new, range.start + text.len()),
    }
}

/// Registers every mock of this module (each under a `mock-*` id).
pub fn register_mocks(registry: &mut ServiceRegistry) {
    let any = Registration::any;
    registry.register::<dyn Pass>(
        any("mock-syntax-pass").dumb_aware(true),
        Arc::new(SyntaxErrorPass),
    );
    registry.register::<dyn Annotator>(
        any("mock-word-annotator").dumb_aware(true),
        Arc::new(WordAnnotator::new("FIXME")),
    );
    registry.register::<dyn crate::daemon::DynExternalAnnotator>(
        any("mock-external"),
        Arc::new(FixmeExternalAnnotator),
    );
    registry.register::<dyn LocalInspection>(
        any("mock-empty-block").dumb_aware(true),
        Arc::new(EmptyBlockInspection),
    );
    registry.register::<dyn GlobalInspection>(
        any("mock-global"),
        Arc::new(TooManyFilesInspection { limit: 100 }),
    );
    registry.register::<dyn IntentionAction>(
        any("mock-uppercase").dumb_aware(true),
        Arc::new(UppercaseIntention),
    );
    registry
        .register::<dyn InspectionSuppressor>(any("mock-suppressor"), Arc::new(CommentSuppressor));
    registry.register::<dyn EntryPoint>(any("mock-main"), Arc::new(MainEntryPoint));
    registry.register::<dyn Resolver>(
        any("mock-resolver").dumb_aware(true),
        Arc::new(FileLocalResolver),
    );
    registry.register::<dyn ReferenceProvider>(
        any("mock-references"),
        Arc::new(NameReferenceProvider::default()),
    );
    registry.register::<dyn UsageSearcher>(any("mock-usages"), Arc::new(TextUsageSearcher));
    registry
        .register::<dyn ReadWriteAccessDetector>(any("mock-access"), Arc::new(AssignmentAccess));
    registry.register::<dyn CompletionContributor>(
        any("mock-words").dumb_aware(true),
        Arc::new(FileWordsCompletion),
    );
    registry.register::<dyn CompletionWeigher>(any("mock-prefix-weigher"), Arc::new(PrefixWeigher));
    registry
        .register::<dyn DocumentationProvider>(any("mock-docs"), Arc::new(StructureDocumentation));
    registry.register::<dyn ParameterInfoProvider>(any("mock-params"), Arc::new(NoParameterInfo));
    registry.register::<dyn FormattingService>(
        any("mock-trailing-whitespace"),
        Arc::new(TrailingWhitespaceFormatter),
    );
    registry
        .register::<dyn TemplateContextType>(any("mock-everywhere"), Arc::new(EverywhereContext));
    registry.register::<dyn RefactoringSupport>(any("mock-rename"), Arc::new(TextRename));
    registry.register::<dyn RefactoringListener>(
        any("mock-listener"),
        Arc::new(RecordingListener::default()),
    );
    registry.register::<dyn Commenter>(any("mock-commenter"), Arc::new(LineCommenter("//")));
    registry.register::<dyn QuoteHandler>(any("mock-quotes"), Arc::new(PairQuotes));
    registry.register::<dyn Indenter>(any("mock-indenter"), Arc::new(BraceDepthIndenter));
    registry.register::<dyn TypedHandler>(any("mock-typed"), Arc::new(PairingTypedHandler));
    registry.register::<dyn SmartEnterProcessor>(
        any("mock-smart-enter"),
        Arc::new(SemicolonSmartEnter),
    );
    registry.register::<dyn MoveStatementHandler>(any("mock-move"), Arc::new(NoMoveStatement));
    registry.register::<dyn StickyScopesProvider>(any("mock-sticky"), Arc::new(StructureSticky));
    registry.register::<dyn BreadcrumbsProvider>(
        any("mock-breadcrumbs"),
        Arc::new(StructureBreadcrumbs),
    );
    MockLspProvider.register(registry);
}

fn identifiers(lang: &LanguageDef) -> &TokenSet {
    &lang.token_set_table().identifiers
}

fn target_for(
    syntax: &SyntaxSnapshot,
    item: &StructureItem,
    confidence: Confidence,
) -> Option<Target> {
    let node = syntax.covering(item.range.start as u32..item.range.end as u32);
    Some(Target {
        file: syntax.file(),
        anchor: Anchor::new(node),
        name: item.name.as_str().into(),
        name_range: item.name_range.start as u32..item.name_range.end as u32,
        kind: item.kind,
        container: None,
        provenance: Provenance::Native(syntax.lang().language_id()),
        confidence,
    })
}

fn flatten(items: &[StructureItem], out: &mut Vec<StructureItem>) {
    for item in items {
        out.push(item.clone());
        flatten(&item.children, out);
    }
}

// ---- daemon ------------------------------------------------------------------

/// Markup kept in a vector; replace semantics per (tool, range).
#[derive(Debug, Default)]
pub struct VecMarkup {
    pub infos: Vec<HighlightInfo>,
}

impl HighlightMarkup for VecMarkup {
    fn replace(&mut self, tool: &ToolId, range: Range<u32>, infos: Vec<HighlightInfo>) {
        self.infos.retain(|info| {
            &info.tool != tool || info.range.end < range.start || info.range.start > range.end
        });
        self.infos.extend(infos);
        self.infos
            .sort_by_key(|info| (info.range.start, info.range.end));
    }

    fn query(&self, range: Range<u32>) -> Vec<&HighlightInfo> {
        self.infos
            .iter()
            .filter(|info| info.range.start <= range.end && range.start <= info.range.end)
            .collect()
    }
}

/// Syntax errors as error highlights.
#[derive(Debug, Clone, Copy, Default)]
pub struct SyntaxErrorPass;

impl Pass for SyntaxErrorPass {
    fn id(&self) -> PassId {
        PassId("mock-syntax")
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
        let tool = ToolId::native("mock-syntax");
        let infos = cx
            .syntax
            .errors()
            .iter()
            .filter(|error| error.range.start <= cx.range.end && cx.range.start <= error.range.end)
            .map(|error| {
                HighlightInfo::new(
                    error.range.clone(),
                    Severity::ERROR,
                    HighlightKind::Error,
                    tool.clone(),
                )
                .message(error.message.clone())
            })
            .collect();
        Ok(PassOutput {
            file: cx.file,
            rev: cx.syntax.rev(),
            tool,
            range: cx.range.clone(),
            infos,
        })
    }
}

/// Weak warning on every identifier or comment containing `word`.
#[derive(Debug, Clone)]
pub struct WordAnnotator {
    word: &'static str,
    all: TokenSet,
}

impl WordAnnotator {
    pub fn new(word: &'static str) -> Self {
        // Every kind id: the annotator filters by text itself.
        let mut all = TokenSet::empty();
        for id in 0..u16::MAX {
            all.insert(archipel_ide_language::ElementType(id));
        }
        all.insert(archipel_ide_language::ElementType(u16::MAX));
        Self { word, all }
    }
}

impl Annotator for WordAnnotator {
    fn interesting(&self) -> &TokenSet {
        &self.all
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn annotate(
        &self,
        n: SyntaxNode<'_>,
        h: &mut AnnotationHolder,
        cx: &AnnotateCx<'_>,
    ) -> Result<(), Cancelled> {
        cx.cancel.check()?;
        let sets = n.snapshot().lang().token_set_table();
        let ty = n.element_type();
        if (sets.comments.contains(ty) || identifiers(n.snapshot().lang()).contains(ty))
            && let Some(at) = n.text().find(self.word)
        {
            let start = n.range().start + at as u32;
            h.new_annotation(Severity::WEAK_WARNING, format!("{} marker", self.word))
                .range(start..start + self.word.len() as u32)
                .create();
        }
        Ok(())
    }
}

/// External annotator finding `FIXME` in the text "off-thread".
#[derive(Debug, Clone, Copy, Default)]
pub struct FixmeExternalAnnotator;

impl ExternalAnnotator for FixmeExternalAnnotator {
    type Info = Arc<str>;
    type Result = Vec<Range<u32>>;
    fn collect(&self, snap: &Snapshot, file: FileId) -> Option<Arc<str>> {
        snap.file_text(file).ok()
    }
    fn run(&self, info: Arc<str>, cancel: &CancelToken) -> Option<Vec<Range<u32>>> {
        cancel.check().ok()?;
        Some(
            info.match_indices("FIXME")
                .map(|(at, word)| at as u32..(at + word.len()) as u32)
                .collect(),
        )
    }
    fn apply(&self, _file: FileId, _rev: FileRev, r: Vec<Range<u32>>, h: &mut AnnotationHolder) {
        for range in r {
            h.new_annotation(Severity::INFORMATION, "external FIXME")
                .range(range)
                .create();
        }
    }
}

// ---- inspections and intentions ----------------------------------------------------

/// Flags code blocks without named children.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyBlockInspection;

impl LocalInspection for EmptyBlockInspection {
    fn id(&self) -> &'static str {
        "MockEmptyBlock"
    }
    fn display_name(&self) -> &'static str {
        "Empty block"
    }
    fn group_path(&self) -> &'static [&'static str] {
        &["Mock"]
    }
    fn description_md(&self) -> &'static str {
        "Reports blocks without statements."
    }
    fn languages(&self) -> LanguageFilter {
        LanguageFilter::Any
    }
    fn default_level(&self) -> Level {
        Level::Warning
    }
    fn enabled_by_default(&self) -> bool {
        true
    }
    fn min_tier(&self) -> CompletenessTier {
        CompletenessTier::G0
    }
    fn interesting(&self) -> Option<&TokenSet> {
        None
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn needs_indexes(&self) -> &[IndexId] {
        &[]
    }
    fn options(&self) -> &'static [OptionDef] {
        &[]
    }
    fn check_element(
        &self,
        n: SyntaxNode<'_>,
        h: &mut ProblemsHolder,
        cx: &InspectCx<'_>,
    ) -> Result<(), Cancelled> {
        cx.cancel.check()?;
        let blocks = &n.snapshot().lang().token_set_table().code_block_like;
        if blocks.contains(n.element_type()) && n.named_children().next().is_none() {
            h.register(ProblemDescriptor {
                range: n.range(),
                message: "Empty block".into(),
                highlight: ProblemHighlightType::Warning,
                fixes: vec![Arc::new(DeleteRangeFix)],
                on_the_fly_only: false,
            });
        }
        Ok(())
    }
    fn check_file(
        &self,
        f: &crate::inspect::FileCx<'_>,
        h: &mut ProblemsHolder,
    ) -> Result<(), Cancelled> {
        let cx = InspectCx {
            snap: f.snap,
            syntax: f.syntax,
            cancel: f.cancel,
            options: f.options,
            on_the_fly: f.on_the_fly,
        };
        let mut nodes = Vec::new();
        walk(f.syntax.root(), f.cancel, |node| nodes.push(node))?;
        for node in nodes {
            self.check_element(node, h, &cx)?;
        }
        Ok(())
    }
}

/// Deletes the problem range.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeleteRangeFix;

impl QuickFix for DeleteRangeFix {
    fn family(&self) -> &str {
        "Delete"
    }
    fn name(&self) -> String {
        "Delete element".into()
    }
    fn fix(&self, d: &ProblemDescriptor, cx: &FixCx<'_>) -> Result<ModCommand, Cancelled> {
        cx.cancel.check()?;
        Ok(ModCommand::UpdateFile {
            file: cx.file,
            edits: vec![TextEdit::delete(d.range.clone())],
            expected_rev: cx.syntax.rev(),
        })
    }
}

/// Reports projects with more than `limit` files.
#[derive(Debug, Clone, Copy)]
pub struct TooManyFilesInspection {
    pub limit: usize,
}

impl GlobalInspection for TooManyFilesInspection {
    fn id(&self) -> &'static str {
        "MockTooManyFiles"
    }
    fn run(
        &self,
        scope: &AnalysisScope,
        g: &GlobalCx<'_>,
        out: &mut dyn ProblemSink,
    ) -> Result<(), Cancelled> {
        g.cancel.check()?;
        let files: Vec<FileId> = match scope {
            AnalysisScope::Files(files) => files.clone(),
            _ => g.snap.files().iter().map(|(id, _)| id).collect(),
        };
        if files.len() > self.limit
            && let Some(first) = files.first()
        {
            out.report(
                *first,
                ProblemDescriptor {
                    range: 0..0,
                    message: format!("{} files", files.len()),
                    highlight: ProblemHighlightType::WeakWarning,
                    fixes: Vec::new(),
                    on_the_fly_only: false,
                },
            );
        }
        Ok(())
    }
}

/// Upper-cases the identifier at the caret.
#[derive(Debug, Clone, Copy, Default)]
pub struct UppercaseIntention;

impl UppercaseIntention {
    fn identifier<'s>(cx: &ActionCx<'s>) -> Option<SyntaxNode<'s>> {
        let leaf = cx.syntax.node_at(cx.caret)?;
        identifiers(cx.syntax.lang())
            .contains(leaf.element_type())
            .then_some(leaf)
            .filter(|leaf| leaf.text().chars().any(char::is_lowercase))
    }
}

impl IntentionAction for UppercaseIntention {
    fn family(&self) -> &'static str {
        "Upper-case identifier"
    }
    fn id(&self) -> &'static str {
        "MockUppercase"
    }
    fn languages(&self) -> LanguageFilter {
        LanguageFilter::Any
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn presentation(&self, cx: &ActionCx<'_>) -> Option<Presentation> {
        let leaf = Self::identifier(cx)?;
        Some(Presentation {
            text: format!("Upper-case '{}'", leaf.text()),
        })
    }
    fn perform(&self, cx: &ActionCx<'_>) -> Result<ModCommand, Cancelled> {
        cx.cancel.check()?;
        let Some(leaf) = Self::identifier(cx) else {
            return Ok(ModCommand::Nothing);
        };
        Ok(ModCommand::UpdateFile {
            file: cx.file,
            edits: vec![TextEdit::new(leaf.range(), leaf.text().to_uppercase())],
            expected_rev: cx.syntax.rev(),
        })
    }
}

/// `// noinspection <id>` on the line before the element.
#[derive(Debug, Clone, Copy, Default)]
pub struct CommentSuppressor;

impl InspectionSuppressor for CommentSuppressor {
    fn is_suppressed(&self, node: SyntaxNode<'_>, tool_id: &str) -> bool {
        let text = node.snapshot().text();
        let start = node.byte_range().start;
        let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let previous = text[..line_start.saturating_sub(1)]
            .rsplit('\n')
            .next()
            .unwrap_or("");
        previous
            .trim()
            .strip_prefix("// noinspection ")
            .is_some_and(|ids| ids.split(',').any(|id| id.trim() == tool_id))
    }
}

/// Functions named `main` are entry points.
#[derive(Debug, Clone, Copy, Default)]
pub struct MainEntryPoint;

impl EntryPoint for MainEntryPoint {
    fn is_entry_point(&self, declaration: SyntaxNode<'_>) -> bool {
        declaration
            .name_node()
            .is_some_and(|name| name.text() == "main")
    }
}

// ---- resolve -----------------------------------------------------------------------

/// Resolves an identifier to same-file declarations with that name
/// (`Heuristic`: it ignores scopes).
#[derive(Debug, Clone, Copy, Default)]
pub struct FileLocalResolver;

impl Resolver for FileLocalResolver {
    fn resolve_at(
        &self,
        snap: &SyntaxSnapshot,
        leaf: SyntaxNode<'_>,
        cx: &ResolveCx<'_>,
    ) -> Result<ResolveResults, Cancelled> {
        cx.cancel.check()?;
        let mut items = Vec::new();
        flatten(&snap.structure(), &mut items);
        let mut results = ResolveResults::default();
        for item in items.iter().filter(|item| item.name == leaf.text()) {
            if let Some(target) = target_for(snap, item, Confidence::Heuristic) {
                results.0.push(crate::resolve::ResolveHit {
                    target,
                    valid: true,
                });
            }
        }
        Ok(results)
    }
    fn completeness(&self) -> ResolveCompleteness {
        ResolveCompleteness::PartialNoUnresolvedDiagnostics
    }
}

/// A reference on an identifier leaf, resolved by [`FileLocalResolver`].
pub struct NameReference {
    anchor: Anchor,
    text: Arc<str>,
    range: Range<u32>,
}

impl Reference for NameReference {
    fn node(&self) -> &Anchor {
        &self.anchor
    }
    fn range_in_node(&self) -> Range<u32> {
        0..self.text.len() as u32
    }
    fn canonical_text(&self) -> Arc<str> {
        self.text.clone()
    }
    fn resolve(&self, cx: &ResolveCx<'_>) -> Result<ResolveResults, Cancelled> {
        let Some(syntax) = cx.snap.syntax(self.anchor.file) else {
            return Ok(ResolveResults::default());
        };
        let edits = cx
            .snap
            .edits_since(self.anchor.file, self.anchor.rev)
            .unwrap_or_default();
        let Some(leaf) = self.anchor.resolve(&syntax, &edits) else {
            return Ok(ResolveResults::default());
        };
        FileLocalResolver.resolve_at(&syntax, leaf, cx)
    }
    fn variants(&self, cx: &ResolveCx<'_>) -> Result<Vec<LookupItem>, Cancelled> {
        cx.cancel.check()?;
        let Some(syntax) = cx.snap.syntax(self.anchor.file) else {
            return Ok(Vec::new());
        };
        let mut items = Vec::new();
        flatten(&syntax.structure(), &mut items);
        Ok(items
            .into_iter()
            .map(|item| LookupItem {
                kind: Some(item.kind),
                ..LookupItem::new(item.name.as_str())
            })
            .collect())
    }
    fn rename_edit(&self, new_name: &str) -> Option<TextEdit> {
        Some(TextEdit::new(self.range.clone(), new_name))
    }
    fn is_soft(&self) -> bool {
        true
    }
}

/// One [`NameReference`] per identifier leaf. `interesting` is empty here
/// because token sets are per language: [`NameReferenceProvider::for_language`].
#[derive(Debug, Clone, Default)]
pub struct NameReferenceProvider {
    interesting: TokenSet,
}

impl NameReferenceProvider {
    pub fn for_language(lang: &LanguageDef) -> Self {
        Self {
            interesting: identifiers(lang).clone(),
        }
    }
}

impl ReferenceProvider for NameReferenceProvider {
    fn interesting(&self) -> &TokenSet {
        &self.interesting
    }
    fn references(&self, n: SyntaxNode<'_>, out: &mut Vec<Box<dyn Reference>>) {
        if identifiers(n.snapshot().lang()).contains(n.element_type()) {
            out.push(Box::new(NameReference {
                anchor: Anchor::new(n),
                text: n.text().into(),
                range: n.range(),
            }));
        }
    }
}

/// Whole-word text occurrences in the scope's open files and the target's
/// file (`Heuristic`).
#[derive(Debug, Clone, Copy, Default)]
pub struct TextUsageSearcher;

impl UsageSearcher for TextUsageSearcher {
    fn search(
        &self,
        target: &Target,
        cx: &ResolveCx<'_>,
        sink: &mut dyn FnMut(Usage) -> bool,
    ) -> Result<(), Cancelled> {
        let mut files: Vec<FileId> = cx.snap.open_files().collect();
        if !files.contains(&target.file) {
            files.push(target.file);
        }
        files.sort();
        for file in files.into_iter().filter(|file| cx.scope.contains(*file)) {
            cx.cancel.check()?;
            let Ok(text) = cx.snap.file_text(file) else {
                continue;
            };
            let word = |c: char| c.is_alphanumeric() || c == '_';
            for (at, _) in text.match_indices(&*target.name) {
                let end = at + target.name.len();
                if text[..at].chars().next_back().is_some_and(word)
                    || text[end..].chars().next().is_some_and(word)
                {
                    continue;
                }
                let range = at as u32..end as u32;
                let kind = if file == target.file && range == target.name_range {
                    UsageKind::Declaration
                } else {
                    UsageKind::Read
                };
                if !sink(Usage {
                    file,
                    range,
                    kind,
                    confidence: Confidence::Heuristic,
                }) {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

/// Left side of an assignment = write.
#[derive(Debug, Clone, Copy, Default)]
pub struct AssignmentAccess;

impl ReadWriteAccessDetector for AssignmentAccess {
    fn access(&self, reference: SyntaxNode<'_>) -> UsageKind {
        let is_left = reference.parent().is_some_and(|parent| {
            parent.kind().contains("assignment")
                && parent
                    .child_by_field_name("left")
                    .is_some_and(|left| left == reference)
        });
        if is_left {
            UsageKind::Write
        } else {
            UsageKind::Read
        }
    }
}

// ---- index -------------------------------------------------------------------------

/// Lower-case file name -> () (the `FileName` index, metadata only).
#[derive(Debug, Clone, Copy, Default)]
pub struct FileNameIndex;

impl FileIndex for FileNameIndex {
    const ID: IndexId = IndexId("FileName");
    const VERSION: u32 = 1;
    type Key = String;
    type Value = ();
    fn accepts(&self, _f: &FileMeta) -> bool {
        true
    }
    fn depends_on_content(&self) -> bool {
        false
    }
    fn index(
        &self,
        input: &IndexInput<'_>,
        out: &mut dyn FnMut(String, ()),
    ) -> Result<(), Cancelled> {
        if let Some(name) = input.file.path.file_name().and_then(|n| n.to_str()) {
            out(name.to_lowercase(), ());
        }
        Ok(())
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn priority(&self) -> IndexPriority {
        IndexPriority::Names
    }
}

/// Identifier word -> occurrence count (a tiny `Words` index).
#[derive(Debug, Clone, Copy, Default)]
pub struct WordCountIndex;

impl FileIndex for WordCountIndex {
    const ID: IndexId = IndexId("MockWords");
    const VERSION: u32 = 1;
    type Key = String;
    type Value = u32;
    fn accepts(&self, f: &FileMeta) -> bool {
        f.len <= crate::vfs::MAX_FILE_BYTES
    }
    fn index(
        &self,
        input: &IndexInput<'_>,
        out: &mut dyn FnMut(String, u32),
    ) -> Result<(), Cancelled> {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for word in input
            .text
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|w| !w.is_empty())
        {
            *counts.entry(word).or_default() += 1;
        }
        for (word, count) in counts {
            out(word.to_string(), count);
        }
        Ok(())
    }
    fn priority(&self) -> IndexPriority {
        IndexPriority::Words
    }
}

// ---- assistance --------------------------------------------------------------------

/// Words of the file starting with the prefix.
#[derive(Debug, Clone, Copy, Default)]
pub struct FileWordsCompletion;

impl CompletionContributor for FileWordsCompletion {
    fn dumb_aware(&self) -> bool {
        true
    }
    fn fill(
        &self,
        params: &CompletionParams<'_>,
        result: &mut CompletionResultSet,
    ) -> Result<(), Cancelled> {
        let mut checkpoint = params.cancel.checkpoint();
        let mut seen = std::collections::BTreeSet::new();
        for word in params
            .syntax
            .text()
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        {
            checkpoint.tick()?;
            if word.len() > params.prefix.len()
                && word.starts_with(params.prefix)
                && seen.insert(word)
            {
                result.add(LookupItem::new(word));
            }
        }
        Ok(())
    }
}

/// Exact-case prefix matches first.
#[derive(Debug, Clone, Copy, Default)]
pub struct PrefixWeigher;

impl CompletionWeigher for PrefixWeigher {
    fn id(&self) -> &'static str {
        "prefix"
    }
    fn weigh(&self, item: &LookupItem, params: &CompletionParams<'_>) -> i64 {
        i64::from(item.label.starts_with(params.prefix))
    }
}

/// The declaration's structure detail as documentation.
#[derive(Debug, Clone, Copy, Default)]
pub struct StructureDocumentation;

impl DocumentationProvider for StructureDocumentation {
    fn documentation(
        &self,
        snap: &Snapshot,
        target: &Target,
        cancel: &CancelToken,
    ) -> Result<Option<Documentation>, Cancelled> {
        cancel.check()?;
        let Some(syntax) = snap.syntax(target.file) else {
            return Ok(None);
        };
        let mut items = Vec::new();
        flatten(&syntax.structure(), &mut items);
        Ok(items
            .into_iter()
            .find(|item| item.name == *target.name)
            .map(|item| Documentation {
                definition: Some(format!("{}{}", item.name, item.detail.unwrap_or_default())),
                markdown: format!("`{}`", target.name),
            }))
    }
}

/// Never shows parameter info.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoParameterInfo;

impl ParameterInfoProvider for NoParameterInfo {
    fn parameter_info(
        &self,
        _snap: &Snapshot,
        _syntax: &SyntaxSnapshot,
        _offset: u32,
        cancel: &CancelToken,
    ) -> Result<Option<ParameterInfo>, Cancelled> {
        cancel.check()?;
        Ok(None)
    }
}

/// Removes trailing whitespace (minimal edits).
#[derive(Debug, Clone, Copy, Default)]
pub struct TrailingWhitespaceFormatter;

impl FormattingService for TrailingWhitespaceFormatter {
    fn id(&self) -> &'static str {
        "trailing-whitespace"
    }
    fn can_format(&self, _file: &FileMeta) -> bool {
        true
    }
    fn format(
        &self,
        syntax: &SyntaxSnapshot,
        range: Option<Range<u32>>,
        _style: &CodeStyle,
        cancel: &CancelToken,
    ) -> Result<Vec<TextEdit>, crate::assist::FormatError> {
        let text = syntax.text();
        let range = range.unwrap_or(0..text.len() as u32);
        let mut edits = Vec::new();
        let mut at = 0usize;
        for line in text.split_inclusive('\n') {
            cancel.check().map_err(|_| FormatError::Cancelled)?;
            let body = line.trim_end_matches(['\n', '\r']);
            let trimmed = body.trim_end_matches([' ', '\t']);
            let start = (at + trimmed.len()) as u32;
            let end = (at + body.len()) as u32;
            if start < end && start >= range.start && end <= range.end {
                edits.push(TextEdit::delete(start..end));
            }
            at += line.len();
        }
        Ok(edits)
    }
}

/// Every offset is in context.
#[derive(Debug, Clone, Copy, Default)]
pub struct EverywhereContext;

impl TemplateContextType for EverywhereContext {
    fn id(&self) -> &'static str {
        "OTHER"
    }
    fn is_in_context(&self, _syntax: &SyntaxSnapshot, _offset: u32) -> bool {
        true
    }
}

// ---- refactoring ------------------------------------------------------------------

/// Renames every identifier leaf with the caret identifier's text in the
/// file (one `UpdateFile`; conflicts when the new name already exists).
#[derive(Debug, Clone, Copy, Default)]
pub struct TextRename;

impl RefactoringSupport for TextRename {
    fn is_available(&self, kind: RefactoringKind, cx: &RefactorCx<'_>) -> bool {
        kind == RefactoringKind::Rename
            && cx.snap.syntax(cx.file).and_then(|s| {
                s.node_at(cx.caret)
                    .map(|n| identifiers(s.lang()).contains(n.element_type()))
            }) == Some(true)
    }
    fn prepare(
        &self,
        kind: RefactoringKind,
        cx: &RefactorCx<'_>,
    ) -> Result<RefactoringPlan, Cancelled> {
        let empty = RefactoringPlan {
            label: format!("{kind:?}"),
            usages: Vec::new(),
            conflicts: Vec::new(),
            command: ModCommand::Nothing,
        };
        let (Some(syntax), Some(new_name)) = (cx.snap.syntax(cx.file), cx.new_name) else {
            return Ok(empty);
        };
        let Some(leaf) = syntax.node_at(cx.caret) else {
            return Ok(empty);
        };
        let old = leaf.text().to_string();
        let ids = identifiers(syntax.lang());
        let mut usages = Vec::new();
        let mut conflicts = Vec::new();
        walk(syntax.root(), cx.cancel, |node| {
            if ids.contains(node.element_type()) {
                if node.text() == old {
                    usages.push(Usage {
                        file: cx.file,
                        range: node.range(),
                        kind: UsageKind::Read,
                        confidence: Confidence::Heuristic,
                    });
                } else if node.text() == new_name {
                    conflicts.push(crate::modcommand::Conflict {
                        message: format!("'{new_name}' already exists"),
                        location: Some((cx.file, node.range())),
                    });
                }
            }
        })?;
        let edits = usages
            .iter()
            .map(|usage| TextEdit::new(usage.range.clone(), new_name))
            .collect();
        Ok(RefactoringPlan {
            label: format!("Rename '{old}' to '{new_name}'"),
            usages,
            conflicts,
            command: ModCommand::UpdateFile {
                file: cx.file,
                edits,
                expected_rev: syntax.rev(),
            },
        })
    }
}

/// Records listener calls.
#[derive(Debug, Default)]
pub struct RecordingListener {
    pub events: Mutex<Vec<String>>,
}

impl RefactoringListener for RecordingListener {
    fn renamed(&self, from: &Target, to: &str) {
        self.events
            .lock()
            .push(format!("renamed {} -> {to}", from.name));
    }
    fn moved(&self, file: FileId, to: &Path) {
        self.events
            .lock()
            .push(format!("moved {file:?} -> {}", to.display()));
    }
}

/// Applies commands to texts held in memory (optimistic revisions).
#[derive(Debug, Default)]
pub struct InMemoryExecutor {
    pub files: HashMap<FileId, (FileRev, String)>,
    pub paths: HashMap<FileId, PathBuf>,
    pub created: Vec<(PathBuf, String)>,
    pub messages: Vec<String>,
}

impl ModCommandExecutor for InMemoryExecutor {
    fn execute(&mut self, command: &ModCommand) -> Result<ExecOutcome, EditError> {
        // Validate every update first: all or nothing.
        let mut staged: Vec<(FileId, FileRev, String)> = Vec::new();
        let mut outcome = ExecOutcome::default();
        let mut stack = vec![command];
        while let Some(command) = stack.pop() {
            match command {
                ModCommand::Composite(parts) => stack.extend(parts.iter().rev()),
                ModCommand::UpdateFile {
                    file,
                    edits,
                    expected_rev,
                } => {
                    let (rev, text) = staged
                        .iter()
                        .rev()
                        .find(|(f, _, _)| f == file)
                        .map(|(_, rev, text)| (*rev, text.clone()))
                        .or_else(|| self.files.get(file).cloned())
                        .ok_or(EditError::UnknownFile(*file))?;
                    if rev != *expected_rev && !staged.iter().any(|(f, _, _)| f == file) {
                        return Err(EditError::Stale {
                            file: *file,
                            expected: *expected_rev,
                            found: rev,
                        });
                    }
                    let new = apply_edits(&text, edits)?;
                    staged.push((*file, FileRev(rev.0 + 1), new));
                }
                ModCommand::CreateFile { path, text } => {
                    self.created.push((path.clone(), text.clone()))
                }
                ModCommand::MoveFile { file, to } => {
                    self.paths.insert(*file, to.clone());
                    outcome.files_changed.push(*file);
                }
                ModCommand::DeleteFile { file } => {
                    self.files.remove(file);
                    outcome.files_changed.push(*file);
                }
                ModCommand::DisplayMessage { message, .. } => {
                    outcome.messages.push(message.clone())
                }
                ModCommand::Navigate { to, .. } => outcome.navigated = Some(to.clone()),
                _ => {}
            }
        }
        for (file, rev, text) in staged {
            self.files.insert(file, (rev, text));
            if !outcome.files_changed.contains(&file) {
                outcome.files_changed.push(file);
            }
        }
        self.messages.extend(outcome.messages.iter().cloned());
        Ok(outcome)
    }
}

// ---- structural editing ---------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct LineCommenter(pub &'static str);

impl Commenter for LineCommenter {
    fn line_comment_prefix(&self) -> Option<&str> {
        Some(self.0)
    }
    fn block_comment(&self) -> Option<(&str, &str)> {
        Some(("/*", "*/"))
    }
}

/// Pairs quotes outside strings and comments.
#[derive(Debug, Clone, Copy, Default)]
pub struct PairQuotes;

impl QuoteHandler for PairQuotes {
    fn should_pair(&self, syntax: &SyntaxSnapshot, offset: u32, quote: char) -> bool {
        let sets = syntax.lang().token_set_table();
        let inside = syntax.node_at(offset).is_some_and(|leaf| {
            std::iter::once(leaf).chain(leaf.ancestors()).any(|n| {
                sets.string_literals.contains(n.element_type())
                    || sets.comments.contains(n.element_type())
            })
        });
        matches!(quote, '"' | '\'') && !inside
    }
}

/// Indent = 4 x enclosing code blocks.
#[derive(Debug, Clone, Copy, Default)]
pub struct BraceDepthIndenter;

impl Indenter for BraceDepthIndenter {
    fn indent_for_line(
        &self,
        syntax: &SyntaxSnapshot,
        line: u32,
        options: &IndentOptions,
    ) -> Option<u32> {
        let text = syntax.text();
        let start = text
            .split_inclusive('\n')
            .take(line as usize)
            .map(str::len)
            .sum::<usize>();
        let first = text[start..]
            .find(|c: char| !c.is_whitespace())
            .map(|at| start + at)?;
        let blocks = &syntax.lang().token_set_table().code_block_like;
        let leaf = syntax.node_at(first as u32)?;
        let depth = leaf
            .ancestors()
            .filter(|n| blocks.contains(n.element_type()) && n.byte_range().start < first)
            .count() as u32;
        Some(depth * options.indent_size)
    }
}

/// Inserts the closing bracket for `(`, `[`, `{`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PairingTypedHandler;

impl TypedHandler for PairingTypedHandler {
    fn before_typed(&self, syntax: &SyntaxSnapshot, caret: u32, typed: char) -> TypedDecision {
        let next = syntax
            .text()
            .get(caret as usize..)
            .and_then(|rest| rest.chars().next());
        match typed {
            '(' => TypedDecision::InsertPair(')'),
            '[' => TypedDecision::InsertPair(']'),
            '{' => TypedDecision::InsertPair('}'),
            ')' | ']' | '}' if next == Some(typed) => TypedDecision::Overtype,
            _ => TypedDecision::Default,
        }
    }
    fn after_typed(&self, _syntax: &SyntaxSnapshot, _caret: u32, _typed: char) -> Option<EditSet> {
        None
    }
}

/// Completes a statement by appending `;` at the end of the caret line.
#[derive(Debug, Clone, Copy, Default)]
pub struct SemicolonSmartEnter;

impl SmartEnterProcessor for SemicolonSmartEnter {
    fn smart_enter(&self, syntax: &SyntaxSnapshot, caret: u32) -> Option<EditSet> {
        let text = syntax.text();
        let end = text[caret as usize..]
            .find('\n')
            .map_or(text.len(), |at| caret as usize + at);
        let line = text[..end].trim_end();
        (!line.ends_with([';', '{', '}'])).then(|| EditSet {
            edits: vec![TextEdit::insert(end as u32, ";")],
            caret: Some(end as u32 + 1),
            selection: None,
        })
    }
}

/// Always falls back to Move Line.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoMoveStatement;

impl MoveStatementHandler for NoMoveStatement {
    fn move_statement(
        &self,
        _syntax: &SyntaxSnapshot,
        _range: Range<u32>,
        _up: bool,
    ) -> Option<EditSet> {
        None
    }
}

fn enclosing(items: &[StructureItem], offset: usize, out: &mut Vec<StructureItem>) {
    if let Some(item) = items
        .iter()
        .find(|item| item.range.start <= offset && offset < item.range.end)
    {
        out.push(item.clone());
        enclosing(&item.children, offset, out);
    }
}

fn line_of(text: &str, offset: usize) -> u32 {
    text[..offset.min(text.len())].matches('\n').count() as u32
}

/// Structure declarations enclosing the first visible line.
#[derive(Debug, Clone, Copy, Default)]
pub struct StructureSticky;

impl StickyScopesProvider for StructureSticky {
    fn sticky_scopes(
        &self,
        syntax: &SyntaxSnapshot,
        first_visible_line: u32,
        max: usize,
    ) -> Vec<StickyScope> {
        let text = syntax.text();
        let offset = text
            .split_inclusive('\n')
            .take(first_visible_line as usize)
            .map(str::len)
            .sum();
        let mut path = Vec::new();
        enclosing(&syntax.structure(), offset, &mut path);
        path.into_iter()
            .filter(|item| line_of(text, item.range.start) < first_visible_line)
            .take(max)
            .map(|item| StickyScope {
                header: item.range.start as u32..item.name_range.end as u32,
                start_line: line_of(text, item.range.start),
                end_line: line_of(text, item.range.end),
            })
            .collect()
    }
}

/// Structure path at the offset.
#[derive(Debug, Clone, Copy, Default)]
pub struct StructureBreadcrumbs;

impl BreadcrumbsProvider for StructureBreadcrumbs {
    fn breadcrumbs(&self, syntax: &SyntaxSnapshot, offset: u32) -> Vec<Crumb> {
        let mut path = Vec::new();
        enclosing(&syntax.structure(), offset as usize, &mut path);
        path.into_iter()
            .map(|item| Crumb {
                label: item.name,
                kind: Some(item.kind),
                range: item.range.start as u32..item.range.end as u32,
            })
            .collect()
    }
}

// ---- provider ----------------------------------------------------------------------

/// Stands in for the LSP provider: registers a completion contributor for
/// every language at `Order::Last`.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockLspProvider;

/// "Server" completion items.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockLspCompletion;

impl CompletionContributor for MockLspCompletion {
    fn dumb_aware(&self) -> bool {
        true
    }
    fn fill(
        &self,
        params: &CompletionParams<'_>,
        result: &mut CompletionResultSet,
    ) -> Result<(), Cancelled> {
        params.cancel.check()?;
        result.add(LookupItem {
            detail: Some("lsp".into()),
            ..LookupItem::new(format!("{}FromServer", params.prefix))
        });
        Ok(())
    }
}

impl AnalysisProvider for MockLspProvider {
    fn id(&self) -> &'static str {
        "mock-lsp"
    }
    fn register(&self, registry: &mut ServiceRegistry) {
        registry.register::<dyn CompletionContributor>(
            Registration::any("mock-lsp-completion")
                .order(Order::Last)
                .dumb_aware(true),
            Arc::new(MockLspCompletion),
        );
    }
}

/// Union of lookup items deduped by label, native first.
#[derive(Debug, Clone, Copy, Default)]
pub struct LabelMerger;

impl Merger<LookupItem> for LabelMerger {
    fn merge(
        &self,
        native: Vec<LookupItem>,
        lsp: Vec<LookupItem>,
        mode: ProviderMode,
    ) -> Vec<LookupItem> {
        match mode {
            ProviderMode::Native => native,
            ProviderMode::Lsp => lsp,
            ProviderMode::Auto | ProviderMode::Both => {
                merge_dedupe(native, lsp, |item| item.label.clone(), true)
            }
        }
    }
}
