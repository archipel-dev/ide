# Archipel IDE platform core: specification index

Status: specification, clean-room. Written by the **specification team** from the IntelliJ IDEA Community sources; to be implemented by a
separate team that reads **only these documents** (and the Archipel docs they link), never JetBrains code.

Scope: the platform core **under** code analysis: virtual file system and documents, the PSI model and references, stubs and indexes, dumb mode,
caching and modification tracking, concurrency and cancellation, project structure as analysis sees it (file index, search scopes), and the extension points
a language plugin implements. It supersedes and deepens the platform parts of `docs/analysis/code-intelligence-spec.md` (WP1/WP2 and parts of WP3/WP5);
disagreements are listed in section 5.

## 1. Documents

| File | Topic | ID prefix | MUST | SHOULD | LATER | Total |
|---|---|---|---|---|---|---|
| [01-vfs-and-documents.md](01-vfs-and-documents.md) | VFS, documents, commit model, file types, language detection, external-change conflicts | `PLAT-VFS` | 66 | 15 | 1 | 82 |
| [02-psi-model.md](02-psi-model.md) | Tree vs elements, parsing contract, incremental reparse, events, validity, smart pointers, navigation elements, references and resolve | `PLAT-PSI` | 91 | 19 | 1 | 111 |
| [03-stubs-and-indexes.md](03-stubs-and-indexes.md) | Index contract, indexable files, update rules, stubs and stub indexes, Java indexes, word/name indexes, `find_class` and short-name cache | `PLAT-IDX` | 60 | 11 | 3 | 74 |
| [04-dumb-mode.md](04-dumb-mode.md) | Dumb/smart state machine, dumb tasks, dumb-aware contract, access types, alternative resolve, behavior table with user-visible texts | `PLAT-DUMB` | 51 | 8 | 1 | 60 |
| [05-caching-and-modification-tracking.md](05-caching-and-modification-tracking.md) | Trackers, cached values, recursion guard, tracker catalog, out-of-code-block history | `PLAT-CACHE` | 43 | 11 | 0 | 54 |
| [06-concurrency-and-cancellation.md](06-concurrency-and-cancellation.md) | Read/write sections, cancellation, non-blocking read actions, progress, parallel read work, mapping to the snapshot model | `PLAT-CONC` | 35 | 7 | 0 | 42 |
| [07-project-structure-and-scopes.md](07-project-structure-and-scopes.md) | File index semantics, order enumeration, every search scope exactly, test vs production, library classes vs sources, SDK | `PLAT-SCOPE` | 57 | 15 | 0 | 72 |
| [08-language-extension-points.md](08-language-extension-points.md) | Language identity, extension lookup rules, parser definition, lexer, highlighter, annotator, folding, editing helpers, navigation services, JVM contributions | `PLAT-LANG` | 48 | 16 | 0 | 64 |
| [09-saving-and-autosave.md](09-saving-and-autosave.md) | Autosave triggers and settings, save points before run/build/VCS/close, save algorithm, vetoes, failures, read-only and conflicts, trailing spaces and caret line, why analysis does not need saving | `PLAT-SAVE` | 42 | 17 | 2 | 61 |
| **Total** | | | **493** | **119** | **8** | **620** |

(Counts are of requirement rows with an explicit priority; they are regenerated from the files, not by hand.)

## 2. How to read and use the requirements

* **IDs are stable.** `PLAT-<AREA>-<NNN>`. Never renumber. A removed requirement keeps its number and is marked `RETIRED`. New requirements take the next free number of their file.
* **Priority** is relative to an IDE that must do **Java, Kotlin and Spring** well first:
  * `MUST`: needed for the first release of code intelligence on a Spring Boot project;
  * `SHOULD`: needed for parity with the reference, scheduled after the MUSTs; the architecture must not make it impossible;
  * `LATER`: described so the design leaves room (remote, shared indexes, dynamic plugin unload, symbol API).
* **Acceptance** is one line, written so that it can become a test named `plat_<area>_<nnn>` (for example `plat_psi_051`). A row without a runnable acceptance in its own file is a defect; report it.
* **Numbers.** A number inside a requirement is either (a) read from the reference (default limits, thresholds, severities) or (b) explicitly called a **budget** or **design target**, meaning an Archipel target that is not a reference fact. Budgets are enforced by benchmarks, not by the platform.
* **Provenance notes** are given where a requirement is Archipel's own design, or where the Spring plugin (not in the Community source tree) is concerned: those are marked "design requirement".
* **Source citations** (`ic: path`) name the reference files read for each document, so reviewers can audit the specification. No code is included. Short user-visible message strings are quoted because they are facts of observable behavior.
* **"Non-binding notes for implementers"** in each file are advice, not requirements.

### Reference version

The source studied is the sparse clone of `JetBrains/intellij-community` at `/home/user/ic-src` (commit `e863da0`, build `271.SNAPSHOT`, dated 2026-10-27).
The earlier Archipel analysis cites the tag `idea/262.10968.63`. Where behavior differs between the two (notably: the out-of-code-block counter is gone, stub definitions are moving to a registry), the documents say so.
Behavior that could not be confirmed by reading the source is listed under "Open questions" in the relevant file.

## 3. Dependency graph and suggested order

```
01 VFS + documents ──► 02 PSI ──► 03 stubs + indexes ──► 04 dumb mode
        │                │               │                    │
        │                ▼               ▼                    ▼
        └──────────► 05 caching ◄── 07 scopes ◄──── (project model, docs/architecture/project-model.md)
                         │
                         ▼
                   06 concurrency  ──►  08 language extension points (consumes all)
```

Suggested build order for the implementation team: **01 (file table, events, commit)**, **06 (cancellation type and job executor)**, **02 (tree promises, smart pointers)**, **05 (trackers and cached values)**, **07 (file index and scopes)**, **03 (index store, stubs, Java indexes)**, **04 (dumb state and behavior table)**, **08 (registry and the first language)**.

## 4. Findings the implementation team should not miss

Each finding states the consequence; the numbers refer to requirements.

1. **The reference dropped its fine-grained "out-of-code-block" tracker.** In the studied version there is one project PSI modification count plus per-language counts; the old structure-only key is a deprecated alias of the global one and is called the "wrong way to optimize". Java class and package caches are cleared on *every* PSI change. Start with the coarse scheme; a finer "interface hash" scheme is allowed only with the property test of PLAT-CACHE-061. (05)
2. **Dumb mode is global and opt-in, not per index.** One state per project, flipped inside an exclusive section; every index read in dumb mode fails unless an access type is declared; the daemon restarts on entering *and* leaving; every transition bumps the modification count. Thresholds that start dumb mode: 20 changed files, 1,048,576 bytes, or 100 ms of counting. User-visible texts are listed verbatim. (04, PLAT-IDX-031/032)
3. **Stubs hold text, not resolved types.** Indexes are keyed by *unresolved short names* (the super-class index stores `List` for `java.util.List`); an entry is `(file, stub-node ids)`. Every consumer verifies by resolve. A stub-based design is only viable if stubs contain the rich declaration data of 03 section 5.1 (types as text, modifiers, annotations with arguments, constant initializers), because **class files have no tree**. A generic tags-based stub is not enough for JVM languages. (03)
4. **Resolve uses class files; sources are for navigation.** Module scopes contain library **classes** roots only. The compiled class maps to its source through the source-file name recorded in the class file, the package path, and the library source roots (libraries sorted by name, then SDKs). A class-file reader (and a JRT/jimage reader for the JDK) is a MUST. (02 PLAT-PSI-090..095, 07 PLAT-SCOPE-064/080)
5. **Scope semantics are exact and surprising.** Membership is decided by the file's single *nearest root* (source root or library classes root); module content that is not under a source root is not in `module_scope`. The `TESTS` option means "tests included", not "tests only". Compile classpaths follow only exported entries beyond the first level. The "dependents" scopes are by module *content*, not source roots, and include dependents' tests. `compare` returns classpath rank and is what `find_class` uses to pick the winner. Scopes are **values**: they are cache keys. (07)
6. **PSI is lossless, with whitespace/comment leaves, edge-binding and error elements.** The reference keeps identity of unchanged elements across reparse via a tree diff, and offers smart pointers. A tree-sitter tree needs synthesized whitespace/comment leaves and a binder layer to meet PLAT-PSI-001..003; identity can be replaced by anchors only if every API that hands out element handles says so. (02)
7. **Commit model.** Edits go to the document; the tree catches up by an asynchronous commit; readers use the *last committed text*; typing handlers must not force a commit; PSI edits require a synchronous commit first and are written back as one undoable change. (01, 02, 08)
8. **Cancellation is a first-class outcome, not an error.** Pending writes cancel readers; non-blocking read actions re-run (so they must be idempotent); results are delivered on the UI thread only if nothing changed in between; parallel searches resume with the unprocessed items. Never map cancellation to "not found" or log it. (06)
9. **Indexes see unsaved text.** Before any query the platform applies pending dirty files and an in-memory overlay of unsaved/uncommitted documents; stamps combine content, index version and per-language sub-version so a Kotlin stub version bump re-indexes only Kotlin. A corrupt index is cleared and rebuilt; startup never fails. (03)
10. **File type and language detection has a fixed order** (overrides, pinned type, special types, exact name, case-insensitive exact name, wildcard (longest first), extension, content detectors, hashbang, text/binary), **size limits** (about 2.5 MB "intelligence" limit, 20 MB content-loading limit) and a **charset order** (declared, BOM, valid UTF-8, explicit setting, default). External change versus unsaved edits has a documented conflict flow with fixed dialog texts. (01)
11. **Reference contributors have a merge rule**: highest priority group wins; lower groups only where ranges do not intersect. Languages must opt in to contributed references. (02 PLAT-PSI-110..113)
12. **Library duplicates are normal**: `find_class` returns the earliest on the classpath, `find_classes` returns all; class files in directories that are not valid package names are rejected; results are cached per scope and cleared on every PSI change. (03 PLAT-IDX-090..097)
13. **Language extensions are shared, stateless, and looked up through the base-language chain**; single-instance extension points take the first registered, multi-instance ones run all; errors in one service must not break others. (08)
14. **Use scope rules for Java members** (private: top-level class; package-private: package; public narrowed by class use scope) decide how cheap Find Usages is; implement them early. (02 PLAT-PSI-132)

## 5. Disagreements with `docs/analysis/code-intelligence-spec.md`

Each row says whether the earlier statement is *superseded*, *accepted with a constraint*, or *flagged* for the owners to decide.

| # | Earlier statement (section) | This specification | Verdict |
|---|---|---|---|
| 1 | D2/1.2: "PSI" = `SyntaxSnapshot` + `SyntaxNode` over tree-sitter, "whitespace is the gap between byte ranges"; "nothing needs to keep PSI identity: anchors are the only identity". | PSI promises whitespace/comment leaves, edge-binding, error elements (PLAT-PSI-003/004), identity of unchanged elements (PLAT-PSI-051) and smart pointers (PLAT-PSI-075). | **Accepted with constraint**: anchors may replace identity (PLAT-PSI-075/080) but whitespace leaves, edge-binding and error elements are MUST; every handle-returning API must say pointer vs tree-bound. |
| 2 | D4/1.11: readers never block writers; immutable snapshots; `Result<_, Cancelled>`. | Same outcomes via a different mechanism; the reference promises no torn reads, cancel-on-write, idempotent re-run, finish-on-UI guard. | **Accepted**: mapping table in 06 section 9; delivery and staleness rules are MUST and must be central, not per feature. |
| 3 | D5/1.4: dependency stamps with early cutoff (`InterfaceEpoch`), caches keyed by (file revision, interface hash). | The reference *removed* its structure/out-of-code-block split; one global count + per-language counts; resolve and class caches cleared on every change. | **Flagged**: coarse scheme is the MUST (PLAT-CACHE-060); the finer scheme is a gated SHOULD with a property test (PLAT-CACHE-061/062/064). |
| 4 | D7/1.6: dumb mode is per index; `IndexNotReady` is a value. | One global state per project, observable table of behaviors; a typed error is fine. | **Accepted with constraint**: global state is the UI-facing contract; per-index readiness is allowed internally with the three conditions of 04 section 9. |
| 5 | 1.6: stub tree "generic default built from `tags.scm`"; "we do not rebuild typed PSI from stubs; a stub hit is a `Target` validated against the live tree". | Stub hits are elements; class files have no tree; stubs hold declaration data (types as text, annotations, constants). | **Accepted with constraint** for the element model; **superseded** for content: the rich JVM stub of 03 section 5.1 is MUST. Tags-based stubs remain a LATER fallback for non-JVM languages. |
| 6 | 1.6: `Words` index with context mask {code, comment, string, literal}; key = hashed word. | Five contexts (code, comments, strings, foreign languages, plain text), both case-sensitive and insensitive hashes stored; hash collisions verified by text scan. | **Superseded** by PLAT-IDX-080..082. |
| 7 | 1.6: whole-store version = sum of index versions; mismatch rebuilds one index. | Per-index version **and** per-sub-indexer version (per language) in stamps; mismatch re-indexes only the files of that sub-indexer. | **Accepted** with the sub-version refinement (PLAT-IDX-007/008). |
| 8 | 1.6: indexing order "scan, open files, project roots by language, rest, libraries"; batches every ~50 ms / 64 files. | Reference orders explicitly queued files first, then the dirty set; no fixed order inside the queue. | **Flagged as design target** (PLAT-IDX-033); keep, but it is not a reference fact. |
| 9 | 1.1: file ids "keep the id when the watcher reports a rename pair, else new id". | The reference sees an external rename as delete + create unless it is a rename it performed. | **Accepted** as SHOULD (PLAT-VFS-006/007). |
| 10 | 1.4: cache storage "bounded LRU with byte weigher, target 64 MiB total". | Weak keys + soft values + low-memory flush. | **Accepted with constraint**: a budget is fine but the low-memory path is MUST (PLAT-CACHE-070/072). |
| 11 | 1.5/1.2: tree-sitter `changed_ranges` as the equivalent of a PSI tree-change event. | Tree-change events are fine-grained, unpaired and discouraged; counters are the preferred mechanism. | **Accepted**: use counters for caches; events only where unavoidable (PLAT-PSI-060..067). |
| 12 | 1.11: typed handlers call `host.sync_syntax(file)` (synchronous reparse). | The reference's typed handlers must not force a commit; the tree is not guaranteed current there. | **Flagged**: acceptable in Archipel because reparse is cheap, but PLAT-LANG-073 states the conservative contract. |
| 13 | Project model (`docs/architecture/project-model.md`): one module per Gradle project; `Project` scope; "dependency tests never on someone else's classpath". | Scope semantics in 07 section 9 (items 1 to 10). | **Flagged**: see the delta table; item 2 (module per source set vs per project) is the one decision that changes `module_scope` results. |

## 6. Cross-cutting open questions

1. Does Archipel ever need **background write sections** (06 PLAT-CONC-005), or do all mutations stay on the UI thread with commit results applied there?
2. **Prebuilt/shared indexes** for the JDK and common libraries (03 PLAT-IDX-011/062): v1 or later? It turns the first-open minutes of JDK indexing into seconds.
3. **Symbol API vs PSI references**: the studied tree runs both. Is Archipel's reference model symbol-first? (02 open question 1, 08 open question 2)
4. **Third-party plugins in v1?** Isolation, statelessness and cancellation rules are required either way (08 PLAT-LANG-103).
5. **Per-source-set modules** for Gradle (07 open question 2).
6. **Spring**: the Spring plugin is not in the Community sources; the Spring-specific requirements (07 PLAT-SCOPE-102/103, 08 section 6) are design requirements and need validation against real Spring Boot projects once scopes and stubs exist.
7. **Kotlin**: the Kotlin plugin is not in the sparse checkout used here. What the platform must offer it (class finder, augment provider, per-language trackers, stub indexes for top-level declarations, expression-bodied inferred types as part of the interface in PLAT-CACHE-061) is derived from platform contracts only; confirm with the Kotlin work package.

## 7. Boundaries: what other specifications own

| Not in this set | Where its platform hooks are specified |
|---|---|
| Daemon (highlighting passes, scheduling), inspections, intentions, quick fixes | annotator contract and severities: 08 section 3.2; dumb/commit/cancel: 04, 06 |
| Completion, parameter info, documentation | extension points listed in 08 section 4; resolve and variants: 02 section 8 |
| Find usages, rename, refactoring engine | reference contract and use scopes: 02; word search and indexes: 03; scopes: 07 |
| Formatter and code style | listed in 08 section 4; spacing rules in parser definition: 08 section 2.2 |
| Project import and external system | data model in `docs/architecture/project-model.md`; semantics required by analysis: 07 |
| Run/debug, VCS, UI toolkit | none |

## 8. Glossary: reference names to names used here

| Reference concept | Name in this set |
|---|---|
| `VirtualFile`, `VirtualFileManager`, `VFileEvent`, `BulkFileListener`, `AsyncFileListener` | virtual file, VFS, change event, listener (sync and prepare/apply flavors) |
| `Document`, `FileDocumentManager`, `PsiDocumentManager`, commit | document, document manager, commit model |
| `FileType`, `LanguageFileType`, `FileTypeRegistry`, `LanguageSubstitutor` | file type, language file type, detection order, substitutor |
| `PsiElement`, `PsiFile`, `FileViewProvider`, `ASTNode`, `IElementType`, `TokenSet` | element, file, view provider, node (syntax tree), element type, token set |
| `PsiReference`, `PsiPolyVariantReference`, `ResolveCache`, `PsiReferenceContributor` | reference, poly-variant reference, resolve cache, reference contributor |
| `SmartPointerManager` | smart pointers |
| `StubElement`, `StubIndex`, `FileBasedIndex`, `IdIndex`, `FilenameIndex` | stub, stub index, file-based index, word index, file-name index |
| `DumbService`, `DumbAware`, `IndexNotReadyException`, `DumbModeAccessType` | dumb mode, dumb-aware, index-not-ready error, access type |
| `CachedValue`, `CachedValuesManager`, `ModificationTracker`, `PsiModificationTracker`, `RecursionManager` | cached value, manager, tracker, PSI modification count, recursion guard |
| `ReadAction`, `WriteAction`, `NonBlockingReadAction`, `ProcessCanceledException`, `ProgressManager` | read/write sections, NBRA, cancellation, progress |
| `ProjectFileIndex`, `OrderEnumerator`, `GlobalSearchScope`, `DependencyScope` | file index, order enumeration, search scope, dependency scope |
| `JavaPsiFacade.findClass`, `PsiShortNamesCache`, `PsiElementFinder` | `find_class`, short-name cache, class finder |
| `Language`, `LanguageExtension`, `ParserDefinition`, `Lexer`, `Annotator`, `FoldingBuilder` | language, language extension, parser definition, lexer, annotator, folding builder |
