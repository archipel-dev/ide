# Platform spec 03: stubs, stub indexes, file-based indexes, name lookup

Status: specification (clean-room). ID prefix `PLAT-IDX`. Priorities: `MUST` / `SHOULD` / `LATER` as in `README.md`.

Supersedes and deepens section 1.6 (indexing, stubs) of `docs/analysis/code-intelligence-spec.md`.
Dumb mode (what is allowed while indexes are not ready) is specified in `04-dumb-mode.md`; the dependency of resolve on
indexes in `02-psi-model.md` and `07-project-structure-and-scopes.md`.

Sources studied: `ic: platform/indexing-api/src/com/intellij/util/indexing/` (`FileBasedIndex.java`,
`FileBasedIndexExtension.java`, `CompositeDataIndexer.java`, `DumbModeAccessType.java`, `ScalarIndexExtension.java`,
`SingleEntryFileBasedIndexExtension.java`), `ic: platform/util/src/com/intellij/util/indexing/IndexExtension.java`,
`DataIndexer.java`, `ic: platform/indexing-api/src/com/intellij/psi/stubs/` (`StubIndex.java`, `StubIndexExtension.java`),
`ic: platform/core-api/src/com/intellij/psi/stubs/` (`IStubElementType.java`, `StubElement.java`),
`ic: platform/core-api/src/com/intellij/psi/StubBuilder.java`, `ic: platform/core-impl/src/com/intellij/psi/tree/IStubFileElementType.java`,
`ic: platform/core-impl/src/com/intellij/psi/stubs/StubBuilderType.java`,
`ic: platform/indexing-impl/src/com/intellij/psi/stubs/` (`StubUpdatingIndex.java`, `StubProcessingHelperBase.java`),
`ic: platform/indexing-impl/src/com/intellij/psi/impl/cache/impl/id/IdIndex.java`, `IdDataConsumer.java`, `IdIndexEntry.java`,
`ic: platform/indexing-impl/src/com/intellij/psi/impl/search/PsiSearchHelperImpl.java`,
`ic: platform/indexing-api/src/com/intellij/psi/search/` (`UsageSearchContext.java`, `FilenameIndex.java`, `FileTypeIndex.java`),
`ic: platform/lang-impl/backend/src/com/intellij/util/indexing/` (`FileBasedIndexImpl.java`, `FileBasedIndexProjectHandler.java`,
`UnindexedFilesScanner.kt`, `UnindexedFilesIndexer.java`, `DumbModeWhileScanningTrigger.kt`),
`ic: platform/lang-impl/src/com/intellij/util/indexing/dependencies/ProjectIndexingDependenciesService.kt`,
`ic: java/java-psi-api/src/com/intellij/psi/impl/java/stubs/index/JavaStubIndexKeys.java`,
`ic: java/java-indexing-serializers/src/com/intellij/psi/impl/java/stubs/serializers/*`,
`ic: java/java-psi-impl/src/com/intellij/psi/impl/JavaPsiFacadeImpl.java`, `ic: java/java-psi-api/src/com/intellij/psi/PsiElementFinder.java`,
`ic: java/java-analysis-impl/src/com/intellij/psi/impl/file/impl/JavaFileManagerImpl.java`,
`ic: java/java-indexing-api/src/com/intellij/psi/search/PsiShortNamesCache.java`.

## 1. Concepts

* **File-based index** `I`: a function from *file content* to a small map `key -> value`, kept in two persistent maps.
  The **forward map** `file -> map` (needed to remove/replace a file's contribution when the file changes) and the
  **inverted map** `key -> {(file, value)}` (needed for queries). Queries take a key and a **scope** and return values and/or files.
* **Indexer**: the pure function `content -> map`. It may use the file's content, its name, its type and the parsed
  tree of *this file only*. It may **not** use the parent directory, sibling files, project settings or other files; otherwise the
  platform cannot know when to re-run it.
* **Stub**: a compact, serializable, tree-shaped summary of a file's *declarations* (classes, methods, fields, parameters,
  annotations, imports, packages, modifiers, type text, selected initializers) with **no statements and no expressions**
  except the few the language marks as stubbed. One stub tree per file is stored by a built-in index called *Stubs*.
* **Stub index**: a named lookup (`key -> stub nodes`) built *from* the stub trees. A language says which keys each stub node
  contributes. Examples: short class name, fully qualified class name, method name, annotation short name.
* **Stub-based element**: an element that can be created from a stub node without parsing the file. Asking such an element
  for something the stub does not hold triggers loading the file's tree.
* **Scope**: a predicate over files (spec 07) that every query takes.
* **Up to date**: a query result is correct for the current state of all files, including *unsaved* editor text.

## 2. Index extension contract

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-001 | MUST | An index is declared with: unique id (string), version (integer), key descriptor (equality, hash, serialization), value descriptor (equality, serialization), indexer, input filter, "depends on file content" flag. | A declaration missing any part is rejected at registration. |
| PLAT-IDX-002 | MUST | Values must satisfy `deserialize(serialize(v)) == v` by equality and hash. A violation is detected in debug builds by round-tripping a sample and reported with the index id. | A value type without proper equality fails the self-check at first indexing. |
| PLAT-IDX-003 | MUST | The indexer is deterministic and side-effect free: same inputs, same output map; the returned map has ordinary map semantics (a missing key yields none). It may not read other files. | Index the same file twice, compare maps equal; a lint forbids VFS access from indexers. |
| PLAT-IDX-004 | MUST | An index declares an **input filter** that is evaluated first on cheap data (file type, name, flags) and only then, if the filter says "unsure", on content. Filtering by file extension alone is discouraged; filter by file type, so a user remapping a type works. | Remap `*.conf` to a language; the language's indexes pick up those files. |
| PLAT-IDX-005 | MUST | A "depends on content" flag of false means the index depends only on name/metadata; such indexes are updated without reading content (cheap). | File-name index updates for 100k renamed files without reading any bytes. |
| PLAT-IDX-006 | MUST | **Single-entry** indexes (one entry per file, key = file id) exist for per-file data such as the stub tree. **Scalar** indexes (value is unit) exist for pure membership. | The stub index uses the single-entry form. |
| PLAT-IDX-007 | MUST | **Composite indexers**: an indexer may delegate to a *sub-indexer* chosen by file (file type/language), each with its own **sub-version**. A change of one sub-version re-indexes only files handled by that sub-indexer. | Bump the Kotlin stub version; only Kotlin files are re-indexed. |
| PLAT-IDX-008 | MUST | Index **version**: when the stored version of an index differs from the declared one, only that index is dropped and rebuilt (and only by a background task); other indexes are untouched. | Bump the word index version; the stub index is not rebuilt. |
| PLAT-IDX-009 | MUST | Rebuild on corruption: any unrecoverable error while reading or writing an index marks that index "requires rebuild", clears it, and schedules a full re-index in the background. Startup never fails because of a corrupt index. | Truncate an index file; the IDE starts, dumb mode is shown, the index is rebuilt, no crash dialog. |
| PLAT-IDX-010 | SHOULD | Optional per-index traits: cache size; whether keys are unique per file (an optimization); whether a `key hash -> file id` mapping is traced so "all keys" can be filtered by project; types exempt from the size limit (use sparingly); whether directories are indexed. | The "all keys" query for a name index returns only keys relevant to the open project when tracing is on. |
| PLAT-IDX-011 | LATER | Shareable index chunks (prebuilt indexes for JDK/libraries downloaded or computed once) with matching hash and version checks. | n/a |

## 3. What is indexed, and when

### 3.1 Indexable files

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-020 | MUST | Indexable: files under module content roots (not excluded, not ignored), under library classes and library source roots of any module dependency, under SDK roots, plus roots contributed by extension. Excluded and ignored files are never indexed. | A file under `build/` (excluded) has no entries in any index. |
| PLAT-IDX-021 | MUST | A file is skipped if larger than the intelligence limit (spec 01 PLAT-VFS-034) unless the index lists the file type as exempt; skipped files are still present in the file-name index. Directories are indexed only by indexes that ask for it. | A 3 MB source file appears in "Go to File" but not in "Go to Symbol". |
| PLAT-IDX-022 | MUST | Symlink following during scanning is on by default with loop protection; the scan never indexes the same real file twice. | A tree with a symlink to itself terminates. |
| PLAT-IDX-023 | MUST | Binary files are indexed only by indexes that declare binary support (class files for stub/class indexes; images never). | A `.png` has no word/stub entries. |
| PLAT-IDX-024 | SHOULD | Index data of files outside the open project but inside another open project's roots is kept separately per project so closing one project does not drop the other's data. Queries are filtered to the asking project. | Two projects sharing a library: queries in project A never return files only in B's content. |

### 3.2 Initial scan, dirty files, update rules

Process (steps are normative; implementation is free):

1. **Scanning** enumerates indexable files (by provider: module content, libraries, SDK, others) using several workers
   (auto-sized), skipping excluded subtrees, and compares each file's **indexing stamp** (a per-file, per-index record
   of "which content version and which index version was indexed") against the current ones. Files whose stamp is stale
   or missing are put in the *indexing queue*. If the previous session ended with an incomplete scan, the next scan is a
   *full* scan (not an incremental one).
2. **Indexing** takes files from the queue, builds input (bytes, text, tree if the indexer needs one, **at most one tree per
   file shared by all indexes**), runs every applicable indexer, and commits batched updates.
3. **Change handling**: VFS events (spec 01) put changed files into a *dirty set*. They are re-indexed either lazily
   (small sets) or in the background in dumb mode (large sets), see thresholds below.
4. **Unsaved documents**: indexes reflect the *editor text*, not disk. Before any query the platform brings the index
   for those documents up to date from their committed text into an **in-memory overlay** that merges over the persistent
   data; the overlay is dropped when the document is saved or closed and reconciled with the disk version.
5. **Ensure up to date**: each query first applies pending dirty files that could affect the queried index, applies
   the overlay, then reads. This is why a query in smart mode always sees the latest edits.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-030 | MUST | Every indexed file has a stamp per index: (content modification stamp or content hash, index version, sub-indexer version). A file is re-indexed iff the stamp differs. A changed timestamp with identical content does not trigger re-indexing when a content hash is available. | `touch` a file: no re-index. Change one byte: re-index. |
| PLAT-IDX-031 | MUST | After a VFS refresh the platform decides *lazy vs dumb*: if the number of changed files reaches **20**, or their total size reaches **1,048,576 bytes**, or counting them takes more than **100 ms**, a background re-index runs in dumb mode; otherwise the files are updated lazily on the next query. All three numbers are configurable. | `git checkout` that changes 500 files: dumb mode with progress. Saving a single file: no dumb mode. |
| PLAT-IDX-032 | MUST | Scanning that finds at least **20** dirty files starts dumb mode for the indexing step (configurable threshold); fewer files are indexed in smart mode without a banner. | Project open with 5 stale files: no dumb mode. |
| PLAT-IDX-033 | MUST | Indexing order inside one run: files explicitly queued (by scanning, or by the caller that discovered them stale) first, then files from the dirty set reported by VFS events. A run is cancellable between files; one file is the unit of non-cancellable work. SHOULD (Archipel design target, not a verified reference rule): within the queue, prefer open and recently opened files, then project sources, then libraries. | Edit a file during a long indexing run: its entries are updated before the run finishes. |
| PLAT-IDX-034 | MUST | Index storage is **application-wide** and keyed by file id: a virtual file is indexed once no matter how many open projects contain it; data survives project close and restart. Queries are filtered to the asking project's files. | Open two projects using the same `spring-core.jar`; it is indexed once. |
| PLAT-IDX-035 | MUST | Index updates are applied in batches and made visible atomically per file: a reader never sees a file's contribution half-removed. | Concurrent readers during a batch never see a file in the forward map without its inverted entries. |
| PLAT-IDX-036 | MUST | A file that fails to index (exception in an indexer, parse failure) is logged with file and index id, contributes nothing for that index, and does not stop indexing of the others. A failing *plugin* indexer must not break the other indexers. | A throwing indexer for one file type: other file types index normally. |
| PLAT-IDX-037 | SHOULD | Indexing progress reports phases and counts: "Scanning files…" (scan), "Analyzing project…" (indexing) with a title "Analyzing Project to Enable Smart Features"; a per-root detail line such as "Indexing library 'x'" or "Indexing module 'y'". Indexing can be paused and resumed by the user ("Analyzing paused" / "Indexing paused due to {reason}"). | Pause button stops work between files; status text changes. |
| PLAT-IDX-038 | MUST | **Project-level re-scan triggers**: project model (roots) changes, file type changes, language level or similar per-file properties changing ("pushed properties"), index version changes, user "invalidate caches", plug-in load/unload that registers or removes indexes. | Add a library: the library is scanned; unchanged modules are not. |
| PLAT-IDX-039 | SHOULD | Per-file **pushed properties** (for example the language level of a Java file or the SDK of a Python file) are computed from the project model, stored on the file, and trigger re-index of files whose property changed. Indexers may read pushed properties. | Change module language level; files in that module re-index. |

## 4. Query contract

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-040 | MUST | Queries (all take a scope and, directly or indirectly, a project): values for a key; files containing a key; iterate values with their file (stop early); files containing **all** of several keys; files containing **any** of several keys; all keys (as a *superset*: it may contain keys that are only in other projects or orphaned, never fewer than the project's keys); per-file data (`file -> map`). | Conformance test for each. |
| PLAT-IDX-041 | MUST | A query result is a pure function of (index state, key, scope). Scope membership is checked per file. Results for equal queries in the same state are equal and **deterministically ordered** (by file id then value order) so UI lists don't flicker. | Run a query 100 times; identical order. |
| PLAT-IDX-042 | MUST | Queries are valid only under read access and in smart mode (or with an explicit dumb-mode access type, spec 04). Outside, they fail with the typed "index not ready" error; they never return partial results silently. | A query in dumb mode returns the typed error. |
| PLAT-IDX-043 | MUST | "All keys" and key-prefix enumeration support early stop and cancellation, and may take an optional file-id filter to narrow keys to a project's files. | Budget: cancel an all-keys scan over 1M keys within 50 ms. |
| PLAT-IDX-044 | MUST | Query calls are cancellation points (a pending write section cancels the read section). Long loops check cancellation at least every 256 entries (budget). | A pending write action makes a long all-keys query abort with `Cancelled`. |
| PLAT-IDX-045 | MUST | The caller must not hold one big read section around a project-wide iterate-files call; iterate-files functions are documented as "do not wrap in a single read action" and tolerate model changes mid-iteration. | n/a, API doc test. |
| PLAT-IDX-046 | MUST | The index timestamp (a monotone number per index) changes whenever the index content changes; used to invalidate caches of query results. | Cache a name list with the index stamp; adding a file invalidates it. |
| PLAT-IDX-047 | SHOULD | A *dumb-mode access type* lets a caller on purpose read indexes while indexing: `reliable-only` returns only entries known to be up to date; `raw` returns whatever is currently stored, possibly stale, never for stub indexes. | A completion contributor lists project names with `raw` access during indexing, results flagged stale. |

## 5. Stubs and stub indexes

### 5.1 What a stub must hold

A stub node has: a kind (element type), a parent, children, and the *serialized fields* chosen by its element type.
For code languages the stub must hold enough that **resolve never needs a method body**:

* for **classes**: name, qualified name, kind (class, interface, enum, annotation, record, anonymous, implicit), modifiers
  (including deprecated, final, sealed), type parameters with bounds *as text*, `extends`/`implements`/`permits` references
  *as text*, base-class reference text for anonymous classes, flags (has deprecated annotation, has doc-comment deprecation),
  source file name for compiled classes, inner/local/member nesting;
* for **methods**: name, modifiers, return type text, type parameters, parameter list with names and type text, varargs flag,
  `throws` list text, constructor flag, annotation-method default value text, "is deprecated" flags;
* for **fields**: name, type text, modifiers, enum-constant flag, *constant initializer text when the field is a candidate
  compile-time constant* (so constant folding across files needs no body);
* for **annotations**: the annotation text (name and arguments text) so annotation-driven features (Spring stereotypes,
  Lombok-like generators, nullability) work without bodies;
* for **imports and package**: package statement, import list entries (qualified name, static flag, on-demand flag);
* for **lambdas/method references**: functional expression markers if functional-interface search must work without bodies.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-050 | MUST | Each language that wants stub indexing provides: a *stub builder* (tree to stub tree; may skip subtrees that cannot contain stubs for speed), a list of *stub element types* each with `create_stub`, `serialize`, `deserialize`, `index_stub`, `create_element(stub)`, `should_create_stub(node)`, and a **stub version** for the file type. | A language with `create_stub` but no `serialize` is rejected. |
| PLAT-IDX-051 | MUST | Stub **version**: increments whenever the stub structure or serialization of that language changes. The effective version of a file's stub is the combination of: file element type's external id, its stub version, its debug name, plus (for template languages) the sum of the versions of the data languages. A mismatch re-indexes exactly the files of that stub builder. | Change a Java stub field, bump Java's version: Java files are re-indexed, XML stubs stay. |
| PLAT-IDX-052 | MUST | Stub **serialization is stable, compact and deduplicated**: names are interned. Ranges and offsets are **not** stored (they would invalidate on every edit). Budget (Archipel target, not a reference fact): a stub tree is under 5% of the source size. | A whitespace-only edit does not change the serialized stub bytes. |
| PLAT-IDX-053 | MUST | The **stub tree hash** (the serialized stub without ranges) is available per file and drives *early cutoff*: if the new stub equals the old, all stub-index entries and everything derived from the stub hash stay valid. | Edit a method body; stub hash unchanged; stub indexes not rewritten. |
| PLAT-IDX-054 | MUST | **Stub indexes** are declared with a key type and an id. Each is filled by the element type's `index_stub(stub, sink)` which emits `(index, key)` occurrences for that stub node. One stub tree feeds every stub index. | A class stub emits both short-name and FQN occurrences. |
| PLAT-IDX-055 | MUST | A stub index entry maps `key -> (file, list of stub node ids)` where a node id is the position of the stub node in the file's flattened stub tree. Resolving an entry to an element loads the file's stub tree (not the AST) and takes the nodes by id. | A short-name lookup returns class elements without parsing any file. |
| PLAT-IDX-056 | MUST | **Self-healing**: if an entry points to a file that has no tree, a file whose stub tree lacks that id, or a node of the wrong kind, the platform logs an internal error with file type and indexing-stamp info, **skips that entry**, schedules a re-index of that file, and continues. It never crashes and never returns a wrong element. | Corrupt one entry; query returns the others; the file is re-indexed; no exception to the user. |
| PLAT-IDX-057 | MUST | Stub index queries: `elements(index, key, project, scope, expected_kind)`, `all_keys(index, project)`, `process_all_keys(index, scope, filter, processor)`, `containing_files(index, key, scope)`, `max_containing_file_count(index, key, scope)`. `elements` yields elements of the expected kind, sorted by file id then stub id, only from files in scope. | Conformance test; wrong expected kind yields an inconsistency report and skips. |
| PLAT-IDX-058 | MUST | The **stub index modification tracker** per project, and **per file element type** (a tracker that changes when any stub of that file type changes) exist for caches that depend on declarations only. | Cache "all Spring components" with the Java-stub tracker; editing a method body does not invalidate it. |
| PLAT-IDX-059 | SHOULD | **Stub/AST switch**: elements obtained from stubs keep identity when AST loads; a file with stubs but unloaded AST uses memory only for the stub; after a content change the stub is discarded. | See PLAT-PSI-031. |
| PLAT-IDX-060 | SHOULD | Stub-based elements document which accessors are *stub-safe* (cheap) vs which load the AST, and tooling can report AST loading in contexts where it is forbidden (library files, indexing). | A debug flag fails a test that loads the AST of a library class during name completion. |
| PLAT-IDX-061 | MUST | Stub builders may build from a **light tree** or from bytecode; the result for the same file via different paths must be equal. Compiled class stubs are built from the class file's structure (not from decompiled text) and include the source file name and deprecation flags. Synthetic classes, synthetic fields and methods, the synthetic static initializer and synthetic record accessors are **not** in the stub; anonymous and local inner classes are present but flagged and are **not** entered in the short-name and FQN indexes. | A class with a lambda (synthetic method) and an anonymous inner class: the stub has no synthetic method; the anonymous class does not appear in Go to Class. |
| PLAT-IDX-062 | LATER | Prebuilt stubs for well-known libraries shipped or downloaded and keyed by content hash. | n/a |

### 5.2 Java stub indexes (the reference set for JVM languages)

All keys are strings unless noted. "Short" means the unqualified name as written, **not resolved** (the stub holds text; resolution happens later).

| Index (key id) | Key | Filled from | Used by |
|---|---|---|---|
| class short names (`java.class.shortname`) | simple name of every non-anonymous class, including nested | class stub | Go to Class, completion of class names, `short names cache` |
| class FQN (`java.class.fqn`, key kept as character sequence) | qualified name of every non-anonymous class; nested classes use `Outer.Inner` | class stub | `find_class(qualified_name, scope)`, imports, resolve of qualified names |
| anonymous base ref (`java.anonymous.baseref`) | short name of the base type of an anonymous class | anonymous class stub | Go to Implementation, inheritor search |
| implicit classes (`java.unnamed.class`) | name of an implicit (unnamed) class | stub | Java 21+ unnamed classes |
| super classes (`java.class.extlist`) | short name of each type in `extends` and `implements`; enums add `Enum`; annotation types add `Annotation` | reference list stub | direct inheritor search, hierarchy |
| method names (`java.method.name`) | method name | method stub | Go to Symbol, `short names cache`, override search |
| field names (`java.field.name`) | field name | field stub | Go to Symbol |
| record components (`java.record.component.name`) | component name | stub | records |
| annotations (`java.annotations`) | short name of each annotation (text up to `(`) | annotation stub | annotated-elements search, framework detection |
| method parameter types (`java.method.parameter.types`) | short type name of each parameter, excluding primitives, wrappers and the method's visible type parameters | method stub | extension/static-member and parameter-type searches |
| static member names/types (`jvm.static.member.name` / `.type`) | name; return/field type short text, for non-private static members | method/field stub | static import completion ("add static import"), "static member" completion |
| module names (`java.module.name`) | module name from `module-info` | module stub | module system |

Additional **file-based** (non-stub) Java indexes exist (functional-expression markers per file, "null as method argument" markers, binary-plus-expression chains); they are specialized and listed as LATER.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-070 | MUST | Provide the Java stub indexes above (at minimum: class short name, class FQN, super classes, method names, field names, annotations, anonymous base ref, module names) populated for **source files and class files** alike, with identical keys for the same class. | The same class from `A.java` and `A.class` yields equal key sets. |
| PLAT-IDX-071 | MUST | Keys are *textual and unresolved*: the super-class index stores `List` for `java.util.List` and for `List<T>` alike; consumers verify by resolving. Therefore every consumer of these indexes **must re-check by resolve** when correctness matters. | A class `class X extends Foo` and an unrelated `Foo` both appear under key `Foo`; inheritor search resolves to filter. |
| PLAT-IDX-072 | MUST | Local, anonymous and implicit-class members are **not** reachable through short-name and FQN indexes except as specified (anonymous only under the base-ref index). | A local class does not appear in Go to Class. |
| PLAT-IDX-073 | SHOULD | Keep indexes small: do not index private members of compiled library classes in member-name indexes if the language says private members are never found from outside; do index them for source. | Measure library index size per jar and record it in the performance docs. |

## 6. Word, name, type and text indexes

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-080 | MUST | **Word (identifier) index**: key = 32-bit hash of an identifier-like word; value = bitmask of the *contexts* in which it occurs in the file: code `0x01`, comments `0x02`, string literals `0x04`, foreign (injected/embedded) languages `0x08`, plain text `0x10`; `0xFF` = any. Both the case-sensitive and the case-insensitive hash of each word are stored. A hash collision yields extra candidate files, never missing ones. | Search `userName` in code context: files where it appears only in comments are not candidates. |
| PLAT-IDX-081 | MUST | The scanner that splits a file into words and contexts is **per language** (lexer-based for languages with a lexer, plain-text scanner otherwise) and may be thread-safe or return a new instance per call. A language can supply its own via the "find usages provider". | Java: a word in a string literal is context `0x04`. |
| PLAT-IDX-082 | MUST | **Text occurrence search** (platform helper) is: (1) look up candidate files by word hash + context mask + scope; (2) order candidates (files that also contain the *container name*, files near the target and open files first); (3) in each candidate, scan the text for the word with word-boundary rules, find the leaf element at each offset, climb to the element whose range contains the occurrence, and call the caller's processor with (element, offset in element); (4) process files in parallel under read access, restarting unfinished files after a write action; (5) stop when the processor stops. Callers verify each hit by resolving. | Find usages of a method returns hits ordered with same-file hits first; typing in the editor during the search cancels and restarts it. |
| PLAT-IDX-083 | MUST | Words shorter than three characters and wildcards do not use the trigram index; longer ones may narrow candidates via trigrams (LATER). | n/a |
| PLAT-IDX-084 | MUST | **File-name index**: `name -> files` for every virtual file in indexable roots **including files over the size limit and directories** if asked, derived from the VFS (no content). Case-insensitive lookup supported. Names are stored as written. | Budget: Go to File finds `README.md` in a project of 100k files within 30 ms. |
| PLAT-IDX-085 | MUST | **File-type index**: `file type -> files`, derived from file type detection; used to enumerate all files of one language in a scope. | List all Kotlin files in a module. |
| PLAT-IDX-086 | SHOULD | **TODO index**: one entry per file; value = count of matches per configured TODO pattern; rebuilt (only this index) when patterns change. Comment tokens come from the language's comment token set. | Change the pattern list: TODO index rebuilds; word index does not. |
| PLAT-IDX-087 | LATER | **Trigram index** `24-bit trigram -> {files}` for text search acceleration. | n/a |

## 7. Name lookup APIs (platform side)

### 7.1 `find_class` and friends

Behavior of the facade over the Java stub indexes (applies to any JVM language that contributes class finders).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-090 | MUST | `find_class(qualified_name, scope)` returns the **best** class: ask each registered *class finder* (first the Java source/class finder, then others such as package-prefix and generated-class finders, in registration order), discard classes that any finder's *filter* rejects for this scope, and among the remaining candidates keep the one that comes **earliest on the classpath of the scope** (scope comparison: later in the classpath loses). Equal candidates keep the first found. | Two jars containing `a.B`: the one earlier in the module's dependency order wins. |
| PLAT-IDX-091 | MUST | `find_classes(qualified_name, scope)` returns **all** candidates: the facade's own, then those of every JVM-language facade, each group sorted best-first by scope comparison. Duplicates of the same source/compiled pair both appear. | Source and compiled twin both appear in an all-scope query. |
| PLAT-IDX-092 | MUST | `has_class(qualified_name, scope[, filter])` is an existence check that must not load more than the index entry. | 100k `has_class` calls do not parse any file. |
| PLAT-IDX-093 | MUST | `find_class` for a **nested class** works with both `Outer.Inner` (source style) and with the FQN index key; names with `$` are not valid source names and are not accepted. | `find_class("a.Outer.Inner")` returns the inner class. |
| PLAT-IDX-094 | MUST | For a candidate from a **class file**, the directory path between the class root and the file must consist of valid identifiers (ignoring multi-release version directories); otherwise the candidate is rejected. | A class file placed under a directory named `1-bad` is not found. |
| PLAT-IDX-095 | MUST | `find_class` results are cached per (scope, name) and the whole cache is cleared on **any** PSI modification count change. Results influenced by recursion prevention are not cached. | After any edit, the cache is empty. |
| PLAT-IDX-096 | MUST | `find_package(qualified_name)` returns a package if any package directory in any source/class root, or any package-declared file, exists for that name, across all finders; results cached until PSI change; `find_package` for a package with only empty directories still returns it. Sub-packages and package classes are queried with a scope. | An empty directory `a/b` makes package `a.b` exist. |
| PLAT-IDX-097 | MUST | In **dumb mode** with *alternative resolve* enabled (spec 04), `find_class` does not use the class indexes: it finds the package, then looks up the class by short name in the package's directories, handling nested classes by finding the outer class first. Without alternative resolve, finders that are not dumb-aware are skipped and any index access raises the typed error. | Go to Declaration during indexing finds project classes by directory lookup (slower, may miss generated or library classes). |
| PLAT-IDX-098 | SHOULD | A **migration/rename finder** and **package-prefix finder** exist as finders: classes found through a configured package prefix of a source root (a source root whose files declare packages relative to a prefix). | A source root with prefix `com.acme` makes `com.acme.Foo` resolve to `Foo.java` at the root. |
| PLAT-IDX-099 | MUST | Finders are a public extension point (needed for Kotlin, Groovy, generated code). Each finder declares whether it is dumb-aware, may supply a class filter per scope, a package filter, `get_class_names(package, scope)` for fast "might this package contain this name" checks, and package directory/file processing. | A Kotlin finder contributes classes found only in Kotlin sources. |

### 7.2 Short-name cache

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-IDX-100 | MUST | `short_names_cache` answers by **unqualified** name within a scope: classes by name; methods by name (and capped variants "if not more than N"); fields by name; files by name; all class/method/field/file names; and the same as processors with early stop. | `classes_by_name("List", project_scope)` returns every project class named `List`. |
| PLAT-IDX-101 | MUST | The cache is the **union** of every registered cache (Java, Kotlin, ...), each answering for its own language; callers can ask for a version that excludes some languages. | Exclude Kotlin from the cache: Kotlin-only names are absent. |
| PLAT-IDX-102 | MUST | `all_*_names` return **all names the index knows** (a superset for a given scope); each name must then be queried with a scope to get elements. A returned name may have no element in a narrower scope. | `all_class_names()` contains a name that `classes_by_name(name, narrow_scope)` returns empty for. |
| PLAT-IDX-103 | MUST | Results are not sorted by relevance; callers sort. Ordering within results is deterministic (PLAT-IDX-041). | Same call twice returns equal order. |
| PLAT-IDX-104 | SHOULD | The member-by-name queries accept a cap and return early when the number of matches exceeds it (used by completion to avoid huge lists). | `methods_by_name_if_not_more_than("get", scope, 100)` returns empty when 5,000 match. |

## 8. Index of differences against the code-intelligence spec

The following designs in `docs/analysis/code-intelligence-spec.md` section 1.6 diverge from the reference behavior; each is
either accepted with a requirement above or flagged:

| Point | Code-intelligence spec | Reference behavior | Resolution |
|---|---|---|---|
| Stub-to-element | "do not rebuild typed PSI from stubs; stub hit is a target validated against the live tree" | Stub hits become real elements; their data comes from the stub; the tree loads only on demand | Accepted as an Archipel choice **only if** the stub holds everything in 5.1 so resolve of library and project declarations never needs the tree; otherwise library resolve is impossible (class files have no tree). See PLAT-IDX-050, 055, 061. |
| Stub content | Generic stubs from `tags.scm` (kind, name, ranges, extras) | Rich declaration data (types as text, modifiers, annotations with arguments, constants) | A tags-based stub is a LATER fallback for non-JVM languages; JVM languages need the rich form (MUST). |
| Dumb mode | "per index, not global" | One global state per project plus per-access override | Flagged in spec 04 (PLAT-DUMB-001..003). |
| Words index masks | "code, comment, string, literal" | five contexts; hash keys (not words) | PLAT-IDX-080 (keep hash keys; verify by text scan). |
| Storage | embedded KV (`redb`) | persistent maps with forward + inverted structure | Compatible; PLAT-IDX-009 (rebuild on corruption) and PLAT-IDX-035 (atomic per-file visibility) are the testable constraints. |
| Index version | "sum of index versions" | per-index version plus per-sub-indexer version | PLAT-IDX-007, 008. |

## 9. Notes for implementers (non-binding)

* If Archipel keeps tree-sitter for parsing, a JVM stub builder can still be written as a *tree-sitter query-driven or
  hand-written traversal* that emits the rich stub of 5.1. What must not be skipped is the information content, not the parser.
* The existing "one parse per file, shared by all indexes" idea (PLAT-IDX-030 step 2) is also the reference design.
* Persistent per-file stamps (PLAT-IDX-030) are what make restart cheap. Without them every start is a full scan.

## 10. Open questions

1. The studied tree is migrating stub element types to a registry of *stub definitions* (factory, serializer, light-stub
   factory) keyed by language; the behavior in this document is unchanged. Archipel should not copy either shape.
2. Index shipping (prebuilt JDK indexes): is it in scope for v1? It reduces first-open indexing of the JDK from minutes to seconds.
3. Exact trigram thresholds and the "short word" rule (PLAT-IDX-083) were not verified numerically; measure on a corpus.
