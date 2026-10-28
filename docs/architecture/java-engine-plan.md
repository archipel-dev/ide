# Native Java engine: plan to parity and jdtls removal

Status: plan. M0 (harness), M1 (classpath and index queries) and the class-name slice of M3 are
implemented, see section 6; the rest is open. Successor of the first slice described in
[`java-intelligence.md`](java-intelligence.md); builds on [`language-packs.md`](language-packs.md),
[`project-model.md`](project-model.md), [`platform-kernel.md`](platform-kernel.md) and Phase 4 of
[`platform-refactor-plan.md`](platform-refactor-plan.md). Specs: `docs/specs/java/01..09`,
`docs/specs/codeinsight/`, `docs/specs/platform/03,04,07`. Clean room: IntelliJ behavior comes only from
those specs; this plan cites requirement IDs, not IntelliJ code.

## 0. Decisions this plan implements (binding, from the product owner)

1. All Java analysis runs in our own engine. jdtls (and kotlin-lsp) are transitional and are removed
   once the native engine reaches parity (criteria in M7).
2. Nothing is hard-coded per framework or per example. `@Test` is offered because JUnit is on the test
   classpath and `Test` is an annotation type in the index, exactly as any other class. Where the current
   code special-cases a name (see 1.5: the `lombok` text checks, the jdtls favorite static members list), the
   plan replaces it by a general mechanism.
3. The owner's three pains are the first measurable goals:
   - typing `@Te` in a JUnit test must offer `@Test` with its import (not String operations);
   - calling a method or using a class that does not exist must be marked red while typing;
   - classes from Maven/Gradle dependencies and the JDK must be found for completion and import.

## 1. Current state

### 1.1 What exists natively (and where)

| Piece | Where | State |
|---|---|---|
| Parser | tree-sitter Java through `archipel-ide-language` (incremental reparse in `archipel-ide-analysis` host, 2k-line reparse p95 < 2 ms) | done; error-tolerant but the analyzer treats `has_error()` coarsely (1.3) |
| Source stubs | `archipel-ide-java/src/source.rs` (`build_from_tree`), `stub.rs` (`FileStub`, `ClassStub`, `MethodStub`, ...) | done for named classes, bodies skipped, local/anonymous classes built on demand (JAVA-STUB-001..007, 019 partial) |
| Class-file reader | `classfile.rs` (constant pool, Signature, InnerClasses, Record, PermittedSubclasses, annotations, MethodParameters/LocalVariableTable), `unit.rs` (`read_jar`, `read_class_dir`, multi-release by language level) | done (JAVA-RES-034/035/038/039/041); private members of libraries dropped |
| JDK reader | `jimage.rs` (JDK 9+ `lib/modules`), `unit.rs::read_jdk_image`, exported packages only | done for JDK 9+; no `rt.jar` (JDK 8), no `ct.sym`, compressed jimage resources skipped |
| Durable indexes | `index.rs` over `archipel-ide-index` (`redb`, stamps: len/mtime/hash per unit, versioned `STUB_VERSION`): `Java:Stub`, `Class`, `ShortName`, `Method`, `Field`, `Annotation`, `SuperClass`, `Package` | done and incremental per unit; dumb mode (`NotReady`) honored; **exact-key lookups only** |
| Resolve scope | `scope.rs` (`ResolveScope::for_file`: module + dependencies + libraries + SDK; test code adds own tests and test-scope libraries; file outside source roots resolves to itself; file outside every module gets `Everything`) | done (JAVA-RES-029/030/033) |
| Environment | `env.rs` (class lookup, hierarchy, file-level name walk, accessibility) | erasure-level only |
| Analyzer | `analyze.rs` (2.1k lines: scope walk, a text-typed `Ty` in `ty.rs`, errors 007/011/012/003, import usage) | zero-candidate errors only, no generics |
| Daemon pass | `diagnostics.rs` (`JavaResolvePass` id `java-resolve`, `JavaService`), registered by `archipel-ide-langpacks/src/packs/java.rs` | on by default (`java.native.analysis`) |
| Import fixes | `import_fix.rs` (static member and class candidates, ordering, choosers), `import_edit.rs` (layout, collapse to `.*`, Optimize Imports function) | done to the level of spec 05; Optimize Imports has no command yet |
| Project model | `archipel-ide-projectmodel`, `archipel-ide-externalsystem` (Maven: POMs + `mvn dependency:build-classpath` per scope; Gradle: init script, per-source-set classpaths; JDK detection `jdk.rs`) | done; model cached on disk |
| Contracts, no Java implementation | `COMPLETION_CONTRIBUTOR`, `COMPLETION_WEIGHER`, `RESOLVER`, `REFERENCE_CONTRIBUTOR`, `LOCAL_INSPECTION_TOOL`, `IMPLICIT_USAGE_PROVIDER` (`archipel-ide-analysis/src/langext.rs`, `assist.rs::CompletionContributor`) | declared; no Java contributor registered (only test fixtures in `analysis/src/testing.rs`) |

Spec coverage (`docs/specs/coverage.md`, generated): Java 63 of 445 requirements implemented, 54 partial;
by file, 01 stubs 19/28, 02 resolve 10/47 (+15 partial), 03 types 0/41, 04 error catalog 5/152, 05 imports
29/66 (+34 partial), 06 completion 0/37, 07 inspections 0/62. Codeinsight 05 (auto-import), 06 (completion)
and 08 (optimize imports): 0.

Tests today: `archipel-ide-java/tests/{classfile,index,source_stub,errors,imports,spring_fixtures}.rs`
(fixtures build a model in a temp dir; libraries are looked up in the developer's `~/.m2` and tests skip
silently when a jar is absent: `tests/common/mod.rs::find_jar`, `imports.rs::libs`), ignored
`corpus.rs` (false-positive hunt on a sources jar), core-level `java_native_flow.rs` (Gradle, ignored),
`java_differential.rs` (native vs jdtls resolve errors on one Gradle project, ignored, 17 vs 17 errors, see
`docs/analysis/java-differential.md`), `real_jdtls.rs` (ignored). There is no executable runner for
`docs/specs/java/cases/`, and that directory holds only the 09 inspection cases (1528); there are no
case files for specs 01 to 08.

### 1.2 What is delegated to jdtls today

Everything that is not in the table above, for `.java` files, goes through `archipel-ide-lsp`
(`JdtlsProvider` in `registry.rs`) and `archipel-ide-core`: **completion** (`completion.rs` has no native
source; the only extra items are live templates), hover/quick doc (`quick_doc.rs`), signature help,
go to definition/implementations, find usages, rename, code actions other than the native import fixes
(`code_actions.rs`, `java.apply.workspaceEdit`), formatting, organize imports as an LSP command, jdtls
diagnostics (reconciled with the native pass by `FileMarkup::reconcile`, with `native_wins` as the
setting), and the **debug adapter**: `java_debug.rs` drives the `vscode.java.*` workspace commands that the
java-debug plugin adds to jdtls (`resolveMainClass`, `resolveClasspath`, `startDebugSession`).

jdtls gets no information from our project model: it launches with its own `-data` directory under the
cache and runs its own Maven/Gradle import (`JdtlsProvider::launch`). So the IDE has two independent
classpaths for the same project (ours in `ProjectModel`, jdtls's in its workspace) and no way to tell
when they disagree.

### 1.3 Why the three owner cases fail (code path traced)

**Case 1: `@Te` offers String operations instead of `@Test`.**

1. Typing `@` reaches `IdeState::completion_after_typing` (`archipel-ide-core/src/completion.rs`).
   `trigger_before_caret` takes the trigger characters from the *running server's capabilities*; `@` is
   one of jdtls's, so `start_completion(CompletionTrigger::Character("@"))` is called with the anchor at the
   caret (after the `@`).
2. `start_completion` calls `self.lsp.request_completion(..)` and nothing else. There is no native
   source: no Java `CompletionContributor` is registered (`packs/java.rs` registers the resolve pass, a
   suppressor, the import fix provider, an activity, jdtls and the toolchain requirement; the
   `COMPLETION_CONTRIBUTOR` point is declared and empty for Java). If the server is not `Ready`, the session
   shows "starting" and only live templates are offered (`Err(LspError::Starting)` branch, `template_completion_items`).
3. Typing `Te` re-filters the response of the `@` request client-side (`refresh_completion`,
   `rank_completions` with `JdtlsProvider::completion_relevance`, which only demotes `Object` members). The
   list is whatever jdtls answered for the document state it had when it received the request.
4. The native class index is never consulted. `JavaIndex::classes_by_short_name` is exact-key, takes a full
   simple name, and is called only by `import_fix::class_candidates` after an unresolved reference exists.
   `Java:Annotation` indexes *uses* of an annotation (classes annotated with it), not annotation types.
5. Consequence: the result depends entirely on jdtls having finished its own import with JUnit on its test
   classpath and having parsed `@Te` as an annotation in the synced text. A list of unrelated `String`
   members is what jdtls returns for an identifier in expression position, i.e. it did not see an
   annotation context (unsynced `didChange`, unfinished import, or recovery of the parse). That the cause
   is jdtls state rather than our code is a hypothesis; M0 adds the trace that settles it (LSP log of the
   request and the text version sent). Either way, the structural defect is certain: with jdtls absent,
   late or wrong the IDE has no annotation-position, type-position or non-imported-class completion
   (JAVA-CMP-009, 018, 019 have zero implementation).

**Case 2: a missing method or class is not marked red while typing.** The native pass exists and, on a
fully loaded project, reports `Cannot resolve symbol`/`Cannot resolve method`. It stays silent in these
verified situations (file references are to `archipel-ide-java/src`):

1. `JavaProject::applies_to` (`api.rs`) needs the file to belong to a module of an *imported* project
   model. No import yet, a failed import, or a folder without Maven/Gradle files gives no native markup at
   all; `ResolveScope::for_file` returns `ScopeKind::Everything` for files outside every module and
   `analyze_text`/`analyze_tree` then return an empty analysis.
2. While the model imports or the index builds (`JavaProject::is_complete` false), unresolved names are
   reported as "Not resolved until the project is fully loaded" (`ProblemKind::Pending`, JAVA-ERR-003),
   not as errors. A Maven import that cannot resolve dependencies is not "loading" but *finished with a
   note*: "sources are imported without libraries" (`externalsystem/src/maven.rs`). The scope then has no
   jars, and the pass either reports every library name or, through the conservative rules below, nothing.
   The pass does not know the classpath is incomplete.
3. `method_invocation` with `has_error()` is skipped wholesale (`analyze.rs::method_invocation`). While
   typing, the tree has ERROR/MISSING nodes around the caret, and a nested or chained call inside such a
   statement is skipped although only a sibling is broken.
4. Conservative rules suppress errors on purpose: an unqualified call is reported only when *every* class
   of the lexical chain has a fully resolvable hierarchy (`unqualified_call`: `incomplete` from
   `methods_named`), and a file whose imports contain `lombok` or that mentions `@lombok.` is never reported
   for members (`Analyzer.lombok`): a hard-coded per-framework special case.
5. Qualified calls are checked only when the receiver's type is a known `Ty::Ref { fqn }`
   (`check_qualified_call`). `Ty` is a type *text* with erasure (`ty.rs::split`); there is no generics model
   (JAVA-TYP-001..041 are 0 of 41), so `list.get(0).missing()`, `stream.map(..).missing()`, `var x = foo()`
   from a generic method, lambda parameters and conditional expressions give `Ty::Unknown` and no error.
6. Constructor calls and argument mismatches are not checked: `new Foo(1, 2)`, `'m(int)' in 'C' cannot be
   applied to ...` (JAVA-ERR-008/009/010/021/022 not implemented; `java-intelligence.md` states it).
7. Cadence and cost: the daemon debounces edits by 300 ms (`daemon/src/scheduler.rs`), then `analyze_tree`
   rebuilds the file stub and re-walks the whole file every run (no per-member reuse, JAVA-RES-046), and
   open buffers are reindexed after a further 400 ms of quiet (`REINDEX_DELAY`), so a new class typed in
   file A is unknown to file B until then. None of this is measured against a budget.

The differential (`docs/analysis/java-differential.md`) shows 17 of 17 agreement, but on a corpus of
hand-written files whose receivers have simple types; it does not exercise 1, 2, 3 or 5.

**Case 3: classes from dependencies and the JDK are not found for completion or import.**

- Native side, what works: Maven/Gradle import yields `Library` entries with class roots
  (`ProjectModel::libraries_for`), `JavaIndex::sync` indexes each jar and the JDK image into stubs, and
  `ResolveScope::for_file` ranks them by classpath position with test-scope libraries only for test code.
  The import-class fix finds `org.junit.jupiter.api.Test` by exact short name (`imports.rs` tests do this
  with real jars from `~/.m2`).
- What fails: (a) completion never queries the index (case 1); (b) the lookup API is exact-key: there is
  no prefix, camel-hump or substring query over short names, no package-member listing and no static-member
  index (JAVA-STUB-014 not implemented), so even a native contributor could not be written yet; (c) the
  short-name entry stores a `ClassRef` (top name + nested path), not kind/flags, so "annotation types only"
  would load a stub per candidate; (d) the classpath itself can be wrong or missing: Maven resolution needs
  the dependency plugin and network on first use, with `provided`/`system` merged and profiles ignored;
  Gradle is one module per project; the JDK is "the JDK that ran the tool" (`detect_jdk`: `JAVA_HOME`/`PATH`),
  not the module's toolchain, and `rt.jar` (JDK 8) and `ct.sym` are unsupported; (e) jdtls, which serves
  completion and the jdtls import fix today, keeps a separate classpath (1.2) that this IDE cannot inspect.

### 1.4 What jdtls-removal needs beyond the owner's cases

Removing jdtls also removes hover, navigation, find usages, rename, signature help, code actions,
formatting and the debug adapter plugin. The plan covers analysis (resolve, errors, completion, imports,
inspections); the remaining jdtls features are listed as prerequisites in M7 so removal is not scheduled
before they have native replacements or an explicit decision (2.10, Risks).

### 1.5 Existing special cases to remove

- `Analyzer.lombok` (`analyze.rs`): silences member errors in any file mentioning lombok. Replace by the
  augment mechanism in M2 (a language extension point for synthetic members, default implementation reading
  annotation-processor output directories from the model; until then, a file with an annotation whose
  type is unresolved is "uncertain" by the general rule, not by name).
- `JDTLS_FAVORITE_STATIC_MEMBERS` (`lsp/src/registry.rs`): a list of well-known test classes to make jdtls
  offer static imports. It disappears with jdtls; the native import search needs no list (JAVA-IMP-010).

## 2. Target architecture (general machinery)

One engine, layered; every upper layer consumes only the layer below, and Kotlin/Groovy can later reuse
layers 1 to 3 (class files and the index are language independent). Crates stay as today; new modules are
named for the existing crate they extend. `archipel-ide-java` is already large (11k lines), so M2 splits it
along these layers into `archipel-ide-java` (facade and pack glue) plus modules `model`, `names`,
`resolve`, `types`, `check`, `complete`, `inspect`; a crate split is only done if compile time demands it.

```
 project model ─► classpath snapshot ─► unit readers ─► stub indexes (durable, incremental)
 (Maven/Gradle,    per module+scope      class file,      short names, packages, members,
  JDK, roots)      (ordered units,       jar/jmod/jrt,    static members, annotations,
                   completeness flag)    source stubs     inheritors, module exports
                                               │
 source PSI (tree-sitter, incremental) ────────┤
                                               ▼
                      resolve (scopes, imports, inheritance, generics)   spec 02
                                               ▼
                      types (subtyping, inference, expected types)       spec 03
                       ┌───────────────┬───────────────┬────────────────┐
                       ▼               ▼               ▼                ▼
                 error checks     completion      import machinery   inspections
                 (spec 04)    contributors (06)   (05, codeinsight   (07, 09)
                                                   05/08)
                       └──── daemon passes / ModCommand / Alt+Enter / completion UI ────┘
```

### 2.1 Project model and classpath per module and scope

The model is the single source of truth for classpaths; jdtls stops being a second one (in the transition
it is *fed* from the model, see 2.9).

- **Classpath snapshot.** A new immutable value `ClasspathSnapshot` per (module, `ClasspathKind`) derived
  from `ProjectModel::classpath`: ordered list of units (own source roots, module dependencies' production
  output, libraries by scope, SDK), the project's language level and a `complete: bool` with a reason
  (`Imported | Cached | DependenciesUnresolved(note) | Importing`). `ResolveScope` becomes a view over it.
  The completeness flag replaces the binary `model_loading`: an import that finished without dependencies is
  reported in the status bar and the pass downgrades to Pending *only for names that could plausibly come
  from the missing libraries* (see 2.4), never silently.
- **Ad-hoc module.** A `.java` file outside every module (scratch, a folder without a build file) gets a
  synthetic module (content root = nearest directory with Java sources, SDK = detected JDK, no libraries)
  instead of `ScopeKind::Everything` + empty analysis. Native errors and completion then work in any
  folder (JDK classes only, which is truthful).
- **Test scope** follows the existing rule (own tests and test-scope libraries only for test sources,
  JAVA-RES-029). Gradle `testFixtures`/custom source sets and Maven profiles are project-model work tracked
  in M1 (the gaps are listed in `project-model.md` "Known gaps"); the engine only consumes the result.
- **JDK.** Per module SDK from the model (toolchain-aware where Gradle reports it, `maven.compiler.release`
  /`release` for the language level). Reader support: JDK 9+ jimage (done), JDK 8 `rt.jar` (new reader
  through `read_jar`), module `exports` filtering (done at package level), `--release` filtering via `ct.sym`
  (JAVA-RES-040, LATER), multi-release jars by language level (done, add tests, JAVA-RES-031).

### 2.2 Class-file and archive readers

`classfile.rs` and `jimage.rs` stay. Additions: `.jmod`/`jrt:` addressing for `--module-path` libraries
and JDK 8 `rt.jar`; `module-info.class` for named-module filtering (JAVA-RES-032, SHOULD); Kotlin metadata
is out of scope here. Readers are pure functions bytes -> stub (JAVA-RES-034, JAVA-STUB-027: a corrupt class
isolates to that class); they run on the index worker pool with cancellation.

### 2.3 Stub indexes: persisted, incremental, prefix-queryable

The unit model (one stub set per source file / jar / class dir / JDK image, stamped len+mtime+hash, redb
store, `STUB_VERSION`) stays. Changes:

- **Name index with ordered keys.** `Java:ShortName` keys are stored in sorted order (redb range scans)
  with a *summary value*: `{ClassRef, kind, flags, deprecated, annotation-target mask}` so position filters
  (annotation types only, interfaces only, `Throwable` subtypes via `Java:SuperClass`) need no stub load.
  A `NameMatcher` (camel-hump, substring, case modes: spec 06 6.4.1, CI-CMP-050..052) is the single
  matcher; today two exist (`archipel-ide-lsp/src/completion.rs::fuzzy_match` and
  `archipel-ide-core/src/name_match.rs`); M1 moves one implementation into a shared crate and deletes
  the other. Queries: `names_matching(pattern, scope, limit)`; an in-memory per-scope name table (rebuilt on
  scope change, 100k to 300k names, a few MB) serves the hot path, the redb index is the durable source.
- **Package index** with child packages and per-package class lists (`Java:Package` today stores only
  existence): `classes_in_package(pkg, scope)`, `subpackages(pkg, scope)` for qualified-name completion and
  JAVA-RES-027.
- **Member indexes:** method and field names (exist, exact), plus the *static member names/types* indexes
  (JAVA-STUB-014) and the method-parameter-types index (JAVA-STUB-015, SHOULD), prefix-queryable.
- **Annotation types.** Annotation *types* are found through `ClassKind::Annotation` in the name summary;
  `@Target`/`@Retention` are read from the annotation class's own stub annotations (already stored as text)
  into the target mask at index time. No library-specific knowledge anywhere.
- **Persistence and incrementality.** Keep unit-level stamps (JAVA-STUB-018, PLAT-IDX-030..036). Add:
  one progress/phase report per PLAT-IDX-037; a *shared library cache* keyed by jar content hash so two
  projects (or two modules) using the same JUnit jar index it once (PLAT-IDX-034, 011); parallel unit
  indexing; a body edit leaves the stub, and so all index entries, byte-identical (exists, tested).
- **Overlay for unsaved text.** The overlay of open buffers (today reindex after 400 ms of quiet) becomes
  synchronous for the stub of the active file: the stub is rebuilt from the incremental syntax tree on each
  edit (it is cheap: bodies skipped) and published to an in-memory overlay layer ahead of the durable index,
  so a class typed in A is visible in B immediately; the durable write stays debounced.

### 2.4 Resolve (spec 02)

Extract the pure resolution from the reporting visitor in `analyze.rs`:

- `resolve` module: a `Resolver` that answers, for any reference node, a `ResolveResult` (targets, candidates
  with validity/accessibility/static-context flags, the module/jar of the target; JAVA-RES-001/002) by the
  scope walk of 2.3 and the file-level order of 2.5 (own classes, single imports, same package, on-demand,
  `java.lang`, static imports). The same walk produces completion variants (JAVA-RES-001), so completion and
  errors cannot disagree.
- **Inheritance with generics:** supertypes carry type arguments (`Signature` attribute / source text
  parsed into the type model of 2.5); member lookup substitutes through the supertype path (JAVA-TYP-013).
- **Overload resolution:** arity, then phases strict/loose/varargs, most specific (JAVA-RES-012/013/015/018),
  using real applicability from 2.5; this replaces the erasure-level `import_fix::applicable`.
- **Incomplete-world discipline replaces special cases.** The rule that suppresses errors becomes explicit
  per reference: a failure is *certain* when the lookup consulted only complete, resolved supertypes and
  the classpath is complete for that name; it is *uncertain* when a supertype is unresolved, when an
  annotation on the class is unresolved (a possible processor), or when the classpath is incomplete and
  the name is not a JDK/own-source name. Uncertain failures are reported at the *cause* (the unresolved
  supertype or annotation, which is itself an error) and are not cascaded into member errors. Synthetic
  members come from an `AUGMENT_PROVIDER` extension point (records, enums, `values()` are built in per
  JAVA-STUB-022/023; annotation-processor output is read from the model's generated-source roots, JAVA-STUB-025
  LATER) instead of a text test for `lombok`.
- **Error-tolerant per-reference analysis.** Replace `node.has_error()` skipping with a check of the
  reference's own span: a reference is analyzed unless an ERROR/MISSING node lies *inside* it; the
  incomplete identifier at the caret is analyzed too (IntelliJ marks `foo.ba` red immediately, JAVA-TYP-032
  forbids cascades of inference errors).
- **Caching (JAVA-RES-046/047, PLAT-CACHE):** per-file resolve results keyed by (syntax revision, index
  epoch of the units the file's scope reads); analysis is split per top-level member so an edit inside one
  method body re-checks that member plus references whose resolution depends on the changed stub (stub
  equality ignores bodies, so most edits re-check one member).

### 2.5 Types (spec 03)

A closed type model (`Ty`: primitives, class types with arguments, arrays, type variables, wildcards,
intersections, captured, null, unknown/error) replaces the text-typed `Ty`; structural equality, canonical
and presentable printing (JAVA-TYP-001..003). Subtyping/containment, conversions, erasure, lub, boxing,
varargs (005, 006, 010, 016, 019, 021, 023), capture (015), substitution through supertypes (013), then
inference (026..031: bound sets, nested generic calls, diamond, implicit lambdas, method references) and
`var` (037). Expression typing on demand, cached per tree revision (JAVA-TYP-025), with cancellation
points and work limits (036). Unknown/erroneous types never produce follow-on errors (032). This is the
largest single block of work (M5) and is deliberately sequenced after resolve and completion: those need
only declared types and erasure-plus-simple-substitution, which M2 introduces on the new model.

### 2.6 Error highlighting while typing

- Checks are pure functions `(resolve result, types) -> Vec<Problem>` from the catalog of spec 04 (message
  text, range and fix by row), in phases that stop at the first error per element (JAVA-ERR-002):
  reference resolution rows first (007, 011, 012, 013..016, 018, 021), then applicability (008, 009, 010),
  then type rows. Fixes (import class/static member, create ...) attach per row through the existing
  `QuickFix`/`ModCommand` contract.
- Delivery by the existing daemon (`java-resolve` pass, `visible_range_first`): debounce from 300 ms to 100
  ms for the Java native pass (configurable `Pass::debounce`; the 300 ms stays for jdtls/external passes),
  visible range first, results shifted by edits until replaced (CI-DMN-020, 052), cancelled by the next
  keystroke (CI-DMN-040). Budget in 3.5.
- Pending ("Not resolved until the project is fully loaded") only while the *name's* answer can still change:
  index not ready, or classpath incomplete and the name could come from a missing library. Everything else is
  reported.
- Coverage telemetry (M0): per analyzed file, counts of checks skipped by reason (`parse-error`,
  `unknown-receiver`, `incomplete-hierarchy`, `incomplete-classpath`, `no-module`), exposed on the control
  socket and in a developer "Why no errors here?" action. This is how the silent cases of 1.3 are measured
  and then driven down.

### 2.7 Completion as contributors (spec 06, codeinsight 06)

Completion is a native pipeline in `archipel-ide-java::complete`, registered on the existing
`COMPLETION_CONTRIBUTOR`/`COMPLETION_WEIGHER` points, run on the interactive job lane with cancellation:

1. **Position classifier** over the error-tolerant syntax tree plus the text before the caret: reference
   (unqualified expression, qualified `q.|`), type position (declaration, `new`, cast, `instanceof`,
   generics, `extends`/`implements`/`throws`/`catch`), **annotation name** (`@|`, `@Te|`, also in a broken
   tree where tree-sitter produced an ERROR node: the classifier looks at the last tokens), annotation
   attribute, import, declaration (names, modifiers), javadoc, comment/string (no completion). `@Te|` and
   `@|` are one general case: "annotation position".
2. **Context:** prefix, invocation count, expected type (spec 6.6, from M5; before it, none), qualifier
   type, enclosing declaration kind and target (for annotation applicability), resolve scope (so test
   libraries appear only in test files).
3. **Contributors** (stop semantics of spec 6.2): reference variants from the resolver's scope walk (locals,
   fields, methods, in-scope classes, statically imported members); member access on the qualifier type
   (incl. inherited, `Object` on interfaces, static vs instance, accessibility); **class names** from the name
   index filtered by the position (annotation positions keep `ClassKind::Annotation` types whose target mask
   fits the annotated element; `implements` keeps interfaces; `throws` keeps `Throwable` subtypes via the
   inheritor index; `new` keeps instantiable classes), with the matcher of 2.3; static members of any class on
   count 2 (JAVA-CMP-022); keywords and modifiers valid in the position; member names; override generation
   (SHOULD); import positions; javadoc (SHOULD).
4. **Items and insertion.** Items are the existing LSP-shaped `CompletionItem` (the core session, ranking
   and popup stay), with `additional_text_edits` computed *at insertion time* by `import_edit` (single-type
   import in sorted position, on-demand thresholds, FQN fallback on a name clash, no edit for `java.lang`/same
   package/already imported/inside an import statement, one undo step: JAVA-CMP-019/020, JAVA-IMP-044/053/055).
   A lone non-imported candidate is never auto-inserted (JAVA-CMP-021). `core/completion.rs` already applies
   additional edits in one undo step (`additional_text_edits_add_imports_in_one_undo_step`).
5. **Ranking:** a `CompletionWeigher` chain per 6.5: prefix quality, learned usage (existing
   `completion_stats.rs`), kind tiers, expected type, statics, proximity (explicit import, same module,
   resolve-scope rank, discouraged packages last), shorter first. Auto-popup rules and invocation counts
   follow codeinsight 06 (CI-CMP-030, 100, 101).
6. **Core wiring.** `IdeState::start_completion` asks a `CompletionSource` trait: the native Java source
   answers synchronously from the overlay/indexes (no process, no network); the LSP source is consulted
   only while jdtls is still installed and `java.completion.source` says so (`native`, default once M3
   lands; `both`: merged and de-duplicated by (kind, FQN/signature) with native first; `jdtls`). Dumb mode:
   while indexes build, the contributors that need only the open file's own syntax still answer
   (CI-CMP-110), plus a footer "Indexing...", as for jdtls today.

`@Te` then works as follows with no special case: the classifier says "annotation position, target =
method"; the name index returns annotation types matching `Te` in the *test* resolve scope; JUnit 5 on the
test classpath contributes `org.junit.jupiter.api.Test` (and JUnit 4's `org.junit.Test` if present); the
item inserts `Test` and an import edit.

### 2.8 Imports (spec 05, codeinsight 05/08)

Already largely native (`import_fix.rs`, `import_edit.rs`). Remaining: the auto-import fix on *any*
unresolved class or static member reference through the new resolver (no per-error code), expected-type
filtering of static candidates (JAVA-IMP-019, after M5), proximity weighers (021), the hint bubble and
the silent auto-import setting (034..037, 047), exclusion sub-popup (036), rebinding of other references
(038), missorted-imports inspection (075), and the *commands*: Optimize Imports (`Ctrl+Alt+O`, CI-OPT)
bound to the library function that exists, on save if configured.

### 2.9 Inspections framework (specs 07 and 09)

`LocalInspection` points exist (`INSPECTION`/`LOCAL_INSPECTION_TOOL`, daemon `inspection/` with a Tier-A
table in `tier_a.rs`/`kinds.rs`, profiles, suppression). The Java work is a *visitor framework over the
resolver and type model*: a `JavaInspection` trait (visit element kinds, report `ProblemDescriptor` with
range, message template, fixes), a registry with short name, group, severity, default state and options
(JAVA-INS-001/004), a shared per-file `InspectionContext` that hands out cached resolve/type queries, and a
batch driver for `inspect batch` cases. The catalog is data-driven where possible (message templates and
options from the spec tables) and code where analysis is needed. Data flow (nullability, constants,
JAVA-INS-062) is a separate engine and out of the first catalog tranche.

### 2.10 Transition and removal of jdtls

- **Feed jdtls from the model** while it exists: at startup and on model change send its classpath via
  `workspace/didChangeConfiguration` (`java.project.referencedLibraries`) or a generated `.classpath`-free
  project description so jdtls and the native engine see the same jars; this removes the "two classpaths"
  failure mode for the transitional period and makes differential runs meaningful. Cost: small; decision
  in Risks.
- **Setting `java.engine`** per feature (diagnostics, completion, imports): `native | both | jdtls`, with
  per-feature graduation by the numbers in M7. `both` shows jdtls items/diagnostics only where native has
  none (existing `reconcile`).
- **What jdtls still does** is listed in 1.2; each item gets a native replacement ticket before removal
  (hover/doc from stubs and source attachments JAVA-RES-037, definition/usages over the resolver and the
  inheritor/member indexes, rename and refactorings via `archipel-ide-core/src/refactor.rs` and `docs/specs/refactoring/`, signature
  help from overload resolution, formatting, and a debug adapter that no longer lives inside jdtls).

## 3. Test harness

The tests are the product: no milestone ships without its cases going green and its baseline raised.

### 3.1 Case runner for `docs/specs/java/cases/*.cases.md`

Same shape as `editor_cases.rs` and `search_cases/` (parse every `###` block, run, compare with the `after`
block, `PASS_BASELINE` that may only rise, `JAVA_CASES_ONLY=JAVA-CMP`, `JAVA_CASES_VERBOSE=1`, a generated
report `docs/parity/java-cases.md`, `cargo xtask java-cases`). Two layers over one parser (reuse
`tests/support/editor_case_format.rs`):

- **Engine layer** (`archipel-ide-java/tests/cases/`, no `IdeState`, fast, hermetic): builds a project
  model from the case (`modules:` lines, `--- path` files, `settings: library=junit5`), runs `inspect`,
  `complete`, `Alt+Enter` over the native pipeline and compares text/markup. Runs in default `cargo test`.
- **Editor layer** (`archipel-ide-core/tests/java_cases.rs`): the same cases through `IdeState` and the real
  key path (typed characters, auto-popup, Enter, undo), including the popup rows and the import edit, with
  no language server registered.

New case files to author (the directory has only the 09 files today; 1528 inspection cases already exist):
`02-resolve`, `03-types`, `04-errors` (one case per catalog row implemented), `05-imports`,
`06-completion` with the owner's cases as the first three:

```
### JAVA-CMP-009 case 1: annotation position offers annotation types from the test classpath
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;
class AppTest {
  @Te<caret>
  void greets() {}
}
```
keys: complete
```after
item: Test (org.junit.jupiter.api)
not item: String
```
keys: complete, choose "Test (org.junit.jupiter.api)"
```after
(file with `import org.junit.jupiter.api.Test;` and `@Test`)
```
```

Extended keys: `complete [count N]` (list `item:`/`not item:` lines in popup order, `rank: a < b`), `choose
"<label>"`. Error cases use the existing `keys: inspect` format.

### 3.2 Fixture projects with real jars

- **Hermetic jars (default).** `crates/archipel-ide-java/tests/fixtures/` holds *our own* tiny Java sources
  for the shapes that matter (annotation with `@Target`, nested classes, generics, a test-framework-like
  `Test`/`Assertions`, a library with a missing dependency) and a `build.rs`-free script
  `tools/java-fixtures/build.sh` that compiles them with the JDK and writes jars to a cache directory
  (`target/java-fixtures/`, content-hash named) when the JDK is present. No third-party binaries in the
  repository (clean room, licences).
- **Real jars (nightly + ignored locally).** A pinned lock file lists coordinates and SHA-256 of JUnit 5
  (`junit-jupiter`, `-api`, `-params`), JUnit 4, AssertJ, Hamcrest, Mockito, Guava, Spring Boot test
  starter, Lombok; `tools/java-fixtures/fetch.sh` downloads them from Maven Central into the same cache
  (verified, offline afterwards). The `~/.m2` lookup in `tests/common/mod.rs` goes away, and tests that
  need a jar *fail* (not skip) when the cache is absent in CI.
- **JDK.** The tests use the JDK the build runs on (`JAVA_HOME`), plus a pinned JDK 17 and 21 in the
  nightly matrix (jimage layout differences), and a JDK 8 `rt.jar` fixture for M1.
- **Project fixtures:** a Maven multi-module project, a Gradle project with a custom source set and a
  project without any build file (ad-hoc module), each with a *golden model JSON* so classpath tests do not
  need to run the build tool; the build-tool import itself stays in the existing ignored real-tool tests.

### 3.3 Differential tests

- **Against javac (ground truth for errors).** The JDK ships the compiler: run `javac -proc:none -d /tmp
  -cp <fixture classpath>` over the corpus and compare `cannot find symbol` /`package does not exist`
  /`method ... cannot be applied` locations with native errors. This gives recall and false-positive rates
  without jdtls, runs offline and is the primary gate for 04 rows.
- **Against jdtls (behavioral reference while it exists).** `java_differential.rs` grows from one project to
  the corpus of 3.4 and records jdtls output as *golden JSON* (`tests/differential/golden/`, refreshed
  with `UPDATE_JDTLS_GOLDEN=1`) so CI compares native output to recorded transcripts without launching
  jdtls; a nightly job refreshes and diffs them. Compared: resolve-kind diagnostics (existing definition
  of agreement: same line, one range contains the other), completion lists at sampled positions, import
  fix candidates.
- **Completion by masking.** For every identifier occurrence in the corpus (sampled), cut the identifier at
  its second character, request completion, record the rank of the original name (hit@1/@5/@20) and
  whether an import edit was produced when needed. Needs no oracle, measures exactly the user's pain.
- **False-positive hunt** (exists as `corpus.rs`) becomes automatic over pinned source jars (AssertJ,
  JUnit, Guava, Spring Boot test) and the repository's own fixtures.

### 3.4 Corpora

Pinned sources jars of AssertJ core, JUnit 5 api/engine, Guava, Spring Boot test (already used by
`corpus.rs`), plus three small Maven/Gradle sample applications authored in the repository (own code).

### 3.5 Performance budgets (initial; calibrated by the M0 benchmark, tightened, never loosened)

All measured by `cargo bench -p archipel-ide-java` with `ARCHIPEL_PERF_ASSERT=1` (as in
`analysis/benches/analysis.rs`) on the CI runner class and reported in `docs/perf/java-engine.md`.

| Operation | Budget |
|---|---|
| Completion at a caret (class names, members, annotations), indexes ready, p95 | < 50 ms, p99 < 100 ms; class-name query over 300k names < 10 ms |
| Native diagnostics after the last keystroke, 2k-line file, warm, wall time of the pass, p95 | < 150 ms (plus the Java pass debounce of 100 ms) |
| Alt+Enter import candidates (class or static), p95 | < 50 ms |
| Cold index: JDK 21 image (exported packages) | < 8 s |
| Cold index: JDK + typical Maven project (Spring Boot web + test starters, about 80 jars, 3 modules) | < 20 s, interactive features degrade to "Indexing..." not to errors |
| Warm start (persisted index, unchanged) to Ready | < 500 ms |
| Add or change one jar / 100 changed source files | < 1 s / < 2 s |
| Index size on disk, JDK + that project | < 150 MB; resident name tables < 40 MB |
| Memory held by resolve caches for 20 open files | < 64 MiB (the existing cache budget) |

## 4. Milestones

Sizes are for one engineer who knows the codebase (S = about 1 week, M = 2 to 3, L = 4 to 6, XL = 7 or
more); two engineers can run M1/M2 and M3/M4 partly in parallel. Each milestone ends with: its cases green
and `PASS_BASELINE` raised, `docs/specs/coverage.toml` updated (generated report shows the new IDs), the
user-visible result for the owner's cases stated below.

### M0: measurement and harness (S, prerequisite)

- Tasks: coverage telemetry and "Why no errors here?" (2.6); LSP trace of the completion request and sent
  document version (settles 1.3 case 1); hermetic fixture jars and the pinned fetch script (3.2); the
  case runner skeleton with engine layer and the first 15 cases written for the three owner pains (all
  failing at first, baseline = what passes); benchmarks with the first numbers (3.5); remove the `~/.m2`
  lookup.
- Crates/files: `archipel-ide-java/tests/{cases,fixtures,common}`, `tools/java-fixtures/`,
  `archipel-ide-core/src/diagnostic_commands.rs` (action), `archipel-ide-control` (status), `xtask/src`
  (`java-cases`), `docs/parity/java-cases.md`.
- Owner effect: none directly; every later claim becomes a number. Requirement IDs: none new (enables
  JAVA-TYP-035 corpus and all `Accept:` lines).

### M1: classpath, jar/JDK index, short-name lookup (M)

- Tasks: `ClasspathSnapshot` with completeness reasons and the ad-hoc module (2.1); ordered, prefix- and
  camel-hump queryable short-name index with summary values, package index with children, static-member and
  parameter-type indexes (2.3); single shared `NameMatcher`; JDK 8 `rt.jar`, multi-release tests, compressed
  jimage fallback, module exports tests; shared library cache by jar hash; synchronous stub overlay for open
  buffers; surface "classpath incomplete" in the status bar and the control socket; project-model gaps that
  hide jars: Maven dependency resolution failure note becomes a visible import diagnostic with a retry
  action, Gradle source sets as modules, JDK per module.
- Crates/files: `archipel-ide-java/src/{index.rs,scope.rs,unit.rs,jimage.rs}` + new `names.rs`,
  `archipel-ide-projectmodel`, `archipel-ide-externalsystem/src/{maven.rs,gradle.rs,jdk.rs}`,
  `archipel-ide-index` (ordered keys, phases), `archipel-ide-langpacks/src/packs/java.rs`.
- Tests: `tests/index.rs` extensions (prefix, hump, package listing, static members, per-scope results
  with test libraries only in test scope, JDK 8/17/21), property test "index of unit == index after
  incremental update", benchmarks of 3.5 (index and name queries), model golden tests.
- Requirement IDs: JAVA-STUB-011, 013..018, 026, 028; JAVA-RES-026, 027, 029..033, 034..036, 038, 039, 041,
  042, 047; PLAT-IDX-011, 020..023, 030..037; PLAT-SCOPE-054, 062..067.
- Owner effect (case 3, data level): `java.lookup(prefix, file)` over the control socket returns
  `org.junit.jupiter.api.Test` for `Te` in a test file and not in a production file, `ArrayList` and `Path`
  from the JDK, in every Maven/Gradle fixture; classpath status says why a jar is missing.

### M2: resolve and "Cannot resolve ..." while typing (L)

- Tasks: split `analyze.rs` into the pure resolver and the checks (2.4); new `Ty` model with declared types
  and simple substitution (not inference), so receivers like `list.get(0)` and chains over library
  signatures have types; per-reference error tolerance; the explicit certain/uncertain rule replacing
  `has_error` skipping and the `lombok` text test; `AUGMENT_PROVIDER` point; error rows 007, 011, 012, 013,
  014..016, 018, 019, 021, 023, 027 with the exact messages; the pending rule narrowed to names that can
  still change; per-member incremental re-analysis and result caching keyed by (revision, index epoch);
  Java pass debounce 100 ms; cross-file freshness through the synchronous overlay; the `@Te` / unknown
  type in a declaration marked red as `Cannot resolve symbol`.
- Crates/files: `archipel-ide-java/src/{resolve,check,types}/` (from `analyze.rs`, `env.rs`, `ty.rs`),
  `diagnostics.rs`, `archipel-ide-analysis` (`Pass::debounce`, `AUGMENT_PROVIDER` in `langext.rs`),
  `archipel-ide-daemon/src/scheduler.rs`.
- Tests: cases for 02 and the implemented 04 rows; `errors.rs` ported; javac differential on the corpus
  (target: zero false positives, recall of `cannot find symbol` >= 98 %); the keystroke-to-markup benchmark;
  `unsaved_analysis.rs`-style flows for typing a method call and a class name; coverage telemetry shows the
  `unknown-receiver` and `parse-error` skip counts falling.
- Requirement IDs: JAVA-RES-001..008, 010..013, 015, 018..021, 023, 026..028, 043, 044, 046; JAVA-ERR-001..004,
  007, 011..016, 018, 019, 021, 023, 027; JAVA-IMP-076; CI-DMN-020, 030..031, 040..052; JAVA-STUB-022, 023.
- Owner effect (case 2): `foo.missing()`, `new Missing()`, `Missing x;` and a missing method on a
  library-typed receiver (also through chains) are red within one debounce while typing, in a project with
  or without a build file; a classpath that failed to import produces a visible classpath note, not silence.

### M3: completion contributors, annotation/type positions, non-imported classes (L)

- Tasks: position classifier (error tolerant) (2.7); contributors 4, 5, 8, 9, 10, 11 of spec 6.2 for
  positions 1 to 3 and 6 to 7 of 6.3 (reference variants, member access, type positions, annotation names and
  attributes, import positions, keywords/modifiers); class-name contributor with import-on-insert and FQN
  fallback; static members on count 2; weigher chain (prefix, usage, kind, scope, proximity, shorter);
  overloads merged; deprecated struck through; exclusions and discouraged packages; native
  `CompletionSource` in core and the `java.completion.source` setting; auto-popup rules.
- Crates/files: `archipel-ide-java/src/complete/`, `archipel-ide-langpacks/src/packs/java.rs` (register
  contributors), `archipel-ide-analysis/src/assist.rs`, `archipel-ide-core/src/completion.rs`
  (`CompletionSource`), `archipel-ide-lsp/src/completion.rs` (matcher moved), `completion_stats.rs`.
- Tests: `06-completion` cases (JAVA-CMP-001..021, 025, 027, 028; the `@Te` case of 3.1 as the headline),
  masking benchmark (hit@1/5/20 against jdtls golden), completion latency benchmark, editor-layer typing
  flows with no language server registered.
- Requirement IDs: JAVA-CMP-001, 003, 005, 008, 009, 012, 013, 016..021, 025, 027, 028, 035; JAVA-RES-001,
  033; CI-CMP-010, 020..023, 030, 031, 040, 041, 050..052, 060..063, 090..093, 100, 101, 110, 111;
  JAVA-IMP-044, 053, 055 (insert path). SHOULD set (002, 010, 011, 014, 015, 022, 024, 029..033) follows in
  M5/M6.
- Owner effect (case 1 and case 3): `@Te` offers `Test` with package tail and inserts `@Test` + the import,
  in a JUnit 5 or 4 project; `ArrLi` offers `ArrayList` (import added), `Path` from the JDK; production files
  do not see test libraries; works with jdtls not installed.

### M4: Alt+Enter "Import class", auto-import and Optimize Imports (S to M)

- Tasks: rebase `import_fix.rs` on the new resolver so every unresolved class/static reference (including
  annotation names, generic arguments, `catch` types, in javadoc `@link`) gets the fix with the candidate
  rules of 5.2/5.3; Optimize Imports command and keymap entry, on-save option; settings page items
  (exclusions, on-demand thresholds, JAVA-IMP-050/083); hint bubble and silent auto-import for a unique
  candidate (JAVA-IMP-034..037, 046, 047); rebinding of other references (038); missorted-imports
  inspection (075).
- Crates/files: `archipel-ide-java/src/{import_fix.rs,import_edit.rs}`, `archipel-ide-core/src/code_actions.rs`
  (+ command), `archipel-ide-ui/src/import_chooser.rs`, `archipel-ide-keymap`.
- Tests: `05-imports` cases (S1..S9 of the spec as cases), existing `imports.rs` ported to hermetic
  fixtures, `codeinsight/05` and `08` cases.
- Requirement IDs: JAVA-IMP-001..007, 010..023, 030..038, 040..049, 050..059, 061, 070..076, 080..083;
  CI-IMP-001..022 and CI-OPT-001..018 (MUST subset).
- Owner effect: Alt+Enter on any red class name offers `Import class` first; Optimize Imports works from
  the menu.

### M5: types, generics, method applicability errors (XL)

- Tasks: full type model and conversions (2.5), supertype substitution, capture, lub, varargs phases;
  inference with bound sets, diamond, lambdas, method references, `var`; expected-type derivation (6.6)
  feeding smart completion and import-candidate filtering; error rows 008, 009, 010, 021, 022 and the type
  rows (incompatible types, bad operand, unreported exception, missing return, ...) in stages ordered by
  prevalence in the javac differential; lambda and method-reference completion items (JAVA-CMP-010);
  smart completion (002, 032, 033).
- Crates/files: `archipel-ide-java/src/types/`, `check/`, `complete/`.
- Tests: `03-types` cases (the scenario table of the spec) and JDK stream/collector idioms
  (JAVA-TYP-035), javac differential extended to `cannot be applied`/`incompatible types`, inference
  work-limit and cancellation tests, benchmarks on the stream-heavy corpus.
- Requirement IDs: JAVA-TYP-001..041 (MUST first), JAVA-RES-012..018, 045; JAVA-ERR-005, 006, 008..010,
  017, 020, 021, 022, 025, 026 and the type rows of 04; JAVA-IMP-018, 019; JAVA-CMP-002, 010, 032, 033.
- Owner effect: calling an existing method with wrong arguments is marked, `list.stream().map(..)...`
  chains complete and check, expected-type ranking in completion.

### M6: inspections framework and default-on catalog (XL, incremental)

- Tasks: `JavaInspection` framework and registry (2.9, JAVA-INS-001..004), local and batch drivers, profile
  and suppression integration with the daemon; tranche 1: the 34 MUST rows of spec 07 plus the unused-symbol
  family (needs the reference tables of 02), then tranches of `09-inspections-catalog-1..10` by default-on
  state and by dependency on the data-flow engine (data-flow engine is its own sub-milestone, JAVA-INS-062
  LATER); quick fixes through `ModCommand`; per-inspection options in the settings UI.
- Crates/files: `archipel-ide-java/src/inspect/`, `archipel-ide-daemon/src/inspection/` (registration only),
  `archipel-ide-langpacks/src/packs/java.rs`.
- Tests: the 1528 cases of `09-inspections-*.cases.md` run through the engine layer
  (`keys: inspect`, `inspect batch`, `Alt+Enter, choose ...`); `PASS_BASELINE` ratchet per tranche.
- Requirement IDs: JAVA-INS-001..004, 058..061 plus the rows of 07; JAVA-ICA-* of catalogs 1 to 10;
  CI-INS-*, CI-FIX-*, CI-ALT-* (as they apply to Java).

### M7: jdtls removal criteria and the removal (L, mostly non-engine)

Not a feature milestone: a gate, then a deletion. Criteria, all measured by the harness of 3 and recorded in
`docs/parity/java-engine.md` per release:

| Area | Threshold to flip the default to `native` | Threshold to delete jdtls |
|---|---|---|
| Resolve errors vs javac on the corpus | false positives 0; recall of `cannot find symbol`/`package does not exist` >= 98 % | >= 99 %, 0 false positives for two consecutive releases |
| Resolve errors vs jdtls golden | no native-only error not confirmed by javac; jdtls-only >= 98 % covered | as left, for all fixtures incl. nightly JDK matrix |
| Completion (masking, hit@1 / hit@5 / hit@20) | each within 2 points of jdtls golden or better | each >= jdtls golden |
| Import-on-insert / Import class candidates | 100 % of the cases of 05/06; same first candidate as jdtls on 95 % of sampled positions | 100 % of the cases |
| Applicability and type errors (008/009/010) | after M5: recall >= 90 % vs javac, 0 false positives | >= 95 % |
| Latency | all budgets of 3.5 met on the CI runner and on the reference laptop | same, plus p99 |
| Spec coverage (`docs/specs/coverage.md`) | Java 02, 04 (resolve and type rows), 05, 06: all MUST implemented; 01 all MUST | plus SHOULD of 05 and 06, 07 MUST |
| Features currently served by jdtls | listed in 1.2: each with native implementation or an explicit, recorded product decision | all done |
| Field data | opt-in telemetry-free "native only" setting used for two releases without a P1 false-positive report | default for two releases |

Steps: `java.engine` defaults to `both` after M3/M4, to `native` per feature as the table allows, then jdtls is
no longer started unless the user selects it; after two releases the `JdtlsProvider`, its toolchain
requirement, `java.apply.workspaceEdit` and `JDTLS_FAVORITE_STATIC_MEMBERS` are deleted, the Java pack drops
`cx.language_server`, and `docs/architecture/language-packs.md` loses the jdtls rows. kotlin-lsp follows the
same plan with the Kotlin pack and is out of scope here.

### Summary

| Milestone | Size | Owner pain addressed |
|---|---|---|
| M0 measurement, harness | S | all (makes them measurable) |
| M1 classpath + jar/JDK index + lookup | M | case 3 (data level) |
| M2 resolve + errors while typing | L | case 2 |
| M3 completion contributors + import on insert | L | cases 1 and 3 |
| M4 Import class, auto-import, Optimize Imports | S to M | cases 2 and 3 |
| M5 types, generics, applicability errors | XL | case 2 (calls with wrong arguments), completion quality |
| M6 inspections framework and catalog | XL, incremental | beyond the three cases |
| M7 parity gate and jdtls removal | L (gate + features) | removal decision |

Order: M0, M1, then M2 and M3 in parallel where staffed (M3 needs the resolver walk of M2 for reference
variants, but class-name and annotation positions need only M1), M4, M5, M6, M7. Cases 1 and 3 can show
user-visible improvement after the class-name subset of M3 (annotation and type positions with import on
insert, which depend on M1 only) if it is split out first; that slice is the recommended first shippable
increment after M1.

## 5. Risks and open questions

1. **Size of the type system (M5).** Inference per spec 03 (bound sets, nested generic calls, lambdas) is
   the biggest unknown and the main source of false positives. Mitigation: nothing is reported on an
   expression whose type is unknown or erroneous (JAVA-TYP-032); applicability errors are enabled per row
   by the javac differential; M2/M3 need only declared types so they are not blocked.
2. **False positives destroy trust faster than misses help.** Mitigation: the certain/uncertain rule
   (2.4), the javac differential as a release gate, the `java.native.analysis` kill switch kept, the coverage
   telemetry; error rows ship dark behind `java.engine.checks.<row>` until the corpus is clean.
3. **Annotation processors and generated code** (Lombok, MapStruct, Dagger, Spring configuration metadata,
   Android R). Without them, errors on generated members are false positives; with a text test they are
   special cases. Plan: `AUGMENT_PROVIDER` plus reading generated-source roots from the model (Gradle
   `kapt`/`annotationProcessor`, Maven `target/generated-sources`); Lombok-style member synthesis by
   *interpreting annotations declared on the processor's own annotation types* is not possible in general,
   so the interim rule is "unresolved or unknown-processor annotation on the class makes its member lookups
   uncertain (no error), but the class's own references are still checked". Open: does the owner accept that
   Lombok-heavy files lose member errors until a real augment provider exists (the alternative is a
   built-in Lombok provider, which the decision forbids as a special case but which could be a *plugin*).
4. **Classpath truth.** Wrong or missing jars are a project-model problem that looks like an engine problem
   (Maven offline failures, profiles ignored, `provided`/`system` merged, Gradle toolchains and source sets,
   Android, Bazel). The completeness flag and status surface mitigate; correctness needs M1's model work
   and is open-ended per build system. Open: do we run the build tool on every sync (current behavior, with
   the trust gap noted in `project-model.md`) or add a pure-Rust Maven resolver for the common cases?
5. **jdtls feature breadth (1.4).** Hover, navigation, usages, rename, refactorings, signature help and the
   debug plugin are not analysis-engine work and each is sizeable; M7 cannot complete before they have
   owners. The debug adapter is the sharpest dependency: java-debug is a jdtls plugin. Open: keep jdtls as an
   *optional debug-only* component after parity, or write a standalone java-debug launcher?
6. **Performance at scale.** 300k-name tables, per-keystroke whole-file checks and JDK stubs in memory
   can blow the budgets; the plan bounds work by member-level incrementality and summary values, but the
   first benchmark (M0) may force a different layout (FST of names, mmap). Budgets are initial numbers, the
   "N" of the index-time budget is calibrated at M0.
7. **Index storage growth and cold start.** JDK + libraries per project duplicate data; the shared
   by-hash library cache (2.3) fixes it but changes `STUB_VERSION` handling (migration plan needed). JDK
   resources that are compressed in jimage are skipped today (counted), a JDK built with compression would
   silently lose classes: needs a decompressor or a loud failure.
8. **Two matchers, two rankers.** `lsp/completion.rs` and `core/name_match.rs` already diverge; the plan
   unifies them in M1/M3, which changes ranking of all languages (Rust, Kotlin via LSP). Needs a snapshot of
   current rankings and an owner decision on whether non-Java ranking may change.
9. **Parser quality.** tree-sitter-java recovers imperfectly around half-typed annotations (`@Te` before a
   method), generics and lambdas; the position classifier therefore also reads tokens before the caret. If
   recovery proves too lossy for M3/M5 the fallback is a hand-written error-tolerant Java parser (large) or
   targeted grammar patches; decide after the M0/M3 masking benchmark.
10. **Spec gaps and conflicts.** Spec 06 leaves usage-statistics persistence open (Q-CMP-2), ties in import
    ordering are Archipel-specific, `--release` filtering (`ct.sym`) is LATER, module imports and multi-
    release selection by language level are partly untested. IntelliJ-equivalence claims need the
    differential harness against recorded jdtls/javac behavior since no IntelliJ binary is run here.
11. **Staffing and sequencing.** M2 and M3 touch `analyze.rs`, `import_fix.rs` and the daemon pass at once;
    without the split of 2.4 early in M2 two engineers will conflict. Doing the module split first (one PR,
    no behavior change, guarded by the existing tests) is the proposed first step of M2.
12. **Open question for the owner:** whether jdtls must keep working (for hover/navigation) while the
    default for diagnostics and completion is native, i.e. whether the intermediate `both` mode with
    jdtls fed from our model (2.10) is the accepted transition, or jdtls is switched off per feature as soon
    as that feature is native.

## 6. Implementation status: M0, M1 and the class-name slice of M3

What exists (details in [`java-intelligence.md`](java-intelligence.md); numbers in
[`../perf/java-engine.md`](../perf/java-engine.md), case results in
[`../parity/java-cases.md`](../parity/java-cases.md)):

- **M0.** The engine-layer case runner `crates/archipel-ide-java/tests/java_cases.rs` runs
  `docs/specs/java/cases/*.cases.md` (`cargo xtask java-cases`, `JAVA_CASES_ONLY`, `JAVA_CASES_VERBOSE`,
  `PASS_BASELINE`, report `docs/parity/java-cases.md`); `06-completion.cases.md` holds the owner's cases and
  their neighbours (every case restates an existing requirement ID). Hermetic fixture jars are built by
  `tools/java-fixtures/build.sh` from our own sources in `crates/archipel-ide-java/tests/fixtures/` (no network;
  JUnit-, Hamcrest- and annotations-like shapes, a JDK 8 style `rt.jar`); `tests/common::fixture_jar` caches
  them by source hash under the target directory. `benches/completion.rs` measures the budgets of 3.5.
  Not done: coverage telemetry and "Why no errors here?", the LSP trace of the completion request, the pinned
  real-jar fetch script, removal of the `~/.m2` lookup of the older tests (they still skip silently).
- **M1.** `ClasspathSnapshot` with `Completeness` (status bar notice and control socket), the ad-hoc module, the
  `Java:Names` index with kind, flags, deprecated and `@Target` masks, prefix and camel-hump queries, package
  listing and child packages, the inheritor closure, the JDK 8 `rt.jar`, the shared library cache. Not done:
  static-member and parameter-type indexes (JAVA-STUB-014/015), the single shared `NameMatcher` (the matcher of
  `names.rs` is a third one next to the two named in 2.3; unifying them changes the ranking of other languages and
  needs the owner decision of risk 8), the synchronous stub overlay, Gradle source sets and per-module JDK, the
  retry action for a failed Maven import, `java.lookup` on the control socket.
- **M3, class-name slice.** The position classifier, the class-name contributor with import on insertion
  (JAVA-CMP-009, 018..021, 025, 029 for annotation targets), the weigher basics, and the native
  `CompletionSource` wired into the completion session with de-duplication against the server's rows and the
  auto-popup after `@` and after `new `. Deviations from 2.7: the source is registered on a new extension point
  `COMPLETION_SOURCE` (the `CompletionContributor` contract needs a snapshot and a syntax tree the session does not
  have yet), and the setting is the switch `java.native.completion` instead of `java.completion.source`
  (`native | both | jdtls`).
