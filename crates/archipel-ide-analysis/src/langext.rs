//! Language extension points (spec `docs/specs/platform/08-language-extension-points.md`,
//! IDs `PLAT-LANG-*`).
//!
//! A *language pack* contributes its parts to the application's
//! [`ExtensionRegistry`] (`archipel-ide-platform`): the language and its file types, the
//! parser definition, highlighting passes, inspections, intentions, completion, folding,
//! structure, commenter and brace matcher, reference providers, import fixes and so on. Each
//! contribution is registered on an extension point declared here (or, for the layers below
//! analysis, next to its trait: `LANGUAGE_SERVER_PROVIDER` in `archipel-ide-lsp`,
//! `TOOLCHAIN_REQUIREMENT_PROVIDER` in `archipel-ide-toolchains`).
//!
//! # How the pieces fit
//!
//! * [`LanguageExtension`] is the language-keyed wrapper (IntelliJ `LanguageExtensionPoint`):
//!   a [`Registration`] (id, language or "any", [`Order`], dumb-awareness) plus the extension
//!   object. A [`LanguagePoint`] names an extension point whose extensions are language
//!   bound. PLAT-LANG-010, PLAT-LANG-012.
//! * The query side stays the [`ServiceRegistry`] (single, all, all-with-any, base-language
//!   fallback, ordering: PLAT-LANG-010/011/014). It is a *cached view* rebuilt from the
//!   extension registry by [`registry_from_extensions`] whenever an extension point changes,
//!   so unregistering or disposing a pack removes its contributions (PLAT-LANG-005).
//! * Extensions are application level shared instances: stateless or thread safe
//!   (PLAT-LANG-013, PLAT-LANG-101). Every trait here is `Send + Sync`.
//!
//! [`ALL_POINTS`] lists every language-bound point that the bridge copies; a test registers
//! one extension per point and checks it arrives, so a new point cannot be forgotten.

use crate::assist::{
    CompletionContributor, CompletionWeigher, DocumentationProvider, FormattingService,
    ParameterInfoProvider, TemplateContextType,
};
use crate::daemon::{Annotator, DynExternalAnnotator, Pass};
use crate::editing::{
    BraceMatcher, BreadcrumbsProvider, Commenter, ExtendSelectionHandler, FoldingBuilder, Indenter,
    MoveStatementHandler, QuoteHandler, SmartEnterProcessor, StickyScopesProvider,
    StructureProvider, TypedHandler,
};
use crate::inspect::{EntryPoint, GlobalInspection, InspectionSuppressor, LocalInspection};
use crate::intention::IntentionAction;
use crate::interface::InterfaceSummarizer;
use crate::modcommand::{ChoiceRow, ModCommand};
use crate::refactor::{RefactoringListener, RefactoringSupport};
use crate::resolve::{
    ReadWriteAccessDetector, Reference, ReferenceProvider, Resolver, UsageSearcher,
};
use archipel_ide_language::{
    CompletenessTier, LanguageDef, LanguageId, Order, Registration, ServiceRegistry,
};
use archipel_ide_platform::{
    DisposableId, ExtensionError, ExtensionPointName, ExtensionRegistry, LoadingOrder,
};
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

// ---------------------------------------------------------------------------------------------
// The language-bound wrapper

/// An extension bound to a language (or to every language: [`LanguageId::ANY`]).
pub struct LanguageExtension<T: ?Sized> {
    pub registration: Registration,
    pub extension: Arc<T>,
}

/// The name of an extension point whose extensions are [`LanguageExtension`]s of `T`.
pub struct LanguagePoint<T: ?Sized + 'static> {
    point: ExtensionPointName<LanguageExtension<T>>,
    /// The spec requirement(s) this point implements, for the pack docs and the developer view.
    spec: &'static str,
}

impl<T: ?Sized + 'static> LanguagePoint<T> {
    pub const fn new(name: &'static str, spec: &'static str) -> Self {
        Self {
            point: ExtensionPointName::new(name),
            spec,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.point.name()
    }

    /// The `PLAT-LANG-*` requirement(s) behind this point.
    pub const fn spec(&self) -> &'static str {
        self.spec
    }

    pub const fn point(&self) -> ExtensionPointName<LanguageExtension<T>> {
        self.point
    }
}

impl<T: ?Sized + 'static> Clone for LanguagePoint<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + 'static> Copy for LanguagePoint<T> {}

/// The id an extension gets in the extension registry: unique per language and registration id.
pub fn extension_id(registration: &Registration) -> String {
    format!("{}:{}", registration.language, registration.id)
}

fn loading_order(registration: &Registration) -> LoadingOrder {
    let scoped = |id: &str| format!("{}:{id}", registration.language);
    match registration.order {
        Order::Default => LoadingOrder::Anywhere,
        Order::First => LoadingOrder::First,
        Order::Last => LoadingOrder::Last,
        Order::Before(id) => LoadingOrder::Before(scoped(id)),
        Order::After(id) => LoadingOrder::After(scoped(id)),
    }
}

/// Registers a language-bound extension. With a `parent` it is removed when the parent is
/// disposed (the pack's disposable). A duplicate (same language and id on the point) is an
/// error (PLAT-LANG-001 for the registry rule, PLAT-LANG-011 for the ordering).
pub fn register_language_extension<T: ?Sized + Send + Sync + 'static>(
    extensions: &ExtensionRegistry,
    point: LanguagePoint<T>,
    registration: Registration,
    extension: Arc<T>,
    parent: Option<DisposableId>,
) -> Result<(), ExtensionError> {
    extensions.register(
        point.point,
        extension_id(&registration),
        loading_order(&registration),
        Arc::new(LanguageExtension {
            registration,
            extension,
        }),
        parent,
    )
}

/// The extensions of a language-bound point in loading order, with their registrations.
pub fn language_extensions<T: ?Sized + Send + Sync + 'static>(
    extensions: &ExtensionRegistry,
    point: LanguagePoint<T>,
) -> Vec<(Registration, Arc<T>)> {
    extensions
        .extension_list(point.point)
        .into_iter()
        .map(|e| (e.registration, e.extension.clone()))
        .collect()
}

/// A point the bridge can copy into a [`ServiceRegistry`] (object safe view of
/// [`LanguagePoint`]).
pub trait BridgedPoint: Sync {
    fn name(&self) -> &'static str;
    fn spec(&self) -> &'static str;
    /// Number of registered extensions.
    fn count(&self, from: &ExtensionRegistry) -> usize;
    /// `(language, registration id)` of every registered extension, in loading order.
    fn registrations(&self, from: &ExtensionRegistry) -> Vec<Registration>;
    fn copy_into(&self, from: &ExtensionRegistry, into: &mut ServiceRegistry);
    /// Adds a change listener for the point; the listener lives until `parent` is disposed.
    fn on_change(
        &self,
        from: &ExtensionRegistry,
        parent: DisposableId,
        listener: Arc<dyn Fn() + Send + Sync>,
    );
}

impl<T: ?Sized + Send + Sync + 'static> BridgedPoint for LanguagePoint<T> {
    fn name(&self) -> &'static str {
        self.point.name()
    }

    fn spec(&self) -> &'static str {
        self.spec
    }

    fn count(&self, from: &ExtensionRegistry) -> usize {
        from.ids(self.point).len()
    }

    fn registrations(&self, from: &ExtensionRegistry) -> Vec<Registration> {
        language_extensions(from, *self)
            .into_iter()
            .map(|(registration, _)| registration)
            .collect()
    }

    fn copy_into(&self, from: &ExtensionRegistry, into: &mut ServiceRegistry) {
        for (registration, extension) in language_extensions(from, *self) {
            into.register::<T>(registration, extension);
        }
    }

    fn on_change(
        &self,
        from: &ExtensionRegistry,
        parent: DisposableId,
        listener: Arc<dyn Fn() + Send + Sync>,
    ) {
        let _ = from.add_change_listener(self.point, parent, move || listener());
    }
}

macro_rules! language_points {
    ($($(#[$meta:meta])* $name:ident: $ty:ty = $ep:literal, $spec:literal;)*) => {
        $(
            $(#[$meta])*
            pub const $name: LanguagePoint<$ty> =
                LanguagePoint::new(concat!("dev.archipel.language.", $ep), $spec);
        )*
        /// Every language-bound point the bridge copies into the [`ServiceRegistry`].
        pub const ALL_POINTS: &[&dyn BridgedPoint] = &[$(&$name),*];
    };
}

language_points! {
    // PLAT-LANG-030..033: the parser of a language (tree-sitter grammar, queries, token sets).
    PARSER_DEFINITION: dyn ParserDefinition = "parserDefinition", "PLAT-LANG-030, PLAT-LANG-033";
    // PLAT-LANG-040..042: highlighting that needs no tree is the lexer/query layer; passes
    // below are the tree and resolve based ones.
    /// A daemon pass (IntelliJ `HighlightingPass` factory): the native resolve pass of a
    /// language, an external annotator pass.
    HIGHLIGHTING_PASS: dyn Pass = "highlightingPass", "PLAT-LANG-050, PLAT-LANG-055";
    ANNOTATOR: dyn Annotator = "annotator", "PLAT-LANG-050..053";
    EXTERNAL_ANNOTATOR: dyn DynExternalAnnotator = "externalAnnotator", "PLAT-LANG-054";
    LOCAL_INSPECTION_TOOL: dyn LocalInspection = "localInspection", "PLAT-LANG-050";
    GLOBAL_INSPECTION_TOOL: dyn GlobalInspection = "globalInspection", "PLAT-LANG-050";
    INSPECTION_SUPPRESSOR: dyn InspectionSuppressor = "inspectionSuppressor", "PLAT-LANG-050";
    IMPLICIT_USAGE_PROVIDER: dyn EntryPoint = "implicitUsageProvider", "PLAT-LANG-093";
    INTENTION_ACTION: dyn IntentionAction = "intentionAction", "PLAT-LANG-050";
    COMPLETION_CONTRIBUTOR: dyn CompletionContributor = "completion.contributor", "PLAT-LANG-100";
    COMPLETION_WEIGHER: dyn CompletionWeigher = "completion.weigher", "PLAT-LANG-100";
    DOCUMENTATION_PROVIDER: dyn DocumentationProvider = "documentationProvider", "PLAT-LANG-012";
    PARAMETER_INFO_HANDLER: dyn ParameterInfoProvider = "parameterInfoHandler", "PLAT-LANG-012";
    FORMATTING_SERVICE: dyn FormattingService = "formattingService", "PLAT-LANG-078";
    LIVE_TEMPLATE_CONTEXT: dyn TemplateContextType = "liveTemplateContext", "PLAT-LANG-012";
    /// Contributed references (`PsiReferenceContributor`); merged with [`merge_references`]
    /// (PLAT-PSI-111).
    REFERENCE_CONTRIBUTOR: dyn ReferenceProvider = "referenceContributor", "PLAT-LANG-081";
    RESOLVER: dyn Resolver = "resolver", "PLAT-LANG-081";
    USAGE_SEARCHER: dyn UsageSearcher = "usageSearcher", "PLAT-LANG-080, PLAT-LANG-094";
    READ_WRITE_ACCESS_DETECTOR: dyn ReadWriteAccessDetector = "readWriteAccessDetector", "PLAT-LANG-080";
    REFACTORING_SUPPORT: dyn RefactoringSupport = "refactoringSupport", "PLAT-LANG-012";
    REFACTORING_LISTENER: dyn RefactoringListener = "refactoringListener", "PLAT-LANG-012";
    FOLDING_BUILDER: dyn FoldingBuilder = "foldingBuilder", "PLAT-LANG-060..064";
    /// `StructureViewBuilder` / `StructureViewFactory`.
    STRUCTURE_VIEW_BUILDER: dyn StructureProvider = "structureViewBuilder", "PLAT-LANG-084";
    COMMENTER: dyn Commenter = "commenter", "PLAT-LANG-070";
    BRACE_MATCHER: dyn BraceMatcher = "braceMatcher", "PLAT-LANG-071";
    QUOTE_HANDLER: dyn QuoteHandler = "quoteHandler", "PLAT-LANG-072";
    TYPED_HANDLER: dyn TypedHandler = "typedHandler", "PLAT-LANG-073";
    SMART_ENTER_PROCESSOR: dyn SmartEnterProcessor = "smartEnterProcessor", "PLAT-LANG-074";
    EXTEND_SELECTION_HANDLER: dyn ExtendSelectionHandler = "extendSelectionHandler", "PLAT-LANG-075";
    INDENTER: dyn Indenter = "indenter", "PLAT-LANG-078";
    MOVE_STATEMENT_HANDLER: dyn MoveStatementHandler = "moveStatementHandler", "PLAT-LANG-074";
    STICKY_SCOPES_PROVIDER: dyn StickyScopesProvider = "stickyScopesProvider", "PLAT-LANG-088";
    BREADCRUMBS_PROVIDER: dyn BreadcrumbsProvider = "breadcrumbsProvider", "PLAT-LANG-088";
    INTERFACE_SUMMARIZER: dyn InterfaceSummarizer = "interfaceSummarizer", "PLAT-LANG-012";
}

/// Builds the query registry from `base` (the platform's own generic services) plus every
/// language-bound extension, and the base-language relations of the declared languages.
pub fn registry_from_extensions(
    extensions: &ExtensionRegistry,
    mut base: ServiceRegistry,
) -> ServiceRegistry {
    for language in extensions.extension_list(LANGUAGE) {
        if let Some(parent) = language.base {
            base.declare_base(language.language_id(), LanguageId(parent));
        }
    }
    for point in ALL_POINTS {
        point.copy_into(extensions, &mut base);
    }
    base
}

/// Calls `listener` whenever any language-bound point, [`LANGUAGE`] or [`FILE_TYPE`] changes
/// (a pack was installed or removed), until `parent` is disposed.
pub fn on_language_extensions_changed(
    extensions: &ExtensionRegistry,
    parent: DisposableId,
    listener: impl Fn() + Send + Sync + 'static,
) {
    let listener: Arc<dyn Fn() + Send + Sync> = Arc::new(listener);
    for point in ALL_POINTS {
        point.on_change(extensions, parent, listener.clone());
    }
    let _ = extensions.add_change_listener(LANGUAGE, parent, {
        let listener = listener.clone();
        move || listener()
    });
    let _ = extensions.add_change_listener(FILE_TYPE, parent, move || listener());
}

// ---------------------------------------------------------------------------------------------
// Language and file type (PLAT-LANG-001..005, PLAT-LANG-020..022)

/// A language (IntelliJ `Language`). One per id; registering a second one with the same id is
/// rejected by the extension registry (PLAT-LANG-001).
#[derive(Debug, Clone)]
pub struct Language {
    /// Stable id (`"java"`); the key of every language-bound extension.
    pub id: &'static str,
    pub display_name: &'static str,
    /// The base language: this is a dialect of it, and its extensions are found by the
    /// dialect (PLAT-LANG-002, PLAT-LANG-014).
    pub base: Option<&'static str>,
    /// PLAT-LANG-003: delegated to the base when present.
    pub case_sensitive: bool,
    /// PLAT-LANG-105: the completeness tier per feature family.
    pub tier: CompletenessTier,
    /// PLAT-LANG-081 / PLAT-PSI-113: the language routes its reference accessor through the
    /// contributed-reference registry.
    pub contributed_references: bool,
}

impl Language {
    pub fn new(id: &'static str, display_name: &'static str) -> Self {
        Self {
            id,
            display_name,
            base: None,
            case_sensitive: true,
            tier: CompletenessTier::G0,
            contributed_references: false,
        }
    }

    /// The language of a tree-sitter definition.
    pub fn of(def: &'static LanguageDef) -> Self {
        Self {
            id: def.id,
            display_name: def.display_name(),
            base: def.base,
            case_sensitive: true,
            tier: def.tier,
            contributed_references: false,
        }
    }

    pub fn language_id(&self) -> LanguageId {
        LanguageId(self.id)
    }

    pub fn contributed_references(mut self, yes: bool) -> Self {
        self.contributed_references = yes;
        self
    }

    pub fn case_sensitive(mut self, yes: bool) -> Self {
        self.case_sensitive = yes;
        self
    }
}

/// The languages (not language-bound: they *are* the language).
pub const LANGUAGE: ExtensionPointName<Language> = ExtensionPointName::new("dev.archipel.language");

/// `is_kind_of`: `language` is `other` or a (transitive) dialect of it (PLAT-LANG-002).
pub fn is_kind_of(extensions: &ExtensionRegistry, language: &str, other: &str) -> bool {
    let mut current = language;
    for _ in 0..16 {
        if current == other {
            return true;
        }
        match extensions
            .find_by_id(LANGUAGE, current)
            .and_then(|language| language.base)
        {
            Some(base) => current = base,
            None => return false,
        }
    }
    false
}

/// A file type: how files are classified (PLAT-LANG-020/021). The type names the language
/// its files are in, so the platform can answer "which language is this file".
#[derive(Debug, Clone)]
pub struct FileType {
    /// Unique name (`"Java"`).
    pub name: &'static str,
    /// The [`Language`] id; `None` for a plain type without language (binary, text).
    pub language: Option<&'static str>,
    pub extensions: &'static [&'static str],
    pub file_names: &'static [&'static str],
}

impl FileType {
    /// The file type of a tree-sitter language definition (its extensions and exact names).
    pub fn of(def: &'static LanguageDef) -> Self {
        Self {
            name: def.name,
            language: Some(def.id),
            extensions: def.extensions,
            file_names: def.file_names,
        }
    }

    pub fn matches(&self, path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        self.file_names.contains(&name)
            || path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
    }
}

pub const FILE_TYPE: ExtensionPointName<FileType> =
    ExtensionPointName::new("dev.archipel.fileType");

/// The file type registered for `path`: exact file names first, then extensions, in loading
/// order (PLAT-LANG-021).
pub fn file_type_for_path(extensions: &ExtensionRegistry, path: &Path) -> Option<Arc<FileType>> {
    let types = extensions.extension_list(FILE_TYPE);
    let name = path.file_name().and_then(|n| n.to_str());
    types
        .iter()
        .find(|t| name.is_some_and(|n| t.file_names.contains(&n)))
        .or_else(|| types.iter().find(|t| t.matches(path)))
        .cloned()
}

/// The language id of `path` through the registered file types (PLAT-LANG-020).
pub fn language_for_path(extensions: &ExtensionRegistry, path: &Path) -> Option<LanguageId> {
    file_type_for_path(extensions, path)?
        .language
        .map(LanguageId)
}

/// Parser definition (PLAT-LANG-030): what turns text into a tree. Archipel's parsers are
/// tree-sitter grammars, so the definition hands out the language's [`LanguageDef`]: grammar,
/// highlight/fold/tag queries and token sets (PLAT-LANG-031..033).
pub trait ParserDefinition: Send + Sync {
    fn definition(&self) -> &'static LanguageDef;
}

/// The parser definition of a static [`LanguageDef`].
pub struct TreeSitterParser(pub &'static LanguageDef);

impl ParserDefinition for TreeSitterParser {
    fn definition(&self) -> &'static LanguageDef {
        self.0
    }
}

// ---------------------------------------------------------------------------------------------
// Import fixes (PLAT-LANG-050, quick fixes with a chooser)

/// The open chooser popup of an import fix (`Method to Import`, `Class to Import`).
#[derive(Debug, Clone)]
pub struct ChooserPopup {
    pub title: String,
    pub rows: Vec<ChoiceRow>,
}

/// Import fixes of a language: the quick fixes of an unresolved name that add an import. When
/// several candidates exist the fix answers [`ModCommand::ChooseFrom`]; the host shows the
/// chooser the provider keeps (state per provider, shared across threads: interior
/// mutability, PLAT-LANG-101 for the *per-file* rule does not apply to a popup).
///
/// Not language-bound: a provider lists the languages it serves.
pub trait ImportFixProvider: Send + Sync {
    /// Stable id and command prefix: the host registers `<id>.chooser`, `<id>.chooseImport`
    /// and `<id>.cancelChooser` for it.
    fn id(&self) -> &'static str;
    /// Language ids this provider serves.
    fn languages(&self) -> &'static [&'static str];
    fn open_chooser(&self, title: String, rows: Vec<ChoiceRow>);
    fn chooser(&self) -> Option<ChooserPopup>;
    /// Chooses row `index`: closes the chooser and returns what applies.
    fn choose(&self, index: usize) -> Option<ModCommand>;
    fn close_chooser(&self);
}

pub const IMPORT_FIX_PROVIDER: ExtensionPointName<dyn ImportFixProvider> =
    ExtensionPointName::new("dev.archipel.language.importFixProvider");

// ---------------------------------------------------------------------------------------------
// Contributed references: the merge rule of PLAT-PSI-111

/// References of one provider, with the provider's priority (default 0.0, higher +100, lower
/// -100; any real number in between).
pub struct PrioritizedReferences {
    pub priority: f32,
    pub references: Vec<Box<dyn Reference>>,
}

/// The merge rule for one host element (PLAT-PSI-111, PLAT-LANG-081): the non-empty results
/// are grouped by priority; **all references of the highest group are kept**; a reference of
/// a lower group is kept only if its range does not intersect any reference already kept from
/// a higher group. Within a group references are kept as they are.
pub fn merge_references(mut groups: Vec<PrioritizedReferences>) -> Vec<Box<dyn Reference>> {
    groups.retain(|group| !group.references.is_empty());
    groups.sort_by(|a, b| b.priority.total_cmp(&a.priority));
    let mut kept: Vec<Box<dyn Reference>> = Vec::new();
    let mut kept_ranges: Vec<Range<u32>> = Vec::new();
    let mut index = 0;
    while index < groups.len() {
        let priority = groups[index].priority;
        let mut end = index;
        while end < groups.len() && groups[end].priority == priority {
            end += 1;
        }
        let mut accepted: Vec<Box<dyn Reference>> = Vec::new();
        for group in &mut groups[index..end] {
            for reference in std::mem::take(&mut group.references) {
                let range = reference.range_in_node();
                let intersects = kept_ranges
                    .iter()
                    .any(|other| range.start < other.end && other.start < range.end);
                if !intersects {
                    accepted.push(reference);
                }
            }
        }
        kept_ranges.extend(accepted.iter().map(|r| r.range_in_node()));
        kept.extend(accepted);
        index = end;
    }
    kept
}

/// The contributed references of `node`: asks every provider whose element types include the
/// node and merges the results by [`merge_references`]. Only languages that opted in
/// ([`Language::contributed_references`]) route through this; for the others `opted_in` is
/// false and the result is empty (PLAT-LANG-081, PLAT-PSI-113).
pub fn contributed_references(
    providers: &[Arc<dyn ReferenceProvider>],
    node: archipel_ide_language::SyntaxNode<'_>,
    opted_in: bool,
) -> Vec<Box<dyn Reference>> {
    if !opted_in {
        return Vec::new();
    }
    let groups = providers
        .iter()
        .filter(|provider| provider.interesting().contains(node.element_type()))
        .map(|provider| {
            let mut references = Vec::new();
            provider.references(node, &mut references);
            PrioritizedReferences {
                priority: provider.priority(),
                references,
            }
        })
        .collect();
    merge_references(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::Cancelled;
    use crate::modcommand::TextEdit;
    use crate::resolve::{LookupItem, ResolveCx, ResolveResults};
    use archipel_ide_language::Anchor;
    use archipel_ide_platform::Disposer;

    struct Probe;
    impl Commenter for Probe {
        fn line_comment_prefix(&self) -> Option<&str> {
            Some("//")
        }
        fn block_comment(&self) -> Option<(&str, &str)> {
            None
        }
    }

    fn registry() -> (Disposer, ExtensionRegistry, DisposableId) {
        let disposer = Disposer::new();
        let root = disposer.new_disposable("test");
        (disposer.clone(), ExtensionRegistry::new(disposer), root)
    }

    #[test]
    fn language_extensions_reach_the_service_registry_in_order_and_leave_with_their_parent() {
        let (disposer, extensions, root) = registry();
        let pack = disposer.new_child(root, "pack").unwrap();
        let put = |id: &'static str, language: LanguageId, order: Order| {
            register_language_extension(
                &extensions,
                COMMENTER,
                Registration::new(id, language).order(order),
                Arc::new(Probe),
                Some(pack),
            )
            .unwrap();
        };
        put("generic", LanguageId::ANY, Order::Default);
        put("java", LanguageId("java"), Order::Default);
        put("first", LanguageId("java"), Order::First);
        let registry = registry_from_extensions(&extensions, ServiceRegistry::new());
        let java: Vec<_> = registry
            .entries::<dyn Commenter>(LanguageId("java"))
            .into_iter()
            .map(|(r, _)| r.id)
            .collect();
        assert_eq!(java, ["first", "java", "generic"]);
        assert_eq!(registry.count::<dyn Commenter>(), 3);
        // Duplicate ids on a point are rejected.
        assert!(
            register_language_extension(
                &extensions,
                COMMENTER,
                Registration::new("java", LanguageId("java")),
                Arc::new(Probe),
                None,
            )
            .is_err()
        );
        // Disposing the pack removes its contributions.
        disposer.dispose(pack);
        let registry = registry_from_extensions(&extensions, ServiceRegistry::new());
        assert_eq!(registry.count::<dyn Commenter>(), 0);
    }

    #[test]
    fn dialects_fall_back_to_the_base_language_through_declared_languages() {
        let (_, extensions, root) = registry();
        let mut js = Language::new("javascript", "JavaScript");
        js.case_sensitive = true;
        let mut ts = Language::new("typescript", "TypeScript");
        ts.base = Some("javascript");
        for language in [js, ts] {
            extensions
                .register(
                    LANGUAGE,
                    language.id,
                    LoadingOrder::Anywhere,
                    Arc::new(language),
                    Some(root),
                )
                .unwrap();
        }
        register_language_extension(
            &extensions,
            COMMENTER,
            Registration::new("js", LanguageId("javascript")),
            Arc::new(Probe),
            Some(root),
        )
        .unwrap();
        let registry = registry_from_extensions(&extensions, ServiceRegistry::new());
        assert!(
            registry
                .get::<dyn Commenter>(LanguageId("typescript"))
                .is_some()
        );
        assert!(is_kind_of(&extensions, "typescript", "javascript"));
        assert!(!is_kind_of(&extensions, "javascript", "typescript"));
        // A second language with the same id is rejected (PLAT-LANG-001).
        assert!(
            extensions
                .register(
                    LANGUAGE,
                    "typescript",
                    LoadingOrder::Anywhere,
                    Arc::new(Language::new("typescript", "Again")),
                    None,
                )
                .is_err()
        );
    }

    #[test]
    fn file_types_name_their_language() {
        let (_, extensions, root) = registry();
        for file_type in [
            FileType {
                name: "Java",
                language: Some("java"),
                extensions: &["java"],
                file_names: &[],
            },
            FileType {
                name: "Starlark",
                language: Some("starlark"),
                extensions: &["bzl"],
                file_names: &["BUILD"],
            },
        ] {
            extensions
                .register(
                    FILE_TYPE,
                    file_type.name,
                    LoadingOrder::Anywhere,
                    Arc::new(file_type),
                    Some(root),
                )
                .unwrap();
        }
        assert_eq!(
            language_for_path(&extensions, Path::new("src/App.JAVA")),
            Some(LanguageId("java"))
        );
        assert_eq!(
            language_for_path(&extensions, Path::new("pkg/BUILD")),
            Some(LanguageId("starlark"))
        );
        assert_eq!(language_for_path(&extensions, Path::new("a.txt")), None);
    }

    #[test]
    fn change_listeners_fire_when_a_pack_leaves() {
        let (disposer, extensions, root) = registry();
        let changes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = changes.clone();
        on_language_extensions_changed(&extensions, root, move || {
            seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        let pack = disposer.new_child(root, "pack").unwrap();
        register_language_extension(
            &extensions,
            COMMENTER,
            Registration::new("c", LanguageId("java")),
            Arc::new(Probe),
            Some(pack),
        )
        .unwrap();
        assert_eq!(changes.load(std::sync::atomic::Ordering::SeqCst), 1);
        disposer.dispose(pack);
        assert_eq!(changes.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    struct Ref(Range<u32>, &'static str, Anchor);
    impl Reference for Ref {
        fn node(&self) -> &Anchor {
            &self.2
        }
        fn range_in_node(&self) -> Range<u32> {
            self.0.clone()
        }
        fn canonical_text(&self) -> Arc<str> {
            self.1.into()
        }
        fn resolve(&self, _: &ResolveCx<'_>) -> Result<ResolveResults, Cancelled> {
            Ok(ResolveResults::default())
        }
        fn variants(&self, _: &ResolveCx<'_>) -> Result<Vec<LookupItem>, Cancelled> {
            Ok(Vec::new())
        }
        fn rename_edit(&self, _: &str) -> Option<TextEdit> {
            None
        }
        fn is_soft(&self) -> bool {
            true
        }
    }

    fn refs(priority: f32, items: &[(Range<u32>, &'static str)]) -> PrioritizedReferences {
        PrioritizedReferences {
            priority,
            references: items
                .iter()
                .map(|(range, name)| {
                    Box::new(Ref(
                        range.clone(),
                        name,
                        Anchor {
                            file: archipel_ide_language::FileId(0),
                            lang: LanguageId("x"),
                            kind: archipel_ide_language::ElementType(0),
                            range: 0..0,
                            name_hash: 0,
                            rev: archipel_ide_language::FileRev(0),
                        },
                    )) as Box<dyn Reference>
                })
                .collect(),
        }
    }

    fn names(list: &[Box<dyn Reference>]) -> Vec<String> {
        list.iter()
            .map(|r| r.canonical_text().to_string())
            .collect()
    }

    #[test]
    fn reference_merge_keeps_the_top_group_and_disjoint_lower_ones() {
        // PLAT-PSI-111 acceptance: priorities 0 and 100 on the same range: only +100 remains;
        // a priority-0 reference on a disjoint sub-range also remains.
        let merged = merge_references(vec![
            refs(0.0, &[(1..9, "low-whole"), (10..14, "low-disjoint")]),
            refs(100.0, &[(1..9, "high-a"), (2..5, "high-b")]),
            refs(
                -100.0,
                &[(12..13, "lowest-inside-low"), (20..22, "lowest-free")],
            ),
            refs(50.0, &[]),
        ]);
        // `high-a` and `high-b` overlap each other: all references of the highest group stay.
        // `lowest-inside-low` intersects the kept `low-disjoint` (10..14).
        assert_eq!(
            names(&merged),
            ["high-a", "high-b", "low-disjoint", "lowest-free"]
        );
    }
}
