# Platform spec 04: dumb mode (indexing state), the dumb-aware contract, user-visible behavior

Status: specification (clean-room). ID prefix `PLAT-DUMB`. Priorities: `MUST` / `SHOULD` / `LATER` as in `README.md`.

Supersedes and deepens the "Dumb mode, redesigned per index" part of section 1.6 of
`docs/analysis/code-intelligence-spec.md`; see section 9 for the disagreement.

Sources studied: `ic: platform/core-api/src/com/intellij/openapi/project/` (`DumbService.kt`, `DumbAware.java`,
`PossiblyDumbAware.java`, `DumbModeTask.java`, `IndexNotReadyException.java`, `DumbModeListenerBackgroundable.kt`,
`DumbModeBlockedFunctionality.java`), `ic: platform/platform-impl/src/com/intellij/openapi/project/DumbServiceImpl.kt`,
`ic: platform/indexing-api/src/com/intellij/util/indexing/DumbModeAccessType.java`,
`ic: platform/lang-impl/backend/src/com/intellij/util/indexing/` (`FileBasedIndexImpl.java`, `FileBasedIndexProjectHandler.java`,
`UnindexedFilesScannerExecutorImpl.kt`, `DumbModeWhileScanningTrigger.kt`),
`ic: platform/core-impl/src/com/intellij/psi/impl/PsiModificationTrackerImpl.java`,
`ic: platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/DaemonListeners.java`,
`ic: java/java-psi-impl/src/com/intellij/psi/impl/JavaPsiFacadeImpl.java`, `ic: java/java-psi-api/src/com/intellij/psi/augment/PsiAugmentProvider.java`,
message bundles: `ic: platform/platform-api/resources/messages/IdeBundle.properties`,
`ic: platform/lang-api/resources/messages/CodeInsightBundle.properties`,
`ic: platform/refactoring/resources/messages/RefactoringBundle.properties`,
`ic: platform/indexing-api/resources/messages/IndexingBundle.properties`, `ic: platform/util/resources/misc/registry.properties`.

## 1. Model

**Dumb mode** is a per-project, global, boolean-like state meaning "indexes are being updated in the background; code that
needs indexes may not run". **Smart mode** is the opposite. It is not a gradient and not per index: *every* index is unavailable
while the project is dumb, even if some would be ready. (Archipel may track readiness per index internally; the observable
contract below is the global one. See section 9.)

Key properties of the reference behavior:

1. A project **starts dumb** and becomes smart when startup activities that need smart mode, and the initial indexing, complete.
2. Dumb mode **starts and ends inside a write section**. Therefore, code running in a read section on a background thread
   never sees the state flip *during* that section. It can only see a different state when it starts a new read section.
3. A counter of *running dumb tasks* decides the state: >0 means dumb. Queueing a task from the UI thread makes the project dumb **immediately**;
   tasks run later, in the background, one after another.
4. Every transition bumps the project's modification counter (spec 05), invalidating all caches that depend on it.
5. Index access while dumb throws a typed **index-not-ready** error unless the caller declared a dumb-mode access type.
6. Features are **opt-in** to dumb mode ("dumb-aware"). The platform skips (or disables) everything else while dumb.

## 2. State machine and tasks

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-DUMB-001 | MUST | Each project has exactly one state, *dumb* or *smart*, queryable cheaply from any thread (the answer is only reliable inside a read section or on the UI thread). A project starts dumb. The "default/template" project and projects without indexing are always smart. | A fresh project reports dumb until initial indexing completes. |
| PLAT-DUMB-002 | MUST | The state changes only inside an exclusive (write) section, one transition at a time. A reader holding a read section never observes a change. | Start dumb mode while a background read action runs: the action is cancelled and restarted, never torn. |
| PLAT-DUMB-003 | MUST | The state is driven by a **count of active dumb tasks**; the state is dumb iff the count is greater than zero. A task increments before it is queued (a failed increment means the task is not run) and decrements after it finishes or is cancelled. | Queue two tasks, cancel one: still dumb; finish the other: smart. |
| PLAT-DUMB-004 | MUST | `queue_task(task)` from the UI thread makes the project dumb **synchronously**; the task body runs later on a background worker with a progress indicator. From other threads, the transition is scheduled on the UI thread. Tasks run **sequentially**. | After `queue_task` returns on the UI thread, `is_dumb()` is true. |
| PLAT-DUMB-005 | MUST | Equal queued-but-not-started tasks **merge**: a task can offer a merge with any task already waiting; the offer's result decides: nothing, replace, or replace with a third task. Default equality: same task type and same equivalence object. | Queue 100 "reindex" tasks quickly: one runs. |
| PLAT-DUMB-006 | MUST | `cancel_task(task)`: if queued it is dropped; if running its progress is cancelled so its next cancellation check throws. | Cancel a running task: it stops between files. |
| PLAT-DUMB-007 | MUST | `complete_just_submitted_tasks()` (UI thread only) runs tasks queued earlier **in the same UI event** under a modal progress and finishes dumb mode, so callers who just triggered a root change can query indexes immediately after (refactorings, project import). It does nothing if tasks were queued earlier or others are already running. | Add a library and immediately resolve in the same UI action: indexes ready after the modal progress. |
| PLAT-DUMB-008 | MUST | Entering or leaving dumb mode **increments the project's PSI/global modification counter** and notifies the PSI modification listeners, so every cache keyed by it is invalidated (resolve results computed in a different mode must never be reused). | A resolve cached in smart mode is recomputed after dumb mode starts. |
| PLAT-DUMB-009 | MUST | **Listeners**: two flavors, `entered` and `exit`. (a) *synchronous*: called inside the write section of the transition, thread undefined, must be fast; (b) *UI-thread*: delivered later on the UI thread. For (b): `entered` and `exit` calls strictly alternate; an `exit` is never reported after a later `entered`, even if modal dialogs delayed delivery (deduplicate against the public state). | Rapid dumb/smart/dumb transitions: listeners see entered, exit, entered, in order, no repeats. |
| PLAT-DUMB-010 | MUST | `run_when_smart(f)`: run `f` on the UI thread now if smart and the project is initialized, otherwise as soon as it becomes smart. Not re-entrant protection: dumb mode may start again while `f` runs. `smart_invoke_later(f[, modality])` is the same but always asynchronous (never inline). Neither runs if the project is closed meanwhile. | Call `run_when_smart` during indexing: runs after exit; close the project first: never runs. |
| PLAT-DUMB-011 | MUST | `wait_for_smart_mode()` blocks a non-UI thread until smart (never call from the UI thread or while holding a read lock: deadlock); a timeout variant returns false on timeout. After it returns dumb mode may already have restarted. | Background thread blocked in `wait_for_smart_mode` resumes after exit. |
| PLAT-DUMB-012 | MUST | The recommended way to "run a read action that needs indexes" is a **non-blocking read action constrained to smart mode** (or its coroutine form). The deprecated "run read action in smart mode" form must not be offered in the new API and is not callable when a read lock is already held (it would deadlock); when already inside a read section the code runs and may fail with index-not-ready. | A smart-constrained non-blocking read action never runs a body while dumb; it re-runs after a transition. |
| PLAT-DUMB-013 | SHOULD | Indexing can be **suspended** while a heavy activity runs (for example a build) and resumed afterwards; user can pause manually; the UI shows "Indexing paused due to {reason}". | Start a build; indexing pauses; at build end it resumes. |
| PLAT-DUMB-014 | MUST | A running dumb task is cancelled when the project closes or when indexes are invalidated by the user ("invalidate caches"); the project closes without waiting for the whole queue. | Close a project during indexing within 2 seconds (budget). |

## 3. What triggers dumb mode (summary of spec 03 thresholds)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-DUMB-020 | MUST | **Project open**: dumb until scanning has enqueued the stale files and indexing them is done; if no file is stale, the project becomes smart as soon as the scan is done and startup activities that need smart mode complete. | Re-open an unchanged project: dumb mode is brief (scan only). |
| PLAT-DUMB-021 | MUST | **Project model change** (roots, SDK, libraries, modules): re-scan; dumb only if at least 20 stale files are found. | Add a small library jar: no dumb mode; add a large one: dumb mode with progress. |
| PLAT-DUMB-022 | MUST | **VFS refresh** that changes at least 20 files, or at least 1,048,576 bytes of files, or whose counting exceeds 100 ms: dumb mode for re-indexing; otherwise lazy update. | See PLAT-IDX-031. |
| PLAT-DUMB-023 | MUST | **Index rebuild** (version change, corruption, user request): full re-index in dumb mode. | Corrupt an index: dumb mode starts and ends with the index rebuilt. |
| PLAT-DUMB-024 | SHOULD | **File type or language-level changes** that change what is indexed: re-index affected files in dumb mode (or lazily when few). | Remap a file type for 1,000 files: dumb mode. |
| PLAT-DUMB-025 | MUST | Scanning itself runs in smart mode by default; it **waits** for any already scheduled dumb tasks to finish first, so extra tasks queued by startup activities finish before scanning starts. (Configurable.) | Open a project: scanning begins after pending dumb tasks end. |

## 4. The dumb-aware contract

A **dumb-aware** feature promises that it neither reads indexes nor calls code that does (resolve of index-backed references,
stub lookup, class finders, short-name caches, find usages) while the project is dumb, or that it catches the typed error and
degrades. Features are marked by a *marker* (a trait implemented by the feature type) or by a per-instance *possibly dumb-aware* query.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-DUMB-030 | MUST | Every extension, action, editor highlighting pass, annotator, folding builder, completion contributor, injector, line-marker provider, file editor, tool window and background activity declares dumb-awareness **explicitly** (default: *not* dumb-aware). The registry refuses to guess. | A new contribution type without a declared default fails registration review (lint). |
| PLAT-DUMB-031 | MUST | While dumb, the platform **filters** contribution lists by dumb-awareness before invoking them (setting "check awareness" is on by default). Non-aware contributions are simply not run. | Register a non-aware annotator; during indexing it is not called. |
| PLAT-DUMB-032 | MUST | A non-aware **action** is disabled while dumb. If the user triggers it anyway (shortcut, menu) the platform shows a balloon: "This action is not available, analyzing the project…" (generic) or "'{action}' will be available once project analysis is finished" (named). If several candidate actions all fail: "None of the following actions are available during the project analysis: {list}". | Press a refactoring shortcut during indexing: balloon with the named text. |
| PLAT-DUMB-033 | MUST | A dumb-aware action that nonetheless hits the typed index-not-ready error is caught by the action runner, which shows the same balloon family and **does not** report an internal error to the user. | A dumb-aware action touching an index shows the balloon, no error dialog. |
| PLAT-DUMB-034 | MUST | Code that must call index-backed APIs from a dumb-aware path **must** check `is_dumb()` first or catch the typed error, or run under an alternative-resolve or access-type wrapper. Catching the typed error and *swallowing it silently into a wrong answer* is forbidden: the feature reports "incomplete". | A completion contributor catching the error returns "no project names, analysis in progress" not an empty-but-final list. |
| PLAT-DUMB-035 | MUST | Contributions can be **possibly dumb-aware**: a per-instance boolean method overriding the marker (for example, a framework detector that is aware only if it uses no index). | Two instances of one type, one aware, one not: filtered individually. |
| PLAT-DUMB-036 | MUST | Index-not-ready is a **typed, recoverable** error (not a panic). It carries the stack of where dumb mode started, in debug builds, to help find the cause. | A debug build error message includes the dumb-mode start site. |
| PLAT-DUMB-037 | MUST | A **cancellation exception must not be swallowed** in the same breath: the typed error is caught narrowly (only this error), never as a generic catch-all that also swallows cancellation. | Lint on catch-alls in extension code. |
| PLAT-DUMB-038 | SHOULD | A UI container that hosts non-dumb-aware content can be **wrapped gently**: during dumb mode it shows the label "Not available until indices are built" (or a localized equivalent) in place of the content and restores the content after exit. | A tool window that needs indexes shows the placeholder during indexing. |

## 5. Dumb-mode access types and alternative resolve

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-DUMB-040 | MUST | `ignore_dumb_mode(access_type, f)` runs `f` allowing index reads in dumb mode. In smart mode it is a plain call. Access types: *reliable data only* (only entries known to be up to date) and *raw index data acceptable* (anything stored, even stale or incomplete; never for stub indexes). | Name completion with `raw` access in dumb mode returns names; with `reliable` it returns those already stable. |
| PLAT-DUMB-041 | MUST | Results obtained with `raw` access must be treated as hints: they are never cached in a way that outlives the dumb state, never used to *modify* code (refactoring), and are flagged to the UI as possibly stale. | A refactoring never takes `raw` data as input. |
| PLAT-DUMB-042 | MUST | **Alternative resolve**: a per-thread, scoped flag that, **only if dumb**, tells resolve and class lookup to use slower non-index strategies (directory and package based lookups). The platform never enables it by itself for background work; explicit user actions (Go to Declaration on a symbol) enable it. It is not a guarantee: some calls still raise index-not-ready. | Go to Declaration of a project class during indexing navigates (slowly); a library class may fail with the notice. |
| PLAT-DUMB-043 | MUST | Alternative resolve is restored (not just cleared) on scope exit, nested uses restore the previous value, and it is not inherited by other threads. | Nested enable/disable restores correctly. |
| PLAT-DUMB-044 | MUST | In alternative-resolve mode, class finders that are not dumb-aware are skipped and the Java class lookup uses package directories (spec 03, PLAT-IDX-097). | See PLAT-IDX-097. |

## 6. Behavior table: what works in dumb mode

This is the *required* observable behavior for the first release. "Works" means the feature runs with no error and with full
function; "Degrades" means it runs with reduced results and says so; "Blocked" means it is disabled with the standard message.

| Feature | Dumb-mode behavior | Message / indication | ID |
|---|---|---|---|
| Typing, editing, undo, save | Works | none | PLAT-DUMB-050 |
| Syntax highlighting (lexer-based), brace matching, comment/uncomment, quote handling | Works | none | PLAT-DUMB-051 |
| Syntax error highlighting from the parser (error elements) | Works | none | PLAT-DUMB-052 |
| Folding | Works for dumb-aware builders (the platform default is "not aware"; builders that use only the tree declare aware); others produce no regions | none | PLAT-DUMB-053 |
| Structure view of the current file | Works if built from the tree only | none | PLAT-DUMB-054 |
| Local resolve (variables, parameters, members of the same file) and highlighting that depends only on it | Works only for features declared aware; otherwise skipped | none | PLAT-DUMB-055 |
| Annotators and inspections | Only dumb-aware ones run; the daemon **restarts when dumb mode starts and when it ends** so results are consistent with the state | none | PLAT-DUMB-056 |
| Go to File, Recent Files, project tree, file name search | Works (file-name data comes from the VFS) | none | PLAT-DUMB-057 |
| Go to Class / Go to Symbol / Search Everywhere (symbol part) | Degrades: shows the text "Analyzing project…" in the popup header, and "Results may be incomplete during the project analysis". The class/symbol tab falls back to file names with the message "'Go To {0}' action is not available during the project analysis, using 'Go To File' instead" | see left | PLAT-DUMB-058 |
| Find in Files (plain text) and text-occurrence search | Degrades: the word index is read with *raw* access (whatever is currently stored, possibly incomplete); no error is raised; results may be incomplete | "Results may be incomplete during the project analysis" in result popups | PLAT-DUMB-059 |
| Find Usages | Blocked: the menu item reads "Search for references (not available during the project analysis)" | see left | PLAT-DUMB-060 |
| Go to Declaration | Degrades with alternative resolve (PLAT-DUMB-042), otherwise the generic balloon | balloon text of PLAT-DUMB-032 | PLAT-DUMB-061 |
| Completion | Degrades: dumb-aware contributors only (word/text completion, keywords, local names, file-local); popup shows "Analyzing project…" | see left | PLAT-DUMB-062 |
| Quick documentation | Blocked for rendered docs: "Cannot render documentation during the project analysis" (italic) | see left | PLAT-DUMB-063 |
| Refactoring (rename, move, safe delete, change signature) | Blocked: "Refactoring is not available until indexes are ready"; rename of a file or symbol: "Cannot update references while project is being analyzed" | see left | PLAT-DUMB-064 |
| Move file | Allowed only after a warning: "Moving files during the project analysis is in progress will not update their usages. The selected element won't be renamed in non-project files. Proceed anyway?" | see left | PLAT-DUMB-065 |
| Custom-folding region navigation | Blocked: "Custom folding navigation will be available after the project analysis is finished." | see left | PLAT-DUMB-066 |
| VCS, terminal, run/debug of already-built targets | Works | none | PLAT-DUMB-067 |
| Build/compile | Works (builds do not use indexes of this IDE) | none | PLAT-DUMB-068 |
| Status bar | Shows progress "Analyzing project…" with a determinate bar and a link/popup; the popup title "Analyzing Project to Enable Smart Features" | see left | PLAT-DUMB-069 |
| Popups and tool windows that list results (Find, hierarchy, bookmarks) | Show a banner "Analyzing project…" while dumb and a **"Reload results"** link after exit | see left | PLAT-DUMB-070 |

Each of PLAT-DUMB-050..070 is MUST for the behavior in the table, except 063, 065, 066 and 070, which are SHOULD.
The acceptance of each is the observable behavior stated in the row (a UI scene test or an integration test that starts a long-running
dumb task and exercises the feature).

## 7. After dumb mode ends

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-DUMB-080 | MUST | When dumb mode ends: (1) all highlighting is recomputed for all open editors (the daemon restart in PLAT-DUMB-056); (2) editor highlighters are rebuilt (file type related effects); (3) pending file type reparse work is processed; (4) editor notifications bars and gutter state are refreshed; (5) parameter info popups are updated; (6) result popups show the reload link. | After indexing finishes, an unresolved-reference highlight that was shown during dumb mode (from a non-index analysis) is recomputed with full information. |
| PLAT-DUMB-081 | MUST | Anything computed while dumb that depended on missing index data must not be cached across the transition. (The counter bump of PLAT-DUMB-008 is the mechanism; component-specific caches also drop on `exit`.) | See PLAT-DUMB-008. |
| PLAT-DUMB-082 | SHOULD | Framework or plugin detection that needs indexes is re-run on `exit`. | Open a Spring project: Spring facets appear after indexing. |
| PLAT-DUMB-083 | MUST | If many editors are open, restart work is prioritized for visible editors first. | The visible editor re-highlights before background tabs. |

## 8. Light mode (reduced IDE without project analysis)

IntelliJ also has a "Light mode" where the project is treated as permanently dumb (a user choice for huge repositories); the message texts
mirror the dumb-mode ones ("Light mode", "Results may be incomplete in Light mode", "This action is not available in Light mode",
"'{action}' is not available in Light mode"). Treat it as LATER: the architecture must allow the dumb state to be *held* indefinitely
by a policy instead of a task, and the UI texts to switch.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-DUMB-090 | LATER | A policy can hold the project dumb permanently; messages switch to the "Light mode" variants. | Enable the policy: no indexing runs; banners read "Light mode". |

## 9. Disagreement with the code-intelligence spec (D7 and section 1.6)

`docs/analysis/code-intelligence-spec.md` proposes *per-index* dumb mode: "features declare which indexes they need and degrade individually",
with `IndexNotReady` as a normal error value.

* **What the reference does**: one global state per project; any index read while dumb (without an access type) fails.
  The per-index idea is a refinement, not a replacement.
* **Why it matters**: mixing ready and not-ready indexes can produce **silently inconsistent results** (for example, a stub index
  entry exists for a class but the word index is behind, so "find usages" silently misses hits). In the reference, this is
  avoided by making the state global and recomputing the daemon on each transition.
* **Resolution**: PLAT-DUMB-001..003 (global state) are MUST and are the *UI-facing* contract. Archipel may implement
  per-index readiness **internally** to end dumb mode earlier for indexes that are complete and consistent as a set, provided that
  (a) the global state flips only when *all* indexes that any smart-mode feature needs are consistent; (b) features gated on a
  subset use a named *capability* ("names", "usages", "hierarchy") that maps to a set of indexes; (c) the observable table in section 6 holds.
* The typed index-not-ready error as a *return value* (Rust `Result`) is fine and encouraged; PLAT-DUMB-037 (never swallow cancellation)
  translates to "never map the `Cancelled` variant to a user-visible error".

## 10. Notes for implementers (non-binding)

* In a design without a global write lock (an immutable-snapshot design), PLAT-DUMB-002 becomes "a snapshot carries the dumb flag;
  a snapshot taken while smart stays smart-consistent for its lifetime, and results computed from it are discarded if the flag flipped
  before delivery". The observable promises (no torn reads, caches dropped on transition) are unchanged.
* The most common bug class is a feature that silently returns *empty* in dumb mode. Add a test helper that runs every
  feature under a forced-dumb state and fails if the result is empty **without** an "incomplete" indication.

## 11. Open questions

1. Registry default for scanning to wait for non-dumb mode is on in the studied tree; Archipel needs a policy for startup activities
   that queue extra dumb tasks (framework detection, import) so the initial "dumb" window does not split into several flickers.
2. Should Search Everywhere show partial results from `raw` access (PLAT-DUMB-040) or only the file-name fallback shown in the
   reference? The reference text says it falls back to file names; Archipel's earlier spec suggested partial symbol results.
