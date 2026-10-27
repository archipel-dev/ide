# Platform spec 07: project structure as analysis sees it, file index, search scopes

Status: specification (clean-room). ID prefix `PLAT-SCOPE`. Priorities: `MUST` / `SHOULD` / `LATER` as in `README.md`.

Complements `docs/architecture/project-model.md` (branch `origin/feat/projectmodel`), which defines Archipel's *data* (modules, content roots, order entries,
libraries, SDKs, `ProjectFileIndex`, `GlobalSearchScope` enum). This document specifies the **semantics analysis requires from those**: which file belongs
to which module/library, what each scope contains **exactly**, how test and production code are separated, how library classes relate to library sources,
and how SDK roots behave. Section 9 lists where the Archipel model must change to match.

Sources studied: `ic: platform/projectModel-api/src/com/intellij/openapi/roots/` (`ProjectFileIndex.java`, `FileIndex.java`, `OrderEnumerator.java`,
`DependencyScope.java`, `OrderEnumerationHandler.java`), `ic: platform/projectModel-impl/src/com/intellij/openapi/roots/impl/` (`OrderEnumeratorBase.java`,
`OrderRootsEnumeratorImpl.java`, `ProjectFileIndexImpl.java`, `ProjectFileIndexFacade.java`),
`ic: platform/projectModel-impl/src/com/intellij/workspaceModel/core/fileIndex/impl/WorkspaceFileIndexDataImpl.kt`,
`ic: platform/core-api/src/com/intellij/psi/search/` (`GlobalSearchScope.java`, `ProjectScope.java`, `EverythingGlobalScope.java`),
`ic: platform/core-impl/src/com/intellij/psi/search/ProjectScopeImpl.java`, `ic: platform/indexing-api/src/com/intellij/psi/search/ProjectAndLibrariesScope.java`,
`ic: platform/lang-impl/src/com/intellij/psi/search/ProjectScopeBuilderImpl.java`,
`ic: platform/analysis-impl/src/com/intellij/openapi/module/impl/scopes/` (`ModuleScopeProviderImpl.java`, `ModuleWithDependenciesScope.kt`,
`ModuleScopeUtil.kt`, `RootContainerScope.kt`, `ModuleWithDependenciesScopeCache.kt`), `ic: platform/analysis-impl/src/com/intellij/openapi/roots/impl/LibraryScopeCache.java`,
`ic: platform/analysis-impl/src/com/intellij/psi/impl/file/impl/ResolveScopeManagerImpl.java`,
`ic: java/java-impl/src/com/intellij/psi/impl/JavaPsiImplementationHelperImpl.java`, `ic: java/java-psi-impl/src/com/intellij/psi/impl/PsiImplUtil.java`.

## 1. Model

```
Project
 ├─ SDK entries (project SDK; modules may inherit it or name their own)
 ├─ Library entries (name; CLASSES roots; SOURCES roots; DOCUMENTATION roots; annotations roots)
 └─ Modules
     ├─ Content roots  (directories the module owns)
     │    ├─ Source folders: production | test | resource | test-resource  (+ generated flag, + package prefix)
     │    └─ Excluded folders
     └─ Order entries (ORDERED list)
          ├─ module source entry   (the module's own roots; position in the list matters for classpath order)
          ├─ SDK entry             (inherited project SDK or module SDK)
          ├─ library entry         (scope: COMPILE | TEST | RUNTIME | PROVIDED; exported flag)
          └─ module entry          (another module; scope; exported flag)
```

Definitions used throughout:

* **Content**: files under a content root of a module, not excluded and not ignored. "In project" for a *file* normally means "in content".
* **Source content**: content that is under a source folder of a module.
* **Test source**: a file under a test source or test resource folder (test-ness is a property of the **folder**, not of the module).
* **Library classes** (a.k.a. binary roots) vs **library sources**: a library has CLASSES roots (jars/directories with `.class` files) and optionally SOURCES roots (source jars or directories). Navigation and resolve use CLASSES;
  sources are only attached for navigation, documentation and searches that ask for them.
* **SDK roots**: the SDK (JDK) behaves like a library attached to modules: classes roots (the runtime image or `rt.jar`-like archives), optional sources (`src.zip`).
* **Dependency scope** of a library or module entry: *Compile* (production compile, production runtime, test compile, test runtime), *Test* (test compile and test runtime only), *Runtime* (production runtime and test runtime),
  *Provided* (production compile, test compile and test runtime; not production runtime).
* **Exported** entry: re-exposed to modules that depend on this module (Gradle `api`, Maven transitive compile).

## 2. File index requirements (what belongs to what)

Lookup rule (normative): to classify a file or directory, walk from the file **up** through its ancestors; the **first** ancestor (including the file itself) that carries any registration (content root,
source folder, library root, SDK root, exclusion) decides. An ignored file or directory on the way stops the walk with the answer "ignored". Nested registrations override outer ones:
an excluded folder *inside* a source folder excludes; a source folder or content root *inside* an excluded folder includes again; a nested module's content root takes its files away from the outer module.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SCOPE-001 | MUST | Implement the lookup rule above for `module_for_file`, `content_root_for_file`, `source_root_for_file`, `class_root_for_file`, `is_excluded`, `is_ignored`, with O(depth) cost and no allocation on the hit path. | Property test on random trees: result equals a naive "deepest registration wins" oracle. |
| PLAT-SCOPE-002 | MUST | `is_in_content(file)`: under a content root, not excluded, not ignored. `is_in_source_content(file)`: under a source folder of a module (any type: production, test, resource, test resource), not excluded or ignored. `is_in_test_source_content(file)`: under a *test* source or test resource folder. `is_in_production_source(file)` = source content and not test. | Files in `src/main/resources` are source content, production; in `src/test/java` source content, test; in `docs/` content but not source content. |
| PLAT-SCOPE-003 | MUST | `is_under_source_root_of_type(file, types)` tells whether the file is under a source folder of one of given root types (production, test, resource, test resource, generated variants), so Java sources can be distinguished from resources. | `application.yml` under `resources` is not under a Java source root of type "production" but is under type "resource". |
| PLAT-SCOPE-004 | MUST | `is_excluded(file)` is true if the file or any ancestor up to its nearest root is excluded **or ignored**. `is_in_project_or_excluded(file)` is true for files under a root even if excluded. | A file in `build/` is excluded but "in project or excluded". |
| PLAT-SCOPE-005 | MUST | Library classes and sources: `is_in_library(file)` (classes **or** sources root of any library or SDK in any module's order entries), `is_in_library_classes(file)`, `is_in_library_source(file)`, `find_containing_libraries(file)`, `find_containing_sdks(file)`, `class_root_for_file(file)` (the classes root, for files that are compiled classes). A file may be in both classes and sources if the same directory is registered as both. | A class inside `lib/foo.jar!/a/B.class` is in library classes; its source jar entry is in library source. |
| PLAT-SCOPE-006 | MUST | `is_in_project(file)` = in content **or** in library/SDK roots (classes or sources) and not ignored. `is_in_source(file)` = source content or library sources. | A jar class is in project but not in content. |
| PLAT-SCOPE-007 | MUST | **Order entries of a file**: the order entries of all modules whose classes or sources roots contain the file, in order. Use only when entries are needed; use `find_containing_libraries` otherwise. | Libraries shared by two modules appear once per module entry. |
| PLAT-SCOPE-008 | MUST | `module_for_file(file, honor_exclusion)`: the module whose content owns the file; with `honor_exclusion = false`, excluded files still report their module. `modules_for_file` returns all modules whose content roots contain it (overlapping roots). | A file under two overlapping content roots returns both from `modules_for_file`. |
| PLAT-SCOPE-009 | MUST | `source_root_for_file(file)` returns the **module source root or library source root** containing it; `content_root_for_file(file[, honor_exclusion])` the containing content root. | n/a |
| PLAT-SCOPE-010 | MUST | `source_root_type_for_file(file)`: the folder type that contains the file, none if not in module sources. `is_in_generated_sources(file)`: under a folder flagged generated. | Annotation-processor output folder reports "generated". |
| PLAT-SCOPE-011 | MUST | **Directory index**: package name by directory (`package_name_by_directory`) is computed from the source root and its *package prefix*: relative path with `/` to `.`, prefixed by the root's prefix; none if not under a source root or library classes/source root, or if a segment is not a valid identifier. Reverse: directories by package name, optionally including library sources. | A source root with prefix `com.acme` and a file at `x/Y.java` is in package `com.acme.x`. |
| PLAT-SCOPE-012 | MUST | The file index answers a *test-source filter* extension first (so build-tool plugins can mark files as tests that the root types do not), falling back to `is_in_test_source_content`. All feature code uses the **filter**, not the raw root type, for "is this test code". | A Gradle `integrationTest` folder marked as tests by a plugin counts as test code. |
| PLAT-SCOPE-013 | MUST | Iterate content: all files and directories under content roots (skipping excluded and ignored), optionally under a given directory and with a custom directory filter; usable outside read sections but tolerant of project-model changes during iteration. | Iterate 100k files while adding a module: no crash; consistent subset. |
| PLAT-SCOPE-014 | MUST | Content under symbolic links: the target is part of content iff the link's location is; loops are cut. | n/a |
| PLAT-SCOPE-015 | SHOULD | **Unloaded modules** (disabled by the user) are not in any scope; scopes can report that unloaded modules *would have been* included so the UI can warn "results may be missing". | Search with an unloaded module shows the warning. |
| PLAT-SCOPE-016 | MUST | The file index is updated incrementally on project-model changes and VFS events; it never requires a full rebuild for a file create/delete. | Create a file under a source root: `is_in_source_content` true immediately after the event. |

## 3. Order enumeration (dependency traversal)

Operations the analysis needs: *which modules, libraries, SDKs and roots does module M see, for which purpose*.

Options of an enumerator (each narrows the result):

* **production only**: skip entries whose scope is not used by production compile or runtime; roots of the module itself and of dependent modules reduce to production source folders;
* **compile only** / **runtime only**: skip entries not on that classpath (by dependency scope table, section 1);
* **recursively**: follow module entries transitively; combined with **exported only**, direct dependencies are always included, but dependencies *of dependencies* only if exported;
* **without SDK**, **without libraries**, **without dependency modules**, **without own module sources**.

Per-entry rules:

* an SDK entry is included only at the first level (never from a dependency module);
* a library entry is included unless libraries are excluded; its roots are CLASSES for dependency purposes;
* when a **test-scope** entry is skipped in production, its roots disappear from the production result;
* by default, a module's **test** classpath includes the **tests of modules it depends on**; build-tool import handlers can turn this off (Gradle/Maven do), and they can also declare that transitive dependencies are already flattened so "recursively" has no effect;
* module sources contribute: production folders always; test folders unless production-only; outputs are separate and not part of scopes.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SCOPE-020 | MUST | Provide the enumerator options above with the semantics above, usable to list entries, libraries, modules and roots (classes or sources), preserving **order entry order** (the classpath order). | Golden tests for the 12 combinations on a 4-module sample. |
| PLAT-SCOPE-021 | MUST | The dependency-scope table of section 1 is implemented as four booleans per scope (production compile, production runtime, test compile, test runtime) and is used by every enumerator filter. | Table test. |
| PLAT-SCOPE-022 | MUST | Import handlers (build-system plugins) may override: per-entry include/exclude, whether dependent modules' tests join the test classpath, whether recursion applies, and may add extra roots for libraries/modules. Handlers are consulted in registration order; first non-default answer wins. | A Gradle handler flattens transitive dependencies; `recursively` becomes a no-op. |
| PLAT-SCOPE-023 | MUST | Entries that cannot be resolved (module entry naming a missing module, library without roots) are skipped, never an error in enumeration. | Delete a module referenced by another: enumeration works. |
| PLAT-SCOPE-024 | MUST | Enumeration is a cancellation point per module visited, and is cached per (module, options) with invalidation on project-root changes (PLAT-CACHE-046). | Enumerate twice, one computation. |
| PLAT-SCOPE-025 | SHOULD | The project-wide enumerator (all modules): only direct entries, dependency modules are not followed. | Used for "all libraries of the project". |

## 4. Search scopes

A **search scope** is a predicate on files with extra structure. It is used by: index queries (spec 03), resolve (`find_class`), find usages, Go to ..., inspections, refactorings.

### 4.1 Scope contract

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SCOPE-030 | MUST | A scope offers: `contains(file)`; `compare(f1, f2)` (positive if `f1` is **earlier on the classpath** than `f2`, negative if later, zero if equal or not comparable; a file not in the scope ranks below any file in it); `is_search_in_module_content(module[, test_sources])`; `is_search_in_libraries()`; `is_force_searching_in_library_sources()`; `project()` (may be none); a display name; and the set of unloaded modules that would belong. | Contract tests with a custom scope. |
| PLAT-SCOPE-031 | MUST | Scopes are **values**: `equals` and `hash` are structural so they can be cache keys (class lookup caches key by scope). Equal scopes behave identically. | Two `module_scope(m)` calls are equal. |
| PLAT-SCOPE-032 | MUST | Scope algebra: `union`, `intersect`, `not`, with these simplifications (observable through `is_search_in_libraries` and equality): the project scope united with a scope that does not search libraries is the project scope; intersected with a scope that does not search libraries is that scope; the *everything* scope absorbs unions and yields the other operand on intersection; an empty scope is neutral for union and absorbing for intersection. | Algebra property tests. |
| PLAT-SCOPE-033 | MUST | `contains` must be **cheap** (index queries call it per candidate file): O(depth) lookups only, no I/O, no allocation. A scope may expose an **enumeration of file ids** (fast set) for index pushdown when its roots are few. | 1M `contains` calls in under 200 ms (budget). |
| PLAT-SCOPE-034 | MUST | Scope membership for a file is determined by the file's **single nearest root**: the module *source* root or the library/SDK *classes* root (or, for library sources, the source root) it sits under; the scope contains the file iff that root is among the scope's roots. **Consequence**: files in a module's content that are not under a source root (docs, build scripts) are in the *module content* scope but not in the *module (source) scope*. | A `README.md` at module root is in `module_content_scope(m)` and not in `module_scope(m)`. |
| PLAT-SCOPE-035 | MUST | Rank order for `compare`: files are ordered by the position of their root in the enumeration (module sources first in the module's own order entries position, then entries in order); two files under the same root are equal (rank ties). | Duplicate class in two libraries: the one from the earlier order entry compares greater. |

### 4.2 Project-level scopes (exact)

| Scope | Contains | Search libraries? | ID |
|---|---|---|---|
| **All scope** (`all_scope(project)`) | every file in content **plus** every file in library classes and sources and SDK roots (i.e. `is_in_project`), for any module, regardless of module dependencies; for the default/template project: *everything* | yes | PLAT-SCOPE-040 |
| **Project scope** (`project_scope(project)`) | the **content scope** minus one edge case: a file that is in project content **and also** under a *classes* root of some library, but **not** under any module source root, is **not** in the project scope. Excludes library jars and SDK. Used by default for "Project Files" in searches. | no | PLAT-SCOPE-041 |
| **Content scope** (`content_scope(project)`) | files for which `is_in_content` holds (module content, including other content entities), not excluded, not ignored; a superset of the project scope by exactly the edge case above. Prefer it unless the edge case matters. | no | PLAT-SCOPE-042 |
| **Libraries scope** (`libraries_scope(project)`) | files in classes **or** sources of all libraries and SDKs in module dependencies (`is_in_library`) | yes (only) | PLAT-SCOPE-043 |
| **Library classes scope** | classes (binary) roots of libraries and SDKs only (`is_in_library_classes`) | yes | PLAT-SCOPE-044 |
| **Everything scope** (`everything_scope(project)`) | all-scope files **plus** temporary and synthetic files (scratches, consoles, in-memory files); for a file with an id: indexable file or in the workspace; hidden scratch types excluded; for files without id: included | yes | PLAT-SCOPE-045 |
| **Empty scope** | nothing | no | PLAT-SCOPE-046 |

All six are MUST except "Everything" and "Library classes" which are SHOULD (PLAT-SCOPE-044, 045).

### 4.3 Module-level scopes (exact)

All module scopes are built from one function of (module, **options**), cached per (module, options) and invalidated on project-root changes. Options (bit flags):

* `COMPILE_ONLY`: use compile-classpath rules (skip runtime-only entries; follow only *exported* entries of dependencies, direct entries always);
* `LIBRARIES`: include library and SDK roots (classes roots);
* `MODULES`: include dependency modules, recursively as per exported rule;
* `TESTS`: include **test** source folders (in addition to production). **`TESTS` means "tests included", not "tests only"**: production sources are always included.

Roots collected: for the module's own entry and dependency-module entries → **source folders**; for library and SDK entries → **classes roots**. A file is in the scope iff its single root (PLAT-SCOPE-034) is one of the collected roots.

| Scope constructor | Options | Meaning | ID |
|---|---|---|---|
| `module_scope(m)` | COMPILE_ONLY + TESTS | the module's own source folders, production **and** test; no libraries, no dependency modules | PLAT-SCOPE-050 |
| `module_scope(m, include_tests)` | COMPILE_ONLY (+ TESTS) | as above, tests optional | PLAT-SCOPE-051 |
| `module_with_libraries_scope(m)` | COMPILE_ONLY + TESTS + LIBRARIES | own sources (prod + test) + library and SDK classes roots visible to this module (compile classpath); no other modules | PLAT-SCOPE-052 |
| `module_with_dependencies_scope(m)` | COMPILE_ONLY + TESTS + MODULES | own + dependency-module sources (prod + test); no libraries | PLAT-SCOPE-053 |
| `module_with_dependencies_and_libraries_scope(m, include_tests)` | COMPILE_ONLY + MODULES + LIBRARIES (+ TESTS) | **the standard resolve scope**: own + dependency modules + libraries + SDK, compile classpath; with `include_tests = false` test source folders, **test-scope libraries and test-scope module entries are excluded** | PLAT-SCOPE-054 |
| `module_runtime_scope(m, include_tests)` | MODULES + LIBRARIES (+ TESTS) | as 054 but **runtime** classpath (no COMPILE_ONLY): includes runtime-scope entries, excludes provided-only | PLAT-SCOPE-055 |
| `module_production_source_scope(m)` | none | production source folders of this module only | PLAT-SCOPE-056 |
| `module_test_source_scope(m)` | TESTS | production **and** test source folders of this module (the name is historical); to get only tests, intersect with a "is test source" filter | PLAT-SCOPE-057 |
| `module_content_scope(m)` | n/a | all files in the module's content (not only source folders) | PLAT-SCOPE-058 |
| `module_content_with_dependencies_scope(m)` | n/a | content of the module and of the modules it depends on | PLAT-SCOPE-059 |
| `module_with_dependents_scope(m)` | n/a | the module plus the modules that use it: its **direct** dependents, plus dependents reached through **re-exporting** modules (a module that depends on a module that exports `m` is reached; a plain `implementation` hop is not). A file belongs iff its **owning module** (by content root, not by source root) is in that set; so it includes the dependents' production code, test code, resources and other content. No libraries. | PLAT-SCOPE-060 |
| `module_tests_with_dependents_scope(m)` | n/a | as 060 restricted to **test** files: a file qualifies iff its module is in the set **and** (the file is test source per the test-source filter, **or** its module depends on the chain through a *production-on-test* dependency) | PLAT-SCOPE-061 |

MUST: 050-054, 056, 058, 060, 061. SHOULD: 051-053, 055, 057, 059. (051-053 are trivial given 054's machinery; do them with it.)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SCOPE-062 | MUST | **Test vs production for the *resolve scope of a file***: production file: `module_with_dependencies_and_libraries_scope(m, false)`; test file (per the test-source filter): `(m, true)`. A production file never resolves test classes or test-scope libraries. | A production class importing a JUnit class is unresolved; a test class resolves it. |
| PLAT-SCOPE-063 | MUST | **Exported rule**: module A → B (direct), B → C (not exported). A's compile-classpath scopes contain B's sources and **not** C's. If B exports C, they contain C. A's *runtime* scope contains both. | Golden test with Gradle `api` vs `implementation`. |
| PLAT-SCOPE-064 | MUST | Libraries and SDK appear in a scope by their **classes** roots. Library **source** roots are not in module scopes; library source files are found only through "all/libraries scope" or by navigating from the compiled class (PLAT-PSI-092). Consequence: resolving through a module scope never yields both the compiled and the source version of a library class. | `find_class("org.x.Y", module_scope_with_libs)` returns one class (compiled). |
| PLAT-SCOPE-065 | MUST | **SDK handling**: each module has an SDK entry (inherited from project or its own); SDK classes roots are in `LIBRARIES`-enabled scopes, at most one SDK per scope; SDK is never added from a *dependency* module's entries. JDK classes (JRT image) resolve like library classes. | Modules with different SDKs resolve `java.util.List` against their own SDK. |
| PLAT-SCOPE-066 | MUST | **Generated sources** are ordinary source folders with a flag; they are in scopes; a "not generated" filter is available to features (for example inspections skip generated code). | Annotation-processor output resolves; inspections do not report in it. |
| PLAT-SCOPE-067 | MUST | **Library resolve scope** (for a file *inside a library*): resolve names against the union of the runtime classpath of all modules that use that library, **preferring the library the file itself belongs to** (its classes win `compare`). For SDK files: the SDK's own classes and sources. For a file not under any module: the all scope plus the file itself. | A class in `a.jar` referencing `b.Foo` resolves in the project's jar `b.jar`, not in another module's unrelated version. |
| PLAT-SCOPE-068 | MUST | **Library use scope** (where usages of a library element may occur): all modules depending on the library (with their dependents) plus library files; if the library is in all modules, simply the all scope. | Usages of a library class are searched in dependent modules only. |
| PLAT-SCOPE-069 | MUST | Scope objects are cached per (module, options) in a size-bounded cache invalidated on project-root changes, so repeated calls return the same value (cheap equality). | Weigher test: cache cap honored. |
| PLAT-SCOPE-070 | MUST | **File scope** and **files scope** (one file; a set of files) with optional hint "contains library files / does not" for speed; **restrict by file types**; **not scope**. | Scope algebra tests. |
| PLAT-SCOPE-071 | SHOULD | **Local search scopes** (set of PSI elements or a file range) are distinct from global ones (they have no index acceleration); `intersect` of global with local is local. | Usage search in a method body uses a local scope. |

## 5. Library classes versus sources: required behavior for JVM

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SCOPE-080 | MUST | **Resolve uses class files.** A reference to `org.springframework.stereotype.Service` resolves to the *compiled* class from the classes root in the resolve scope. Declaration navigation then maps to sources if attached (PLAT-PSI-091/092). | Without attached sources, Go to Declaration shows the decompiled mirror; with sources, `Service.java`. |
| PLAT-SCOPE-081 | MUST | **Index both classes and sources** of libraries (both are in indexable roots) so search-by-name over the *all scope* can find source files; but ensure **no duplicates** appear to the user in "Go to Class": when a compiled class and its source twin both match, show the source one (or the compiled one) once. | `Go to Class "ArrayList"` lists one `ArrayList` for the JDK. |
| PLAT-SCOPE-082 | MUST | Attaching or detaching a library sources root changes `navigation_element` for its classes and invalidates the cached source-file lookup (PLAT-PSI-094); it does **not** change resolve scopes. | Attach sources: navigation updates; resolve results unchanged. |
| PLAT-SCOPE-083 | MUST | Library **version conflicts** on the same classpath: two jars with `a.B` in one module's scope: the earlier order entry wins `find_class`; both are returned by `find_classes`; features that need *all* (override search, inheritors) see both. | See PLAT-IDX-090/091. |
| PLAT-SCOPE-084 | SHOULD | **Multi-release** class roots: the effective class for a language level comes from the versioned directory; class lookup validates directory chains skipping version dirs (PLAT-IDX-094). | A multi-release jar resolves the version matching the module's language level. |
| PLAT-SCOPE-085 | SHOULD | **Annotation roots** (external nullability annotations) and **documentation roots** are separate library root types and are not in scopes. | n/a |

## 6. Project model changes and invalidation

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SCOPE-090 | MUST | Any change to modules, content/source/excluded folders, libraries, SDKs, order entries or scopes of dependencies: (1) bumps the *project root modification tracker*; (2) invalidates the scope caches; (3) invalidates resolve/use scope caches; (4) emits the PSI "roots changed" event; (5) schedules a re-scan for indexing (PLAT-IDX-038). | Change a dependency scope from Compile to Test: production resolve of that library fails after the change, without restart. |
| PLAT-SCOPE-091 | MUST | The model is changed atomically (all edits of one import apply in one write section) so readers never observe a half-imported project. | A background reader during import sees either the old or the new model. |
| PLAT-SCOPE-092 | MUST | The import result (project model) has a stable identity per module (name) so that caches keyed by module survive a re-import that did not change the module's roots. | Re-import without changes keeps scope caches (no invalidation storm). |
| PLAT-SCOPE-093 | SHOULD | Fine-grained notification: which modules/libraries changed, so scope caches of unrelated modules survive. | Changing one module's source folder invalidates only scopes depending on it. |

## 7. How JVM languages, Spring and frameworks use scopes (platform contract)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SCOPE-100 | MUST | Every language service that resolves cross-file names obtains its scope from the **file's resolve scope** (PLAT-PSI-131), never from the project scope. | A Kotlin file in `module-b` does not see `module-c` classes absent from its dependencies. |
| PLAT-SCOPE-101 | MUST | Resolve-scope **contributions** may *replace* (first non-null wins) or *enlarge* (union) the default per file; they are re-evaluated when contributions change and are cached with the file (PLAT-PSI-133). | A plugin enlarges the scope of Groovy build scripts with the build tool's API jars. |
| PLAT-SCOPE-102 | MUST | Frameworks (Spring) that search **bean definitions** across the project use *module content with dependents* / *project content* scopes, not the all scope, unless the user asked for libraries; and bean searches in library jars use the library classes scope restricted to the library entries of the module. | Spring bean lookup in module A finds beans in A's dependencies' sources and in dependency jars' stubs, not in unrelated modules. |
| PLAT-SCOPE-103 | MUST | Test-scope handling for Spring tests: `@SpringBootTest` classes in test sources see production and test beans; production code does not see test beans. | Test-only `@TestConfiguration` bean is not injectable in production code. |
| PLAT-SCOPE-104 | SHOULD | **Package-prefix source roots** and **additional library roots** (build-tool-generated) are modeled and participate in scopes like ordinary ones. | A Kotlin source root with package prefix resolves `com.acme.Foo`. |

## 8. Required test fixtures (for conformance)

1. A **4-module** project: `app → core (implementation)`, `core → util (api)`, `app → util (via core, exported)`, `test-support → core`. Libraries: one compile-scope, one test-scope, one runtime-scope, one provided-scope. Verify every constructor of 4.3 against hand-written expected file sets.
2. A **Gradle-like** project with two source sets in one module (`main`, `test`) where test adds a Test-scoped library: production scope excludes it, test scope includes it.
3. A **library duplicate** project: two jars containing `a.B`, in two orders.
4. A **sources attached** project: class jar + source jar; navigation vs resolve.
5. A **nested exclusion** tree: excluded in source, source in excluded, nested module.
6. **Symlink** loop.

## 9. Delta against Archipel's current model (`archipel-ide-projectmodel`)

Reading `docs/architecture/project-model.md`, the following must change or be confirmed so the semantics above hold. (These are *spec findings*, not implementation instructions.)

| # | Archipel today | Needed | Requirement |
|---|---|---|---|
| 1 | `GlobalSearchScope::{Project, All, ...}` with constructors mirroring IntelliJ; semantics of "Project" not specified | `Project` = content minus pure-library files (4.2) and must differ from `All` exactly by libraries/SDK | PLAT-SCOPE-040/041 |
| 2 | One module per Gradle project (not per source set) | Acceptable **only if** test/production separation is expressed by source folder test flags and library `Test` scope **and** the scopes in 4.3 honor `TESTS` for both; otherwise the first-release resolve of production code would see test-only libraries | PLAT-SCOPE-054, 062 |
| 3 | `module_with_dependencies_and_libraries` exists; test flag semantics not stated | `include_tests = false` excludes test folders *and* Test-scope libraries and modules | PLAT-SCOPE-054 |
| 4 | Library entries with `exported` flag; compile classpaths follow only exported entries | Same as the exported rule (direct always, transitive only if exported) | PLAT-SCOPE-063 |
| 5 | A dependency's tests are never on someone else's classpath | Matches Gradle/Maven behavior; but the **platform default** is the opposite; keep the Archipel rule as the build-tool handler (PLAT-SCOPE-022) so a future non-build-system project (plain directory) can choose | PLAT-SCOPE-022 |
| 6 | "Libraries have class roots only for Java libraries" | For JVM, class roots required; for Rust/Cargo libraries sources roots are the content: resolve must use *source* roots for those languages | PLAT-SCOPE-064 (JVM) and language-specific note |
| 7 | `Sdk` JDK classes root is the `lib/modules` jimage | The JRT image must be readable as a file system tree (PLAT-VFS-025) so `java.util.List` has a file | PLAT-VFS-025 |
| 8 | File index rebuilt from a model snapshot; does not see later-created files except through roots | Acceptable (roots define membership), but nearest-root lookup must be O(depth) and updated on VFS events for ignored/excluded changes | PLAT-SCOPE-001, 016 |
| 9 | `GlobalSearchScope` has `contains(index, path)` (path-based) | Scopes must work on file **ids** and support `compare` (classpath rank) and a library flag; add rank | PLAT-SCOPE-030, 035 |
| 10 | Provided scope merged with `system` (Maven) | Provided = compile classpath only; OK, but document the merge | PLAT-SCOPE-021 |

## 10. Open questions

1. IntelliJ's content scope vs project scope differ only in an edge case (PLAT-SCOPE-041/042). Does Archipel need both, or one with the stricter rule?
2. Source *sets* as modules (IntelliJ Gradle import creates `app.main`, `app.test`): the studied scope code treats test-ness per source folder; the alternative of two modules would change `module_scope` results. Decide once, document in the project-model doc.
3. Shared sources across several modules or targets (Kotlin Multiplatform "shared source set"): the studied tree has an experimental code-insight-context layer for files that belong to several modules at once. Not specified here (LATER).
