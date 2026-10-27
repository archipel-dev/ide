# Platform spec 05: caching, cached values, modification trackers, out-of-code-block changes

Status: specification (clean-room). ID prefix `PLAT-CACHE`. Priorities: `MUST` / `SHOULD` / `LATER` as in `README.md`.

Supersedes and deepens section 1.4 of `docs/analysis/code-intelligence-spec.md` and decision D5 there; see section 8.

Sources studied: `ic: platform/core-api/src/com/intellij/psi/util/` (`CachedValue.java`, `CachedValueProvider.java`,
`CachedValuesManager.java`, `PsiModificationTracker.java`), `ic: platform/core-impl/src/com/intellij/util/CachedValueBase.java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/` (`PsiCachedValue.java`, `PsiCachedValueImpl.kt`, `PsiModificationTrackerImpl.java`),
`ic: platform/util/src/com/intellij/openapi/util/` (`ModificationTracker.java`, `RecursionManager.java`, `RecursionGuard.java`),
`ic: platform/core-api/src/com/intellij/openapi/roots/ProjectRootModificationTracker.java`,
`ic: platform/core-api/src/com/intellij/openapi/roots/FileIndexFacade.java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/source/resolve/ResolveCache.java`,
`ic: java/java-psi-impl/src/com/intellij/psi/impl/source/ClassInnerStuffCache.java`,
`ic: java/java-psi-impl/src/com/intellij/psi/impl/JavaPsiFacadeImpl.java`.

## 1. Model

* A **modification tracker** is a monotone counter ("stamp") describing *some state's* change history. Equal stamp means "unchanged".
* A **cached value** holds a computed result and the **dependency stamps** observed when it was computed. It is *valid* iff every
  dependency's current stamp equals the remembered one. Validation is **lazy**: nothing is invalidated eagerly; the check happens when
  the value is requested.
* A cached value is held through a **soft reference**: the system may discard it any time (memory pressure), so computation must be repeatable.
* Computations may be re-entered (cyclic dependencies in code models); a **recursion guard** prevents infinite recursion and marks results
  that depended on a cut-off as *not cacheable*.

```
get_cached_value(owner, provider):
    slot = owner.user_data[key]                      # created once per (owner, provider type/key)
    if slot has value and all(dep.stamp == remembered) : return value
    result = provider()                               # may be run concurrently on several threads; must be idempotent
    store (result.value, [(dep, dep.stamp_now) for dep in result.dependencies])   # soft
    return result.value
```

## 2. Modification tracker contract

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CACHE-001 | MUST | A tracker exposes one cheap monotone `u64` stamp. It never decreases within a process. It is safe to call from any thread without locks. | 1M concurrent reads, no lock contention shown in a profile. |
| PLAT-CACHE-002 | MUST | Every tracker documents precisely **what it tracks and when it increments**. A tracker that increments on every read ("ever changed") and one that never increments ("never changed") exist as constants. | Docs lint: each tracker type has a non-empty "increments when" paragraph. |
| PLAT-CACHE-003 | MUST | A tracker's value is only meaningful for **equality** with an earlier read. Callers never subtract or compare order across trackers. | n/a (API shape: expose `changed_since(token)` instead of the number if desired). |
| PLAT-CACHE-004 | MUST | Trackers can be composed (a tracker that changes when any of several change) without losing monotonicity. | Composite over A and B changes if either does. |
| PLAT-CACHE-005 | MUST | Stamp increments are visible to a thread that starts a read section after the write section that caused them. | After a write section, a new read section sees the new stamp. |

## 3. Cached value contract

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CACHE-010 | MUST | A cached value is created with a **provider** (a function without side effects) returning a **result**: the value and a list of dependency items. The value may be absent (cache "none"). | Provider returns `None` with deps; second call does not recompute while deps are unchanged. |
| PLAT-CACHE-011 | MUST | Dependency item kinds and the stamp each contributes: (a) a tracker: its stamp; (b) a **virtual file**: its content modification stamp; (c) a **document**: its modification stamp; (d) a **tree element**: the modification stamp of its containing file, or "invalid" if the element is invalid; a **directory element**: the project PSI modification count; (e) the **global PSI modification count constant**: the project's PSI modification count; (f) another cached value: valid iff that value is itself valid; (g) a weak/optional wrapper: a gone referent counts as changed. Anything else is an error. | Unit test per kind. |
| PLAT-CACHE-012 | MUST | A stamp of "gone or negative" is always treated as **changed**; equality of the remembered and current stamp means unchanged; every other difference means changed. | A dependency on a deleted file never validates. |
| PLAT-CACHE-013 | MUST | **Fast path**: if all dependencies of a value are (project PSI modification count) or items that imply a change of that count when they change (physical elements of physical files, project-root tracker), only the project count is stored and compared (one comparison instead of N). | Value with 50 element dependencies in physical files validates with one stamp compare. |
| PLAT-CACHE-014 | MUST | The value is held by **soft reference** (or the language's closest equivalent: an evictable cache with a memory budget). When evicted the next access recomputes. Providers must therefore be repeatable. | Force eviction; value recomputed once. |
| PLAT-CACHE-015 | MUST | The computation may run **concurrently** on several threads for the same cached value; the cache is thread-safe but **not atomic**. All computed results must be *equivalent*; whichever finishes first is stored, the others are returned to their callers (or discarded). | Two threads request the same value: both get an equivalent result. |
| PLAT-CACHE-016 | MUST | A value is never stored with dependency stamps newer than the data it read. The reference achieves this because computation runs in a read section that excludes writes, and stamps are read when the result is stored. An implementation without a global read/write exclusion (immutable snapshots) **must read the stamps from the snapshot the computation used** (or before it started), so a change that happened during the computation invalidates the stored value on the next access. | Modify a dependency while a computation on an older snapshot runs: the stored value is invalid at the next access. |
| PLAT-CACHE-017 | MUST | **Context independence**: if a cached value is stored on an owner object X, its provider may depend only on X and on long-lived global state (services). It may not capture: parameters of the calling method (other than X), the object that created the cache if X can outlive it, thread-locals, or tree elements near X (they can change while X lives). Violations are detected by occasional debug-mode checks which re-create the provider in a different context and compare the results. | A provider capturing a method parameter fails the debug check. |
| PLAT-CACHE-018 | MUST | **Result equivalence**: for equal inputs, a recomputation yields an equivalent result: equal values; element results equivalent by `are_elements_equivalent`; resolve results with equal targets in equal order; collections with equal size and element order. Debug builds randomly recompute cached values and compare. | Self-check enabled in tests catches a provider that returns elements in hash order. |
| PLAT-CACHE-019 | MUST | A **recursion guard** wraps computations that may re-enter: the second entry for the same key on the same thread returns "none" immediately. A stack stamp taken **before** the computation tells afterwards whether the result depended on a cut-off (`may_cache_now`). Cached values store results only if `may_cache_now` is true. The guard call must be **inside** the provider, not around the cache access (else nothing is ever cached). | A cyclic inference: the outer call caches; inner results computed during the cycle are not cached. |
| PLAT-CACHE-020 | MUST | A computation can **prohibit caching** since a point (for example when it gave up because the nesting was too deep) so that all values computed since that point are not cached. | Depth-limited inference marks results non-cacheable. |
| PLAT-CACHE-021 | SHOULD | Memoization of results inside one recursion-guarded computation (thread-local, until the outermost guarded call finishes) avoids exponential blowup; it is dropped when the computation had side effects. | A diamond-shaped dependency graph is computed linearly. |
| PLAT-CACHE-022 | SHOULD | A cached-value **profiler** can be enabled in debug builds to report per value type: hit/miss ratio, computation time, invalidations by dependency. | Profile after a typing session lists the worst invalidators. |
| PLAT-CACHE-023 | MUST | Cancellation during a provider leaves no partial value and does not poison the cache. | Cancel mid-provider, then request: recomputed normally. |

## 4. The manager (where values live)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CACHE-030 | MUST | The manager stores a cached value in the **user data** of an owner (tree element, file, project, library, module, any object with typed user data) under a key; "get or create" semantics. The key may be derived from the provider's class when no explicit key is given. | Two `get` calls with the same key return the same slot. |
| PLAT-CACHE-031 | MUST | Variants: plain (with explicit or default key); **parameterized** (value depends on a parameter passed at access time; used rarely; the parameter must not affect validity); **project-wide PSI-dependent** helper (a value stored on an element and invalidated on any PSI change); and creation of a free-standing cached value object. | Conformance tests per variant. |
| PLAT-CACHE-032 | MUST | A value stored on a tree element is dropped with the element (when the element is invalidated or collected) and does not keep the element alive beyond what its user data already does. | No leak when 10k elements are created and dropped. |
| PLAT-CACHE-033 | SHOULD | **Value tracking**: a cached value may declare that its *result object itself* is a dependency-capable object (for example, a cached value of another cached value) whose validity participates. | A value of type "cached value" invalidates its holder when inner becomes invalid. |
| PLAT-CACHE-034 | MUST | Values for a *disposed* project are never returned. | After project close, `get` on a stale element returns the invalid-element error or recomputes, never returns old data. |

## 5. Catalog of required trackers

| ID | Pri | Tracker | Increments when |
|---|---|---|---|
| PLAT-CACHE-040 | MUST | **PSI modification count** (per project) | any **physical** tree change (add, remove, replace, children changed, move, rename of file or directory, file added/removed/moved in loaded dirs); file type changes; project-root changes; plug-in unload of PSI; **entering or leaving dumb mode**. Not for non-physical trees, not for pure "writable" flag changes. |
| PLAT-CACHE-041 | MUST | **Per-language PSI tracker** | any change touching an element or file of that language; plus every *generic* change of the line above (roots changed, file types changed, unloaded PSI, directory removed). A tracker for "any language" changes on every change. |
| PLAT-CACHE-042 | SHOULD | **Language predicate tracker** | any of the languages matching a predicate changed |
| PLAT-CACHE-043 | MUST | **VFS structure modification count** | any create, delete, move, rename in the VFS |
| PLAT-CACHE-044 | MUST | **File content stamp** (per virtual file) | each content change |
| PLAT-CACHE-045 | MUST | **Document stamp** (per document) | each edit |
| PLAT-CACHE-046 | MUST | **Project root modification tracker** | project model roots change: module roots, library roots, SDK, dependency list, exclusion |
| PLAT-CACHE-047 | MUST | **Dumb-mode tracker** | each entry into or exit from dumb mode |
| PLAT-CACHE-048 | MUST | **Stub index tracker** (per project, per file element type) | stub of any file (of that type) changed |
| PLAT-CACHE-049 | MUST | **Index timestamp** (per index) | the index content changed |
| PLAT-CACHE-050 | SHOULD | **Extension point change tracker** | contributions registered or removed for an extension point |
| PLAT-CACHE-051 | SHOULD | **Settings trackers** (code style, inspection profile, language level) | the relevant setting changed |
| PLAT-CACHE-052 | SHOULD | **Library/classpath tracker** | library content changed on disk (jar replaced) |
| PLAT-CACHE-053 | SHOULD | **Module-system tracker** (Java `module-info` files) | any `module-info` changed |

Requirements on the catalog:

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CACHE-054 | MUST | The trackers whose catalog priority is MUST are required for the first release; the SHOULD ones follow. | Presence tests. |
| PLAT-CACHE-055 | MUST | The PSI modification count is incremented **before** or **atomically with** the first moment a reader could observe the changed tree; and a notification (listener topic) is published on the write thread each time it changes, so UI caches can drop. | A cached value computed before an edit is invalid after it. |
| PLAT-CACHE-056 | MUST | **Do not over-use the global PSI count.** Documentation of the API says: depending on it means "invalid on literally every PSI change in the project", useless if the value is requested whenever the user edits. Prefer per-language trackers, file dependencies, or stamps of the inputs actually read. Caveat that a per-language tracker is **not sufficient** for cross-language resolve (Java resolve depends on Kotlin and other JVM languages). | Java resolve caches depend on a tracker that includes all JVM languages. |
| PLAT-CACHE-057 | MUST | Computed data that is *derived from a file's own tree only* should depend on that file (its element or its stamp), not on the global count. | A per-file outline cache survives edits in other files. |
| PLAT-CACHE-058 | MUST | Computed data that depends on **declarations of other files** needs a dependency that changes when those declarations change. In the reference this is the global or per-language PSI count; Archipel's finer scheme (section 7) must be at least as conservative. | See section 7 acceptance tests. |
| PLAT-CACHE-059 | SHOULD | The modification count API gives a **token** (opaque) and `has_changed(token)`; internal numeric values are not exposed. | n/a |

## 6. Out-of-code-block modification (what it was, what the reference does now)

Historically, IntelliJ separated edits **inside a code block** (a method body: cannot change what other files can resolve) from
**outside** (declarations, imports, package, annotations: can change resolve everywhere). A separate counter for the second kind
let Java caches ignore body edits. The studied tree **no longer has that split**:

* there is one project PSI modification count;
* the old "structure" key still exists as a **deprecated alias of the global key**, scheduled for removal, with a note that it is rarely
  supported by JVM language plugins and is the "wrong way" to optimize;
* a legacy method that bumps the "out-of-code-block" counter now bumps the one global counter;
* Java caches that need declaration-level stability depend on the global count or on per-class counters compared manually
  (a class's inner-members cache stores the global count and recomputes when it differs);
* `JavaPsiFacade`'s class and package caches are **cleared entirely on every PSI modification count change**.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CACHE-060 | MUST | The first-release cache layer may use the **single global count + per-language counts** exactly as the reference does. This is correct by construction. | All resolve-dependent caches depend on a tracker in the catalog; no cache depends on "structure-only" data. |
| PLAT-CACHE-061 | MUST | If Archipel adds a **finer invalidation** (declaration interface hash, "early cutoff"), the following are all part of a file's *interface* and must change its interface stamp when edited: package declaration; imports (including static and on-demand); names, modifiers, type parameters and bounds of every class; supertypes; names, modifiers, parameter lists, return types, `throws` of every method; fields' names, types and modifiers; **initializers of fields that can be compile-time constants**; **annotations with their arguments** on any declaration; enum constants; record components; module declarations; **return types inferred from bodies** (expression-bodied functions without declared type; Kotlin; Java `var` is local and excluded); default values of annotation members; the *list of files* in the package (directory membership); language level; and implicit declarations produced by annotation processors, framework contributions or compiler plugins. | Property test: for 10,000 random edits on a corpus, whenever an edit changes the resolve result of any reference in any other file, the interface stamp of the edited file changed (no false "unchanged"). |
| PLAT-CACHE-062 | MUST | A finer scheme must treat **edits in non-Java dependencies of resolve** conservatively: framework metadata files (Spring configuration, `META-INF/spring.factories`, XML configs, `application.properties`) that contribute to resolve are part of the dependency set; when unknown, fall back to the global count. | Editing `application.properties` invalidates Spring property-key resolution. |
| PLAT-CACHE-063 | MUST | Caches that are cleared "on every change" (resolve cache, class lookup cache, resolve and use scope caches) are cleared in the *before* phase of physical changes (PLAT-PSI-067), not lazily after. | Run a stress test with concurrent resolve and edits; no stale hits. |
| PLAT-CACHE-064 | SHOULD | Measure the cost of the coarse scheme before building the fine one: typing in a 3,000-line file with 500 cached resolves should keep re-resolve of visible references under the daemon budget. | Benchmark recorded in `docs/perf`. |

## 7. Memory, eviction and explicit dropping

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CACHE-070 | MUST | A **low-memory signal** (process RSS near the configured limit) drops resolve caches and evictable cached values; trees of closed files are dropped (stubs and indexes stay). | Simulate pressure: resolve caches empty, editor still responsive. |
| PLAT-CACHE-071 | MUST | `drop_resolve_caches()` (any thread) and `drop_psi_caches()` (write thread) exist for batch operations; the latter clears all values that depend on the global PSI count plus resolve caches. | Batch mode loop over 10,000 files keeps memory bounded. |
| PLAT-CACHE-072 | MUST | Caches have **bounded** size: weak keys and soft values (or an LRU with a byte budget); the sum of cache budgets is documented. Empty and "none" results are stored strongly because they are small. | Memory report lists budgets. |
| PLAT-CACHE-073 | SHOULD | A cache of derived data may be **persisted** on disk (for example highlighting markup, folding state, inlay hints) and revalidated by (file stamp, tool version). A mismatch discards it. | Reopen a large file: highlights appear from cache and are replaced by fresh ones. |
| PLAT-CACHE-074 | MUST | `Invalidate caches` (user action) clears persistent caches (VFS, indexes, persisted markup) and all in-memory caches, then restarts indexing; it asks for confirmation and offers "Invalidate and Restart". | After the action, the IDE restarts with rebuilt indexes. |

## 8. Disagreement with the code-intelligence spec (D5 and section 1.4)

The code-intelligence spec proposes dependency stamps with **early cutoff**: `Dep::{File, Interface, LanguageInterface, Index, DumbMode, Settings}`
and an `InterfaceEpoch` that does not bump if a file's declaration summary is unchanged. It cites that `ResolveCache` is cleared before every PSI
change and that the global count is "too coarse".

* **Agreement**: the structure is the same as the reference (lazy validation, soft values, per-language trackers, dumb mode as a
  dependency).
* **Disagreement 1: "Interface epoch" is not what the reference does.** The reference *dropped* its code-block/structure split. Any finer scheme is
  an Archipel invention and carries correctness risk: **IntelliJ's own experience is that a structure-only counter was a "wrong way to optimize"**.
  Therefore PLAT-CACHE-060 makes the coarse scheme the MUST and PLAT-CACHE-061/062/064 make the finer scheme a gated SHOULD with a property
  test that must pass before it replaces the coarse scheme.
* **Disagreement 2: `Dep::DumbMode(u64)` as a *separate* item**: the reference folds the dumb transition into the global PSI count itself
  (PLAT-DUMB-008). Keep both; the global count must bump on dumb transitions.
* **Disagreement 3: cache size "64 MiB total"**: the reference uses weak/soft references and a low-memory watcher, not a fixed total. A byte
  budget is acceptable (PLAT-CACHE-072) but must have the low-memory path (PLAT-CACHE-070).
* **Disagreement 4: results computed under recursion guard**: the earlier spec does not mention cache poisoning by cut-off recursion. PLAT-CACHE-019/020 are MUST.

## 9. Notes for implementers (non-binding)

* In Rust, "soft reference" becomes an LRU with a byte weigher plus a global budget; "user data on an element" becomes a typed
  slot map on the owner (file snapshot, project, module) keyed by a zero-sized key type.
* Make provider purity checkable: pass the provider a `DepCollector` and forbid access to anything not routed through it (reads of other files
  register dependencies automatically). That removes most of PLAT-CACHE-017's failure modes.
* Run the idempotence self-check (PLAT-CACHE-018) in CI on the whole test corpus; it is the cheapest way to find non-deterministic resolvers.

## 10. Open questions

1. Does Archipel's project model (a `ProjectModel` rebuilt from scratch on import) need an *incremental* root tracker, or is a single counter
   bumped per import enough? The reference bumps on any root change; a rebuild bumps once per import (fine).
2. Persisted caches (PLAT-CACHE-073) are SHOULD; which are worth it first: highlighting, folding, inlay hints?
