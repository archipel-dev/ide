# Platform refactor plan: an IntelliJ-shaped core

Status: proposal. Owner: IDE core. Related: `docs/analysis/code-intelligence-spec.md` (the
analysis pipeline, WP1–WP9), `docs/analysis/editions-roadmap.md` (language packs).

## Why

Testing v0.1.0 on a real Spring Boot project showed that the IDE does not yet behave like
IntelliJ where it matters most. For example, an unresolved `assertThat` was not marked in the
editor and offered no import, but the test run failed on it. The individual features exist.
What is missing is the platform underneath them: the structure that makes every feature see the
same project, the same files and the same answers.

What the code looks like today (main at `7b25af9`):

| Finding | Evidence | IntelliJ counterpart |
|---|---|---|
| One god object holds all IDE state | `IdeState` has ~95 fields; 73 files in `archipel-ide-core` add `impl IdeState` blocks; the crate is ~47k lines | `Application` and `Project` with lazily created services, each owning its own state |
| Features call each other directly | Toolchains, LSP, runner, Git and the daemon are reached through `self.<field>` from any command | `MessageBus` topics and `ExtensionPointName`s; features subscribe, they do not reach in |
| The UI reaches into internals | 46 places in `archipel-ide-ui` read `state.lsp`, `state.toolchains`, `state.git` or `state.runner` directly; the UI crate is ~42k lines | UI reads models through services and `DataContext`; actions go through `AnAction` |
| No project model | Gradle and Maven knowledge lives in `archipel-ide-gradle`/`runner`; no module, SDK or library entries that analysis can query | `ModuleManager`, `ModuleRootManager`, `OrderEntry`, `Sdk`, `Library`, `ProjectFileIndex` |
| No build-system import layer | Each build tool is handled ad hoc (Gradle model, Bazel targets, Maven detection) | External System (`ExternalSystemManager`, project resolvers, data nodes, auto-import) |
| Two analysis paths merged by hand | LSP diagnostics and native markup are merged in `problems.rs`; `Daemon::diagnostics_published` exists but nothing calls it | One `DaemonCodeAnalyzer`; external tools (LSP) plug in as an `ExternalAnnotator`-style pass |
| Startup and indexing are implicit | Toolchain setup, LSP start, Gradle model load and indexing start in `open_project` in a fixed order | `StartupActivity`, `DumbService` with queued `DumbModeTask`s, `ProjectActivity` |

None of these is a bug on its own, but together they explain the class of bugs we see.
A feature works in its own test, then fails in a real project because it sees a different
picture of the project than its neighbours.

## Ground rules

- **Behavior and architecture come from IntelliJ; the code is ours.** We study IntelliJ
  Community's public source (Apache-2.0), its documentation (IntelliJ Platform SDK) and its
  observable behavior. We write our own Rust code with the IntelliJ names (as in the kit). We
  copy no JetBrains code or artwork and decompile nothing: Community is published as source,
  and Ultimate-only features are specified from observed behavior only. This keeps the
  clean-room rule in `AGENTS.md`.
- **Behavior is specified by recorded scenarios, not by memory.** Each subsystem gets a
  scenario corpus: a small project, a sequence of user actions, and the expected result
  observed in IntelliJ (markup, popup items, file contents after a fix). The corpus is the
  acceptance test. Example: `java/unresolved-static-import`: in a Gradle project with AssertJ,
  typing `assertThat(x)` marks `assertThat` red as *Cannot resolve method 'assertThat'*.
  Alt+Enter then lists *Import static method 'org.assertj.core.api.Assertions.assertThat'*
  first, and choosing it adds the import.
- **Strangler, not rewrite.** Every phase ships in a release, keeps all tests and ux-smoke
  green, and moves one area behind the new structure. No long-lived branch.
- **Each phase deletes code.** A phase is done when the old path is gone, not when the new one
  exists.

## Target structure

```
Application (process)                 Project (one per open project)
 ├─ services (settings, toolchains,    ├─ services (lazy, typed, owned state)
 │   keymap, LSP registry, updates)    ├─ MessageBus topics (VFS, roots, dumb mode, problems, run)
 ├─ MessageBus (app topics)            ├─ ProjectModel: modules, SDKs, libraries, source roots
 └─ ExtensionPoints (languages,        ├─ ExternalSystem: Gradle, Maven, Bazel, Cargo importers
     file types, inspections, run      ├─ ProjectFileIndex / scopes
     config types, generators, ...)    ├─ PSI + stubs + indexes, DumbService
                                       ├─ DaemonCodeAnalyzer (native passes + external passes)
                                       └─ UI binds through DataContext and AnAction
```

New core crates (names follow IntelliJ; final split decided in phase 1):

| Crate | Owns | Replaces |
|---|---|---|
| `archipel-ide-platform` | `Application`, `Project`, service container, `MessageBus`, extension points, `Disposable` | the god object's wiring in `archipel-ide-core/src/lib.rs` |
| `archipel-ide-projectmodel` | modules, content/source roots, SDKs, libraries, order entries, `ProjectFileIndex`, search scopes | scattered root/classpath knowledge in `project`, `gradle`, `runner`, `android` |
| `archipel-ide-externalsystem` | import pipeline, data nodes, auto-import, sync view; Gradle, Maven, Bazel, Cargo resolvers | ad-hoc model loading in `open_project` and the build crates |
| `archipel-ide-startup` | `StartupActivity`, `DumbService`, background task queue with progress | the fixed sequence in `open_project` |

`archipel-ide-core` shrinks to command registrations and becomes thin; `archipel-ide-ui`
reads through `DataContext`/services only.

## Phases

Each phase lists what it delivers, what it removes, and how we know it is done.

### Phase 0: scenario corpus and harness (first, small)
- `xtask scenario` runs a recorded scenario headless (project fixture, keystrokes, expected
  markup/popup/file result) against the real `IdeApp`, reusing the control socket and
  kittest input.
- Seed the corpus with 30 scenarios from real use, starting with the reported ones:
  unresolved symbol + import fix (Java, Kotlin), missing dependency, wrong type,
  unused import, Spring Boot run, Bazel/Maven/Gradle sync, search, rename, go to declaration.
- The expected results are recorded in IntelliJ Community by hand. Only text goes into
  `docs/parity/scenarios/`: markup, popup items and resulting files. No screenshots of
  JetBrains UI are committed.
- Done when: the corpus runs in CI and reports pass/fail per scenario (failures allowed, the
  number is tracked in the parity dashboard).

### Phase 1: platform kernel
- `Application`/`Project`, typed service container with lazy init and `Disposable`
  ownership, `MessageBus` with typed topics, extension point registry.
- Move four services out of `IdeState` as the template: toolchains, LSP registry, settings,
  notifications. The UI reads them via `Project::service::<T>()`.
- Done when: those four fields are gone from `IdeState`, and a lint test fails on new
  `IdeState` fields.

### Phase 2: project model and external system import
- Project model with modules, source/test/resource roots, SDK, library order entries
  (with sources and javadoc jars), `ProjectFileIndex`, scopes (production, test, libraries).
- External system pipeline: Gradle (Tooling API model via the existing init-script
  approach), Maven (effective POM and dependency resolution through `mvn` or the wrapper),
  Bazel (query + aspects), Cargo (`cargo metadata`). Sync view, auto-import on build file
  change, and toolchain setup as a step of import instead of a side effect of open.
- jdtls and other servers receive the imported classpath instead of re-discovering it.
- Done when: the scenarios for "sync a Gradle/Maven/Bazel Spring project" and "library
  classes are found by Go to Class" pass. Root and classpath code in other crates is
  deleted.

### Phase 3: one daemon, external passes
- The daemon becomes the only producer of editor markup. LSP diagnostics come in as an
  external pass (IntelliJ's `ExternalAnnotator` contract: collect, apply, apply to file),
  deduplicated with native results per the spec's coexistence rules.
- Alt+Enter assembles native intentions and LSP code actions in one list with IntelliJ's
  ordering (errors' fixes first, then intentions).
- Delete the hand merge in `problems.rs`.
- Done when: `java/unresolved-static-import` and the other Phase-0 markup scenarios pass
  with jdtls, and the Problems view, error stripe, F2 and squiggles read one source.

### Phase 4: Java/Kotlin resolve on the project model
- Continue WP4–WP6 of the code-intelligence spec (language queries, stubs, resolve,
  types) for Java first, now on top of the project model: library jars are indexed as
  class-file stubs.
- Native *Cannot resolve symbol*, *Import class / static member* fixes, auto-import on
  completion, unused-import inspection, optimize imports.
- Done when: the import scenarios pass without jdtls, and the differential test against
  jdtls (spec 1.10) shows no regressions.

### Phase 5: UI on DataContext and actions
- All remaining UI reads go through `DataContext`, and every command is an `AnAction` with
  `update()` presentation. The UI crate keeps rendering only.
- Done when: no `archipel-ide-ui` file reads `state.<field>` of another subsystem (lint test),
  and the god object is gone.

### Afterwards
Language packs per the editions roadmap plug in through the extension points from Phase 1
and the project model from Phase 2, instead of each adding code to the core.

## Order and size

| Phase | Depends on | Rough size | User-visible result |
|---|---|---|---|
| 0 Scenario corpus | none | small | a measurable "how close to IntelliJ" number |
| 1 Platform kernel | 0 | medium | none (enables the rest) |
| 2 Project model + import | 1 | large | reliable sync, libraries found, jdtls sees the classpath |
| 3 One daemon | 1, 2 | medium | errors and import fixes show up consistently |
| 4 Java/Kotlin resolve | 2, 3 | large | IntelliJ-like Java without depending on jdtls |
| 5 UI on DataContext | 1 | medium, spread out | fewer UI state bugs |

Phases 3 and 5 can run in parallel with 4 once 1 and 2 have landed. A short-term fix for the
reported import problem (Phase 3's external pass for jdtls) can ship before Phase 2 if needed.

## Risks

- **Scope creep.** IntelliJ is decades of work. The scenario corpus decides what "done" means
  for each release; anything without a scenario is not in scope yet.
- **Refactor stalls feature work.** Every phase ships, and the old path is deleted in the same
  phase, so we never maintain two systems for long.
- **Accidental copying.** Reviewers check new subsystems against the clean-room rule. Notes
  on IntelliJ design go in `docs/` in our own words, with links to the public source.
