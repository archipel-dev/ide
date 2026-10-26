# Native code-analysis pipeline

Archipel IDE analyzes code natively, in the shape of the IntelliJ Platform: a file system
feeds a document model, a syntax tree is built per file, references resolve through caches
invalidated by modification stamps, indexes answer global questions, a daemon runs ordered
highlighting passes, fixes and refactorings are data, and every capability is an extension
point keyed by language. A language server is one optional provider behind the same
extension points, not the semantic architecture.

This page describes the frozen contracts of wave 0 (crate `archipel-ide-analysis` and the
syntax layer of `archipel-ide-language`) and how the later packages plug into them.

## Crates and layering

```
archipel-ide-ui        (egui shell; calls IdeState::poll_analysis once per frame)
archipel-ide-core      analysis_host.rs (IdeState.analysis), analysis_modules.rs (one line per package)
  |
  +-- archipel-ide-analysis   host/snapshot, VFS facade, jobs, cancellation, stamps, cached values,
  |                           extension contracts of every package, mocks and fixtures
  +-- archipel-ide-language   LanguageDef registry, element types, token sets, SyntaxSnapshot/SyntaxNode,
  |                           Anchor, InjectedFragment, ServiceRegistry, generic query features
  +-- archipel-ide-text       rope, edit journal (ByteEdit), EditMap
```

Later packages add crates beside `archipel-ide-analysis`: `-index` (WP2), `-daemon` (WP3),
`-resolve` (WP5), completion/format/templates (WP6), `-refactor` (WP7), `-lsp-provider` (WP8)
and `-lang-*` tiers (WP9). None of them imports `egui`; none edits `IdeState` or
`archipel-ide-core/src/lib.rs`: each fills its entry in `analysis_modules.rs` (services into
the `ServiceRegistry`, commands into the command registry) and talks to the others only
through the traits of `archipel-ide-analysis`.

## The pipeline

1. **Files.** `FileTable` interns paths to dense `FileId`s; ids survive renames and are never
   reused. The `Vfs` trait reads contents (`DiskVfs`: 2 MiB cap, binary files rejected;
   `MemVfs` for tests). Watcher batches become `VfsChange { created, deleted, moved, modified }`
   (`VfsChange::classify`). Open buffers shadow the disk (`DocumentOverlay`).
2. **Documents.** The text engine's journal (`ByteEdit`s from `TextDocument::take_edits`) is the
   single source of truth for what changed. Revisions (`FileRev` = `TextDocument::revision`)
   are compared, never text.
3. **Syntax ("PSI").** `SyntaxSnapshot` is an immutable parse of one file revision (one `Arc`:
   tree, text, language, lazily computed errors and injected fragments). `SyntaxNode` is a `Copy`
   handle with navigation (`parent`, `children`, `child_by_field`, `find_leaf_at`, `covering`,
   `preorder`, `name_node`). `ElementType` is the grammar's kind id; `TokenSet` a bitset over a
   language's kinds; each `LanguageDef` declares its token sets by kind name (`comments`,
   `string_literals`, `identifiers`, `brackets`, `statement_like`, `declaration_like`,
   `code_block_like`; a registry test checks every name exists in its grammar). Incremental
   reparse (`SyntaxTree::reparse`) returns `Reparse { edited, changed, highlight }`; the host
   publishes `AnalysisEvent::SyntaxChanged { file, rev, ranges }`.
4. **Anchors.** Tree-sitter nodes are not stable, so persistent references are `Anchor`s (file,
   language, kind, range, name hash, revision). `Anchor::resolve(snapshot, edit_map)` maps the
   range through the `EditMap` of the edits since the anchor's revision and accepts only a node
   of the same kind (and name, for named nodes), falling back to the nearest same-kind,
   same-name node; it never returns a node of another kind (property-tested).
5. **EditMap.** The composition of any number of edits as a normalized monotone step function
   (shift pieces and collapse pieces). `map` is O(log pieces); typing a word is one piece;
   `compose` equals mapping through both maps in turn (property-tested against direct mapping).
   `Snapshot::edits_since(file, rev)` composes the host's per-file journal.
6. **Injections.** `SyntaxSnapshot::fragments()` / `fragment_at(offset)` return
   `InjectedFragment`s, each with its own child snapshot. Injected trees are parsed over the host
   text with included ranges, so fragment offsets are host offsets (identity offset map).
7. **Host and snapshots.** `AnalysisHost` (UI thread) is the only mutator:
   `apply(Change::{Open, Edit, Reset, Close, Vfs, SettingsChanged, IndexCommit, DumbMode,
   InterfaceComputed})`. Each change bumps stamps, records the edit journal, publishes events and
   revokes jobs. `snapshot()` is O(1); a `Snapshot` sees one consistent world (open buffers and
   their trees, disk files parsed lazily into a 64-entry LRU, stamps, index readiness, the service
   registry, the value cache) and never blocks `apply` (copy on write).
8. **Stamps and cached values.** `Dep` is one input with its stamp: `File(file, rev)`,
   `Interface(file, epoch)`, `LanguageInterface`, `Index`, `DumbMode`, `Settings`, `Never`.
   `Db::cached(slot, key, cancel, f)` returns the stored value while every recorded dep is
   current, else recomputes (`f` records deps through a `DepCollector`; nested reads propagate
   deps). Validation is lazy, on read. Early cutoff: after an edit the host recomputes the file's
   interface hash (declarations without positions, `InterfaceSummarizer`, default from the tags
   structure; debounced 300 ms, on a background job); the `InterfaceEpoch` bumps only when the
   hash changed, so an edit inside a function body leaves every `Interface`-dependent value of
   other files valid. Storage is one byte-weighed LRU per host (64 MiB default) with a
   memory-pressure hook (`HostConfig::memory_limit`, Linux RSS).
9. **Extension registry.** `ServiceRegistry` stores trait objects per slot (`dyn Trait`) and
   language. `get` = first of (language, base-language chain, "any"); `all` = union in that order,
   deduplicated by service id (the most specific registration wins), ordered by `Order::{First,
   Last, Before(id), After(id)}` then registration order (stable topological sort; cycles keep
   registration order). Registrations carry `dumb_aware`; `all_available(lang, dumb)` filters
   while indexes are not ready. Every `LanguageDef` has a `services` hook; generic tree-sitter
   services (folding, structure, extend selection, braces, interface hash) are registered for
   "any".
10. **Edits as data.** Fixes, intentions and refactorings return a `ModCommand` (sealed enum:
    `UpdateFile { file, edits, expected_rev }` with optimistic concurrency, create/delete/move,
    navigate, rename, template, conflicts, choose, message, ...). `preview` applies the edits to a
    copy; one `ModCommandExecutor` applies commands in one undo group.

## Extension contracts by package

| Package | Module | Traits and types |
|---|---|---|
| WP2 index | `index` | `FileIndex` (+ `IndexKey`/`IndexValue`), `IndexInput`, `IndexQuery`, `IndexStore` (+ `MemIndexStore`), `IndexState`, `IndexNotReady`, `IndexPriority`, `Snapshot::require::<I>()` |
| WP3 daemon | `daemon`, `inspect`, `intention`, `highlight` | `Pass`, `PassApply`, `HighlightMarkup`, `Annotator`, `ExternalAnnotator` (+ object-safe `DynExternalAnnotator`), `LocalInspection`, `GlobalInspection`, `InspectionSuppressor`, `EntryPoint`, `IntentionAction`, `QuickFix`, Alt+Enter ordering (`IntentionGroup`, `Priority`, `sort_actions`), `Severity`, `HighlightInfo` |
| WP4 structure | `editing` | `Commenter`, `BraceMatcher`, `QuoteHandler`, `FoldingBuilder`, `StructureProvider`, `ExtendSelectionHandler`, `Indenter`, `TypedHandler`, `SmartEnterProcessor`, `MoveStatementHandler`, `StickyScopesProvider`, `BreadcrumbsProvider` |
| WP5 resolve | `resolve`, `target` | `Reference`, `ReferenceProvider`, `Resolver`, `UsageSearcher`, `ReadWriteAccessDetector`, `SearchScope`, `FileSet`, `Target`, `Confidence`, `Provenance`, `Usage` |
| WP6 assistance | `assist` | `CompletionContributor`, `CompletionWeigher`, `DocumentationProvider`, `ParameterInfoProvider`, `FormattingService`, `TemplateContextType` |
| WP7 refactoring | `refactor`, `modcommand` | `RefactoringSupport`, `RefactoringListener`, `RefactoringPlan`, `ModCommand`, `ModCommandExecutor` |
| WP8 LSP provider | `provider` | `AnalysisProvider`, `Merger`, `ProviderMode`, `ProviderSettings`, `Feature`, `merge_dedupe` |

`archipel_ide_analysis::testing` has an in-memory implementation of every trait (registered by
`register_mocks`) and a `Fixture` project (Java, Rust and Markdown files on a `MemVfs`), so each
package develops against the contracts before the real implementations exist
(`tests/contracts.rs` exercises all of them).

## Threading and cancellation

- The UI thread owns the `AnalysisHost`. Background work gets a `Snapshot` and a `CancelToken`
  and returns `Result<T, Cancelled>`; `Cancelled` is a value, never a panic and never shown.
- `Executor`: `std::thread` pool, `max(2, cores - 1)` workers with three queues (`Interactive`,
  `Daemon`, `Background`) plus one interactive-only lane, so completion and hover never queue
  behind indexing. Created lazily on the first job.
- `AnalysisHost::spawn(JobSpec { priority, revoke, coalesce }, task)`: `Revoke::OnAnyChangeOf(
  All | Files | Settings)` cancels the job when a matching change is applied; a job with the same
  coalescing key cancels its predecessor. Results come back through a `JobHandle` or an
  `on_done` callback run by `AnalysisHost::poll` on the UI thread.
- Cancellation points: `cancel.check()?`, or a `Checkpoint` in hot loops that polls every 64
  ticks (`cancel::walk` for tree walks); a revoked job stops within 256 checks (tested).
- Typed handlers needing a fresh tree call `AnalysisHost::sync_syntax(file)`.
- In the app, `IdeState::poll_analysis` (once per frame) mirrors `IdeState::documents` into the
  host: the editor's own incremental tree is passed as `SyntaxUpdate` (no second parse), with
  the changed ranges drained from `LanguageAnalysis::take_syntax_changes`; plain-text buffers are
  copied after a 300 ms typing pause.

## Indexing (`archipel-ide-index`)

The index package implements the contracts of `analysis::index` and owns everything behind
`Snapshot::require::<I>()`.

- **Storage.** `IndexStore` has two backends: `MemIndexStore`/`MemStore` and `RedbStore`
  (`redb`, MIT OR Apache-2.0, pure Rust). One database file per index (`<id>.redb`) plus
  `files.redb` with the file table, under `<cache>/archipel/index/<project hash>/`. An index
  stores `key ++ file id -> value` (inverted map), `file id -> keys` (forward map, for removal),
  `file id -> content hash` and its version and completeness. A version or settings change drops
  one index only; a damaged or truncated file is deleted and rebuilt, never a startup failure; an
  open file is never replaced (a second window falls back to memory). Writes are buffered and
  applied in one transaction per `commit` (batch), readers see the last commit.
- **Indexes.** `FileName`, `FileType`, `Words` (hash of an identifier -> mask of code, comment,
  string, literal), `Trigram` (text only), `Todo` (pattern -> occurrences in comments), `Stubs`
  (file -> serialized declarations from the tags query: kind, flags, name, ranges, parent,
  extras such as `extends`) and the stub-derived `Stub:ShortName`, `Stub:Qualified`,
  `Stub:SuperTypes`, `Stub:Members`. One parse and one stub tree per file feed all of them.
  `StubBuilders` lets a language tier replace the generic builder; `StubInterface` (the interface
  hash of the stub tree) replaces the structure-based default of the early-cutoff epoch.
- **Scheduler** (`IndexService`, UI thread, `poll` per frame; work runs as `Background` host jobs):
  scan and name indexes first, then open and changed files, source files, the rest; every file is
  read once, hashed (content plus language) and fed only to indexes that have not seen this
  content; batches commit after ~50 ms or 48 files and publish `Change::IndexCommit`. Text-only
  indexes (`Trigram`) are built in a second pass once everything else is ready and do not keep the
  IDE in dumb mode. Removals run alone so a late batch cannot resurrect a deleted file.
- **Dumb mode** is per index: `IndexState` is `Absent`, `Building {done,total}`, `Ready` or
  `Failed`, `Snapshot::require` returns `IndexNotReady` until `Ready`, and `Change::DumbMode`
  is on while an index that is not `dumb_aware` is not ready. Readers (`readers.rs`) return
  `Result<_, IndexNotReady>` so features degrade individually.
- **Overlay.** Unsaved buffers are indexed into an in-memory layer (`OverlayStore`) that shadows
  the persistent entries of that file; closing the buffer reconciles with the disk.
- **Local History** (`history`): content-addressed blob store (SHA-256, deflate) and a per-path
  revision log on `redb`, retention by age and size. The host records revisions before it
  overwrites a file on save and when an external change is applied.

## Native and LSP coexistence

The LSP client (`archipel-ide-lsp`) stays a transport. WP8 wraps it as adapters implementing the
traits above, registered for the "any" language with `Order::Last`, so a language's native
services win. Per language and feature, `ProviderMode` (`Auto`, `Native`, `Lsp`, `Both`) decides
who answers: `Auto` uses native when the language's `CompletenessTier` reaches the feature's
tier (`Feature::native_tier`) and the LSP otherwise; `Both` merges (`Merger`, `merge_dedupe`) and
logs differences. Resolve asks native first and falls back to the LSP unless native was `Exact`
(`provider::needs_fallback`). Every semantic answer carries a `Confidence`; tools that need exact
resolution are muted below their `min_tier`.

## Graduation ladder

Each `LanguageDef` declares a `CompletenessTier`:

| Tier | Native capability | Turns on |
|---|---|---|
| G0 | syntax (tree-sitter + queries) | highlight, folding, structure, braces, selection, comments, syntax errors |
| G1 | file-local resolve (`locals.scm`) | local usages, highlight usages, local rename, unused-local inspections |
| G2 | project name resolution | cross-file navigation, Find Usages, "unresolved reference", nominal rename |
| G3 | types, members, inheritance | implementations, member completion, override/implement |
| G4 | inference, data flow | data-flow inspections, exact refactorings |

All languages start at G0. A language moves up one feature at a time: a golden corpus under
`crates/archipel-ide-lang-<x>/tests/corpus/`, a differential test against recorded language-server
transcripts, and gate numbers (resolve agreement >= 98 % where native claims `Exact`, zero false
"unresolved", usage recall >= 99 %, latency budgets). Passing flips the default `ProviderMode` for
that feature; demotion is a one-line registry change.

## Budgets

| Operation | Budget | Where measured |
|---|---|---|
| keystroke to frame, 50k-line file | < 16 ms p95 | `cargo bench -p archipel-ide-ui --bench typing` (PERF-008) |
| incremental reparse + changed ranges | p95 < 2 ms | `cargo bench -p archipel-ide-analysis --bench analysis` |
| host `apply(Edit)` + snapshot | O(open files), no text copy | same bench |
| caches | 64 MiB LRU | `HostConfig::cache_bytes` |
| syntax trees | open files + 64 closed | `HostConfig::closed_trees` |
