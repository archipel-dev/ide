# Project model and external system import

Status: first slice of Phase 2 of the platform refactor (`platform-refactor-plan.md`). The model
and the importers exist and are queryable; no subsystem reads them yet except the control socket.

Until now each feature found its own picture of the project: the index walked the project
root, the runner asked Gradle or Maven for a classpath, Android parsed Gradle files, toolchain
setup looked at marker files, jdtls discovered the build itself. Nothing could answer "which
jar provides `org.assertj.core.api.Assertions`" or "is this file test code". Two crates change
that.

| Crate | Role | IntelliJ counterpart |
|---|---|---|
| `archipel-ide-projectmodel` | pure data (serde, no UI, no processes) | `ModuleManager`, `ModuleRootManager`, `OrderEntry`, `Library`, `Sdk`, `ProjectFileIndex`, `GlobalSearchScope` |
| `archipel-ide-externalsystem` | build tool resolvers, data nodes, data services, import pipeline | External System: `ExternalSystemManager`, `ExternalSystemProjectResolver`, `DataNode`, `ProjectDataService` |

In the IDE, `IdeState::project_model` (`archipel-ide-core/src/project_model.rs`) holds the
imported model, runs the import on a worker thread and schedules auto-import.

Everything here follows IntelliJ's public design and names; the code is ours (clean room, see
`AGENTS.md`).

## The model

```
ProjectModel { name, root, project_sdk, sdks[], libraries[], modules[] }
Module       { name, module_type, content_entries[], order_entries[], compiler_output,
               language_level, external_project }
ContentEntry { root, source_folders[], excluded_folders[] }
SourceFolder { path, root_type: JpsModuleSourceRootType, package_prefix }
OrderEntry   = InheritedSdk | ModuleSdk | ModuleSource
             | Library { library, scope, exported } | Module { module, scope, exported }
Library      { name, classes[], sources[], javadoc[], coordinates }
Sdk          { name, sdk_type, home, version, classes[], sources[] }
```

`JpsModuleSourceRootType` has `Source`, `TestSource`, `Resource`, `TestResource`, `Generated`
and `GeneratedTest` (in IntelliJ the generated flag is a property of the root; one enum is
simpler and loses nothing). `DependencyScope` is `Compile`, `Test`, `Runtime`, `Provided`
with the JPS rules for which classpath (`ClasspathKind`: production/test, compile/runtime) a
scope is on. All paths are absolute and lexically normalized; the model never touches the disk
(except `Sdk::jdk_at`, which looks for `lib/modules` and `lib/src.zip`).

`ProjectModel` answers dependency questions (`OrderEnumerator` in IntelliJ):

- `modules_with_dependencies(module, kind)`, `libraries_for(module, kind)`,
  `classpath(module, kind)`. Compile classpaths follow only exported entries of dependencies,
  runtime classpaths follow everything. A dependency's tests are never on someone else's
  classpath.
- `module_sdk(module)` resolves `InheritedSdk` to the project SDK.

### ProjectFileIndex

`model.file_index()` builds a `ProjectFileIndex` (hash maps keyed by root; every lookup walks
the ancestors of the path, so it is O(depth)). The nearest registered root wins: a source folder
inside an excluded directory stays a source folder, a nested module owns its own files.

| Question | Method |
|---|---|
| owning module | `module_for_file`, `module_for_file_ext(.., honor_exclusion)` |
| roots | `source_root_for_file`, `content_root_for_file`, `package_name_by_directory`, `package_name_for_file` |
| kind of file | `is_in_content`, `is_in_source_content`, `is_in_test_source_content` (= `is_in_test_source`), `is_in_production_source`, `is_in_generated_sources`, `is_under_source_root_of_type`, `is_excluded` |
| libraries | `is_in_library`, `is_in_library_classes`, `is_in_library_sources`, `library_for_path` (accepts `lib.jar` and `lib.jar!/org/A.class`), `is_in_sdk`, `sdk_for_path` |
| everything | `is_in_project`, `is_in_source` (module or library sources), `order_entries_for_file` |

### Scopes

`GlobalSearchScope` is an enum with `contains(&index, path)`: `Everything`, `All` (content and
libraries), `Project`, `ProjectProduction`, `ProjectTests`, `Libraries`, `Modules {..}`,
`Directory`, and `Union`/`Intersection`/`Not`. The constructors mirror IntelliJ:
`module_scope`, `module_with_dependencies`, `module_with_dependencies_and_libraries`,
`module_production_with_dependencies`.

## External system import

```
ProjectResolver::resolve(request) -> DataNode tree          (per build system, runs the tool)
ProjectDataManager::import_data(trees) -> ModifiableModelsProvider -> ProjectModel
import_project(registry, manager, request) -> ImportResult { model, systems, diagnostics }
```

`DataNode` is `NodeData` (keyed by `DataKey`: `Project`, `Module`, `ContentRoot`, `Library`,
`LibraryDependency`, `ModuleDependency`, `Sdk`, `Diagnostic`) plus children. A resolver only
describes the build; the data services apply the nodes, ordered like IntelliJ's
`ProjectDataService.getOrder`: project, SDK, module, content root, library, module dependency,
library dependency, diagnostics. Other crates can register more services
(`ProjectDataManager::register`). The model is rebuilt from scratch on every import, so there is
no orphan removal step. Several build systems in one directory (for example Gradle and Cargo)
contribute to the same model; clashing module names get the system in brackets.

`ExternalSystemRegistry` (the `ExternalSystemManager` extension point) lists the resolvers:
`detect(root)` finds those that apply, `is_build_input(root, path)` tells auto-import which
file changes make the model stale. A failing resolver is a diagnostic; the import only fails if
every resolver fails. Tools run through `process::run_plan`, which applies the managed tool
environment (managed JDK, Gradle, Maven on `PATH`, `JAVA_HOME`), supports cancellation and a
time limit. Cargo, Gradle and Maven first try the local cache (`--offline` / `-o`) and repeat
online when that leaves holes, unless the user asked for offline only.

### Gradle

The init script of `archipel-ide-gradle` (the one the Gradle tool window already used) now also
exports, per project: every source set with `sources` (Java, Kotlin, Groovy dirs), `resources`,
annotation processor `generated` dir, `isTest` (the `test` set or one compiled for a `Test`
task), `classesDirs`, `resourcesDir`, and the resolved `compileClasspath` and
`runtimeClasspath` with group, name, version, classifier, file, and project dependencies as
`projectPath`; plus `buildDir`, `languageLevel` and unresolved dependencies. Gradle is the
wrapper or the managed/`PATH` Gradle (`GradleBuild::program`).

- One module per Gradle project, named like IntelliJ's qualified names (`root.app.core`).
  Production source sets decide `Compile`/`Runtime`/`Provided` (on both classpaths, runtime only,
  compile only); test source sets add `Test` scoped dependencies production does not have.
  Dependencies are flat (Gradle's resolved view), so project dependencies are direct module
  dependencies even when Gradle got them transitively.
- Libraries are named `Gradle: group:name:version[:classifier]`. Source and javadoc jars are
  the ones already next to the jar (Maven layout) or in a sibling hash directory of Gradle's
  `files-2.1` cache. Nothing is downloaded for them.
- Excluded: the build directory, `.gradle`, and (root) `.archipel`.

### Maven

POMs are parsed directly (`quick-xml`): modules (recursively), `build.sourceDirectory`,
`testSourceDirectory`, `directory`, `outputDirectory`, `testOutputDirectory`, `resources` and
`testResources`, `${...}` properties (`project.basedir`, `project.build.directory`, own and
inherited properties), build settings inherited from parent POMs inside the reactor, and
build-helper `add-source`/`add-test-source`. `target/generated-sources/annotations` and
`generated-test-sources/test-annotations` are generated roots. Aggregator (`pom`) modules have a
content root without sources.

Dependencies come from `mvn -q --fail-at-end dependency:build-classpath` run three times with
`-DincludeScope=compile|runtime|test`; the sets are classified by difference (on both compile
and runtime classpath = `Compile`, compile only = `Provided`, runtime only = `Runtime`, test
only = `Test`). In a multi-module build the command adds `compile` with the compiler and
resources skipped so sibling modules resolve to their `target/classes` (otherwise Maven cannot
resolve them before they are built); those entries become module dependencies. Maven is `mvnw`
or the managed/`PATH` `mvn`. Coordinates come from the local repository layout
(`~/.m2/repository`, `localRepository` of `settings.xml`), source and javadoc jars from the same
directory. When Maven cannot run, the model still has all roots and a warning says libraries
are missing.

### Cargo

`cargo metadata --format-version 1`: workspace members are modules (`ModuleType::Rust`, the
edition as language level). `lib` and `bin` targets give the source folder (the directory of
`src_path`; `src/bin` is inside `src`), `test`, `bench` and `example` targets give test folders
(`tests`, `benches`, `examples`), `build.rs` gives none; `target` is excluded. Packages that are
not members become libraries `Cargo: name:version` whose **sources** root is the package
directory (registry crates are source, there are no class files). The transitive closure of
normal dependencies is attached with scope `Compile`, build dependencies as `Provided`,
dev-dependencies as `Test`; members become module dependencies. The SDK is the `rustc` sysroot
with `rust-src` attached when installed.

### Bazel

Best effort. `bazel query 'kind("(java|kt_jvm)_(library|binary|test|import) rule", //...)'
--output=xml` gives rules; each JVM rule with `srcs` becomes a module named after its label
(`pkg:name`, root package just `name`), content root = package directory. Bazel declares no
source roots, so the root of a Java file is its path minus its `package` declaration (a file
whose directory does not match its package keeps its own directory); `resources` follow
`resource_strip_prefix`; `*_test` rules give test folders; `bazel-*` links are excluded.
`deps` and `exports` on other imported rules are module dependencies (`runtime_deps` runtime),
`@repo//:x` labels are libraries `Bazel: @repo//:x` with the jar looked up in the
`rules_jvm_external` repository of `bazel info output_base` when it was already fetched.

Known gaps: no aspects, so no `javacopts`, Java version, annotation processor output or
generated sources; `genrule` outputs, `cc_*` and other languages are ignored; Kotlin and Scala
specifics are not read; rules without sources (`java_binary` with only `runtime_deps`) are not
modules; library jars are only found when the output base is populated; `query` needs the Bazel
server and, for Bzlmod, network on the first run. This was verified against canned query output
only (no Bazel in the build container).

## Integration in the IDE

- `IdeState::project_model: ProjectModelState`. On project open the cached model
  (`.archipel/externalsystem/project-model.json`) is shown at once and a background import starts
  (after missing JDK/Gradle/Maven were installed by toolchain setup). Build file changes
  (`is_build_input`) schedule an import after 1.5 s of quiet; changes the import caused itself
  (`Cargo.lock`) are ignored for a few seconds. `ARCHIPEL_EXTERNAL_IMPORT=off` disables the
  automatic start.
- Commands: `externalSystem.refresh` (Sync Project with Build Files; queues when an import
  runs), `externalSystem.cancel`, `externalSystem.model` (the snapshot below).
- Status bar: spinner and the importer's progress line ("Gradle: resolving dependencies"); click
  cancels. Finished and failed imports show in the status line.
- Control socket `state` has `projectModel`: `state` (`idle`, `importing`, `ready`, `failed`),
  `systems`, `modules` (name, type, system, SDK, `contentRoots`, `sourceFolders` with their
  type, `excluded`, module and library dependencies with scopes), `libraries` (count), `sdks`,
  `diagnostics`, `imports` (finished imports, for scenarios that wait for one). The harness
  treats a running import as busy.

## Call sites to move onto the model

These still do their own discovery and should read the model next (each is deleted or shrinks):

1. **Index roots** (`archipel-ide-core/src/indexing_commands.rs`, `archipel-ide-index`): index
   `ProjectFileIndex` content plus library class roots and SDK roots instead of the project
   directory walk.
2. **Runner classpath** (`archipel-ide-runner`, `archipel-ide-gradle` run plans,
   `gutter_commands.rs`, `java_debug.rs`): `ProjectModel::classpath(module, kind)` for run and
   debug of a single class.
3. **Android** (`archipel-ide-android/src/project.rs`): module list and plugins from the model
   (Android source sets are not imported yet, see below).
4. **Toolchain requirements** (`archipel-ide-toolchains/src/providers/mod.rs`,
   `project_requirements`): derive the needed JDK version from `Module::language_level` and the
   build tool from `external_project.system`; make toolchain setup a step of import.
5. **jdtls and other servers** (`archipel-ide-lsp`, `language` module): send the imported
   classpath, source roots and SDK in the initialization options instead of letting the server
   rediscover them.
6. **Gradle tool window and sync** (`gradle_commands.rs`, `gradle_window.rs`): `gradle.sync`
   and the cached `GradleModel` can be fed from the same import; the "Load Gradle Changes"
   banner then becomes auto-import.
7. **Test detection** (`test_nav.rs`, `archipel-ide-runnables`, Problems "test files"):
   `is_in_test_source`.
8. **Search scopes** (`search_scopes.rs`, Find in Files): `GlobalSearchScope` for Project
   Files, Module, Project Test Files, Scratches and Libraries.

## Known gaps

- Gradle: one module per project, not per source set (IntelliJ can create `app.main` and
  `app.test`); the JDK is the one that ran Gradle, not the Java toolchain of a module; Android
  source sets (`android {}`) are not exported; included builds (composite) appear as external
  libraries or not at all; Kotlin multiplatform source sets are only seen through `allSource`;
  version catalogs and convention plugins only matter through what Gradle resolves.
- Maven: profiles are ignored (so are `dependencyManagement` and plugin configuration other than
  build-helper); only `${...}` of the forms above is interpolated; resolution needs the
  `maven-dependency-plugin`, which is downloaded on first use; running `compile` in multi-module
  builds creates `target/` directories; no `system` scope paths; `provided` and `system` scope
  are merged.
- Cargo: no `cfg` target filtering (an edge for any target counts), no feature resolution
  beyond what `cargo metadata` reports, procedural macro output and build script output
  (`OUT_DIR`) are not roots, no `rust-project.json`.
- Libraries have class roots only for Java libraries; the model has no module-level libraries and
  no per-module library overrides; no library "attached sources later" editing; no SDK
  configuration UI. `Sdk` JDK classes root is the `lib/modules` jimage, not `jrt:` entries.
- No VFS: the file index is rebuilt from a model snapshot, it does not see files created later
  except through its roots. Paths are not canonicalized by the model; importers canonicalize
  what the tools report.
- Auto-import runs the build tool, which executes project build scripts. There is no trust
  prompt yet (the IDE has no project trust concept); a trust step belongs before the first
  import.
- The sync is not shown in a "Sync" tool window with per-step output yet; only the status bar
  progress and diagnostics in the model exist.

## Verification

See the test files: `archipel-ide-projectmodel/src/tests.rs` (file index, scopes, classpaths),
`archipel-ide-externalsystem/tests/` (pipeline, Gradle with a real Gradle and a file based
repository, Maven fixtures plus an ignored real-Maven test, Cargo canned and real
`cargo metadata`, Bazel canned query output, ignored Spring Boot imports with the IDE's own
generator) and `archipel-ide-core/src/project_model.rs` (background import, cache, Sync command,
debounced auto-import). `cargo run -p archipel-ide-externalsystem --example import -- <dir>
[class...]` imports any project and prints the model (and the library that provides each class).
