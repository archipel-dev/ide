# Archipel IDE code-intelligence platform: implementation spec

Status: design spec for coding agents. Clean-room: every IntelliJ fact below was read from the
Community source at tag `idea/262.10968.63` and is restated in our own words; no code is copied. The
Rust signatures are original designs. Paths marked `IC:` are relative to
`/tmp/claude-0/-home-user-kit/6fb3db41-ee3a-5066-a8f4-de01084c378e/scratchpad/ic-build/`; paths marked
`IDE:` are relative to `/home/user/ide/` (the repo is being renamed Archipel IDE -> Archipel IDE and crates
`archipel-ide-*` -> `archipel-ide-*`; this document uses the CURRENT paths and says "ex-" where a rename
matters). Paths marked `KIT:` are relative to `/home/user/kit/`.

Reading order: Part 0 (decisions) -> Part 1 (architecture) -> Part 2 (per-subsystem catalog) ->
Part 3 (languages) -> Part 4 (work packages) -> Part 5 (archipel-kit trait) -> Appendices.

---------------------------------------------------------------------------------------------------

## Part 0. Executive summary and decisions

IntelliJ's intelligence is not "a parser plus a language server". It is a pipeline: a virtual file
system feeds a document model; a PSI tree is built per file by a language's `ParserDefinition`;
references resolve PSI to PSI through caches invalidated by modification counters; file-based and
stub indexes make global questions (usages, Go To, implementations) cheap; a daemon runs ordered
passes (syntax, annotators, inspections, line markers, inlays) with visible-range priority and
cancel-on-typing; quick fixes and intentions hang off the highlights; refactorings are
find-usages + conflict check + preview + one undoable edit. Everything is an extension point keyed
by language. IntelliJ's own LSP client (`IC:platform/lsp-impl`) is just one more provider plugged into
those same extension points. That is exactly the shape the user wants for Archipel.

Decisions (D = decision, each is binding for the work packages):

- D1. Native pipeline first. A new crate family (`archipel-ide-analysis`, `-index`, `-resolve`,
  `-daemon`, `-refactor`, `-lsp-provider`, plus language tiers) models IntelliJ's architecture. The
  existing LSP client (`IDE:crates/archipel-ide-lsp`) stays a low-level transport/client and becomes
  ONE provider behind the same extension points (section 1.10).
- D2. "PSI" = `SyntaxSnapshot` + `SyntaxNode` over tree-sitter. tree-sitter gives a lossless
  concrete syntax tree (every token is a node; whitespace is the gap between byte ranges) and
  incremental reparse. Tree-sitter nodes are not stable handles, so Archipel adds `Anchor`
  (smart-pointer equivalent) and `ElementType`/`TokenSet` tables per language (section 1.2).
- D3. Languages plug in through one `LanguageDef` object (ParserDefinition + Language + FileType +
  SyntaxHighlighterFactory analogue) that also carries a typed registry of optional services
  (commenter, brace matcher, folding, structure, resolver, inspections, ...). Services are looked up
  by language with base-language fallback and an "any language" slot, like IntelliJ's
  `LanguageExtension` (`IC:platform/core-api/src/com/intellij/lang/LanguageExtension.java`).
- D4. Readers never block writers. Instead of IntelliJ's read/write lock plus
  `ProcessCanceledException`, the UI thread owns a mutable `AnalysisHost`; background work gets an
  immutable `Snapshot` (cheap Arc/rope/tree clones) and a `CancelToken`, and returns
  `Result<_, Cancelled>` (no panics). A stale result is discarded or range-mapped (section 1.11).
- D5. Caches use dependency stamps with early cutoff, not IntelliJ's global "clear on every PSI
  change". IntelliJ's `ResolveCache` is flushed before every physical PSI change
  (`IC:platform/core-impl/src/com/intellij/psi/impl/source/resolve/ResolveCache.java`, the
  `clearCacheOnPsiChange` constructor path), and its own javadoc warns the global
  `PsiModificationTracker.MODIFICATION_COUNT` is too coarse and recommends per-language trackers
  (`IC:platform/core-api/src/com/intellij/psi/util/PsiModificationTracker.java`, `forLanguage`).
  Archipel keys caches by (file revision, per-file "interface hash" epoch) so an edit inside a
  function body does not invalidate other files' resolves (section 1.4).
- D6. Every semantic answer carries a `Confidence` (`Exact`, `Likely`, `Heuristic`) and every
  language declares a `CompletenessTier` (G0-G4, section 3.1). Inspections that need exact
  resolution (unresolved reference, unused symbol) are muted unless the language is complete enough.
  This is how a tree-sitter-only language avoids false positives.
- D7. Indexes are persistent, per-project, incremental, versioned, and have a forward map (file ->
  entries, for removal) and an inverted map (key -> files). Dumb mode is per index, not global:
  features declare which indexes they need and degrade individually (section 1.6).
- D8. Edits from quick fixes, intentions and refactorings are data: a `ModCommand` value (sealed
  enum) executed by one `ModCommandExecutor`. IntelliJ moved to this model
  (`IC:platform/analysis-api/src/com/intellij/modcommand/ModCommand.java`) because it makes previews,
  background computation and undo uniform; it fits Rust ownership even better.
- D9. Tree-sitter query files are the default implementation of generic features (folds, tags/
  structure, locals/scopes, indents, text objects, brackets, sticky scopes, TODO comments,
  highlights). A language upgrades from query-driven to hand-written Rust per feature without API
  changes because both sit behind the same trait (section 3).
- D10. Parity bookkeeping: each work package ends with parity rows updated in `docs/parity/*.md` with
  evidence, never by editing `docs/parity/dashboard.md` (AGENTS.md rule).

What Archipel IDE already has (so we do not rebuild it): incremental tree-sitter parse + highlighting
(`archipel-ide-language/src/syntax.rs:123`, `lib.rs:97`), generic folding (`folding.rs:257`), tags-based
structure (`structure.rs:159`), tree-walk Extend Selection (`selection.rs:63`), bracket matching
(`brackets.rs:28`), injections (`injection.rs:103`), a fully featured LSP client and manager
(`archipel-ide-lsp`), LSP-backed completion/hover/signature/code actions/rename/hierarchies/code vision
in `archipel-ide-core`, a budgeted in-memory symbol index (`quick_nav.rs:233`), a problems model
(`problems.rs`), live templates (`archipel-ide-templates`), rope text engine with journaled byte edits
(`archipel-ide-text/src/lib.rs:41-60`). What it does NOT have: any semantic model that is not an LSP
response, any persistent index, any daemon/pass scheduler, any inspection/intention engine, any
cache/invalidation infrastructure, any cancellation model beyond per-request LSP cancel, any
native refactoring.

---------------------------------------------------------------------------------------------------

## Part 1. Target architecture of the native pipeline

### 1.0 Crate map (current names; rename later is mechanical)

```
                         archipel-ide-ui (egui shell)  /  archipel-kit (widgets, editor model)
                                     |                         ^
        language-service adapter ----+---- implements KIT host trait (Part 5)
                                     v
 archipel-ide-core (IdeState glue, commands; thin)      archipel-ide-refactor (NEW, WP7)
      |                                              archipel-ide-lsp-provider (NEW, WP8)
      +--> archipel-ide-daemon (NEW, WP3)  passes, highlights, inspections, intentions, ModCommand
      +--> archipel-ide-resolve (NEW, WP5) references, targets, scopes, usages search, navigation
      +--> archipel-ide-index (NEW, WP2)   storage, FileBasedIndex, StubIndex, dumb mode, history store
      +--> archipel-ide-analysis (NEW, WP1) host/snapshot, cancel, stamps, cached values, executor,
      |                                 extension registry, LanguageDef, VFS facade
      +--> archipel-ide-language (EXISTS)  tree-sitter registry + generic query features (WP4)
      +--> archipel-ide-text (EXISTS)      rope, edits journal, typed ops
      +--> archipel-ide-lsp (EXISTS)       transport/client/manager  (unchanged role: low level)
 per-language tiers (WP9): archipel-ide-lang-java, -rust, -kotlin, -ts, -py, -go, -data
```

Dependency rule: the new crates must not import `egui`/`eframe` (AGENTS.md layering). Everything
UI-facing is a data model plus a `poll()`/revision pattern that the existing code already uses
(`IdeState::poll_*`, `RepaintHook`, `archipel-ide-core/src/lib.rs:241`).

### 1.1 VFS and document model

IntelliJ: `VirtualFile`/`VirtualFileSystem` (`IC:platform/core-api/src/com/intellij/openapi/vfs/`),
persistent VFS snapshot database (`IC:platform/platform-impl/src/com/intellij/openapi/vfs/newvfs/persistent/PersistentFS.java`,
`FSRecordsImpl.java`), bulk file events (`IC:platform/core-api/.../vfs/newvfs/BulkFileListener.java`),
`Document` (`IC:platform/core-api/src/com/intellij/openapi/editor/Document.java`) with the rule that
PSI is built from committed documents, not from disk. Key facts to preserve: (1) one stable identity
per file (file id) that survives rename/move; (2) documents hold unsaved text and PSI/index queries
see the document, not the disk, when a file is open; (3) external changes arrive as batched events
and reconcile with unsaved edits (conflict if both changed).

Archipel IDE today: `Project` + `FileEntry` bounded tree (`archipel-ide-project/src/lib.rs:45,115`),
`ProjectWatcher` with `FileChanges` (`archipel-ide-project/src/watcher.rs:19,63`), `Document { path, text:
TextDocument, analysis, selections, disk_hash, conflict, folds }` (`archipel-ide-core/src/lib.rs:189`),
disk sync/conflict handling (`disk_sync.rs`). Identity is `PathBuf`; there is no file id, no
snapshot, no overlay.

Archipel design (WP1):
- `FileId(u32)` interned by a `FileTable` (path <-> id; rename/move keeps the id when the watcher
  reports a rename pair, else new id). Persisted in the index store so index data survives restarts.
- `Vfs` facade (no new filesystem crate; wraps `archipel-ide-project` + watcher): `FileMeta { id, path,
  len, mtime, content_hash(lazy), language, is_open }`. Batched `VfsChange { created, deleted, moved,
  modified }` events delivered to the host, coalesced per frame.
- `DocumentOverlay`: open buffers shadow disk. `Snapshot::file_text(FileId) -> Arc<Rope-snapshot>`
  returns the buffer if open else disk contents (read lazily, size-capped, binary-sniffed like
  `archipel-ide-search` MAX_FILE_BYTES = 2 MiB, `archipel-ide-search/src/lib.rs:12`).
- The text engine stays `archipel-ide-text`; its journal (`EditJournal`, `ByteEdit`,
  `archipel-ide-text/src/lib.rs:41-60`, `take_edits` at `:295`) is the single source of truth for
  "what changed", feeding the syntax layer (already done in `Document::refresh_analysis`, `lib.rs:224`)
  and, new, `AnalysisHost::apply(Change::Edit { file, edits, new_revision })`.
- Revision numbers: `TextDocument::revision()` (`archipel-ide-text/src/lib.rs:363`) is the per-file
  `FileRev`. Never compare text; compare revisions.

### 1.2 PSI equivalent: syntax layer

IntelliJ: `PsiElement` navigation/text API (`IC:platform/core-api/src/com/intellij/psi/PsiElement.java`),
`PsiFile`, AST nodes behind it, `ParserDefinition` (lexer, parser, file node type, whitespace/comment/
string `TokenSet`s, `createElement`, `createFile`; `IC:platform/core-api/src/com/intellij/lang/ParserDefinition.java`),
`IElementType`/`TokenSet`, incremental reparse limited to "reparseable" blocks with diffing of old
and new trees so unchanged PSI keeps identity (`IC:platform/core-impl/src/com/intellij/psi/impl/BlockSupportImpl.java`:
`reparseRange`, `findReparseableNodeAndReparseIt`, `diffTrees`, `DiffLog`), smart pointers that
survive edits (`IC:platform/core-api/src/com/intellij/psi/SmartPointerManager.java`), injected
languages (`InjectedLanguageManager`).

Archipel IDE today: `SyntaxTree { spec, tree }` with incremental `apply_edits` returning changed ranges
(`archipel-ide-language/src/syntax.rs:11,123`), `LanguageAnalysis` bundling tree + highlight layers +
diagnostics (`lib.rs:43`). No typed nodes, no identities, nothing outside the editor thread.

Archipel design (WP1, crate `archipel-ide-analysis` + modules in `archipel-ide-language`):

```rust
/// Immutable parse result. Cheap to clone (Arc). The unit of "PSI file".
pub struct SyntaxSnapshot {
    pub file: FileId, pub rev: FileRev, pub lang: &'static LanguageDef,
    text: TextSnapshot,            // rope clone, O(1)
    tree: tree_sitter::Tree,       // ref-counted
    injections: Arc<[InjectedFragment]>,   // child snapshots + offset maps
    errors: Arc<[SyntaxError]>,    // ERROR/MISSING nodes, lazily
}
/// Borrowing handle; Copy. All navigation is O(1)/O(depth), no allocation.
pub struct SyntaxNode<'s> { snap: &'s SyntaxSnapshot, node: tree_sitter::Node<'s> }
impl SyntaxNode<'_> {
    fn element_type(&self) -> ElementType;       // interned per language: kind-id -> u16
    fn range(&self) -> Range<u32>; fn parent(&self)->Option<Self>; fn first_child(..); fn next_sibling(..);
    fn prev_sibling(..); fn children(..)->impl Iterator; fn named_children(..);
    fn find_leaf_at(&self, offset: u32) -> Option<Self>;           // findElementAt
    fn child_by_field(&self, field: FieldId) -> Option<Self>;      // grammar fields = free "named slots"
    fn text(&self) -> TextSlice;  fn is_error(&self)->bool; fn is_missing(&self)->bool;
    fn ancestors(&self) -> impl Iterator; fn covering(&self, range) -> Self; // getParentOfType
}
```

Element types and token sets: `ElementType(u16)` is an index into the language's
`ElementTypeTable` built from the grammar (`Language::node_kind_count`, `node_kind_for_id`,
`field_name_for_id`) so no per-language enum is required for generic features. A `TokenSet` is a
bitset over that table (`[u64; N]`); per language we define `whitespace` (usually empty because
tree-sitter has no whitespace nodes), `comments`, `string_literals`, `identifiers`, `brackets`,
`statement_like`, `declaration_like`, `code_block_like`. Typed AST wrappers for hand-optimized
languages are generated from `node-types.json` (rust-analyzer style): `trait AstNode { fn cast(n)
-> Option<Self>; fn syntax(&self) }`. This is the "createElement" hook: a language that wants typed
elements provides `fn typed_root(&self, snap) -> Box<dyn Any>`; generic code never needs it.

Anchors (smart pointers). tree-sitter node ids are only meaningful inside one tree, so persistent
references use:

```rust
pub struct Anchor { file: FileId, kind: ElementType, range: Range<u32>, name_hash: u32, rev: FileRev }
impl Anchor {
    fn new(node: SyntaxNode) -> Self;
    fn resolve(&self, snap: &SyntaxSnapshot, journal: &EditMap) -> Option<SyntaxNode>;
}
```
`EditMap` maps old offsets to new ones through the `ByteEdit` journal between `rev` and the
snapshot (positions inside a replaced span collapse). Resolution: map the range, take
`descendant_for_byte_range`, accept if kind matches and (when named) the name hash matches; else
search siblings of the same kind for the name; else `None`. Property test: random edits applied to a
file never make `resolve` return a node of another kind (tests/ pattern: `archipel-ide-language/tests/incremental.rs`).
Optimization (do not rely on it): tree-sitter reuses unchanged subtrees, so a pointer-id equality
check can short-circuit anchors for unchanged subtrees; verify with a test before depending on it.

Incremental model (BlockSupport analogue): on edit, `Tree::edit` + reparse (already in
`SyntaxTree::apply_edits`) and `Tree::changed_ranges(old)` give `ChangedRanges`, the equivalent of
a PSI tree-change event. The host publishes `SyntaxChanged { file, rev, ranges }` to subscribers
(daemon, indexes, code vision). Unlike IntelliJ's AST diffing, nothing needs to "keep PSI identity":
anchors are the only identity.

Injections (`InjectedLanguageManager`, `InjectedGeneralHighlightingPass`): exists as queries
(`injection.rs:103`, depth cap `MAX_DEPTH=4`). Promote to `InjectedFragment { host_ranges, lang,
snapshot, offset_map }` so highlight, folding, completion, inspections can recurse into fragments
through one API (`snap.fragment_at(offset)`).

### 1.3 Language plugin interface and extension registry

IntelliJ: `Language` (`IC:platform/core-api/src/com/intellij/lang/Language.java`, base-language chain),
`LanguageFileType` (`IC:platform/core-api/src/com/intellij/openapi/fileTypes/LanguageFileType.java`),
`FileTypeManager` (`IC:platform/ide-core/.../fileTypes/FileTypeManager.java`), and per-language
extension points declared as `LanguageExtension<T>` / keyed extension points in XML
(`IC:platform/platform-resources/src/META-INF/LangExtensionPoints.xml`, `LangExtensions.xml`,
`EditorExtensionPoints.xml`). The platform's own LSP plugin shows the intended compositional
pattern: it registers folding, completion, rename, hierarchy, usage search, documentation, import
optimizer, formatting service, selection handler, code vision, inlay hints and intentions with
`language=""` (any) at `order="last"` in `IC:platform/lsp-impl/resources/intellij.platform.lsp.impl.xml`.

Archipel IDE today: `LanguageSpec`/`LANGUAGES` static table = id, name, extensions, file names,
shebangs, grammar, highlight/injection/locals/folds/tags queries, capture-name overrides
(`archipel-ide-language/src/registry.rs:8,75`); `detect(path, source)` at `:447`. `locals`/`folds` fields
exist but are `None` for every language. The LSP side has its own parallel table
(`archipel-ide-lsp/src/registry.rs:37` `ServerSpec`, `:113` `LanguageServerProvider`). Two tables that
must be unified.

Archipel design (WP1): replace `LanguageSpec` by `LanguageDef` (keep `LanguageSpec` as a type alias
during migration so existing call sites compile):

```rust
pub struct LanguageDef {
    pub id: LanguageId, pub display_name: &'static str, pub base: Option<LanguageId>,   // TS -> JS, Kotlin script -> Kotlin
    pub file_types: &'static [FileTypeMatcher],      // ext, exact name, glob, shebang, first-line regex
    pub grammar: GrammarSource,                      // static tree-sitter LanguageFn (later: loadable .so/wasm)
    pub element_types: OnceLock<ElementTypeTable>,   // built from grammar on first use
    pub token_sets: TokenSetSpec,                    // by kind NAME so it is data, resolved at table build
    pub queries: LanguageQueries,                    // highlights, injections, locals, folds, tags, indents,
                                                     // brackets, textobjects, sticky, todo, doc-comments, reads/writes
    pub tier: CompletenessTier,                      // G0..G4 (section 3.1)
    pub services: fn(&mut ServiceRegistry),          // registers optional trait objects (below)
}
pub struct ServiceRegistry { /* typed slots: Commenter, BraceMatcher, QuoteHandler, FoldingBuilder,
   StructureProvider, ExtendSelectionHandler*, Annotator*, LocalInspection*, IntentionAction*,
   ReferenceProvider*, Resolver, CompletionContributor*, DocumentationProvider, ParameterInfoProvider,
   RefactoringSupport, FormattingModelBuilder, Indenter, TypedHandler, SmartEnterProcessor,
   InlayHintsProvider*, CodeVisionProvider*, LineMarkerProvider*, SpellcheckStrategy, StubBuilder,
   IndexExtension*, ReadWriteAccessDetector, EntryPoint*, ... */ }
```
Lookup: `reg.get::<dyn Commenter>(lang)` -> first of (language, base chain, "any"), or `all::<dyn
Annotator>(lang)` -> union in declared order with explicit `order: Before(id)|After(id)|First|Last`
(IntelliJ uses `order="first, after liveTemplates"`, see `LangExtensions.xml:678`). Every service
declares `dumb_aware()`; the registry filters when indexes are not ready (IntelliJ `DumbAware`,
`PossiblyDumbAware`).

Defaults: the generic services (tree-sitter query driven) are registered for the "any" language so
a new grammar added to the table gets folding, structure, brackets, extend selection, comment,
sticky scopes, TODO, locals-resolve with zero Rust code. Per-language crates (`archipel-ide-lang-*`)
register overrides. Providers from LSP register with `order=Last` so native wins on conflict.

Tree-sitter grammar loading: stay with compiled-in grammar crates (Cargo.toml lists 20); a later
package can add dynamic loading, behind `GrammarSource`. Do not design for it now.

### 1.4 Modification stamps, cached values, memoization

IntelliJ: `ModificationTracker`, `CachedValue`/`CachedValuesManager`/`CachedValueProvider.Result` with
dependencies (`IC:platform/core-api/src/com/intellij/psi/util/CachedValue.java`,
`CachedValuesManager.java`, `PsiModificationTracker.java`), `ResolveCache` keyed weakly by reference
with soft values (8 maps: physical x incomplete-code x poly), cleared before every PSI change and on
low memory (`IC:platform/core-impl/.../resolve/ResolveCache.java`), `DumbService.modificationTracker`.

Archipel design (WP1):
```rust
pub enum Dep {
    File(FileId, FileRev),            // that file's text changed
    Interface(FileId, InterfaceEpoch),// that file's declaration summary (stub hash) changed
    LanguageInterface(LanguageId, u64),// any file of a language changed its interface
    Index(IndexId, IndexStamp),       // an index received writes
    DumbMode(u64),                    // entered/left dumb mode
    Settings(u64), Never,
}
pub struct Cached<T> { value: Arc<T>, deps: SmallVec<[Dep; 4]> }
impl Db { fn cached<K: Hash, T>(&self, slot: Slot<K,T>, key: K, f: impl FnOnce(&mut DepCollector)->Result<T,Cancelled>) -> Result<Arc<T>,Cancelled>; }
```
Validation is lazy (compare stored stamps with current ones on read, no eager invalidation fan-out),
exactly like `CachedValue.hasUpToDateValue`. The early cutoff: after each reparse the file's
`InterfaceSummary` (the stub tree of declarations, 1.6) is recomputed in the background-cheap path; if
its hash is unchanged the `InterfaceEpoch` does not bump, so every dependent resolve cache stays
valid. Edits inside a method body therefore only invalidate that file's `File(rev)`-keyed data.
Cache storage: bounded LRU with byte weigher (target 64 MiB total, configurable), values behind
`Arc`, dropped on a memory-pressure hook (a periodic RSS check; Linux cgroup/`/proc`, macOS
`task_info`) mirroring IntelliJ's low-memory flush.

### 1.5 References and resolve

IntelliJ: `PsiReference` (element, range-in-element, `resolve`, canonical text, `handleElementRename`,
`bindToElement`, `isReferenceTo`, `getVariants`, `isSoft`; `IC:platform/core-api/src/com/intellij/psi/PsiReference.java`),
`PsiPolyVariantReference.multiResolve` returning `ResolveResult{element, validResult}`,
`PsiReferenceContributor`/`PsiReferenceProvider` registered per language/pattern,
scope walking with `PsiScopeProcessor.execute(element, state)` that stops when it returns false
(`IC:platform/core-api/src/com/intellij/psi/scope/PsiScopeProcessor.java`), `ResolveCache` wrappers with
`incompleteCode` flag (resolve in broken code), `GlobalSearchScope` (project, module, libraries,
union/intersection; `IC:platform/core-api/src/com/intellij/psi/search/GlobalSearchScope.java`), and
`ReferencesSearch`/`DefinitionsScopedSearch` query objects in
`IC:platform/indexing-api/src/com/intellij/psi/search/searches/`.

Archipel IDE today: none natively. Navigation = LSP `definition`/`typeDefinition`/`implementation`/
`references` (`archipel-ide-core/src/navigation.rs:27,222`, `Feature` in `archipel-ide-lsp/src/manager.rs:72`).
Highlight usages missing (IJ-ED-072, IJ-NAV-032). The tags queries already give declarations
(`structure.rs:159`); the `locals` field in the registry is the intended but unused hook.

Archipel design (WP5, crate `archipel-ide-resolve`):

```rust
/// A declaration (PsiNamedElement + NavigationItem). Persistable, cheap to clone.
pub struct Target { pub file: FileId, pub anchor: Anchor, pub name: Arc<str>, pub name_range: Range<u32>,
    pub kind: SymbolKind, pub container: Option<Arc<str>>, pub provenance: Provenance, pub confidence: Confidence }
pub enum Provenance { Native(LanguageId), Lsp(ServerKey), Library(LibraryId) }

pub trait Reference: Send + Sync {
    fn node(&self) -> &Anchor; fn range_in_node(&self) -> Range<u32>; fn canonical_text(&self) -> Arc<str>;
    fn resolve(&self, cx: &ResolveCx<'_>) -> Result<ResolveResults, Cancelled>;  // multi-resolve by default
    fn variants(&self, cx: &ResolveCx<'_>) -> Result<Vec<LookupItem>, Cancelled>;  // completion
    fn is_reference_to(&self, t: &Target, cx: &ResolveCx<'_>) -> Result<bool, Cancelled>; // default: resolve contains t
    fn rename_edit(&self, new_name: &str) -> Option<TextEdit>;                    // handleElementRename
    fn is_soft(&self) -> bool;                                                      // unresolved is not an error
}
pub trait ReferenceProvider: Send + Sync {   // PsiReferenceContributor
    fn interesting(&self) -> &TokenSet;      // visit only these element types (inspection-style optimization)
    fn references(&self, n: SyntaxNode<'_>, out: &mut Vec<Box<dyn Reference>>);
}
pub struct ResolveResults(pub SmallVec<[ResolveHit; 1]>); pub struct ResolveHit { pub target: Target, pub valid: bool }
pub trait Resolver: Send + Sync {            // per-language core resolver: the thing languages optimize
    fn resolve_at(&self, snap: &SyntaxSnapshot, leaf: SyntaxNode<'_>, cx: &ResolveCx<'_>) -> Result<ResolveResults, Cancelled>;
    fn completeness(&self) -> ResolveCompleteness;  // Complete | PartialNoUnresolvedDiagnostics
    fn scope_processor_hint(..);                    // optional: visible-name enumeration for completion
}
pub struct ResolveCx<'a> { snap: &'a Snapshot, cancel: &'a CancelToken, scope: &'a SearchScope, incomplete_code: bool }
```
`SearchScope`: `Project`, `Directory(path)`, `Files(set)`, `Module(id)`, `Libraries`, `Union`,
`Intersection`, `CurrentFile`, plus saved custom scopes (IJ-NAV-065 `search_scopes.rs` exists in core,
reuse its model). `contains(FileId)` must be O(1) via file-id bitsets; `GlobalSearchScope` in IntelliJ
also does ordering (`compare`) for "closest first" (used by completion proximity weighers and Go To).

Generic resolver ladder (all languages get L0-L2 for free; per-language crates replace L1/L2 with
exact logic and add L3):
- L0 local scope: `locals.scm` captures (`@local.scope`, `@local.definition[.kind]`, `@local.reference`).
  Walk enclosing scopes outward matching the name; honors shadowing and "definition must precede use"
  per capture predicate. Exact for locals/params/imports inside a file. (The registry already has
  `locals: Option<&str>` and `has_locals`, `registry.rs:47`.)
- L1 same-file top level: tags definitions by name (`structure()` items).
- L2 project name lookup: `SymbolNameIndex` candidates narrowed by import statements visible in the
  file (query capture `@local.import` with `@module`/`@name`), marked `Confidence::Heuristic` when more
  than one candidate or no import evidence. Never used to report "unresolved".
- L3 language resolver (module system, types, classpath): Java first (Part 3).
- L-LSP: provider adapter (section 1.10) as an alternative `Resolver`.

Reference search = `ReferencesSearch` analogue (this is the hot path for Find Usages, Rename, Safe
Delete, code-vision counts, unused detection):
1. `target.search_names()` (name plus aliases such as accessor names, `fn` vs trait-qualified).
2. `WordIndex` lookup: candidate files containing the identifier hash in the wanted contexts
   (code, comment, string, literal; IntelliJ's `IdIndex` value is a bitmask of `UsageSearchContext`,
   `IC:platform/indexing-impl/src/com/intellij/psi/impl/cache/impl/id/IdIndex.java`; the key is a hash of the word,
   `IdIndexEntry`), intersected with `scope`. Falls back to trigram/text scan if the index is
   not ready (dumb mode degrade, section 1.6).
3. In each file: scan for the identifier text with word-boundary checks, map each hit to the leaf
   node, ask `ReferenceProvider`s for references at that node, keep those where `is_reference_to`.
4. Emit `Usage { file, range, kind(Read|Write|ReadWrite|Declaration|Import|Doc|Comment|String),
   confidence }` as a cancellable stream (so the Find tool window fills progressively, matching
   `archipel-ide-search`'s streaming pattern, `archipel-ide-search/src/lib.rs:371`).
`ReadWriteAccessDetector` (`IC:platform/core-api/src/com/intellij/codeInsight/highlighting/ReadWriteAccessDetector.java`)
is the per-language hook for the read/write kind; default = "assignment target = write" from
`@local.reference.write` captures.

### 1.6 Indexing, stubs, dumb mode

IntelliJ: `FileBasedIndexExtension<K,V>` = id + indexer (file content -> key/value map) + key
descriptor + value externalizer + version + input filter + `dependsOnFileContent`
(`IC:platform/indexing-api/src/com/intellij/util/indexing/FileBasedIndexExtension.java`,
`IC:platform/util/src/com/intellij/util/indexing/IndexExtension.java`); concrete indexes: `IdIndex`,
`TodoIndex` (`IC:platform/todo/backend/src/com/intellij/psi/impl/cache/impl/todo/TodoIndex.java`, a
single-entry-per-file index whose value maps pattern to occurrence count, rebuilt when TODO
patterns change), `FilenameIndex` (`IC:platform/indexing-api/.../psi/search/FilenameIndex.java`),
`TrigramIndex` (`IC:platform/indexing-impl/.../find/ngrams/TrigramIndex.java`), `FileTypeIndex`,
`StubUpdatingIndex` (`IC:platform/indexing-impl/.../psi/stubs/StubUpdatingIndex.java`) and the
`StubIndexExtension`/`StubIndex` layer that stores per-file serialized "stub" trees of declarations
(`StubElement`, `IStubElementType`; `IC:platform/core-api/.../psi/stubs/StubElement.java`) so queries
by name return PSI without parsing the file (`IC:platform/indexing-api/.../psi/stubs/StubIndex.java`:
`processElements`, `getAllKeys`). Dumb mode: `DumbService` (`IC:platform/core-api/src/com/intellij/openapi/project/DumbService.kt`:
`isDumb`, `runWhenSmart`, `waitForSmartMode`, state flow, modification tracker), `DumbAware`/
`PossiblyDumbAware` markers on extensions, `IndexNotReadyException` thrown by index access while dumb.
Index implementation internals live in `IC:platform/indexing-impl/src/com/intellij/util/indexing/`
(`UpdatableIndex`, `FileBasedIndexEx`, `VfsAwareIndexStorage`, `IndexedHashesSupport`, content queue,
scanning/diagnostics). Up-to-date semantics: before a query IntelliJ flushes pending document
changes into the index for the queried files ("ensure up to date").

Archipel IDE today: `SymbolIndex` = per-file `Vec<SymbolEntry>` from tags outlines, cached by (size,
mtime) stamp, budgeted refresh on the UI thread, max 5,000 files / 1 MB each
(`quick_nav.rs:233-300`, `refresh_symbol_index` at `:729`); `archipel-ide-search` is a streaming text
scan with no index. PLAT-009 "Persistent indexing" and IJ-NAV-080 "Background indexing with
progress and dumb mode" are Missing.

Archipel design (WP2, crate `archipel-ide-index`):

Storage model (decision): two layers behind trait `IndexStore`.
- Per-index inverted map `key -> sorted posting list of FileId` plus forward map `FileId -> encoded
  (key, value) list`; forward map is required for incremental removal/update of a file (IntelliJ keeps
  the same pair). Values (e.g., usage-context masks, TODO counts) live in the forward map and the
  posting entry.
- Backing engine: an embedded, pure-Rust, transactional KV store (recommended: `redb`; fallback: an
  append-only segment log + `fst` key sets + rebuild-on-corruption). Requirements that decide it: crash
  safe (a corrupted index is dropped and rebuilt, never a startup failure), MVCC readers (snapshots
  see a consistent view while the writer commits batches), single-writer, mmap-friendly, no C
  dependency. Location: `<cache dir>/jetthing/index/<project-hash>/<index-id>.<version>`. Whole-store
  version stamp = sum of index versions; mismatch for one index rebuilds only that index.
- An in-memory implementation of the same trait for tests and for the open-buffer overlay.

Index extension contract:
```rust
pub trait FileIndex: Send + Sync + 'static {
    const ID: IndexId; const VERSION: u32; type Key: IndexKey; type Value: IndexValue;
    fn accepts(&self, f: &FileMeta) -> bool;                    // input filter (language, size, not binary)
    fn depends_on_content(&self) -> bool { true }               // false = name/meta only (cheap)
    fn index(&self, input: &IndexInput<'_>, out: &mut dyn FnMut(Self::Key, Self::Value)) -> Result<(), Cancelled>;
    fn dumb_aware(&self) -> bool { false }
    fn priority(&self) -> IndexPriority;                         // Names < Stubs < Words < Trigrams < Todo
}
pub struct IndexInput<'a> { pub file: &'a FileMeta, pub text: &'a str, pub syntax: Option<&'a SyntaxSnapshot> } // syntax built on demand, shared across indexes of the same file
pub trait IndexQuery { fn files_for_key(&self, k) -> Result<FileSet, IndexNotReady>; fn values(&self, k, scope) -> ..; fn all_keys(&self, prefix_matcher) -> ..; }
```
One parse per file feeds all indexes (IntelliJ builds one PSI/stub per file for the stub index; do the
same: the indexing pipeline parses once, runs every applicable `FileIndex::index`).

Built-in indexes (ship in WP2 unless noted):
| Id | Key -> Value | Used by |
|---|---|---|
| `FileName` | lowercase file name -> () (meta only, built during the scan) | Go to File, Search Everywhere files |
| `FileType` | LanguageId -> () | scopes, per-language jobs |
| `Words` | hash(identifier word) -> context mask {code, comment, string, literal} | Find Usages, Rename in comments/strings, text occurrence search |
| `Trigram` | 24-bit trigram -> () | Find in Files acceleration, wildcard/regex narrowing (IJ-NAV-064 perf) |
| `Todo` | file -> [(pattern idx, range)] single entry | TODO tool window, editor TODO highlight (IJ-TW-030) |
| `Stubs` | file -> `StubTree` (serialized declarations) | all stub indexes below |
| `Stub:ShortName` | declaration short name -> (file, stub id, kind) | Go to Class/Symbol, completion of project symbols |
| `Stub:Qualified` | qualified name -> same | imports, FQN navigation |
| `Stub:SuperTypes` | super type short name -> subtype entries | Go to Implementation, type hierarchy, overriders |
| `Stub:Members` | (container, member name) -> entries | member completion, override/implement |
| language-defined | e.g. annotations, extension receivers, test markers | per language |

`StubTree` format (generic default built from `tags.scm`; per-language `StubBuilder` can enrich):
a flat array of nodes `{ kind: SymbolKind, flags (visibility, static, abstract, test, deprecated),
name: InternedStr, range: Range<u32>, name_range: Range<u32>, parent: u32, extras: SmallVec<(ExtraKey,
InternedStr)> }` where extras carry `extends/implements/receiver/annotation/type-text/params-text`.
Unlike IntelliJ we do not rebuild typed PSI from stubs; a stub hit is a `Target` whose `anchor` is
validated against the live tree (re-resolved via `Anchor::resolve` on navigation; fall back to
name-based search in the file if the file changed since). This avoids the whole class of
"stub/AST mismatch" problems.

Interface hash = hash of the stub tree minus ranges. It drives the early-cutoff `InterfaceEpoch` (1.4).

Incremental update:
- Sources of change: (a) `VfsChange` batches from the watcher; (b) document commits of open buffers;
  (c) index version bumps; (d) settings that parameterize an index (TODO patterns -> rebuild `Todo`
  only, as `TodoIndex` does by requesting a rebuild on pattern change).
- Open buffers: unsaved text goes to an in-memory `OverlayStore` layered on the persistent store for
  those files; queries merge overlay-over-persistent; saving or closing reconciles (reindex from disk).
  This is the analogue of IntelliJ flushing document changes before an index query.
- Skip work when `content_hash` (xxh3/blake3 of bytes) is unchanged though mtime moved (touch,
  git checkout). Hash lazily, cache in the file table.
- Scheduling: indexing is a background `Job` with `Priority::Background`; order = (1) scan tree and
  build `FileName`+`FileType` (fast, makes Go to File work within a second), (2) open and recently
  opened files, (3) project source roots by language priority, (4) the rest, (5) libraries/JDK
  (stub-only, never `Words`). Batches commit every ~50 ms or 64 files; each commit bumps `IndexStamp`
  and publishes `IndexUpdated { id }` so UI lists refresh.
- Cancellation: indexing yields to interactive jobs between files (cooperative; one file = the unit
  of non-cancellable work; per-file parse capped by a node-count/time budget, oversize files skipped
  and recorded as `Skipped(reason)` for diagnostics).
- Targets (validate in WP2 benches, not IntelliJ facts): FileName for 50k files < 1 s; Stubs+Words
  for 10k typical source files < 30 s on 4 cores; steady-state RSS for index structures < 200 MiB
  (mmap'd storage excluded); typing latency unaffected (PERF-008 bench, `docs/parity/platform.md`).

Dumb mode, redesigned per index:
```rust
pub enum IndexState { Absent, Building{done:u32,total:u32}, Ready, Failed }
pub trait Db { fn index_state(&self, id: IndexId) -> IndexState; fn require<I: FileIndex>(&self) -> Result<IndexReader<I>, IndexNotReady>; }
```
`IndexNotReady` is a normal error value (IntelliJ throws `IndexNotReadyException`, a runtime
exception). Features declare `requires: &[IndexId]` and the UI state machine uses it. Behavior table
(mirrors what IntelliJ blocks while indexing, but finer):

| Works while indexing (dumb-aware) | Degrades | Blocked until ready |
|---|---|---|
| syntax highlight, folding, structure (file), brace match, extend selection, comment, typed handlers, local resolve (L0/L1), syntax errors, live templates, formatting, Go to File (FileName ready first), Find in Files (text scan), LSP-backed features | Go to Class/Symbol and Search Everywhere (show partial results with an "indexing, results may be incomplete" row), Find Usages (falls back to text-scan candidates + local resolve, flagged `Heuristic`), completion (locals + file words + LSP) | global inspections, Safe Delete, Move, project-wide rename correctness checks, "unused public symbol" |

UI: status-bar progress item and a banner in popups; `IJ-NAV-080`'s evidence requires a scene
screenshot of the indexing banner (`xtask scenes`).

Local History (IJ-ED-028, IJ-TW-034) reuses `archipel-ide-index`'s store: content-addressed blob store +
per-file revision log (see 2.30). IntelliJ reference: `IC:platform/lvcs-impl`, `lvcs-api`.

### 1.7 Daemon: highlighting passes

IntelliJ: the daemon runs `TextEditorHighlightingPass`es registered with ids and prerequisites
(`IC:platform/analysis-impl/src/com/intellij/codeHighlighting/Pass.java`: UPDATE_FOLDING=1, POPUP_HINTS=3,
UPDATE_ALL=4, LOCAL_INSPECTIONS=7, EXTERNAL_TOOLS=8, WOLF=9, LINE_MARKERS=11, INJECTED_GENERAL=14),
scheduled by `PassExecutorService` (`IC:platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/PassExecutorService.java`),
restarted after document changes with a configurable delay (default 300 ms,
`IC:platform/analysis-impl/src/com/intellij/codeInsight/daemon/DaemonCodeAnalyzerSettings.java`
`AUTOREPARSE_DELAY_DEFAULT`). `GeneralHighlightingPass`
(`IC:platform/analysis-impl/src/com/intellij/codeInsight/daemon/impl/GeneralHighlightingPass.java`) splits
PSI elements into "inside" and "outside" the visible/priority range (`Divider`), runs
`HighlightVisitor`s and `Annotator`s over them (`AnnotatorRunner`), records whether any ERROR exists,
and cancels/restarts itself if a write action is pending. Highlights are `HighlightInfo`
(range, `HighlightInfoType`, severity, description/tooltip, quick fixes) applied to the markup model
by `UpdateHighlightersUtil`/`HighlightInfoUpdater`, which recycles old highlighters to prevent
flicker. Two-phase pass contract: `collectInformation` on a background thread, `applyInformation` on
the UI thread (`ProgressableTextEditorHighlightingPass`). Severities are ordered integers:
INFORMATION=10, TEXT_ATTRIBUTES=11, SERVER PROBLEM=100, INFO=200, WEAK WARNING=200, WARNING=300,
ERROR=400 (`IC:platform/analysis-api/src/com/intellij/lang/annotation/HighlightSeverity.java`);
`SeverityRegistrar` lets users define custom ones. Other passes: `LocalInspectionsPass`
(`IC:platform/lang-impl/.../daemon/impl/LocalInspectionsPass.java`), `ExternalToolPass`,
`LineMarkersPass`, `ShowIntentionsPass`, `IdentifierHighlighterPass` (usages at caret),
`CodeFoldingPass` (`IC:platform/foldings/src/.../CodeFoldingPass.java`), `InlayHintsPass`.

Archipel IDE today: no daemon. Syntax highlights are computed in the editor path
(`LanguageAnalysis::ensure_highlighted`, `lib.rs:165`); problems = syntax errors (tree-sitter ERROR/
MISSING, `syntax_problems`, `problems.rs:181`) replaced by LSP diagnostics once the server has published
(`document_problems`, `problems.rs:214`); counters/index in `problems.rs:228`. Severity enum
`problems.rs:21` (Error, Warning, WeakWarning, Hint).

Archipel design (WP3, crate `archipel-ide-daemon`):
```rust
pub struct HighlightInfo {
    pub range: Range<u32>, pub severity: Severity,            // ordered ints as above (reuse values)
    pub kind: HighlightKind,                                   // ERROR, WARNING, UNUSED, DEPRECATED, UNRESOLVED, TYPO, SEMANTIC(key), TEXT_ATTRS(key)...
    pub message: Option<Arc<str>>, pub tooltip: Option<Arc<str>>,
    pub fixes: SmallVec<[FixDescriptor; 2]>,                   // lazily materialized (cheap descriptor, expensive action)
    pub tool: ToolId,                                          // pass or inspection id: ownership for replace-on-update
    pub group: Option<ProblemGroup>, pub problem_range_for_gutter: Option<Range<u32>>, pub file_level: bool,
}
pub trait Pass: Send + Sync {
    fn id(&self) -> PassId; fn after(&self) -> &[PassId];      // IntelliJ pass prerequisites
    fn dumb_aware(&self) -> bool; fn visible_range_first(&self) -> bool;
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled>;    // background, on a Snapshot
}
pub trait PassApply { fn apply(&self, out: PassOutput, markup: &mut HighlightMarkup); }  // UI thread, cheap
```
Built-in passes in dependency order: `Syntax` (lexical highlights, brace pairs, syntax errors; runs
synchronously on the edit thread because incremental; today's behavior), `Folding`, `StickyScopes`,
`General` (visitors + annotators over visible range first, then rest), `LocalInspections`,
`ExternalAnnotators`, `LineMarkers`, `InlayHints`, `CodeVision`, `IdentifierHighlighter` (caret-driven,
separate trigger), `ProblemFiles` (project-level "files with errors" for the tree = Wolf analogue;
exists as `ProblemIndex`, `problems.rs:108`), `Injected`.

Scheduling rules (copy the proven behavior, drop the lock dependency):
1. Trigger: `SyntaxChanged`, `IndexUpdated`, `InterfaceEpochChanged` of a dependency, settings
   change, `LspDiagnosticsPublished`, editor visible-range change (scroll) and caret change (only
   `IdentifierHighlighter`).
2. Debounce non-syntax passes by a setting (default 300 ms like IntelliJ); `Syntax` has none.
3. Visible-range first: each pass is run twice, `range=visible` then `range=rest`; results of the first
   are applied before the second starts (IntelliJ `Divider.divideInsideAndOutsideAllRoots`).
4. Cancel-on-typing: a new `SyntaxChanged` flips the `CancelToken` of all non-Syntax passes of that
   file; the markup keeps old highlights (range-shifted by the edit journal, like IntelliJ's range
   highlighters moving with edits) until a pass replaces its slice.
5. Replace semantics: each `(ToolId, range-restriction)` slice is replaced atomically; identical
   infos (same range+kind+message) are kept in place (the "recycler" idea) to avoid flicker.
6. Priorities: `Interactive > Daemon(visible) > Daemon(rest) > Background(index/batch)`; the daemon
   pool has at least one thread free for interactive work.
7. Output budget: cap infos per file (e.g., 2,000) and per tool; beyond that collapse into a
   file-level "too many problems" info (IntelliJ similarly limits highlighting in huge files via
   `HighlightingLevelManager`/file size).
8. Observability: per-pass timings and cancel counts in a debug panel (`diagnostics.rs` log exists);
   perf CI asserts budgets (below).

`HighlightMarkup` is the equivalent of `MarkupModel`: sorted range store with shifting on edit, query
by line range, per-tool replace; the editor (kit `RangeHighlight`, `KIT:crates/archipel-kit/src/editor/decorations.rs:32`)
and error stripe consume it; `Problem` (`problems.rs:47`) becomes a view over `HighlightInfo`.

Annotators (`IC:platform/analysis-api/src/com/intellij/lang/annotation/Annotator.java`: per-element
`annotate(element, holder)`, dumb-aware flag) and external annotators (`ExternalAnnotator.java`:
three phases `collectInformation(file)` on a read lock, `doAnnotate(info)` with no lock for slow work,
`apply(file, result, holder)`):
```rust
pub trait Annotator: Send + Sync { fn interesting(&self)->&TokenSet; fn dumb_aware(&self)->bool;
    fn annotate(&self, n: SyntaxNode<'_>, h: &mut AnnotationHolder, cx: &AnnotateCx<'_>) -> Result<(), Cancelled>; }
pub trait ExternalAnnotator: Send + Sync { type Info: Send; type Result: Send;
    fn collect(&self, snap: &Snapshot, file: FileId) -> Option<Self::Info>;         // fast, sync
    fn run(&self, info: Self::Info, cancel: &CancelToken) -> Option<Self::Result>;   // slow, off-thread: LSP, cargo check, javac, linters
    fn apply(&self, file: FileId, rev: FileRev, r: Self::Result, h: &mut AnnotationHolder); }
```
`AnnotationHolder` offers `new_annotation(severity, message).range(..).with_fix(..).create()`.
External results are tagged with the `FileRev` they were computed for and shifted through the edit
journal if the file changed meanwhile (same as range highlighters; if the shift would corrupt the
range, drop it).

### 1.8 Inspections and profiles

IntelliJ: `LocalInspectionTool` = `buildVisitor(holder, isOnTheFly)` or `checkFile`, `runForWholeFile`,
stable `getID`/short name used for suppression
(`IC:platform/analysis-api/src/com/intellij/codeInspection/LocalInspectionTool.java`,
`InspectionProfileEntry.java`: suppression via `InspectionSuppressor`, option containers);
`ProblemsHolder.registerProblem` -> `ProblemDescriptor` with `ProblemHighlightType` (GENERIC_ERROR_OR_WARNING,
LIKE_UNKNOWN_SYMBOL, LIKE_DEPRECATED, LIKE_UNUSED_SYMBOL, ERROR, WARNING, WEAK_WARNING, INFORMATION,
LIKE_MARKED_FOR_REMOVAL, POSSIBLE_PROBLEM, ...; `ProblemHighlightType.java`) and `LocalQuickFix`
(`QuickFix.java`, `LocalQuickFix.java`: family name, `applyFix`, preview, batch availability);
`GlobalInspectionTool` + `AnalysisScope` for "Inspect Code"
(`IC:platform/analysis-api/.../GlobalInspectionTool.java`, `reference/EntryPoint.java`);
`InspectionProfileImpl` (`IC:platform/analysis-impl/src/com/intellij/codeInspection/ex/InspectionProfileImpl.java`) = tool states (enabled, severity, per-scope overrides, options) layered
on a base profile; `LocalInspectionsPass` filters by `isToolEnabled(key, file)`; the visitor
optimization lets a tool declare which element kinds it cares about
(`IC:platform/analysis-impl/src/com/intellij/codeInsight/daemon/impl/InspectionVisitorOptimizer.java`).
Scale reference: the Java plugin registers 180 `localInspection`s in `java/java-backend/resources/META-INF/Inspections.xml`
(159 enabled by default: 3 ERROR, 125 WARNING, 4 WEAK WARNING, 26 INFORMATION, 1 TEXT ATTRIBUTES) and
680 in `InspectionGadgets.xml` (271 enabled by default: 208 WARNING, 10 WEAK, 53 INFORMATION). Java
"error" highlighting (unresolved symbol, type mismatch) is NOT an inspection: it is a
`HighlightVisitor` in the general pass (`IC:java/java-analysis-impl/src/com/intellij/codeInsight/daemon/impl/analysis/HighlightVisitorImpl.java`);
unused-symbol is `UnusedSymbolUtil.java` in the same package family.

Archipel IDE today: nothing (IJ-ED-111, -113, -114, -115 Missing/Partial; `IJ-SET-051` settings page Missing).

Archipel design (WP3):
```rust
pub trait LocalInspection: Send + Sync {
    fn id(&self) -> &'static str;                 // short name = suppression id = settings key (stable!)
    fn display_name(&self) -> &'static str; fn group_path(&self) -> &'static [&'static str]; fn description_md(&self) -> &'static str;
    fn languages(&self) -> LanguageFilter; fn default_level(&self) -> Level; fn enabled_by_default(&self) -> bool;
    fn min_tier(&self) -> CompletenessTier;       // D6: muted below this tier for the file's language
    fn interesting(&self) -> Option<&TokenSet>;   // None = whole-file; Some = dispatched per matching element
    fn dumb_aware(&self) -> bool; fn needs_indexes(&self) -> &[IndexId];
    fn options(&self) -> &'static [OptionDef];    // typed schema -> auto-generated settings UI
    fn check_element(&self, n: SyntaxNode<'_>, h: &mut ProblemsHolder, cx: &InspectCx<'_>) -> Result<(), Cancelled> { Ok(()) }
    fn check_file(&self, f: &FileCx<'_>, h: &mut ProblemsHolder) -> Result<(), Cancelled> { Ok(()) }
}
pub trait GlobalInspection { fn run(&self, scope: &AnalysisScope, g: &GlobalCx<'_>, out: &mut dyn ProblemSink) -> Result<(), Cancelled>; }
pub struct ProblemDescriptor { range, message, highlight: ProblemHighlightType, fixes: Vec<Arc<dyn QuickFix>>, on_the_fly_only: bool }
```
One element-kind dispatch table is built per file from all enabled inspections, so one tree walk
feeds every tool (the `InspectionVisitorOptimizer` idea). Profile file format (TOML, keeps
`archipel-ide-settings` conventions): `.jetthing/inspection-profiles/<name>.toml` (project) and
`<config>/inspection-profiles/` (app); `Project Default` selected in `settings.toml`; schema:
`[tools.<id>] enabled, level, [[tools.<id>.scopes]] scope="tests" enabled level, [tools.<id>.options]`.
Suppression: `noinspection <id>[, id]` comments recognized per language through an
`InspectionSuppressor` service (default: a comment on the previous line or the statement/declaration
line), language-native forms where they exist (Rust `#[allow(..)]` mapping, Java
`@SuppressWarnings("id")`); "Suppress for statement/declaration/file" quick fixes generated generically.
Batch ("Inspect Code", IJ-ED-113): `BatchRunner` iterates an `AnalysisScope` on Background threads,
parses on demand with an LRU of snapshots, runs local tools (`on_the_fly=false`) and global tools,
streams `ProblemDescriptor`s into the existing Problems model/tool window (`problems_ui.rs`).
`EntryPoint` service (`IC:platform/analysis-api/.../reference/EntryPoint.java`) marks declarations as
externally used (main functions, test methods, framework annotations, `pub` items of binary crates).

### 1.9 Quick fixes, intentions, Alt+Enter popup, ModCommand

IntelliJ: `IntentionAction` (text, `isAvailable(project, editor, file)`, `invoke`, `startInWriteAction`,
preview; `IC:platform/analysis-api/src/com/intellij/codeInsight/intention/IntentionAction.java`),
`PriorityAction.Priority` TOP/HIGH/NORMAL/LOW/BOTTOM, `IntentionManager`, the newer pure model
`ModCommandAction` + sealed `ModCommand` (update file text, create/delete/move file, navigate,
select, start template, start in-place rename, show conflicts, open options, display message,
highlight, register tab-out, composite; `IC:platform/analysis-api/src/com/intellij/modcommand/`).
The popup is assembled in `CachedIntentions` (`IC:platform/lang-impl/src/com/intellij/codeInsight/intention/impl/CachedIntentions.java`)
from five sets: error fixes, inspection fixes, intentions, gutter actions, notification actions;
the bulb icon is the error-fix icon if the error set is non-empty, else the intention bulb. Order is
`DefaultIntentionsOrderProvider`: weight = group priority + priority-action weight, sort descending
by weight then ascending by text. Group priorities: ERROR 30, INSPECTION 20, REMOTE_ERROR 10 (problems
at other offsets in the same line), NOTIFICATION 7, OTHER (plain intentions) 0, GUTTER -5, ADVERTISEMENT -10,
EMPTY_ACTION -10. Priority weights: TOP +20, HIGH +3, NORMAL 0, LOW -3, BOTTOM -20
(`IC:platform/lang-impl/.../intention/impl/IntentionGroup.java`, `DefaultIntentionsOrderProvider.java`).
Languages can replace ordering through the `intentionsOrderProvider` language extension.

Archipel IDE today: Alt+Enter popup lists LSP code actions: quick-fixes first, then "yellow" refactor/
source intentions (`archipel-ide-core/src/code_actions.rs:18,59,119,180`; UI `intentions_ui.rs`; IJ-ED-120
Partial w5, IJ-ED-121 Partial w3). Edits arrive as LSP WorkspaceEdit applied by `workspace_edit.rs`.

Archipel design (WP3, shared with WP7 for the action library):
```rust
pub trait IntentionAction: Send + Sync {
    fn family(&self) -> &'static str; fn id(&self) -> &'static str;               // stable id for enable/disable settings (IJ-SET intentions page)
    fn languages(&self) -> LanguageFilter; fn dumb_aware(&self) -> bool; fn priority(&self) -> Priority { Priority::Normal }
    fn presentation(&self, cx: &ActionCx<'_>) -> Option<Presentation>;             // None = not available here (= isAvailable + text)
    fn perform(&self, cx: &ActionCx<'_>) -> Result<ModCommand, Cancelled>;         // pure: returns a command, mutates nothing
}
pub trait QuickFix: Send + Sync { fn family(&self)->&str; fn name(&self)->String; fn fix(&self, d:&ProblemDescriptor, cx:&FixCx<'_>)->Result<ModCommand,Cancelled>;
    fn batch_ok(&self)->bool{true} }
pub enum ModCommand {                      // sealed; one executor
    Nothing, Composite(Vec<ModCommand>),
    UpdateFile { file: FileId, edits: Vec<TextEdit>, expected_rev: FileRev },     // optimistic concurrency
    CreateFile{..}, DeleteFile{..}, MoveFile{..},
    Navigate { target: Target|Range, select: Option<Range> }, StartRename { anchor: Anchor, name: String },
    StartTemplate(TemplateSession), ShowConflicts(Vec<Conflict>), Choose(Vec<(String, ModCommand)>),
    DisplayMessage{..}, OpenUrl(..), CopyToClipboard(..), Highlight(Vec<Range>), RegisterTabOut{..},
}
```
Preview for free: apply the command's `UpdateFile` edits to a copy of the buffer and diff with
`similar` (already a workspace dependency) for the existing diff UI (`diff_ui.rs`). Undo: the
executor opens one `UndoGroup` across all touched documents (see WP7 project-wide undo).
Assembling the popup (`AlertSet`): query (a) fixes attached to highlights at the caret line
(`HighlightInfo.fixes`) -> ERROR group if severity is Error else INSPECTION group, with
`REMOTE_ERROR` for fixes of problems elsewhere on the line, (b) all available `IntentionAction`s
at the caret/selection filtered by language/context, (c) LSP code actions via the provider adapter
(`quickfix` kind -> ERROR/INSPECTION group by the diagnostic severity, `refactor.*`/`source.*` ->
intentions), then dedupe by (family, text) with native preferred, sort with the formula above,
cache per (file rev, caret offset) so the bulb is instant. Submenus ("Fix all in file", "Suppress",
"Edit inspection profile", "Disable inspection") are generic wrappers generated from the originating
tool id.

### 1.10 LSP as one optional provider: coexistence, dedupe, graduation

IntelliJ's own LSP client is the precedent: it is a normal plugin whose adapters implement the
platform extension points rather than a parallel stack: highlighting pass factory, completion
contributor and weigher, parameter info handler, code vision and inlay providers, reference
providers, usage searcher, documentation target provider, formatting service, folding builder,
workspace-symbol Go To contributors, breadcrumbs collector, highlight-usages handler, call/type
hierarchy providers, select-word handler, rename handler, intentions
(`IC:platform/lsp-impl/resources/intellij.platform.lsp.impl.xml`, impl under
`IC:platform/lsp-impl/src/com/intellij/platform/lsp/impl/features/`). Registered with language "any" and
`order="last"` so a language's native support wins. Archipel copies that.

Archipel IDE today: LSP is wired straight into IdeState command handlers (`completion.rs`,
`navigation.rs`, `quick_doc.rs`, `signature_help.rs`, `code_actions.rs`, `refactor.rs`,
`hierarchy.rs`, `code_vision.rs`, `problems.rs`), each doing its own request/poll cycle through
`LspManager::request`/`take_response` (`archipel-ide-lsp/src/manager.rs:783,870`) and its own state in
`IdeState`. Capability gating is the `Feature` enum (`manager.rs:72`) plus `supports()` (`:835`).

Archipel design (WP8, crate `archipel-ide-lsp-provider`, depends on `archipel-ide-lsp` + the new crates):
- `LspProvider` = a bundle of adapters implementing the new traits, registered in the
  `ServiceRegistry` for "any language" with `order=Last`, enabled per language by the setting
  `analysis.<lang>.provider = auto | native | lsp | both` (default `auto`: use native where
  `tier >= G2` for that feature, LSP otherwise; `both` is for differential testing).
- Adapters: `LspExternalAnnotator` (diagnostics -> `HighlightInfo`, code actions -> `FixDescriptor`s,
  consuming `LspDiagnostic`, `client.rs:80`), `LspResolver` (definition/typeDefinition/implementation/
  declaration -> `ResolveResults` with `Provenance::Lsp`), `LspReferencesSearcher` (references ->
  `Usage`), `LspCompletionContributor` (+ `lsp::completion::fuzzy_match` reuse, `archipel-ide-lsp/src/completion.rs`),
  `LspDocumentationProvider` (hover), `LspParameterInfoProvider`, `LspStructureProvider`
  (documentSymbol), `LspFoldingProvider` (foldingRange), `LspSelectionHandler` (selectionRange),
  `LspInlayProvider`/`LspCodeVisionProvider` (inlayHint, codeLens: existing `code_vision.rs` logic
  moves here), `LspRenameSupport`, `LspFormattingService` (formatting, rangeFormatting, onTypeFormatting),
  `LspHierarchyProvider`, `LspWorkspaceSymbolContributor` (existing `workspace_symbol.rs`),
  `LspSemanticTokens` (new: semantic highlighting, IJ-ED-071), `LspDocumentHighlight` (highlight
  usages fallback), `LspCallbackBridge` (apply-edit requests).
- Position/offset conversion lives in one place (`LspPos`): UTF-16 columns <-> byte offsets via the
  rope (`TextDocument::lsp_position`, `char_from_lsp_position`, `archipel-ide-text/src/lib.rs:389-425`).
- Request lifecycle: every adapter call goes through `LspBroker::request(feature, file, rev, params,
  cancel)`, which (1) checks `Feature`/capability support, (2) syncs the document revision first
  (`sync_document`, `manager.rs:361`), (3) cancels the previous in-flight request of the same
  (feature, file) key, (4) drops responses whose `rev` is older than the file's current revision
  unless the adapter can range-map them. This replaces the per-feature poll loops now in `IdeState`.
  The broker runs on its own thread pool slot so a slow server never blocks the daemon.

Coexistence and dedupe rules (the merge layer is `Merger<T>` with one policy per feature):

| Feature | Policy |
|---|---|
| Highlights/diagnostics | Group by `ToolId` namespace (`native:*`, `lsp:<server>`). Same range+same normalized code/message => keep one (native `Exact` > LSP > native `Heuristic`). Native syntax-error diagnostics are hidden once the LSP has published for that revision unless the language is `>= G2` (preserves today's `document_problems` behavior, `problems.rs:214`). Differences are logged in `both` mode. |
| Resolve / Go to Declaration | Ask native first. `Exact` => done. `None`/`Heuristic` => also ask LSP; union deduped by (file, name_range); LSP results marked `Provenance::Lsp`. Disambiguation popup unchanged. |
| Implementations, type decl, super methods | Native when the language has a type index (`Stub:SuperTypes`, tier >= G3); else LSP; union if both answer. |
| Find Usages | Native index search + resolve; LSP `references` unioned (dedupe by location) while tier < G3; at G3+ LSP only on explicit "also ask language server". |
| Rename / refactor | One engine per operation, never mixed. Native only if every reference resolved `Exact` and the index is ready; otherwise LSP rename; mismatch => show both previews when `both` is on. |
| Completion | Union. Same (label, kind, insertText) => LSP item wins (richer edits), native weights merged. Native supplies keywords, locals, file words, project symbols immediately (< 20 ms) so the popup is never empty while the server thinks. Order by the weigher chain (2.8). |
| Quick fixes / intentions | Union; dedupe by normalized family+text; native `ModCommand` preferred; LSP actions keep their `WorkspaceEdit` converted to `ModCommand::UpdateFile`. |
| Hover / quick doc | Native declaration signature + doc comment first, LSP hover appended as "type info" section (or replacing if native is `Heuristic`). |
| Structure, folding, selection, brackets | Native only for `>= G1`; LSP as fallback for languages without a grammar. |
| Inlay hints, code vision | Providers are additive by kind (parameter names/types from LSP while native lacks types; usage counts native from index). Dedupe by (offset, kind). |
| Formatting | `FormattingService` chain: first service that `can_format(file)`; defaults native > external tool (rustfmt) > LSP. User-selectable per language. |

Graduation ladder (how a language moves LSP -> native): the per-language `CompletenessTier`
(section 3.1) is raised only when the conformance gate for that tier passes in CI:
1. A golden corpus per language in `crates/archipel-ide-lang-<x>/tests/corpus/` (real open-source
   projects pinned by commit, plus synthetic edge cases).
2. A `differential` test binary runs native and the real language server on the corpus for the
   feature (definition, references, diagnostics-code set, completion top-N) using the recorded server
   transcripts (so CI does not need the server; `archipel-ide-fake-lsp` replay harness exists,
   `archipel-ide-lsp/src/bin/archipel-ide-fake-lsp.rs`, plus an opt-in live run like `tests/real_servers.rs`).
3. Gate numbers (initial; tune with data): resolve agreement >= 98% where native claims `Exact`,
   0 false-positive "unresolved" on the corpus, usage-search recall >= 99% vs the server, P95 latency
   budgets (1.11). Native-only misses are acceptable when native returns `None` (falls back to LSP);
   wrong answers are not.
4. Promotion flips the default `analysis.<lang>.provider` for that feature only; LSP remains for the
   rest. Demotion is a one-line registry change (kill switch via settings).

### 1.11 Threading, cancellation, snapshots, budgets

IntelliJ model: single EDT for UI and writes, read actions on any thread, write actions wait for
readers to stop, readers poll `ProgressManager.checkCanceled()` and die with
`ProcessCanceledException`; `NonBlockingReadAction` restarts background computations after writes
(`IC:platform/core-api/src/com/intellij/openapi/application/NonBlockingReadAction.java`, `ReadAction.java`);
`DaemonProgressIndicator` per daemon run; `CoreProgressManager`
(`IC:platform/core-impl/src/com/intellij/openapi/progress/impl/CoreProgressManager.java`).

Archipel model (WP1 owns `archipel-ide-analysis::{host, job, cancel, executor}`):
- `AnalysisHost` (UI thread, `&mut`): `apply(Change)` is the only mutator of the analysis world:
  `Edit`, `FileCreated/Deleted/Moved`, `SettingsChanged`, `IndexCommit`. It bumps stamps, republishes
  events, and revokes: every running `Job` registered with `Revoke::OnAnyChangeOf(Scope)` gets its
  `CancelToken` set (`AtomicBool`/generation compare).
- `Snapshot` = `Arc<WorldView>`: file table, per-open-file `SyntaxSnapshot` (lazy build for others,
  LRU of closed-file trees, cap by node count), index readers (MVCC read txns), settings, stamps. Taking
  one is O(1). Holding one never blocks `apply`.
- `Job<T>`: `Executor::spawn(Priority, Revoke, move |snap, cancel| -> Result<T, Cancelled>)` and a
  completion callback delivered on the UI thread through a channel polled once per frame (the existing
  `poll_*`/`RepaintHook` pattern, `lib.rs:309`). Coalescing key: spawning a job with the same key
  cancels the old one (NonBlockingReadAction `coalesceBy`). `expire_when(snapshot_rev changed)`.
- Cancellation points: `cx.check()?` inside tree walks (every ~256 nodes), between files in index and
  search loops, and before each LSP await. Cancel is cooperative; no `catch_unwind`; no panics on user
  input (AGENTS.md). `Cancelled` must never be converted into a user-visible error.
- Result freshness: `Job` results carry `(file, rev)`. Applying a result to a newer revision is
  allowed only for range-mappable outputs (highlights, inlays) via `EditMap`; navigation targets are
  re-resolved through `Anchor`.
- Thread pool: `std::thread` based, N = max(2, cores-1) workers with three priority queues
  (Interactive, Daemon, Background) and one dedicated interactive lane so completion/hover/typed
  handlers never queue behind indexing. No async runtime dependency; parking_lot is already in the
  workspace. tree-sitter `Parser` instances come from a thread-local pool (they are `Send`, not
  `Sync`); `Query` objects are `static OnceLock`s (already done in `folding.rs:folds_query`).
- Typed handlers (Enter, `}`, quotes) need an up-to-date tree synchronously: they call
  `host.sync_syntax(file)` which reparses incrementally on the UI thread (IntelliJ commits the
  document before such handlers). Budget < 2 ms for typical edits; the host falls back to
  "text-only" handling when a reparse would exceed 8 ms.

Budgets (targets to be enforced by benches and CI; not IntelliJ facts):
| Operation | Budget |
|---|---|
| keystroke -> new frame (existing PERF-008) | < 16 ms on a 50k-line file |
| incremental reparse + changed-range highlight | p95 < 2 ms |
| initial highlight of visible range, 100 KB file | < 50 ms (already lazy: `INITIAL_HIGHLIGHT_BYTES`, `lib.rs:33`) |
| syntax-only problems after edit | < 5 ms |
| daemon visible-range results after idle (300 ms debounce) | p95 < 150 ms |
| daemon full-file inspections, 5k-line file | p95 < 2 s, cancellable |
| completion popup first items (native) | p95 < 50 ms; LSP items merged when they arrive |
| Go to Declaration (native Exact) | p95 < 30 ms warm, < 200 ms cold |
| Find Usages first batch | < 150 ms, streaming |
| Go to Class/Symbol keystroke | p95 < 30 ms over 100k symbols |
| memory: syntax trees | open files + LRU 64 closed files; hard cap 256 MB trees |
| memory: caches | 64 MB resolve/cached-value LRU; index RSS < 200 MB |

---------------------------------------------------------------------------------------------------

## Part 2. Subsystem catalog: IntelliJ source, Archipel IDE today, Archipel design

Legend for the "Source of truth" line: NATIVE-Q = tree-sitter query driven, language-agnostic;
NATIVE-L = hand-written per language; INDEX = needs a Archipel IDE index; LSP = optional provider.
Parity IDs are in docs/parity/*.md (weights in parentheses).

### 2.1 VFS and document model
See 1.1. Parity: PLAT-002/003/021/022 (exist), IJ-ED-144 line separators/encoding (2, Missing;
`archipel-ide-text/src/format.rs:36 detect_line_separator` exists, preserve on save is the gap),
IJ-ED-143 indentation detection (2, `format.rs:79 detect_indent` exists, wire to editor/status).
Source of truth: NATIVE (Rust).

### 2.2 PSI, ParserDefinition, FileType, Language
See 1.2-1.3. IJ also has `FileViewProvider` for multi-root/template languages: Archipel equivalent is
`injections` + `LanguageDef.base`. Parity: PLAT-011 (Partial), PLAT-012, IJ-ED-070/074.

### 2.3 References, resolve, scopes
See 1.5. Parity: IJ-NAV-020 (5, Partial), IJ-ED-071 (semantic highlighting, 2, Missing), IJ-JAVA-011/013.

### 2.4 Stubs and indexes, dumb mode, incremental reindex
See 1.6. Parity: IJ-NAV-080 (5, Missing), PLAT-009 (Missing), IJ-NAV-081 (shared indexes, 1; out of
scope; document "possible later via exported index snapshots").

### 2.5 Daemon passes, annotators, inspections, severities, profiles
See 1.7-1.8. Parity: IJ-ED-110 (5, Partial), IJ-ED-111 (3, Partial), IJ-ED-112 (Implemented),
IJ-ED-113/114/115 (Missing), IJ-ED-023 error stripe/inspection widget (2, Missing; kit has stripe
support: `KIT:crates/archipel-kit/src/editor/decorations.rs:14-60` `HighlightSeverity`,
`RangeHighlight`), IJ-ED-106 error description tooltip (2, Missing), IJ-SET-051 profiles page (Missing),
LSP-001 (Partial).

### 2.6 Quick fixes and intentions
See 1.9. Parity: IJ-ED-120 (5), IJ-ED-121 (3), IJ-ED-122/123/124/125/126, IJ-JAVA-016.
Starter native intention library (own wording, tree-sitter based so it works for every language
that has the needed node kinds): flip binary operator operands, split/join declaration and
assignment, invert if/else, swap if/else-if, toggle string quotes/escape, convert string
concatenation to template/format, add/remove braces around a single-statement body, expand/collapse
`else if` chains, convert `for` <-> iterator forms (per language), suppress inspection (generic),
"Add to dictionary" (spelling). Language-specific libraries live in `archipel-ide-lang-*`.

### 2.7 Completion (platform side)
IntelliJ: `CompletionContributor` registered per language/pattern, each filling a `CompletionResultSet`
(`IC:platform/analysis-api/src/com/intellij/codeInsight/completion/CompletionContributor.java`); results are
`LookupElement`s (presentation, insert handler, lookup strings, object, auto-complete flags;
`IC:platform/analysis-api/.../lookup/LookupElement.java`) decorated with priority via
`PrioritizedLookupElement`; the `CompletionSorter` orders them by a chain of weighers
(`CompletionSorter.java`, `LookupElementWeigher.java`, `CompletionWeigher.java`). Registered weighers
(`IC:platform/analysis-impl/resources/META-INF/AnalysisImpl.xml` lines 87-91 and
`IC:platform/platform-resources/src/META-INF/LangExtensions.xml` lines 644-662): first `priority`, then
`prefix` (prefix-matching quality), then the stats weigher (usage statistics), explicit proximity,
proximity (distance classes: opened-in-editor, same-directory, same source root, same module,
in-resolve-scope, directory type, sdk/library) and last `grouping`. Platform contributors with
language "any": `DefaultCompletionContributor`, `WordCompletionContributor` (words from the file when
nothing else offers), `FilePathCompletionContributor`, `LiveTemplateCompletionContributor`, reference
variants contributor (ordered `last`), command completion. Session behavior in
`IC:platform/lang-impl/src/com/intellij/codeInsight/completion/CompletionProgressIndicator.java` and
`LookupArranger`. Typed/auto-popup behavior is configured in code-insight settings.

Archipel IDE today: LSP-driven `CompletionSession` with camel-hump matching and ranking, snippets with
tab stops, usage stats, auto-import edits, docs panel (`archipel-ide-core/src/completion.rs:23-546`,
`archipel-ide-lsp/src/completion.rs`, `snippet.rs`, `completion_stats.rs`, popup in `completion_ui.rs`);
parity IJ-ED-080 (5, Partial), IJ-ED-081 auto popup while typing (5, Missing in parity table but the
roadmap lists "auto-popup" done: verify and correct the row), IJ-ED-084 auto-import (3, Missing),
PLAT-420 (Partial).

Archipel design (WP6, crate `archipel-ide-completion`; session UI state stays in core/ui):
```rust
pub trait CompletionContributor: Send + Sync {
    fn languages(&self) -> LanguageFilter; fn dumb_aware(&self) -> bool;
    fn complete(&self, p: &CompletionParams<'_>, out: &mut ResultSet) -> Result<(), Cancelled>;
}
pub struct CompletionParams<'a> { snap: &'a Snapshot, file: FileId, offset: u32, original_leaf: Option<SyntaxNode<'a>>,
    position: SyntaxNode<'a>,   // leaf in a copy of the file with a dummy identifier inserted at the caret (IntelliJ's trick: parse code with a placeholder so the tree is valid)
    kind: CompletionKind /*Basic|Smart|SecondBasic*/, invocation: u32, auto_popup: bool, prefix: Prefix }
pub struct LookupItem { pub label: Arc<str>, pub lookups: SmallVec<[Arc<str>;2]>, pub kind: SymbolKind, pub tail: Option<..>, pub type_text: Option<..>,
    pub insert: InsertAction /*plain | snippet | edits | command*/, pub weights: Weights, pub target: Option<Target>, pub deprecated: bool, pub provenance: Provenance }
pub trait CompletionWeigher { fn id(&self)->&'static str; fn weigh(&self, it:&LookupItem, cx:&WeighCx)->Weight; }  // chain order: priority, prefix, stats, explicit-proximity, proximity, grouping
```
Dummy-identifier trick (reproduce the proven approach): to complete in broken code the engine
parses `text[..offset] + MARKER + text[offset..]` incrementally from the live tree (apply the edit to
a clone of the tree: tree-sitter makes this cheap) and the contributor navigates from the marker
leaf. Never mutate the live buffer. Native baseline contributors, all tier-independent: keywords
(from grammar anonymous alphabetic token kinds filtered by a per-language keyword-context query),
locals in scope (`locals.scm` L0), same-file declarations, in-file words, project symbols via
`Stub:ShortName` with import-insertion `ModCommand` (language hook), file paths in strings,
live-template abbreviations and postfix templates (`archipel-ide-templates`). LSP contributor adds the
server's items. Matcher: reuse camel-hump `fuzzy_match`/`match_class` (`archipel-ide-lsp/src/completion.rs`);
move it to `archipel-ide-completion` and re-export from `archipel-ide-lsp` to avoid dependency inversion.
Stats weigher: existing usage-statistics store (`completion_stats.rs`). Auto-popup typed-handler hook
in WP4's typed handler chain; `Ctrl+Space` x2 second basic (IJ-ED-083) = `invocation=2` (include
statics/unimported through `Stub:*`). Smart completion (IJ-ED-082) needs expected types: language
tier G3+ only.

### 2.8 Find usages and highlight usages
IntelliJ: `FindUsagesHandlerFactory` (EP `findUsagesHandlerFactory`) -> `FindUsagesHandler`
(primary/secondary elements, options, `processElementUsages`;
`IC:platform/lang-impl/src/com/intellij/find/findUsages/FindUsagesHandler.java`, `FindUsagesHandlerFactory.java`,
which also has an `OperationMode.HIGHLIGHT_USAGES`), `UsageInfo` with smart pointers and
`isNonCodeUsage` (`IC:platform/core-api/src/com/intellij/usageView/UsageInfo.java`), `UsageViewManager`
(`IC:platform/usageView/src/com/intellij/usages/UsageViewManager.java`) with grouping rules (file, module,
package, usage type, read/write), search via `ReferencesSearch` + `UsageSearchContext`
(`IC:platform/indexing-api/.../psi/search/searches/ReferencesSearch.java`). Highlight usages in file:
`HighlightUsagesHandlerBase` with `myReadUsages`/`myWriteUsages` and `IdentifierHighlighterPass` for the
automatic caret-driven version (`IC:platform/lang-impl/src/com/intellij/codeInsight/highlighting/HighlightUsagesHandlerBase.java`,
`.../daemon/impl/IdentifierHighlighterPass.java`); handler factories for keywords (exit points of a
method, loop break/continue, if/else) are separate `HighlightUsagesHandlerFactory` registrations.

Archipel IDE today: LSP `references` into the Find tool window and usage popup
(`navigation.rs`, IJ-NAV-030/031 Partial); highlight usages Missing (IJ-ED-072 w3, IJ-NAV-032 w2);
grouping Missing (IJ-NAV-033).

Archipel design (WP5): `UsageSearch::find(target, options, scope, sink)` per 1.5; options =
{include comments/strings, search overriders/implementations, search in libraries, read/write filter}.
Hooks: `FindUsagesHandler` trait (primary/secondary targets, e.g., a Java field + getter/setter,
an enum constant's name in a `match`, a Rust item re-exported by `pub use`), `UsageGrouping`
rules as data (file, directory, language-defined container, usage kind, read/write) feeding the
Find/Usage popup UI. Highlight usages: `IdentifierHighlighterPass` equivalent in the daemon (caret
move, 150-200 ms debounce is a setting to tune), file-local: resolve leaf at caret -> `Target` ->
local scan with `is_reference_to`; read/write via `ReadWriteAccessDetector`; fallback when unresolved
(tier G0/G1): same-name identifiers within the same L0 scope from `locals.scm`, else same-name
identifiers in the file (marked non-strict). Keyword handlers: `HighlightUsagesHandlerFactory` trait,
query-driven default for "matching keyword group" (`@exit.return`, `@loop.break` captures).
Source of truth: NATIVE-Q for G0/G1 fallback, INDEX (`Words`) + NATIVE-L resolver for exact, LSP
`documentHighlight`/`references` as provider.

### 2.9 Go to declaration, implementation, type, super
IntelliJ: `GotoDeclarationHandler` EP and `GotoDeclarationOrUsageHandler2`
(`IC:platform/analysis-api/.../navigation/actions/GotoDeclarationHandler.java`,
`IC:platform/lang-impl/.../navigation/actions/GotoDeclarationOrUsageHandler2.kt`: when the caret is on a
declaration the action shows usages instead), `TypeDeclarationProvider`,
`GotoImplementationHandler`/`ImplementationSearcher` (`IC:platform/lang-impl/.../navigation/`),
`DefinitionsScopedSearch` for subtypes/overriders, `NavigationItem`/`Navigatable` as the target type.
Archipel IDE today: LSP definition/typeDefinition/implementation + super method via typeHierarchy
(`navigation.rs`, `hierarchy.rs:372`), IJ-NAV-020/021/022/023.
Archipel design (WP5): `NavigationService::declarations(file, offset)`:
`GotoDeclarationHandler`s (plugin hooks, e.g., string literal path -> file, import path -> module
file, build-script references) -> reference at offset -> `Resolver` -> merge with LSP (1.10). On a
declaration: usages (`Find Usages` popup). Implementation: `Stub:SuperTypes` lookup by the target's
short name, validated by resolving each candidate's super-type reference to the target (tier G3+),
else LSP. Type declaration: language hook `type_of(leaf) -> Option<Target>` (G3+), else LSP.
Super method: native from `Stub:Members` + `SuperTypes` (G3+), else LSP typeHierarchy (today's path).

### 2.10 Symbol/class/file indexes, Go To, Search Everywhere
IntelliJ: `ChooseByNameContributor` EPs for class/symbol/file (names list + items by name with the
typed pattern; `IC:platform/lang-api/src/com/intellij/navigation/ChooseByNameContributor.java`),
`GotoClassContributor` adds qualified name/separator/kind labels, `ChooseByNameModel`/`ChooseByNameRegistry`
(`IC:platform/lang-api/.../ide/util/gotoByName/ChooseByNameModel.java`), Search Everywhere contributors
declared in `IC:platform/platform-resources/src/META-INF/SearchEverywhereCodeInsightContributors.xml`;
names come from stub/file indexes, matching is camel-hump/minuscule matching, items are
`NavigationItem`s with presentation. Archipel IDE today: `quick_nav.rs` (rank/rank_strict camel-hump
matcher `:90-130`, `SymbolIndex` `:233`, tabs All/Classes/Files/Symbols/Actions/Text `:25-80`, workspace
symbols over LSP `workspace_symbol.rs`), parity IJ-NAV-001..006.
Archipel design (WP2 data + WP5/WP6 UI glue): replace `SymbolIndex` internals with
`Stub:ShortName`/`FileName` readers; keep the ranking functions. `GotoContributor` trait mirrors the
EP: `names(scope, include_libs) -> NameStream`, `items(name, pattern, scope) -> Vec<NavItem>`; matching
done once centrally by a `Matcher` over the name stream (IntelliJ's design: ask contributors for ALL
names, filter with the matcher, then expand matches); cache names per (index stamp). LSP workspace
symbols adapt as another contributor. Progressive rendering: results stream into the popup within
frames. Type-tab "Classes" = kinds Class/Interface/Enum/Struct/Trait/Type; "Symbols" = rest. Add
`file:line:col` suffix parsing (exists), `Ctrl+Click` target presentation. Path-aware ranking for equal
names (closest to current file first, `SearchScope::compare`).

### 2.11 Structure view and File Structure popup
IntelliJ: `StructureViewModel`/`StructureViewTreeElement`/`TreeBasedStructureViewBuilder`
(`IC:platform/editor-ui-api/src/com/intellij/ide/structureView/`), sorters, filters (fields, inherited,
public only...), groupers, `ExpandInfoProvider`, `FileEditorPositionListener` (select the element at the
caret), `getCurrentEditorElement`. Archipel IDE today: `StructureItem` tree from tags (`structure.rs:57,159`),
`path_at` for breadcrumbs (`structure.rs:239`), `structure_view.rs` UI (395 lines), File Structure
popup with speed search (IJ-NAV-040 Partial, IJ-TW-020), `SymbolKind` (`structure.rs:12`).
Archipel design (WP4): `StructureProvider::structure(snap) -> Vec<StructureNode>` with node
= {name, `SymbolKind`, detail (signature), modifiers (visibility/static/abstract/test/deprecated), ranges,
children, `inherited: bool` for supertypes' members (G3+)}. Default: tags query with new optional
captures `@modifier.*` and `@detail`. Sorters/filters are data keyed by `SymbolKind` + modifiers so the UI
needs no language code. Anonymous classes/lambdas: capture kind `Other` filtered by a "show anonymous"
toggle. LSP `documentSymbol` as fallback provider. Parity: IJ-NAV-040 (3), IJ-TW-020, PLAT-037.

### 2.12 Folding
IntelliJ: `FoldingBuilder`/`FoldingBuilderEx.buildFoldRegions(root, document, quick)` returning
`FoldingDescriptor`s (range, placeholder, group, dependencies, collapsed-by-default,
never-expands flag, can-be-removed-when-collapsed, gutter mark for single line;
`IC:platform/core-api/src/com/intellij/lang/folding/`), `CustomFoldingBuilder` for `region/endregion`
comments, `CodeFoldingPass` (`IC:platform/foldings/`) which runs a quick pass first then the full one.
Archipel IDE today: `fold_ranges` (`folding.rs:257`) generic bracket/tag/comment/import-run regions,
`FoldKind { Block, Comment, Imports, OneLineBody }`, `FoldState` with edit shifting and persisted
collapse state (`archipel-ide-core/src/folds.rs:12-241`), commands collapse/expand/all/selection
(IJ-ED-060 Implemented, 061/062 Partial). `LanguageSpec.folds` hook unused.
Archipel design (WP4): query captures `@fold`, `@fold.imports`, `@fold.comment`, `@fold.doc`,
`@fold.region` with `#set! placeholder "..."` and `#set! collapsed` metadata; custom region comments
generic (`region`/`endregion` markers from `LanguageDef`). Add `FoldingGroup` (collapse together, e.g.,
a getter's signature and body), dependency invalidation, "collapse by default" per `FoldKind` mapped to
settings (IJ settings Editor > General > Code Folding), keep the "quick" pass = cached last result
shifted by edits. Gutter fold markers remain the kit's concern (`FoldingModel`, `KIT` model/folding.rs).

### 2.13 Extend / shrink selection
IntelliJ: `SelectWordHandler` runs registered `ExtendWordSelectionHandler`s (`canSelect(element)`,
`select(element, text, cursor, editor) -> ranges`;
`IC:platform/lang-api/.../editorActions/ExtendWordSelectionHandler.java`, EP
`extendWordSelectionHandler`), base implementations in
`IC:platform/lang-impl/.../editorActions/wordSelection/` (`WordSelectioner`, `LineCommentSelectioner`,
`BlockCommentSelectioner`, `NaturalLanguageTextSelectioner`, `PlainTextLineSelectioner`,
`InjectedFileReferenceSelectioner`). Each step picks the smallest range strictly larger than the
current selection among all handlers' ranges for the element and its parents.
Archipel IDE today: Implemented (IJ-ED-039) via `selection::expand` (`selection.rs:63`) with shrink history
in `Document.selection_history` (`lib.rs:199`). Gap: language-specific steps (statement lists, argument
lists one-by-one, method chains, string escape/segments, camel-hump words, XML tag content/attrs, doc
comment tags).
Archipel design (WP4): trait `ExtendSelectionHandler { fn can_select(&self, n:&SyntaxNode)->bool;
fn ranges(&self, n:&SyntaxNode, text:&TextSlice, caret:u32)->SmallVec<[Range<u32>;4]>; }` with the
current tree walk becoming the default "any" handler; add `@select.group` query captures for list-like
nodes (arguments, parameters, array elements, statements in a block, match arms) so sibling ranges
(first..k) are offered. The kit consumes it through the host trait (Part 5).

### 2.14 Brace matching, quote handling, typed handlers, smart enter
IntelliJ: `PairedBraceMatcher` (pairs with structural flags, `isPairedBracesAllowedBeforeType`,
`getCodeConstructStart`; `IC:platform/analysis-api/src/com/intellij/lang/PairedBraceMatcher.java`),
`BraceMatchingUtil` (`IC:platform/lang-impl/.../highlighting/BraceMatchingUtil.java`),
`TypedHandlerDelegate` (before/after char typed hooks;
`IC:platform/lang-api/.../editorActions/TypedHandlerDelegate.java`), `QuoteHandler`,
`BackspaceHandlerDelegate`, `EnterHandlerDelegate` with result values Default/Continue/
DefaultForceIndent/DefaultSkipIndent/Stop (`IC:platform/lang-impl/.../editorActions/enter/EnterHandlerDelegate.java`),
`SmartEnterProcessor` (Complete Current Statement;
`IC:platform/lang-api/.../editorActions/smartEnter/SmartEnterProcessor.java`), `LineIndentProvider`
(`IC:platform/code-style-api/.../codeStyle/lineIndent/LineIndentProvider.java`).
Archipel IDE today: bracket match from the tree (`brackets.rs:28`), plain-text pairing/overtype/
backspace-pair/Enter indent in `archipel-ide-text/src/ops.rs:469-650` (options `PairOptions`; the parity
table still says IJ-ED-003/012 Partial/Missing: check against the roadmap line "pairing, smart Enter,
word moves" and fix the rows), word movement `ops.rs:658-695`.
Archipel design (WP4): `TypedHandler` chain in `archipel-ide-language/src/typing.rs`:
`before_char(ctx, ch) -> Handled|Continue`, `after_char(ctx, ch) -> Vec<TextEdit>`,
`on_enter(ctx) -> EnterAction {Default, Custom(edits, caret), SkipIndent}`, `on_backspace`,
`smart_enter(ctx) -> Option<Vec<TextEdit>>`. Context = `TypingCx { snap (fresh), offset, in_string/in_comment (tree-based), indenter }`.
Generic default behavior driven by queries/data: auto-pair only when the next token is not an
identifier and the surrounding node kind is not string/comment (this is the syntax-aware improvement
over today's character heuristics), re-indent on typed closing brace and on `else`/`case`
dedents (`indents.scm`), continue line comments and doc-comment asterisks on Enter, Enter between
`{}` / `()` / tags, auto-close tags (XML/HTML), `Ctrl+Shift+Enter` complete statement = language hook with
generic fallback "add missing `;` and close open bracket then newline". Brace matcher: `brackets.scm`
(`@open`,`@close` pairs incl. XML tags, angle brackets in generics only when node kind says so) with the
existing token fallback.

### 2.15 Commenter
IntelliJ: `Commenter` (line prefix(es), block prefix/suffix, "commented block comment" delimiters for
nesting, `blockCommentRequiresFullLineSelection`; `IC:platform/core-api/src/com/intellij/lang/Commenter.java`),
per-language registration. Archipel IDE: hard-coded prefix by path (`archipel-ide-core/src/editor_commands.rs:11
line_comment_prefix`), toggle in `archipel-ide-text/src/ops.rs:235`, IJ-ED-010 Implemented, IJ-ED-011
block comment Missing (2).
Archipel: `LanguageDef.commenter: Commenter { line: SmallVec<&str>, block: Option<(&str,&str)>,
nested_block: Option<(&str,&str)>, block_needs_full_lines: bool }` (data); also inside injected
fragments/JSX/markdown-in-html contexts choose the commenter of the language at the caret
(`fragment_at`). Toggle decisions use comment nodes from the tree (token set) rather than string
prefix checks, so `//` inside a string literal is never treated as a comment.

### 2.16 Formatter and code style
IntelliJ: `FormattingModelBuilder` produces a `FormattingModel` of `Block`s (text range, sub-blocks,
`Indent`, `Alignment`, `Wrap`, `Spacing` between siblings, child attributes for new children,
incomplete flag;
`IC:platform/code-style-api/src/com/intellij/formatting/FormattingModelBuilder.java`, `Block.java`),
engine in `IC:platform/code-style-impl`, settings `CodeStyleSettings`/`CommonCodeStyleSettings`
(per-language indent, wrapping, blank lines, spaces; `IC:platform/code-style-api/.../psi/codeStyle/`),
`LanguageCodeStyleSettingsProvider` (what the settings page shows per language;
`IC:platform/lang-api/.../codeStyle/LanguageCodeStyleSettingsProvider.java`), `PostFormatProcessor`,
`LineIndentProvider`, plus `.editorconfig` overlay. Archipel IDE today: Reformat via LSP formatting only
(`refactor.rs:372 reformat_code`, IJ-ED-140 Partial w5); `detect_indent` (`format.rs:79`); no code
style model (IJ-SET-049, IJ-SET-050 EditorConfig, w3 Missing).
Archipel design (WP6, crate `archipel-ide-format`): three layers, shippable independently.
(1) `CodeStyle` data model: `{ indent: IndentOptions{use_tab, indent, tab_size, continuation, label_indent}, per-language keyed
options (wrapping, blank lines, spaces, braces) as `StyleKey`->value, right margin, line separator }`,
resolved per file from: language defaults < app settings < project `.jetthing/code-style.toml` <
`.editorconfig` (own small parser for sections, globs, `indent_style`, `indent_size`, `tab_width`,
`end_of_line`, `trim_trailing_whitespace`, `insert_final_newline`, `max_line_length`) < detected indent
(optional, setting). (2) `Indenter` (LineIndentProvider analogue) driven by `indents.scm` captures
(`@indent.begin`, `@indent.end`, `@indent.branch`, `@indent.align`, `@indent.dedent`, `@indent.auto`,
`@indent.ignore`): powers Enter, typed-brace reindent, Auto-Indent Lines (IJ-ED-006), paste reindent
(IJ-ED-141), Tab/Shift+Tab block indent (IJ-ED-005). (3) Full `FormattingModelBuilder` per language
(Block tree + spacing rules + wrap/align) for languages that graduate; default engine = a straightforward
two-pass block formatter (compute indent/spacing from the Block tree, emit minimal text edits, never
rewrite unchanged whitespace so carets/folds survive). `FormattingService` chain picks native model,
external formatter command (rustfmt, google-java-format, prettier; process spawned from a toolchain
resolved binary, `archipel-ide-toolchains`), or LSP. Reformat selection/file, optimize imports (language
hook, LSP `source.organizeImports` fallback = today).

### 2.17 Live templates and postfix templates
IntelliJ: `TemplateImpl`, `TemplateSettings`, template context types per language (`TemplateContextType`,
`LiveTemplateContextBean`; `IC:platform/analysis-api/.../codeInsight/template/`, `IC:platform/analysis-impl/.../template/impl/TemplateImpl.java`,
`IC:platform/lang-impl/.../template/impl/TemplateSettings.java`), `Macro` functions for variables,
`LiveTemplateCompletionContributor`, bundled sets such as
`IC:java/java-backend/resources/liveTemplates/Java.xml`, `IC:plugins/kotlin/code-insight/live-templates-k2/resources/liveTemplates/Kotlin.xml`;
postfix: `PostfixTemplate`, `PostfixTemplateProvider`, expression selectors that find the expression
before the dot via PSI (`IC:platform/lang-impl/.../template/postfix/templates/`).
Archipel IDE today: `archipel-ide-templates` (templates, variables, evaluate, bundled sets, user templates;
`lib.rs:20-357`), core integration with Tab expansion, Ctrl+J, surround, postfix `.if .for ...`
(`live_templates.rs:112-424`, `snippet_session.rs`), contexts chosen by file path/context labels
(`templates_for(path)` `lib.rs:215`).
Archipel (WP6): make context detection syntactic: `TemplateContext { Statement, Expression, Declaration,
Comment, String, TopLevel, ClassBody, Other }` computed from the tree via per-language `@context.*`
captures; `Macro` registry as trait (`className`, `methodName`, `fileName`, `date`, `time`, `user`,
`enum(...)`, `complete()`, `suggestVariableName()`, `capitalize`, `camelCase`, `snakeCase`, `typeOf`
(G3)); postfix expression selector = innermost expression node ending at the dot (query
`@expression`), choose via popup if several. All bundled template text is authored by us (clean-room:
do not transcribe IntelliJ's XML; abbreviations such as `sout`/`psvm`/`fori` are public documented
names and may be reused, bodies written fresh). Parity IJ-ED-130/131/086 (Partial) and IJ-ED-132 file
templates (Missing; `new_class.rs` exists).

### 2.18 Refactoring framework
IntelliJ: `RefactoringSupportProvider` per language (availability and handlers for introduce
variable/constant/field/parameter/functional parameter, extract method/interface/superclass/class/
module, pull up, push down, change signature, safe delete flag, in-place rename/introduce flags;
`IC:platform/refactoring/src/com/intellij/lang/refactoring/RefactoringSupportProvider.java`),
`RefactoringActionHandler` (`invoke` on editor or on elements), `BaseRefactoringProcessor` lifecycle
(`findUsages` -> `preprocessUsages` (conflicts, can cancel) -> optional preview in a usage view ->
`performRefactoring` inside one command/undo; `IC:platform/refactoring/src/com/intellij/refactoring/BaseRefactoringProcessor.java`),
rename: `RenameHandler` EP, `RenamePsiElementProcessor.forElement` picks a processor that can add
related elements (`prepareRenaming`), find collisions, rename in comments/strings, automatic renamers
for related symbols, in-place rename template (`IC:platform/lang-impl/src/com/intellij/refactoring/rename/`:
`RenameProcessor.java`, `inplace/`, `AutomaticRenamingDialog`), safe delete
(`IC:platform/lang-impl/.../refactoring/safeDelete/SafeDeleteProcessor.java`: find usages, report
unsafe ones, "delete anyway"), move (`MoveHandler.java`), `RefactoringElementListener` and
`RefactoringListenerManager` (let other subsystems, e.g. run configs and breakpoints, follow renames),
`RefactoringHelper` post-processing, conflict dialog (`ConflictsDialogBase`), `suggested`
(suggested refactoring after manual edits), extract method skeleton in `refactoring/extractMethod`,
change signature in `refactoring/changeSignature`.
Archipel IDE today: all LSP-based: rename session with live apply (`refactor.rs:106-372`), extract/inline
as LSP code actions (`refactor.rs:408`), safe delete only for project-tree files
(`project_view.rs`), `workspace_edit.rs` applies WorkspaceEdits. Parity IJ-REF-001/002/003 (3, Missing),
010 (5), 021/022/040 (5, Partial), 011/020/024/025/050 (3, Missing), 060 (3 Partial).
Archipel design (WP7, crate `archipel-ide-refactor`):
```rust
pub trait RefactoringProcessor {                                  // BaseRefactoringProcessor
    type Options; fn title(&self)->String;
    fn find_usages(&self, cx:&RefactorCx<'_>, sink:&mut dyn UsageSink) -> Result<(), Cancelled>;
    fn preprocess(&self, usages:&[Usage], cx:&RefactorCx<'_>) -> Result<Vec<Conflict>, Cancelled>;   // collisions, shadowing, visibility
    fn perform(&self, usages:&[Usage], cx:&RefactorCx<'_>) -> Result<ModCommand, Cancelled>;         // pure edit description
}
pub trait RefactoringSupport: Send+Sync {                         // RefactoringSupportProvider
    fn rename(&self, t:&Target)->Option<Box<dyn RenameSupport>>;  fn safe_delete(..); fn move_(..);
    fn extract(&self, kind:ExtractKind, sel:&Selection)->Option<Box<dyn ExtractSupport>>; fn change_signature(..);
    fn inplace_rename_ok(&self, t:&Target)->bool; }
pub trait RenameSupport { fn related(&self, t:&Target)->Vec<RelatedRename>; fn collisions(&self, t:&Target, new:&str, cx)->Vec<Conflict>;
    fn validate_name(&self, new:&str)->Result<(),String>; fn comment_string_occurrences(&self)->bool; }
pub trait RefactoringListener { fn element_renamed(&self, from:&Target, to:&Target); fn element_moved(..); }  // run configs, breakpoints, bookmarks
```
Executor: `ModCommandExecutor` (shared with WP3) applies a `ModCommand` atomically: verify each
`expected_rev`, apply edits to open documents through the text engine and to closed files via
`Project::write_text`, record ONE project-wide undo group (new `ProjectUndo`: a stack of
`UndoGroup { docs: Vec<(FileId, per-doc undo token)>, file_ops: Vec<InverseOp> }` in
`archipel-ide-core`; file operations undone by inverse ops, deletions moved to a trash dir under the
cache), and Local History label ("Before Rename X"). Preview = group usages by file/kind, show diff,
unchecked usages excluded (IJ-REF-002). In-place rename = `ModCommand::StartRename` -> the existing
`RenameSession` UI state machine (`refactor.rs:106`), but candidates now come from native references
with LSP prepareRename as validator.
Native refactorings by tier (generic = works with any `Resolver` giving `Exact`): Rename (generic),
Safe Delete (generic), Move file/class (language hook for import/package updates), Extract
variable/constant/parameter/method (needs per-language expression/statement boundaries and data
flow: tier G3, start with Java; until then LSP code actions), Inline variable (generic for single
assignment locals via L0), Change Signature (G3+). Refactor This popup (IJ-REF-001) = list of
available handlers at caret.

### 2.19 Code Vision and inlay hints
IntelliJ: `InlayHintsProvider` (declarative; `createCollector(file, editor)` returning a collector
that adds presentations at offsets; `IC:platform/lang-api/src/com/intellij/codeInsight/hints/declarative/InlayHintsProvider.kt`),
`CodeVisionProvider` (`computeCodeVision`, anchors, relative ordering, click handlers, one entry per
line by default; `IC:platform/lang-impl/src/com/intellij/codeInsight/codeVision/CodeVisionProvider.kt`),
`InlayHintsPass`, `LineMarkerProvider`/`RelatedItemLineMarkerProvider` for gutter icons
(`IC:platform/lang-api/.../daemon/LineMarkerProvider.java`).
Archipel IDE today: LSP codeLens/inlayHint merged with LSP references for usage counts
(`code_vision.rs:29-431`, IJ-ED-104 Partial), gutter run icons via `archipel-ide-runnables` (IJ-ED-018).
Archipel (WP3 engine, WP5 data, WP8 LSP adapters): `InlayHintsProvider::collect(&self, cx, sink)`
with `InlayHint { offset, placement (Inline|AfterLine|AboveLine), parts: Vec<Part{text, target:
Option<Target>, tooltip}>, kind }`, settings per provider id/kind, `CodeVisionProvider` with
`id`, `anchor`, `relative_order`, `compute(cx) -> Vec<(Range, Entry)>`, async entries ("loading")
and `on_click(entry)`. Native providers: usages count (cheap estimate from `Words` count, upgraded
to exact via `UsageSearch` for declarations in the visible range only, cached by (target anchor,
index stamp, interface epochs)), implementations count (`Stub:SuperTypes`), "N usages / M tests",
VCS author (existing git blame data, `annotations.rs`), parameter-name hints for call arguments
(needs call-to-declaration resolve, tier G2+; otherwise LSP), chained-call/type hints (G3 / LSP).

### 2.20 TODO index
IntelliJ: `TodoIndex` (single entry per file; value = map of pattern -> count; rebuilt on pattern
change; `IC:platform/todo/backend/src/com/intellij/psi/impl/cache/impl/todo/TodoIndex.java`),
`IndexPatternBuilder` per language says which tokens are comments and how much prefix/suffix to skip
(`IC:platform/indexing-impl/.../psi/impl/search/IndexPatternBuilder.java`), `TodoPattern`
(`IC:platform/indexing-api/.../psi/search/TodoPattern.java`), `TodoConfiguration`
(`IC:platform/editor-ui-ex/.../ide/todo/TodoConfiguration.java`; default patterns match `todo` and
`fixme`, case-insensitive, word-bounded, to end of line per public docs). Archipel IDE: nothing (IJ-TW-030
w2, IJ-SET-060 w1).
Archipel (WP2 index + WP4 comment tokens): `Todo` index as in the 1.6 table, comment ranges from
`token_sets.comments` of the parsed tree (so `TODO` inside a string is ignored), patterns from settings
(`todo.patterns`, regex + case flag + highlight color key), editor highlight of matching comment text
in the daemon `Syntax` pass (cheap, uses the same regex on comment nodes), TODO tool window listing
current file / project / scope with grouping by file, filters, and navigate-on-click (UI in
`archipel-ide-ui/src/toolwindows.rs` pattern; new file `todo_window.rs`). Fallback when a language has no
grammar: scan with comment delimiter data from `Commenter`.

### 2.21 Spellchecking hook
IntelliJ: `SpellCheckingInspection` as a local inspection, `SpellcheckingStrategy` per language
producing `Tokenizer`s for PSI elements (`IC:spellchecker/src/com/intellij/spellchecker/inspections/SpellCheckingInspection.java`,
`tokenizer/SpellcheckingStrategy.java`, `tokenizer/Tokenizer.java`), dictionaries and quick fixes
(rename to suggestion, save to dictionary, ignore; `spellchecker/quickfixes`, `engine`, `dictionary`),
dedicated TYPO severity (`SpellCheckerSeveritiesProvider.java`).
Archipel (WP3 hook, WP6 or later engine): `SpellcheckStrategy { fn tokenize(&self, n:SyntaxNode, out:&mut
Vec<TextToken>) }` default: comments, doc comments, string literals (not escapes/URLs/format
specifiers), identifiers split on camel/snake boundaries; `Dictionary` trait (bundled English word
list, user dictionary file in config dir, project dictionary `.jetthing/dictionary.txt`); inspection
`Typo` at weak-warning/TYPO level with fixes (replace by suggestion via edit distance over the word
list, add to user/project dictionary, ignore). Licensing: the bundled word list needs a clean-room source
(public-domain or permissively licensed list; do not extract IntelliJ's bundled dictionaries). Parity
IJ-ED-118 (1), IJ-SET-058 (1): low priority.

### 2.22 Parameter info
IntelliJ: `ParameterInfoHandler<Owner, Param>`: find the owner element at caret for creating/updating,
show with candidates, `updateParameterInfo` (current parameter index), `updateUI` (highlight range),
overload switching, close chars, "could show in lookup"
(`IC:platform/lang-api/src/com/intellij/lang/parameterInfo/ParameterInfoHandler.java`). Archipel IDE: LSP
signatureHelp with active-parameter logic and triggers (`signature_help.rs:19-210`, IJ-ED-101 Partial).
Archipel (WP6): `ParameterInfoProvider { fn owner_at(&self, snap, offset) -> Option<ArgsOwner>;
fn signatures(&self, owner, cx) -> Vec<Signature>; fn current_index(&self, owner, offset) -> usize }`.
Generic implementation: enclosing node matching `@call.args` (query) and its callee leaf -> `Resolver`
-> declarations with `@detail`/params from tags; current index by counting separator tokens at the
owner's depth (comma tokens as direct children of the arguments node, so nested calls and string
commas are never miscounted); named/default/variadic parameters are language hooks. LSP fallback = today.

### 2.23 Quick documentation
IntelliJ: `DocumentationProvider` (`generateDoc`, `generateHoverDoc`, `getQuickNavigateInfo` for
Ctrl+hover, rendered doc comments, URL list; `IC:platform/analysis-api/src/com/intellij/lang/documentation/DocumentationProvider.java`)
and the newer target-based API (`DocumentationTargetProvider` under
`IC:platform/lang-impl/src/com/intellij/platform/backend/documentation/`). Archipel IDE: LSP hover rendered
from Markdown (`quick_doc.rs:22-142`, `archipel-ide-lsp/src/markup.rs`, IJ-ED-100 Partial, IJ-ED-102
Quick Definition Missing, IJ-ED-103 Type Info Missing, IJ-ED-105 rendered Javadoc Missing).
Archipel (WP6): `DocumentationProvider::document(target) -> DocContent { signature_md, doc_md, links,
sections }` assembled natively: declaration header text from the stub node range (signature up to
body), doc comment = adjacent comment nodes selected by the `@doc` capture and a per-language doc-comment
parser (`///`, `/** */`, `"""`) into Markdown (Javadoc tags -> sections; Rust doc markdown as is),
rendered by the existing `archipel-ide-markdown` crate. LSP hover appended for resolved types. Quick
Definition = show the target's source slice in a read-only editor popup (kit read-only `EditorComponent`
viewer mode, `KIT:docs/editor-model.md` "viewer(true)").

### 2.24 Sticky lines, breadcrumbs
IntelliJ: `StickyLinesModel` holds sticky line ranges (`IC:platform/platform-impl/src/com/intellij/openapi/editor/impl/stickyLines/StickyLinesModel.java`),
`StickyLinesCollector` (re-collect when the PSI modification stamp changed) and
`StickyLinesLanguageSupport`, which enables the feature for languages that provide breadcrumbs
(`BreadcrumbsProvider`, `IC:platform/editor-ui-api/.../ui/breadcrumbs/BreadcrumbsProvider.java`):
sticky scopes = the same container elements as breadcrumbs, shown when their first line scrolled out
above the viewport. Archipel IDE: Missing (IJ-ED-019 w2, IJ-SET-046); breadcrumbs partly present
(`path_at`, PLAT-018). Archipel (WP4): `StickyScopes::enclosing(snap, first_visible_line) ->
Vec<StickyScope{start_line, end_line, header_range, kind}>` from the structure/scope query
(`@scope.sticky` or default: multi-line `@definition.*` and statement blocks flagged `@sticky`), outermost
first, capped by a setting (IntelliJ docs: default 5), recomputed only when the file revision changed.
Kit consumes it via the host trait (Part 5).

### 2.25 Local history
IntelliJ: `lvcs-api`/`lvcs-impl` (`IC:platform/lvcs-impl/src/com/intellij/history/`): per-project change
log with labels, revisions per file/directory, reverts. Archipel (WP2 store, WP7 UI/command; IJ-ED-028
w3, IJ-TW-034): `archipel-ide-index::history` = content-addressed blob store (zstd-compressed, hash =
BLAKE3 or SHA-256 already in workspace deps via toolchains) + per-path revision log (time, blob hash,
cause: save, external change, refactoring label, VCS operation, manual label); retention by age/size
(settings, default a few days like the IDE docs); show history, diff against current (existing
`diff_ui.rs`), revert file/selection, recent changes view. No VFS hooks: the host writes a revision
before overwriting on save, before applying external changes, before `ModCommand` execution.

### 2.26 Language injection and templating
Existing `injection.rs`; promoted to `InjectedFragment` (1.2). Parity IJ-ED-074 (Partial).

---------------------------------------------------------------------------------------------------

## Part 3. Languages: completeness tiers and the order of per-language work

### 3.1 CompletenessTier (per language, per feature family)

| Tier | Meaning | What turns on |
|---|---|---|
| G0 | syntax only (tree-sitter + queries) | highlight, folding, structure, breadcrumbs, sticky lines, brackets, extend selection, comment, typed handlers/indent, TODO, syntax errors, file-local word-based highlight usages, spelling, live templates |
| G1 | + file-local resolve (`locals.scm`) | exact local usages, highlight usages, rename local, unused local/parameter/import inspections, semantic coloring of locals, local completion, inline local |
| G2 | + project-level name resolution (imports/modules/packages by name, no types) | Go to Declaration across files, Find Usages (Exact for top-level/nominal symbols), Go to Class/Symbol exact, auto-import, "unresolved reference" error (unmuted), Rename/Safe Delete/Move for nominal symbols, parameter info for plain calls, quick doc from signatures |
| G3 | + types, members, inheritance, overload candidates | Go to Implementation/Super/Type, member completion, override/implement, smart completion, type hints, extract method/change signature (with flow in G4), unused public API checks |
| G4 | + generics, inference, flow/nullability/data-flow | IntelliJ-class inspections (data flow issues, constant conditions), exact refactorings, inline method |

Native results always carry `Confidence`; tiers gate which tools may fire. The tier is a registry
value, not code: raising it is a one-line change plus the conformance gate (1.10).

### 3.2 Starting point in Archipel IDE (what each language already has)

Grammars (20, `Cargo.toml` workspace): rust, java, kotlin (sg), groovy, python, go, c, cpp, javascript,
typescript (+tsx), bash, css, html, xml, json, yaml, toml, properties, markdown, starlark. Servers
(`archipel-ide-lsp/src/registry.rs:458`): rust-analyzer, jdtls, kotlin, starlark, pyright, gopls, typescript,
clangd, lemminx, taplo, yaml, json. Language-specific depth today: Java/Kotlin/Rust have structure, selection,
runnables (`archipel-ide-runnables`), live-template sets, Gradle/Maven/Cargo build detection; Rust is the
verified LSP path; Java/Kotlin parity rows are mostly Missing (IJ-JAVA-011/013/018/040 weight 5).
Gap of the generic data: `locals`/`folds` query slots are empty for all 20 languages
(`registry.rs`, every `LanguageSpec`), so WP4 first adds query files (Part 4).

### 3.3 Recommended order

1. All grammars to G0 via query files (WP4). Cheap, parallelizable per language, immediately improves
   every language. Includes `indents.scm`, `folds.scm`, `brackets.scm`, `textobjects.scm`, `sticky` data,
   comment tokens, tags with modifiers.
2. Data and markup languages to a full native tier (WP9c): JSON, YAML, TOML, XML, HTML, properties,
   Markdown, CSS, Starlark/Bazel labels. Reasons: no type system, tree-sitter output is faithful,
   native resolve is key-path based and exact, and it exercises the pipeline end-to-end (duplicate-key
   inspections, structure by key path, XML tag matching, references from Gradle/Maven/Bazel files).
   Needed for the build-tool parity areas (`intellij-build-tools.md`, `bazel.md`).
3. Java to G2 then G3 (WP9a). Biggest parity weight (IJ-JAVA-011/013/018 at 5, 014/016 at 3), a regular
   grammar, and jdtls stays as fallback and as the differential oracle until the gate passes. Needs a
   JVM stub reader for classes in JDK/jars (`.class`/`jrt` file parser producing `StubTree`s; the
   format is a public spec; clean-room) so imports of library types resolve.
4. Rust to G2 (WP9b, in parallel with Java by a second agent): crate/module tree from `Cargo.toml` +
   `mod`/`use`/`pub use`, items by path, locals. Rust is the IDE's own language and the verified LSP
   path. Keep rust-analyzer authoritative for macros, traits, method resolution and inference; run
   `provider=both` to measure. G3 for Rust is not planned natively.
5. Kotlin (WP9d): G1/G2 natively sharing the JVM stub index with Java (Kotlin/Java interop requires one
   JVM symbol table, like IntelliJ's), semantics stay with the Kotlin server until a later decision.
6. TypeScript/JavaScript, Python, Go (WP9e): G1 then G2 (ES module/package/import resolution is
   tractable and file-path driven); semantics stay LSP.

Per-language crate shape (so agents do not collide): `crates/archipel-ide-lang-<x>/` with `src/lib.rs`
registering one `LanguageDef` override set via `fn register(reg: &mut ServiceRegistry)`, `queries/*.scm`
overrides, `src/resolve.rs`, `src/stubs.rs`, `src/inspections/*.rs`, `src/intentions/*.rs`,
`src/refactor.rs`, `tests/corpus/`, `tests/conformance.rs`. The app registers all crates in one list
(`crates/archipel-ide-core/src/analysis_modules.rs`, created in the M0 seam).

---------------------------------------------------------------------------------------------------

## Part 4. Prioritized backlog as parallel work packages

### 4.0 Conventions common to all packages

Required checks (AGENTS.md): `cargo fmt --all --check`, `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p xtask -- parity --check`,
`cargo run -p xtask -- design-lint`. Conventional Commits. Never commit `docs/parity/dashboard.md`.

Test conventions to follow (existing patterns):
- Unit tests inline (`#[cfg(test)] mod tests`), integration tests in `crates/<crate>/tests/*.rs`
  (`archipel-ide-language/tests/incremental.rs`, `archipel-ide-lsp/tests/fake_server.rs`,
  `archipel-ide-core/tests/fake_lsp_flow.rs`, `generate_hierarchy.rs`).
- Property tests with `proptest` for any invariant over edits/trees (`archipel-ide-language` dev-dep).
- Commands are typed and tested through the registry (`line_actions_run_through_the_registry` pattern in
  `archipel-ide-core`); every user-visible action = a command id registered in `archipel-ide-command`, with its
  IntelliJ action id mapped in `archipel-ide-core/src/keymap_commands.rs` `ACTION_COMMANDS` (AGENTS.md).
- LSP behavior tested against `archipel-ide-fake-lsp` (scriptable server binary,
  `archipel-ide-lsp/src/bin/archipel-ide-fake-lsp.rs`) and opt-in real servers (`tests/real_servers.rs`).
- UI evidence: `xtask scenes` Xvfb screenshots + `docs/parity/evidence/<topic>.md`; parity rows updated
  with exact test names in the `Tests` column, conservative statuses (nothing `Verified`).
- Perf: `cargo bench -p archipel-ide-ui --bench typing` (PERF-008) must not regress; each new hot path gets a
  bench in its crate (`benches/`), with the budgets of 1.11 as assertions in a `perf` test gated by env.
- New crates: no `egui` dependency, `#![forbid(unsafe_code)]` unless justified, no panics on user input,
  errors are structured (`thiserror`).

Integration seam (created in M0 by WP1, so no one else edits `IdeState` or `lib.rs` again):
`crates/archipel-ide-core/src/analysis_host.rs` (owns the one new `IdeState.analysis: AnalysisServices`
field and per-frame `poll_analysis()`), `crates/archipel-ide-core/src/analysis_modules.rs` (a table of
`fn register(&mut ServiceRegistry, &mut CommandRegistry)` entries, one line per package/language crate),
and one empty module declaration per package in `lib.rs`. Each package adds commands in its own
`<package>_commands.rs`. Cross-package data flows through traits in `archipel-ide-analysis`, never through
`IdeState` fields.

### 4.1 Execution order (waves)

- Wave 0, contract freeze (WP1 M0, about one iteration): trait/struct signatures of `archipel-ide-analysis`
  with in-memory implementations and mock fixtures, the integration seam, the migration plan for
  `LanguageSpec`. All other packages start from the frozen contracts and mocks.
- Wave 1, language-agnostic pipeline core (parallel): WP1 (finish), WP2, WP3, WP8. WP4 may start here
  because it only needs `LanguageDef` queries, not the daemon.
- Wave 2, generic features via queries and the generic resolver (parallel): WP4, WP5, WP6, WP7.
- Wave 3, per-language optimization (parallel, independent crates): WP9a-e.

### 4.2 Package table (summary)

| WP | Name | Crates / files owned (exclusive) | Depends on |
|---|---|---|---|
| WP1 | Analysis core | `archipel-ide-analysis/**`; `archipel-ide-language/src/{lib,registry,syntax,injection,def,node,anchor}.rs`; `archipel-ide-core/src/{analysis_host,analysis_modules}.rs`; additive `archipel-ide-text` edit-map helpers; `archipel-ide-project/src/watcher.rs` batching | none |
| WP2 | Index, dumb mode, TODO, local history | `archipel-ide-index/**`; `archipel-ide-core/src/{indexing_commands,todo_commands,history_commands}.rs`; `archipel-ide-ui/src/{index_status,todo_window,history_ui}.rs` | WP1 contracts |
| WP3 | Daemon, inspections, intentions | `archipel-ide-daemon/**`; `archipel-ide-core/src/{problems,code_actions,inspection_commands}.rs`; `archipel-ide-ui/src/{problems_ui,intentions_ui,inspections_settings,error_stripe}.rs`; inspection settings page registration | WP1 contracts |
| WP4 | Generic structural features (queries) and typed editing, kit language service | `archipel-ide-language/{queries/**,src/{folding,structure,selection,brackets,sticky,indent,typing,textobjects,commenter,service}.rs}`; `archipel-ide-text/src/{ops,transform}.rs`; `archipel-ide-core/src/{editor_commands,folds}.rs`; `archipel-ide-ui/src/{editor,editor_keys,structure_view,editor_language}.rs` | WP1 (LanguageDef) |
| WP5 | Resolve, usages, navigation | `archipel-ide-resolve/**`; `archipel-ide-core/src/{navigation,quick_nav,hierarchy,test_nav,resolve_commands}.rs`; `archipel-ide-ui/src/{navigation_ui,palette,usages_popup}.rs` | WP1; WP2 (index; mock until ready) |
| WP6 | Editing assistance: completion, quick doc, parameter info, format/code style, templates, generate | `archipel-ide-completion/**`, `archipel-ide-format/**`, `archipel-ide-templates/**`; `archipel-ide-core/src/{completion,completion_stats,signature_help,quick_doc,live_templates,snippet_session,generate}.rs`; `archipel-ide-lsp/src/{completion,snippet,markup}.rs` (moved code); `archipel-ide-ui/src/{completion_ui,doc_ui,settings_live_templates,settings_codestyle}.rs` | WP1, WP4 (indenter), WP5 (variants) |
| WP7 | Refactoring framework and native refactorings | `archipel-ide-refactor/**`; `archipel-ide-core/src/{refactor,workspace_edit,modcommand_exec,project_undo}.rs`; safe-delete part of `project_view.rs`; `archipel-ide-ui/src/{refactor_popup,conflicts_dialog,refactor_preview}.rs` | WP3 (ModCommand preview), WP5 (usages) |
| WP8 | LSP provider bridge and graduation harness | `archipel-ide-lsp-provider/**`; additive broker code in `archipel-ide-lsp/src/manager.rs`; `archipel-ide-core/src/{lsp_commands,toolchain_lsp}.rs`; `xtask` `conformance` subcommand | WP1, WP3 contracts |
| WP9a-e | Per-language tiers | `archipel-ide-lang-{java,rust,data,kotlin,web}/**` (one agent each) | WP1-WP8 as needed |

File-overlap rules: a file is owned by exactly one package. Where two packages need the same file,
the owner exposes a small extension point and the other adds a module in its own crate. Known hotspots:
`archipel-ide-ui/src/editor.rs` (2,478 lines; WP4 only; others publish decorations through
`HighlightMarkup`/`InlayModel` read by it), `archipel-ide-core/src/lib.rs` (WP1 only, seam), `quick_nav.rs` (WP5
only; WP2 supplies index readers behind the old `SymbolIndex` API).

### 4.3 Prioritized backlog by parity ID

Weight-5 rows first (status as recorded in the parity docs; some editor-basics rows look stale against
the roadmap, verify before work and correct them as part of the package).

| Weight / status | Parity ID | Feature | Package |
|---|---|---|---|
| 5 Missing | IJ-NAV-080 | background indexing + dumb mode | WP2 |
| 5 Missing | PLAT-009 | persistent indexing | WP2 |
| 5 Missing | IJ-JAVA-011 | Java semantic error highlighting | WP9a (+WP3 pass) |
| 5 Missing | IJ-JAVA-013 | Java navigation | WP9a (+WP5) |
| 5 Missing | IJ-JAVA-018 | Java refactorings engine | WP9a (+WP7) |
| 5 Missing | IJ-JAVA-040 | Kotlin language support | WP9d |
| 5 Missing | IJ-ED-081 | auto completion popup (roadmap says done: verify) | WP6 |
| 5 Partial | IJ-ED-110 | on-the-fly error highlighting | WP3 |
| 5 Partial | IJ-ED-120 | Alt+Enter quick fixes | WP3 |
| 5 Partial | IJ-ED-080 | basic completion | WP6 |
| 5 Partial | IJ-ED-140 | reformat code | WP6 |
| 5 Partial | IJ-ED-003 | smart indent on Enter | WP4 |
| 5 Partial | IJ-NAV-001/002/003 | Search Everywhere, Go to Class, Go to File | WP5 (+WP2) |
| 5 Partial | IJ-NAV-020 | Go to Declaration or Usages | WP5 |
| 5 Partial | IJ-NAV-030 | Find Usages | WP5 |
| 5 Partial | IJ-REF-010 | Rename | WP7 |
| 5 Partial | IJ-REF-021 / 022 / 040 | extract method / variable, inline | WP7 (+WP9a) |
| 5 Partial | IJ-JAVA-012 | Java completion | WP9a (+WP6) |
| 4-3 Missing | IJ-ED-072, IJ-NAV-032 | highlight usages | WP5 (+WP3 pass) |
| 3 Missing | IJ-ED-012, 013 | pairing (verify, may exist), complete statement | WP4 |
| 3 Missing | IJ-ED-084 | completion auto-import | WP6 (+WP9) |
| 3 Missing | IJ-ED-113 | Inspect Code batch | WP3 |
| 3 Missing | IJ-ED-123 | override/implement methods | WP6 (+WP9a) |
| 3 Missing | IJ-ED-028 | local history | WP2 |
| 3 Missing | IJ-REF-001/002/003 | Refactor This, preview/conflicts, project undo | WP7 |
| 3 Missing | IJ-REF-011/020/024/025/050 | rename file/class, change signature, introduce field/parameter, move | WP7 |
| 3 Missing | IJ-SET-049 / 050 / 051 | code style, EditorConfig, inspection profiles | WP6 / WP6 / WP3 |
| 3 Missing | IJ-JAVA-014 / 016 / 017 | Java inspections, fixes/intentions, generate | WP9a |
| 3 Partial | IJ-ED-121 / 122 / 125 / 130 / 111 / 100 / 101 / 104 | intentions, generate, optimize imports, live templates, severities, quick doc, param info, inlays | WP3 / WP6 / WP6 / WP6 / WP3 / WP6 / WP6 / WP3+WP8 |
| 3 Partial | IJ-NAV-004 / 022 / 031 / 040 | Go to Symbol, implementation, usages popup, file structure | WP5 / WP5 / WP5 / WP4 |
| 3 Partial | IJ-ED-060.. | folding group of rows (IJ-ED-061/062 Partial) | WP4 |
| 2-1 | IJ-ED-006, 011, 019, 024, 035, 036, 082, 083, 102, 103, 106, 118, 143, 144, IJ-NAV-025, 026, 041-043, IJ-TW-030, IJ-SET-046, 058, 060 | long tail | per package above |

---------------------------------------------------------------------------------------------------

### WP1. Analysis core (foundation)

Scope: sections 1.1-1.4, 1.11 and the ServiceRegistry of 1.3. Concretely: (a) `archipel-ide-analysis`
crate with `FileId`/`FileTable`, `Vfs` facade + `DocumentOverlay`, `AnalysisHost`/`Snapshot`/`Change`,
`Job`/`Executor`/`CancelToken`/`Cancelled`, `Dep`/`Cached`/`Db::cached`, `ServiceRegistry` with ordering
and base-language fallback, `ModCommand` enum (pure data), `Severity`/`HighlightInfo` base types shared
with WP3, `Target`/`Confidence`/`Provenance` shared with WP5, `Anchor`/`EditMap`; (b) in
`archipel-ide-language`: `LanguageDef` (with `pub type LanguageSpec = LanguageDef` shim), element-type
tables, `TokenSet`s, `SyntaxSnapshot`/`SyntaxNode`, `InjectedFragment`, and re-implement
`LanguageAnalysis` as a facade so current callers (`Document`, `quick_nav`, `runnables`) keep compiling;
(c) the M0 integration seam in `archipel-ide-core`; (d) memory/perf benches.
Parity: PLAT-011, PLAT-012, PERF-008 (must not regress); enabler for IJ-NAV-080.
Dependencies: none.
Tests: proptest "anchors never resolve to a different kind after random edits" and "EditMap composition =
direct mapping"; cancellation (a job observing a revoke returns `Cancelled` within 256 checks); cached
value validity (dep bump invalidates; interface-hash early cutoff keeps dependents valid); registry order and
base-language fallback unit tests; extend `tests/incremental.rs` with changed-range events; bench for
reparse + changed range; golden test that the existing language tests (highlight, structure, folds,
selection) pass unchanged on the new snapshot types.
Done when: all existing `archipel-ide-language` and `archipel-ide-core` tests pass; typing bench within 5% of
baseline; docs `docs/analysis/architecture.md` (new) plus the `docs/architecture.md` layer list updated; the
seam files exist with one line per later package; no `egui` in new crates.

### WP2. Index, dumb mode, TODO, local history

Scope: 1.6. `archipel-ide-index`: `IndexStore` trait + in-memory impl + `redb` (or documented fallback) impl,
`FileIndex`/`StubIndex` contracts, built-ins `FileName`, `FileType`, `Words`, `Trigram`, `Todo`, `Stubs`
(generic builder from tags) and the stub indexes (`ShortName`, `Qualified`, `SuperTypes`, `Members`),
indexing scheduler and priorities, content-hash skip, overlay for open buffers, per-index readiness and
`IndexNotReady`, progress model, `index.status`/`index.rebuild` commands, status-bar progress widget,
TODO tool window and TODO settings, `history` module (blob store + revision log) with Local History
commands and a minimal Show History dialog.
Parity: IJ-NAV-080 (5), PLAT-009, IJ-NAV-001..004 (data side), IJ-TW-030, IJ-SET-060, IJ-ED-028 (3),
IJ-TW-034.
Dependencies: WP1 contracts (mock `Snapshot` until real).
Tests: storage conformance suite run on both backends; corruption/truncation -> rebuild without error;
version bump rebuilds one index; property test "incremental index after random edit/rename/delete
sequences equals a fresh full index"; dumb-mode state machine (`require` returns `IndexNotReady` until
`Ready`); overlay precedence; TODO pattern change rebuilds only `Todo`; bench: 20k generated files
(targets in 1.6); scenes: indexing banner, TODO window, history dialog (`xtask scenes`).
Done when: Go to File works within 1 s of project open on the benchmark tree; `quick_nav` can switch to the
new readers behind the old `SymbolIndex` API (switch done by WP5); index survives restart; PLAT-009 and
IJ-NAV-080 rows updated with evidence.

### WP3. Daemon, inspections, intentions

Scope: 1.7-1.9. `archipel-ide-daemon`: `Pass` scheduler (visible-range first, debounce, cancel-on-edit,
prerequisites), `HighlightInfo`/`HighlightMarkup` (range store with edit shifting, per-tool replace,
recycler), built-in passes (`Syntax`, `General` with `Annotator`s, `LocalInspections`,
`ExternalAnnotators`, `IdentifierHighlighter`, `LineMarkers`, `ProblemFiles`), `LocalInspection`/
`GlobalInspection` engine with element-kind dispatch, profiles (TOML), suppression, batch runner,
`IntentionAction`/`QuickFix` + Alt+Enter assembly (groups, priorities, ordering formula, dedupe, cache),
error stripe data, `Problem` compatibility view. Ship the Tier-A inspection set (Appendix B) so the
pipeline is exercised in Wave 1 without needing resolve. Settings page "Editor > Inspections" (IJ-SET-051).
Parity: IJ-ED-110 (5), 120 (5), 111, 113, 114, 115, 121, 023, 106, LSP-001, IJ-SET-051, hosting for
IJ-JAVA-014/016.
Dependencies: WP1 contracts; consumes LSP via WP8 adapters (until then existing `problems.rs` LSP path stays).
Tests: scheduler tests with an injected clock and fake passes (ordering, debounce, cancel, visible-first);
markup shifting through random edits (proptest: highlights never exceed document bounds, never
split a char); profile TOML round trip + layering; suppression comment tests per language; Alt+Enter
ordering golden tests for the formula in 1.9 (group + priority weight; ties by text); quick-fix
application through `ModCommand` and undo; fake-LSP flow test proving diagnostics + code actions appear through the
adapter with dedupe; Xvfb scenes (popup, stripe, inspection settings).
Done when: edits never block on the daemon (bench), typing cancels in-flight passes, Tier-A inspections
run on all G0 languages, problems tool window and F2 navigation read from `HighlightMarkup`, existing
LSP behavior unchanged, rows updated.

### WP4. Generic structural features via queries, typed editing, kit language service

Scope: 2.11-2.15, 2.24 and Part 5. Author query files for all 20 grammars (`folds.scm`, `indents.scm`,
`brackets.scm`, `textobjects.scm`/select groups, `tags.scm` with `@modifier`/`@detail`, sticky scopes,
`doc` captures, comment token sets, commenter data, keyword sets for completion), the `TypedHandler`/
`Indenter`/`SmartEnter`/`MoveStatement` implementations, `StickyScopes`, Commenter (block comment),
the `LanguageServices` implementation for archipel-kit, command wiring (Complete Current Statement, Move
Statement, Auto-Indent Lines, Comment with Block Comment, Next/Previous Method, Code Block start/end,
matching brace), parity row corrections for editor-basics rows.
Parity: IJ-ED-003 (5), 005, 006, 011, 012, 013, 019, 024, 035, 036, 060-062, 073, 143, IJ-NAV-025,
IJ-NAV-040, IJ-SET-046, PLAT-035/036/037, IJ-ED-141 (reindent part).
Dependencies: WP1 `LanguageDef` (can start on the current registry; migration is mechanical).
Tests: query golden tests (`crates/archipel-ide-language/tests/golden/<lang>/<feature>.{src,expected}`
files produced by a `bless` env var like `UPDATE_SNAPSHOTS` in the kit); proptest for extend selection
(strictly growing ranges, ends at the file), folds (no illegal overlap), brackets (pairs symmetric);
typed-handler before/after string tables; kit conformance test calling the trait methods with a fake kit
`Document`; scenes for sticky lines and smart Enter.
Done when: each of the 20 grammars passes the golden suite for the features it supports (feature matrix
file `docs/analysis/language-features.md` generated by a test), the kit trait is implemented and the
IDE editor behaves per the editor-model "differences" list (re-indent on `}`, syntax word stops, smart
enter, move statement, syntax-aware extend selection).

### WP5. Resolve, usages, navigation, Search Everywhere

Scope: 1.5, 2.3, 2.8-2.10. `archipel-ide-resolve`: `Target`/`Reference`/`ReferenceProvider`/`Resolver`,
generic L0-L2 resolver from `locals.scm` + tags + stub name index, `SearchScope`, `UsageSearch`
(`ReferencesSearch` analogue with `Words`/trigram narrowing and text-scan fallback), highlight-usages
engine (daemon pass body), `GotoDeclarationHandler`, `NavigationService`, `GotoContributor` model and
Search Everywhere providers (migrate `quick_nav.rs`), usage grouping rules and usages popup,
read/write access, semantic coloring of resolved identifiers (opt-in), hierarchy providers (native via
`Stub:SuperTypes` when G3) and LSP fallback through WP8.
Parity: IJ-NAV-001..004 (5/5/5/3), 006, 020 (5), 021-023, 026, 030 (5), 031-033, 041-043, 065, IJ-ED-071,
IJ-ED-072, IJ-JAVA-013 (framework side).
Dependencies: WP1; WP2 index (develop against the in-memory store first).
Tests: resolve fixtures with inline markers (comments such as `//^ def:foo`, `//^^^ ref:foo`) run by one
harness against every language's `Resolver` (shared `resolve_conformance` test helper in the crate);
usage search vs a naive full-text oracle (recall/precision on synthetic projects); scope containment
tests; cancellation; streaming order; Go To ranking tests migrated from `quick_nav::tests`; scenes
21-26 must remain unchanged.
Done when: G1 resolve works for all grammars with `locals.scm`; Find Usages and Highlight Usages work
natively at G1 (file-local) and G2 (project) for the languages that reach those tiers; Search Everywhere
and Go To use the index; all previous LSP navigation continues via adapters.

### WP6. Editing assistance (3 independently assignable sub-streams)

6a Completion platform, quick doc, parameter info (2.7, 2.22, 2.23): `archipel-ide-completion` contributors,
dummy-identifier engine, weigher chain, native baseline contributors, LSP merge; `DocumentationProvider`
and `ParameterInfoProvider` generic implementations; Quick Definition/Type Info.
Parity: IJ-ED-080 (5), 081 (5), 082-085, 089, 090, 100-103, PLAT-420.
6b Formatter and code style (2.16): `archipel-ide-format` code style model, `.editorconfig` parser, `Indenter`
integration, `FormattingModel` engine and `FormattingService` chain, code style settings page.
Parity: IJ-ED-140 (5), 141, 143, 144, IJ-SET-049, IJ-SET-050.
6c Templates, generate, surround, imports (2.17, 2.6 library): syntactic template contexts, macros,
postfix selectors, file templates, Generate/Surround/Override-Implement UI hooks, Optimize Imports and
auto-import plumbing (language hooks).
Parity: IJ-ED-086, 122-126, 130-132, IJ-JAVA-017.
Dependencies: WP1; 6a needs WP5 (variants) and WP2 (project symbols) but starts with locals/keywords/words;
6b needs WP4's `Indenter`.
Tests: completion ranking golden tests (weigher order, prefix classes) ported from the existing
camel-hump tests in `archipel-ide-lsp/src/completion.rs`; marker-based "complete at ^" fixtures per
language; doc rendering snapshots; parameter index counting tables (nested calls, strings with
commas); formatter idempotence (format(format(x)) == format(x)) and "minimal edit" tests;
`.editorconfig` section/glob cases; template macro unit tests; existing `archipel-ide-templates` tests kept.
Done when: native baseline completion shows in < 50 ms and merges with LSP items without duplicates;
Reformat works natively for at least Java or via the external-formatter chain for Rust.

### WP7. Refactoring framework and native refactorings

Scope: 2.18. `archipel-ide-refactor` processors, `RefactoringSupport` registry, conflicts and preview model,
`ModCommandExecutor` + `ProjectUndo` + History labels in `archipel-ide-core`, Refactor This popup, rename
(generic over `Resolver` `Exact`) including related-element renamers (file with class, test classes,
getter/setter pairs where the language provides them), rename in comments/strings via `Words`, Safe
Delete, Move (file/class with language import update hook), Introduce/Extract framework (selection
validation, name suggestion, in-place template) with per-language implementations delegated to WP9,
`RefactoringListener` for run configs/breakpoints/bookmarks, migration of the LSP rename session.
Parity: IJ-REF-001/002/003 (3), 010 (5), 011, 020, 021/022/040 (5, framework), 023-025, 050, 060, IJ-JAVA-018.
Dependencies: WP3 (`ModCommand` preview/diff), WP5 (usages and `Exact` resolve), WP2 (history).
Tests: rename property test "after rename, resolve of every former usage yields the renamed
declaration, and no text outside usages changed"; conflict detection tables; project-wide undo
restores byte-identical files and the file tree (including create/move/delete); fake-LSP rename
equivalence (`both` mode); preview dialog scene.
Done when: native rename and safe delete work at G2 for Java and the data languages; undo is one step;
the old LSP-only rename path still works under `provider=lsp`.

### WP8. LSP provider bridge and graduation harness

Scope: 1.10. `archipel-ide-lsp-provider`: all adapters listed in 1.10, `LspBroker` (feature gating,
revision sync, per-key cancellation, stale drop), position conversion, `Merger<T>` policies, per-language
`analysis.<lang>.provider` settings, divergence logging for `both`, the conformance/differential
harness (`xtask conformance <lang> <feature>`), migration of existing LSP glue out of the core modules
as each traits-based consumer lands (coordinated by deprecating, not deleting, old paths first).
Parity: PLAT-016, LSP-001, IJ-ED-104, IJ-ED-071 (semantic tokens provider), regression guard for every
LSP-backed row.
Dependencies: WP1 and WP3 contracts; consumers WP3/5/6/7 call traits only.
Tests: adapter tests against `archipel-ide-fake-lsp` for every adapter (request mapping, UTF-16 conversion,
stale response drop, cancellation of the previous request); `Merger` policy tests (table in 1.10);
transcript-replay differential test skeleton with a tiny sample corpus; `both` divergence logging test.
Done when: with `provider=lsp` for all languages behavior equals today's (existing fake-LSP flow tests
green through the adapters), and `provider=auto` merges native G0/G1 results correctly.

### WP9. Per-language tiers (each sub-package independent; all follow 3.3 and the crate shape)

9a Java -> G2 then G3 (`archipel-ide-lang-java`): package/import/class/member resolution, JVM stub reader for
JDK/jars (shared module `archipel-ide-lang-jvm` consumed by 9d), type hierarchy index, overload candidate lists,
inspection set from Appendix B, intentions, extract variable/method (statement-level flow), generate code,
Optimize Imports, completion contributors (members, statics, auto-import), semantic error highlighting
for the unambiguous cases (unresolved symbol, missing return, duplicate declarations) gated by tier.
Parity: IJ-JAVA-011/012/013/014/016/017/018, IJ-ED-123, IJ-REF-021/022/040.
9b Rust -> G2 (`archipel-ide-lang-rust`): module tree and item-path resolution, `use` graph, locals, unused
imports/variables, missing-`mod` file detection, derive/attribute-aware structure; rust-analyzer remains
the semantic oracle (`provider=both` in CI).
9c Data and markup (`archipel-ide-lang-data`): JSON/YAML/TOML/XML/HTML/properties/Markdown/CSS/Starlark: key-path
structure and references, duplicate keys, tag matching/auto-close, schema-less validation, Gradle/Maven/
Bazel label references, Markdown link references.
9d Kotlin G1/G2 sharing `archipel-ide-lang-jvm`; 9e TypeScript/JavaScript, Python, Go G1/G2.
Each: conformance corpus + differential gate vs the language server (1.10) before any tier promotion;
parity rows updated with measured agreement numbers in the evidence file.

---------------------------------------------------------------------------------------------------

## Part 5. Language service interface for archipel-kit (the host trait)

### 5.1 Why and where

The kit's editor model (`KIT:docs/editor-model.md`, `KIT:crates/archipel-kit/src/editor/model/`) is
deliberately language-poor: its `Language` struct is only comment prefixes, bracket pairs, quotes and
`close_curly_on_enter` (`model/settings.rs:223`). The doc lists what it cannot do without services:
"no re-indent on `}`, no syntax token boundaries for word stops", Smart Enter = Start New Line, Move
Statement = Move Line, Extend Selection is a plain-text chain, sticky lines and inlays are "set up by the
caller". `KIT:crates/archipel-kit/src/host.rs` is the right home: it already holds the host-side service
traits (`ActionManager`, `KeymapManager`, `PropertiesComponent`, `Clock`) with in-memory implementations
for tests and small apps, and the rule "no filesystem or app policy in the kit".

Constraints that shape the trait: immediate-mode (called every frame, so cheap, cached, never
blocking: slow answers use a poll style), pure-data results (the model applies `Edit`s inside its own
undo group), byte offsets (kit convention, `Edit.offset` is bytes) and `\n`-normalized text (the IDE
translates CRLF itself, IJ-ED-144), no globals, `&self` methods (interior mutability is the host's
business), a default implementation that reproduces today's plain-text behavior.

### 5.2 Proposed trait (add to `KIT:crates/archipel-kit/src/host.rs`, re-export from `host`)

```rust
/// What the kit's editor needs from the application's code-intelligence layer.
/// IntelliJ counterparts: Commenter/BraceMatcher/QuoteHandler (static data, already `Language`),
/// TypedHandlerDelegate, EnterHandlerDelegate, SmartEnterProcessor, LineIndentProvider,
/// ExtendWordSelectionHandler, FoldingBuilder, StickyLinesCollector, DocumentationProvider,
/// IdentifierHighlighterPass, BreadcrumbsProvider, HighlightVisitor/inspections.
///
/// Every method takes the document by reference plus its revision; an implementation keys its caches on
/// `(doc_id, revision)`. Results are valid for exactly that revision. Methods must return within about a
/// millisecond (answer from caches); long work happens on the host's side and shows up in later frames.
pub trait LanguageServices {
    // ---- identity and static data ------------------------------------------------------------
    /// The language of the document (comment prefixes, pairs, quotes), possibly different at `offset`
    /// inside an injected fragment (JS in HTML, code in Markdown). IntelliJ `Language`/`Commenter`.
    fn language_at(&self, doc: &DocView<'_>, offset: usize) -> Rc<Language>;

    // ---- word and token boundaries -----------------------------------------------------------
    /// Token class at `offset` (identifier, keyword, string, escape, number, operator, comment,
    /// whitespace). Lets Ctrl+Arrow, double click and delete-word stop at syntax boundaries
    /// (string contents vs quotes, operators as one word, camel humps per setting).
    fn token_at(&self, doc: &DocView<'_>, offset: usize) -> Option<TokenInfo>;

    // ---- selection and navigation ------------------------------------------------------------
    /// Extend Selection: the smallest syntactic range strictly containing `range`. The kit keeps the
    /// history for Shrink Selection; the service is stateless.
    fn extend_selection(&self, doc: &DocView<'_>, range: Range<usize>) -> Option<Range<usize>>;
    /// Partner of the bracket/tag at `offset` (offsets of both delimiters), syntax-aware (ignores
    /// brackets in strings/comments).
    fn matching_brace(&self, doc: &DocView<'_>, offset: usize) -> Option<(usize, usize)>;
    /// Enclosing code block start/end for Ctrl+[ / Ctrl+].
    fn code_block(&self, doc: &DocView<'_>, offset: usize) -> Option<Range<usize>>;
    /// Breadcrumb path (outermost first) at `offset`.
    fn breadcrumbs(&self, doc: &DocView<'_>, offset: usize) -> Vec<Crumb>;

    // ---- structure for display ---------------------------------------------------------------
    /// Fold regions for the whole document (the kit's `FoldingModel` ingests them, keeps the user's
    /// collapsed state across revisions by matching start offsets and placeholders).
    fn fold_regions(&self, doc: &DocView<'_>) -> Vec<FoldSpec>;
    /// Sticky scopes for the first visible logical line: enclosing containers whose header lines have
    /// scrolled out of view, outermost first, at most `max`.
    fn sticky_scopes(&self, doc: &DocView<'_>, first_visible_line: usize, max: usize) -> Vec<StickyScope>;

    // ---- typing assistance (pure edit descriptions) ---------------------------------------------
    /// Before the typed character is inserted: overtype a closer, insert a pair, wrap, or decline.
    /// Token-aware (no pairing inside strings/comments where the language says so).
    fn before_typed(&self, doc: &DocView<'_>, caret: usize, typed: char) -> TypedDecision;
    /// After insertion: re-indent on a typed `}`/`else`/`case`, auto-close a tag, add a space, etc.
    /// `caret` is the position after the character.
    fn after_typed(&self, doc: &DocView<'_>, caret: usize, typed: char) -> Option<EditSet>;
    /// Enter: indentation of the new line, splitting `{}`, continuing line/doc comments. `None` = the
    /// kit's plain behavior.
    fn on_enter(&self, doc: &DocView<'_>, caret: usize, selection: Option<Range<usize>>) -> Option<EditSet>;
    /// Complete Current Statement (Ctrl+Shift+Enter).
    fn smart_enter(&self, doc: &DocView<'_>, caret: usize) -> Option<EditSet>;
    /// Move Statement/Element Up/Down. `None` = fall back to Move Line.
    fn move_statement(&self, doc: &DocView<'_>, range: Range<usize>, up: bool) -> Option<EditSet>;
    /// Indentation (in columns) for `line` as computed by the language (Auto-Indent Lines, paste
    /// reformat, Tab on an empty line); `None` = copy the previous line.
    fn indent_for_line(&self, doc: &DocView<'_>, line: usize) -> Option<usize>;

    // ---- decorations the editor paints (data, host decides policy) -------------------------------
    /// Highlights for the error stripe and squiggles (diagnostics/inspections), same type the
    /// component already takes (`RangeHighlight`).
    fn highlights(&self, doc: &DocView<'_>) -> Rc<[RangeHighlight]>;
    /// Highlight Usages of the symbol at the caret (read/write/text), updated by the host on caret moves.
    fn occurrences(&self, doc: &DocView<'_>, caret: usize) -> Rc<[Occurrence]>;
    /// Hover content for the pointer at `offset`; poll again next frame while `Pending`.
    fn hover(&self, doc: &DocView<'_>, offset: usize) -> HoverPoll;
    /// Optional tokenizer backed by the real parser, replacing the kit's line-based highlighters.
    fn syntax_highlighter(&self, doc: &DocView<'_>) -> Option<Rc<dyn SyntaxHighlighter>>;
}

/// A document and the identity under which the service caches its results.
pub struct DocView<'a> { pub doc: &'a Document, pub id: u64, pub revision: u64, pub path_hint: Option<&'a str> }
pub struct TokenInfo { pub class: TokenClass, pub range: Range<usize> }
#[non_exhaustive] pub enum TokenClass { Identifier, Keyword, String, StringEscape, Number, Operator, Punctuation, Comment, DocComment, Whitespace, Other }
pub struct Crumb { pub label: String, pub kind: CrumbKind, pub range: Range<usize> }
pub struct FoldSpec { pub range: Range<usize>, pub placeholder: String, pub collapsed_by_default: bool, pub kind: FoldKind, pub group: Option<u32> }
pub struct StickyScope { pub header: Range<usize>, pub start_line: usize, pub end_line: usize }
pub enum TypedDecision { Default, Overtype, InsertPair(char), Wrap { open: char, close: char }, Consume }
pub struct EditSet { pub edits: Vec<Edit>, pub caret: Option<usize>, pub selection: Option<Range<usize>> } // applied in the action's undo group
pub struct Occurrence { pub range: Range<usize>, pub access: Access } // Read | Write | Text
pub enum HoverPoll { None, Pending, Ready(HoverContent) }
pub struct HoverContent { pub range: Range<usize>, pub markdown: String, pub actions: Vec<HoverAction> }

/// The default: today's plain-text behavior (every method answers `None`/empty/`Default`).
pub struct NoLanguageServices;
impl LanguageServices for NoLanguageServices { /* all defaults */ }
```
Rust note: give every method a default body in the trait (returning `None`/empty/`Default`) so a host
can implement only what it has, and so new methods are not breaking changes (mark enums
`#[non_exhaustive]`, as the kit already does).

### 5.3 Required kit changes (small, additive)

1. `host.rs`: the trait and types above plus `NoLanguageServices`.
2. `model::Document` gets a `revision() -> u64` counter incremented on every edit (today it has none;
   `Edit` is the only change record, `model/document.rs:10`). `DocView` is built by the component.
3. `ActionContext` gains `pub services: Option<&'a dyn LanguageServices>` and `with_services(..)`
   (`model/action.rs:325`); `execute` consults it for: `Type` (before/after typed), `Enter`, `StartNewLine`,
   `CompleteStatement`, `MoveStatementUp/Down`, `SelectWord`/`UnSelectWord`, `PreviousWord`/`NextWord` stops,
   `CodeBlockStart/End`, `MatchBrace`, `Tab`/`UnindentSelection`/paste reindent (`indent_for_line`),
   `CommentByLineComment/BlockComment` (via `language_at`). Without services the model is unchanged
   (all existing tests stay valid; add a test double `ScriptedServices` in the kit for new tests).
4. `EditorComponent` takes `.language_services(&dyn LanguageServices)`; it feeds `fold_regions` into
   `FoldingModel`, paints `sticky_scopes` as a pinned header band (new paint piece; measure against
   Android Studio Islands per `docs/design-rules.md`), draws `occurrences`/`highlights`, shows `hover`
   through the existing tooltip delay (`HoverInfo`, `decorations.rs:178`), and uses
   `syntax_highlighter`.
5. `SyntaxHighlighter` (`editor/highlight.rs:125`) is line-based with a carry `state`; a parser-backed
   highlighter needs the line's offset. Add a defaulted method `highlight_range(&self, doc: &Document,
   range: Range<usize>, out: &mut Vec<Span>)` whose default calls `highlight_line`; the Archipel IDE
   implements `highlight_range`.

### 5.4 How Archipel IDE implements it

`crates/archipel-ide-ui/src/editor_language.rs` (WP4) implements `LanguageServices` over a handle to the
analysis host: each call builds `DocView -> (FileId, snapshot)` through a small per-frame cache keyed by
`(doc id, revision)`; the document is kept in sync by converting the kit's `Edit { offset, removed,
inserted }` stream into `archipel-ide-text` `ByteEdit`s (bytes in both; CRLF re-expansion handled at this
boundary). Mapping to the packages: `language_at`/`token_at`/`extend_selection`/`matching_brace`/
`code_block`/`breadcrumbs`/`fold_regions`/`sticky_scopes`/typed handlers/`on_enter`/`smart_enter`/
`move_statement`/`indent_for_line` come from WP4 (`archipel-ide-language` `service.rs`); `highlights` from the
WP3 `HighlightMarkup` (converted to `RangeHighlight`, severity map Error/Warning/WeakWarning/Hint ->
`HighlightSeverity::{Error,Warning,WeakWarning,Info}`); `occurrences` from WP5's highlight-usages
engine; `hover` from WP6 `DocumentationProvider` + LSP via WP8 (the poll state lives in the daemon's
`Interactive` lane); `syntax_highlighter` wraps `SyntaxTree::highlight` (`syntax.rs:156`).
Test: a conformance test in `archipel-ide-ui` drives the kit's `Editor` + `execute` with the Archipel IDE
services on small Java/Rust/Kotlin/JSON documents and asserts the editor-model "differences" list is
closed (re-indent on `}`, token word stops, smart enter, move statement, syntax extend selection).

---------------------------------------------------------------------------------------------------

## Appendix A. IntelliJ source index (tag idea/262.10968.63)

All under `IC:platform/` unless noted.
- VFS/document: `core-api/src/com/intellij/openapi/vfs/{VirtualFile,VirtualFileSystem}.java`; `core-api/.../newvfs/BulkFileListener.java`; `platform-impl/.../vfs/newvfs/persistent/{PersistentFS,FSRecordsImpl}.java`; `core-impl/.../vfs/impl/VirtualFileManagerImpl.java`; `core-api/.../editor/Document.java`; `lvcs-api`, `lvcs-impl`.
- PSI/language: `core-api/.../psi/{PsiElement,PsiReference,PsiPolyVariantReference,ResolveResult,PsiReferenceContributor,PsiReferenceProvider,PsiFileFactory,SmartPointerManager}.java`; `core-api/.../lang/{Language,ParserDefinition,PsiParser,LanguageExtension,Commenter,folding/*}.java`; `core-api/.../lexer/Lexer.java`; `core-api/.../openapi/fileTypes/{FileType,LanguageFileType}.java`; `editor-ui-api/.../fileTypes/{SyntaxHighlighter,SyntaxHighlighterFactory}.java`; `core-impl/.../psi/impl/{BlockSupportImpl,DiffLog,PsiModificationTrackerImpl}.java`; `core-impl/.../psi/impl/source/resolve/ResolveCache.java`; `core-api/.../psi/util/{CachedValue,CachedValuesManager,PsiModificationTracker}.java`; `core-api/.../psi/scope/PsiScopeProcessor.java`; `core-api/.../psi/search/GlobalSearchScope.java`.
- Indexing: `indexing-api/.../util/indexing/FileBasedIndexExtension.java`; `util/src/.../util/indexing/IndexExtension.java`; `indexing-api/.../psi/stubs/{StubIndex,StubIndexExtension}.java`; `core-api/.../psi/stubs/StubElement.java`; `core-impl/.../psi/tree/IStubFileElementType.java`; `indexing-impl/.../psi/stubs/StubUpdatingIndex.java`; `indexing-impl/.../psi/impl/cache/impl/id/{IdIndex,IdIndexEntry}.java`; `indexing-impl/.../find/ngrams/TrigramIndex.java`; `indexing-api/.../psi/search/{FilenameIndex,FileTypeIndex,TodoPattern}.java`; `todo/backend/.../todo/TodoIndex.java`; `indexing-impl/.../util/indexing/` (storage, scanning, diagnostics); `core-api/.../project/{DumbService.kt,DumbAware,IndexNotReadyException}`.
- Daemon: `analysis-impl/.../codeInsight/daemon/impl/{GeneralHighlightingPass,HighlightInfo,HighlightInfoType,Divider,AnnotatorRunner,UpdateHighlightersUtil,HighlightInfoUpdaterImpl,InspectionVisitorOptimizer,SeverityRegistrar}.java`; `analysis-impl/.../codeHighlighting/{Pass,TextEditorHighlightingPassRegistrar}.java`; `lang-impl/.../daemon/impl/{PassExecutorService,LocalInspectionsPass,ExternalToolPass,LineMarkersPass,ShowIntentionsPass,IdentifierHighlighterPass}.java`; `analysis-impl/.../daemon/DaemonCodeAnalyzerSettings.java`; `analysis-api/.../lang/annotation/{Annotator,ExternalAnnotator,HighlightSeverity}.java`.
- Inspections/profiles: `analysis-api/.../codeInspection/{LocalInspectionTool,GlobalInspectionTool,InspectionProfileEntry,ProblemsHolder,ProblemHighlightType,QuickFix,LocalQuickFix,reference/EntryPoint}.java`; `analysis-impl/.../codeInspection/ex/InspectionProfileImpl.java`; Java catalog: `IC:java/java-backend/resources/META-INF/{Inspections,InspectionGadgets}.xml`, descriptions in `IC:java/java-impl/resources/inspectionDescriptions`, Java error highlighting `IC:java/java-analysis-impl/.../daemon/impl/analysis/HighlightVisitorImpl.java`, unused symbols `.../daemon/impl/UnusedSymbolUtil.java`.
- Intentions/mod commands: `analysis-api/.../codeInsight/intention/{IntentionAction,IntentionManager,PriorityAction}.java`; `lang-impl/.../intention/impl/{CachedIntentions,IntentionGroup,DefaultIntentionsOrderProvider,IntentionsOrderProvider}.java`; `analysis-api/.../modcommand/*`.
- Completion: `analysis-api/.../codeInsight/completion/{CompletionContributor,CompletionService,CompletionSorter,CompletionWeigher,PrioritizedLookupElement}.java`; `analysis-api/.../lookup/{LookupElement,LookupArranger,LookupElementWeigher}.java`; `lang-impl/.../completion/CompletionProgressIndicator.java`; weigher registration `analysis-impl/resources/META-INF/AnalysisImpl.xml` (lines 87-91) and `platform-resources/src/META-INF/LangExtensions.xml` (lines 644-678).
- Find usages/navigation: `lang-impl/.../find/findUsages/{FindUsagesHandler,FindUsagesHandlerFactory}.java`; `core-api/.../usageView/UsageInfo.java`; `usageView/.../usages/UsageViewManager.java`; `indexing-api/.../psi/search/searches/{ReferencesSearch,DefinitionsScopedSearch}.java`; `lang-impl/.../highlighting/HighlightUsagesHandlerBase.java`; `lang-impl/.../navigation/{GotoImplementationHandler,ImplementationSearcher}.java`, `navigation/actions/{GotoDeclarationOrUsageHandler2.kt,TypeDeclarationProvider}.java`; `analysis-api/.../navigation/actions/GotoDeclarationHandler.java`; `lang-api/.../navigation/{ChooseByNameContributor,GotoClassContributor}.java`; `lang-api/.../ide/util/gotoByName/ChooseByNameModel.java`; `platform-resources/src/META-INF/SearchEverywhereCodeInsightContributors.xml`.
- Structure/editor: `editor-ui-api/.../ide/structureView/*`; `foldings/src/.../CodeFoldingPass.java`; `lang-api/.../editorActions/{ExtendWordSelectionHandler,TypedHandlerDelegate,BackspaceHandlerDelegate,smartEnter/SmartEnterProcessor}.java`; `lang-impl/.../editorActions/{SelectWordHandler,QuoteHandler,wordSelection/*,enter/EnterHandlerDelegate}.java`; `analysis-api/.../lang/PairedBraceMatcher.java`; `lang-impl/.../highlighting/BraceMatchingUtil.java`; `platform-impl/.../editor/impl/stickyLines/*`; `editor-ui-api/.../ui/breadcrumbs/BreadcrumbsProvider.java`.
- Format/code style: `code-style-api/.../formatting/{FormattingModelBuilder,Block}.java`; `code-style-api/.../psi/codeStyle/{CodeStyleSettings,CommonCodeStyleSettings,lineIndent/LineIndentProvider}.java`; `lang-api/.../codeStyle/LanguageCodeStyleSettingsProvider.java`; `code-style-impl`.
- Templates: `analysis-api/.../codeInsight/template/{Macro,TemplateContextType,LiveTemplateContextBean}.java`; `analysis-impl/.../template/impl/TemplateImpl.java`; `lang-impl/.../template/impl/TemplateSettings.java`, `.../template/postfix/templates/*`; bundled sets `IC:java/java-backend/resources/liveTemplates/`, `IC:plugins/kotlin/code-insight/live-templates-k2/resources/liveTemplates/`.
- Refactoring: `refactoring/.../lang/refactoring/RefactoringSupportProvider.java`; `refactoring/.../refactoring/{BaseRefactoringProcessor,RefactoringActionHandler,ConflictsDialogBase}.java`, `listeners/*`, `changeSignature/*`, `extractMethod/*`; `lang-impl/.../refactoring/{rename/*,safeDelete/SafeDeleteProcessor,move/MoveHandler}.java`.
- Hints/code vision/parameter info/docs/spelling: `lang-api/.../codeInsight/hints/declarative/InlayHintsProvider.kt`; `lang-impl/.../codeInsight/codeVision/CodeVisionProvider.kt`; `lang-impl/.../hints/InlayHintsPass.kt`; `lang-api/.../daemon/LineMarkerProvider.java`; `lang-api/.../lang/parameterInfo/*`; `analysis-api/.../lang/documentation/DocumentationProvider.java`; `lang-impl/.../platform/backend/documentation/DocumentationTargetProvider.java`; `IC:spellchecker/src/com/intellij/spellchecker/{inspections,tokenizer,quickfixes}`.
- IntelliJ's LSP client as provider precedent: `lsp-impl/resources/intellij.platform.lsp.impl.xml`, `lsp-impl/src/com/intellij/platform/lsp/impl/features/*`.
- Default tooling data: `platform-resources-en/src/{inspectionDescriptions,intentionDescriptions}`, `lang-impl/resources/intentionDescriptions`, Java intention/inspection descriptions under `IC:java/java-impl/resources/`.

## Appendix B. Starter inspection and intention catalog (Archipel-authored)

Tier A (all grammars, no resolve; ship in WP3): syntax error / missing delimiter (exists as problems),
duplicate key (JSON/YAML/TOML/properties/XML attribute), trailing/mixed indentation, empty block or
catch body, unreachable code after return/break/continue/throw (statement kinds), unused local variable and
unused import within the file (needs G1 locals), shadowed variable, assignment in condition, constant
condition (`if (true)`), redundant parentheses, self-assignment, duplicate branch bodies, TODO/FIXME-marked
comments (informational), typo (spelling, weak), very long line.
Tier B (Java, G2/G3; names of comparable IntelliJ inspections present in `Inspections.xml`, used only as
a scope reference, implement with our own logic and messages): unused declaration/assignment
(`UnusedAssignment`), redundant cast and type arguments (`RedundantCast`, `RedundantTypeArguments`),
silly assignment, static access via instance (`AccessStaticViaInstance`), field/parameter can be local
(`FieldCanBeLocal`, `ParameterCanBeLocal`), deprecated and marked-for-removal usage (`Deprecation`,
`MarkedForRemoval`), Optional misuse (`OptionalGetWithoutIsPresent`, `OptionalAssignedToNull`),
wrong package statement (`WrongPackageStatement`), duplicate throws, unreachable catch, numeric overflow,
suspicious collection method calls (`SuspiciousMethodCalls`), overwritten map key (`OverwrittenKey`),
text-block migration, comparator simplifications, Javadoc reference/declaration checks. Real dataflow
issues (`DataFlowIssue`, `ConstantValue`, nullability) are G4 and out of scope for the first cut.
Scale reference only: 180 + 680 registered Java local inspections in IntelliJ (159 + 271 enabled by default).
Intentions (generic, tree-based): see 2.6.

## Appendix C. Risks and open questions

1. Tree-sitter grammar quality varies (error recovery around incomplete code, Kotlin/Groovy grammars are
   weaker). The dummy-identifier completion trick and `incomplete_code` resolve flag are mandatory
   mitigations; track grammar bugs in the golden suites rather than patching grammars.
2. Persistent index engine choice (`redb` vs custom segments) is a decision for WP2's first iteration;
   the `IndexStore` trait isolates it. Reject engines needing C toolchains (offline/packaging policy).
3. `ModCommand` as the only edit path means LSP `WorkspaceEdit` conversion must be exact (including
   resource operations); keep the existing `workspace_edit.rs` tests as the oracle.
4. Native resolve false positives are worse than missing features: D6 (confidence + tier gating) and the
   differential gate are the safety net; never unmute "unresolved reference" below G2.
5. Memory: closed-file syntax trees are evicted; stubs live on disk. Watch tree-sitter parse of very
   large generated files (skip above a size/node budget and mark `Skipped`).
6. Licensing: spelling dictionaries, template text, inspection messages and icons must be authored
   fresh or come from permissively licensed sources (clean-room rule in `AGENTS.md`); do not extract
   IntelliJ bundled resources even though the source tree is readable.
7. Rename churn: crate names change to `archipel-ide-*`; WP1 should centralize crate names in one
   `[workspace.dependencies]` block per new crate so the rename is a single diff.
8. Parity table accuracy: several editor-basics rows (IJ-ED-003, 012, 031-034, 081) appear stale against
   the roadmap and code (`archipel-ide-text/src/ops.rs` has pairing, newline indent, word movement). Each owning
   package must re-verify and correct its rows before claiming progress.
9. Settings/UI for profiles, code style, spelling, TODO patterns use `archipel-ide-settings` registry
   (`docs/architecture.md` "Settings extension point"); do not invent a second settings mechanism.
