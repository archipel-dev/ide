# Platform spec 06: concurrency, read and write actions, cancellation, non-blocking reads

Status: specification (clean-room). ID prefix `PLAT-CONC`. Priorities: `MUST` / `SHOULD` / `LATER` as in `README.md`.

Supersedes and deepens section 1.11 and decision D4 of `docs/analysis/code-intelligence-spec.md`; see section 9.

Sources studied: `ic: platform/core-api/src/com/intellij/openapi/application/` (`Application.java`, `ReadAction.java`,
`WriteAction.java`, `NonBlockingReadAction.java`, `ReadConstraint.kt`, `coroutines.kt`, `ModalityState.java`),
`ic: platform/core-api/src/com/intellij/openapi/progress/ProgressManager.java`, `ic: platform/util/base/multiplatform/src/com/intellij/openapi/progress/ProcessCanceledException.kt`,
`ic: platform/util/base/multiplatform/src/com/intellij/openapi/diagnostic/ControlFlowException.kt`,
`ic: platform/core-api/src/com/intellij/util/concurrency/ThreadingAssertions.java`,
`ic: platform/ide-core-impl/src/com/intellij/openapi/progress/util/ProgressIndicatorUtils.java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/source/resolve/ResolveCache.java`,
`ic: platform/core-impl/src/com/intellij/psi/impl/PsiDocumentManagerBase.java`,
`ic: platform/indexing-impl/src/com/intellij/psi/impl/search/PsiSearchHelperImpl.java`,
`ic: platform/platform-impl/src/com/intellij/openapi/project/DumbServiceImpl.kt`,
`ic: platform/util/resources/misc/registry.properties`.

## 1. Model

IntelliJ's threading model, restated:

* **Three lock modes.** *Read* (R): read access to PSI, VFS, documents, indexes, project model. *Write-intent* (IW): read access plus the right to
  upgrade to write; held by the **write thread** (today the UI thread). *Write* (W): read and modify. Compatibility: R with R and with IW; IW with R only; W with nothing.
  A write section waits for all readers to leave; readers that are *cancellable* are asked to leave.
* **Everything that touches shared model data happens in a read or write section.** Background work takes read sections;
  changes happen in write sections on the write thread (and, increasingly, on background write sections: document commit, VFS refresh, dumb state, file type reparse).
* **Cooperative cancellation.** A background read section must be ready to be told "stop" at any moment, because a write is pending. The signal is
  delivered at the next *cancellation point* (`check_canceled`) as a special control-flow exception (`ProcessCanceledException`, a kind of cancellation).
  It must **never be swallowed, logged as an error or converted into a user-visible error**; it must unwind to the infrastructure which then either
  discards the work or *restarts* it ("non-blocking read action").
* **Re-run, don't resume.** Because a cancelled read section is re-run from the start, it must be **idempotent**: free of side effects (or with
  idempotent ones) and independent of results stored by an earlier attempt.
* **Results are validated on delivery.** The world can change between "computed in the background" and "applied in the UI": the delivery step runs under a
  guarantee that no write happened since the computation finished (finish-on-UI-thread), or re-validates (expire conditions).

## 2. Locks and sections

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CONC-001 | MUST | Model data (VFS structure, documents, trees, project model, indexes) may be read only inside a **read section** or **write section** and changed only inside a **write section**; debug builds assert this on entry to every public accessor that has a documented requirement. | A thread without a section calling `children()` fails an assertion in debug. |
| PLAT-CONC-002 | MUST | Exactly one thread holds write intent / the write section at a time. Write sections wait for all active read sections to finish or be cancelled (cancellable ones); new read sections that start while a write is pending **wait** (blocking read) or **fail fast with a cancellation signal** (cancellable read). | A background reader never starves a pending write for more than the time to its next cancellation point. |
| PLAT-CONC-003 | MUST | A read section may be **nested** in another read or write section on the same thread; a write section may be nested in a write section. **Upgrading read to write** on one thread is a programming error that fails fast (not a silent deadlock). | Starting a write section inside a read section returns an `upgrade not allowed` error. |
| PLAT-CONC-004 | MUST | Write sections run on the **write thread** (UI thread by default). From another thread, a write request is *scheduled* on the write thread and blocks the caller until done unless the call is the async variant. Documentation warns about deadlock when the write thread is blocked waiting for the caller. | Request a write from a worker while the UI thread is idle: works. |
| PLAT-CONC-005 | SHOULD | **Background write sections** are supported for operations that do not need the UI thread: document commit (async), VFS refresh application, dumb-mode counter changes, file-type re-parse. A background write section still excludes readers and is **short**. | Async commit applies its result in a background write section; typing never blocks on it. |
| PLAT-CONC-006 | MUST | The write thread must **never block** for I/O, locks held by background threads that wait for it, or long computation. Long work uses background tasks with progress. | A watchdog detects UI-thread stalls above 200 ms and records a thread dump (budget; see `docs/perf`). |
| PLAT-CONC-007 | MUST | Callbacks scheduled "later" on the UI thread carry a **modality state** (the dialog nesting level at scheduling time); they run only when the current modality is the same or less modal. A "non-modal" and a "current" and an "any" modality exist. | A callback scheduled before a modal dialog opens does not run until the dialog closes, unless it asked for "any". |
| PLAT-CONC-008 | MUST | Threading annotations/assertions exist as first-class helpers: "must be on UI thread", "must not be on UI thread", "requires read section", "requires write section", "must not hold a read section". Used by the kit and the platform. | Test helpers for each. |

## 3. Read sections: blocking and cancellable

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CONC-010 | MUST | Two kinds of read sections exist: **blocking** (not cancelled by pending writes; blocks writes until finished; documented for UI thread and short model-consistency reads only) and **cancellable** (cancelled at the next cancellation point when a write is pending or running; may be run several times). Background code uses cancellable. | A blocking read on a worker thread triggers a lint warning in extension code. |
| PLAT-CONC-011 | MUST | When a cancellable read section is already active on the thread, a nested "blocking" request does **not** become non-cancellable: only the **outermost** boundary controls cancel/retry behavior. | Nested blocking inside cancellable can still be cancelled by the outer write request. |
| PLAT-CONC-012 | MUST | A low-level "single attempt" cancellable read fails with a *cannot read* cancellation (a kind of cancellation) if a write section is pending or running at the start or at the next cancellation point. If the thread already holds read or write access it just runs the code. | Start the attempt during a write: immediate `CannotRead`. |
| PLAT-CONC-013 | MUST | Reading while a write is running on **another thread** blocks (blocking) or fails fast (cancellable); on the write thread itself, reads inside its own write section run immediately. | n/a |
| PLAT-CONC-014 | MUST | PSI/index/resolve calls made from a read section are **cancellation points** internally: resolve cache access, class lookup, index queries, tree walks (every cached resolve access and every `find_class` call checks cancellation), so most code needs only occasional explicit checks. | A pending write cancels a tight `find_class` loop. |
| PLAT-CONC-015 | MUST | Long loops in analysis code call `check_canceled()` at least every 256 iterations or every 1 ms of work, whichever first. Tree walkers and index iterators call it themselves. | A CPU-bound inspection over 50k nodes aborts within 20 ms of a pending write (budget). |

## 4. Cancellation rules

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CONC-020 | MUST | Cancellation is a **distinct, first-class outcome**, not an error: in Rust terms, a `Cancelled` variant (or a dedicated error type) that every fallible analysis function can return and that `?` propagates. It is **never** logged as an error, **never** shown to the user, **never** converted to "not found" or "empty". | Grep-lint: no `Cancelled` mapped to `None`, `Err(other)`, or a logged error. |
| PLAT-CONC-021 | MUST | Code that catches broadly (all errors) must **re-propagate cancellation** first. Catching only a specific typed error (for example "index not ready") is allowed. | A generic error handler in a feature test re-raises `Cancelled`. |
| PLAT-CONC-022 | MUST | **Non-cancellable sections** exist for code that must finish consistently (wiring a tree mirror, applying a diff); inside, cancellation checks do not fire; they must be tiny. | A cancel request arriving in a non-cancellable section is delivered right after it. |
| PLAT-CONC-023 | MUST | Cancellation has a **source**: a pending write; an explicit cancel of the task (user pressed Cancel, task superseded); a coalescing replacement; expiry (owner disposed, project closed). All use the same mechanism so cleanup is identical. | Each source test ends with the same cleanup path. |
| PLAT-CONC-024 | MUST | A cancelled computation leaves **no observable partial state**: caches are not poisoned (PLAT-CACHE-023), index batches are atomic per file (PLAT-IDX-035), UI models are replaced only by completed results. | Cancel at random points in 1,000 runs; invariants hold. |
| PLAT-CONC-025 | MUST | Each long-running activity has a **cancellation token** tied to its owner (project, editor, task); disposing the owner cancels it. Tokens can be linked (child cancelled when parent is). | Close an editor: its pending highlighting computation is cancelled. |
| PLAT-CONC-026 | SHOULD | Cancellation exceptions caught in a worker thread when waiting on another thread's cancelled computation are re-created in the current thread (to preserve stack info), not shared. | n/a |

## 5. Non-blocking read actions (the standard way to do background analysis)

A **non-blocking read action** (NBRA) is a builder:

```
nonblocking(computation)                  # runs in a cancellable read section on a background executor
   .in_smart_mode(project)                # constraint: only run while the project is smart
   .with_documents_committed(project)     # constraint: only run while all documents are committed
   .expire_when(predicate)                # discard if the predicate is true when checked
   .expire_with(owner)                    # discard when the owner is disposed
   .coalesce_by(key...)                   # a new submission with equal keys cancels the previous one
   .finish_on_ui_thread(modality, f)      # deliver the result on the UI thread, guaranteed no write in between
   .submit(executor) -> cancellable promise
```

Semantics:

1. A submission runs the computation when its constraints hold. If a write section starts, the computation is cancelled and **re-queued**, then runs
   again when constraints hold and no write is pending.
2. Expiry conditions are evaluated twice, each time while read access is held: just before the background computation starts and again just before the finish step.
3. `finish_on_ui_thread`: after the computation, the finish callback runs on the UI thread **in a state where no write happened since the computation
   finished**; if one did, the whole thing re-runs.
4. Coalescing: submitting with equal keys cancels earlier running or queued computations (typical: one highlighting job per file, one completion per editor).
5. The result of a synchronous variant is **not safe** to use after the read section ends unless the caller revalidates.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CONC-030 | MUST | Provide the NBRA facility with the options above (smart constraint, committed constraint, expire when, expire with, coalesce by, finish on UI thread with modality, submit with executor, cancellation of the returned promise). | API conformance tests for each option. |
| PLAT-CONC-031 | MUST | A computation must be safe to run **more than once** (idempotent). The facility documents that it may be started several times and may be cancelled at any moment. | A counter incremented inside the computation shows ≥1 runs; tests verify side effects are idempotent. |
| PLAT-CONC-032 | MUST | The finish step runs on the UI thread only if the owner is alive, the modality state matches, and no write happened since the computation finished; otherwise the work is retried or dropped (expired). | Close the project between compute and finish: finish never runs. |
| PLAT-CONC-033 | MUST | A computation constrained to *smart mode* never starts, and is cancelled and re-queued if dumb mode begins during it. A computation constrained to *committed documents* waits for the commit and is re-queued by any new uncommitted change. | Start indexing during a smart-constrained computation: it restarts after smart mode returns. |
| PLAT-CONC-034 | MUST | Coalescing must use keys that identify the *kind and target* of work (for example type + file); keys from different callers must not collide. | Two features using the same key by accident are detected by a debug check on origin. |
| PLAT-CONC-035 | SHOULD | A coroutine-style equivalent exists with the same semantics: read, smart read, constrained read (blocking and non-blocking variants), and read-then-write with a re-check. The non-blocking variants restart on cancellation by write. The blocking variants delay writes. | A coroutine readAction restarts after a write. |
| PLAT-CONC-036 | MUST | Synchronous execution of an NBRA from a thread that already holds read access runs the computation directly (no cancel-on-write), fails if constraints are unmet, and is documented as unsafe for stale results. | n/a |
| PLAT-CONC-037 | SHOULD | **Read-then-write**: compute in a cancellable read section, then apply a write section only if nothing changed in between (re-validate by stamps); otherwise recompute. Used by quick fixes and refactorings that prepare in the background. | A preparation computed before an edit is recomputed, not applied. |

## 6. Progress and background tasks

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CONC-040 | MUST | User-visible background work is represented by a **task** with: title, optional cancel button, determinate/indeterminate progress, text, secondary text, and a modality (modal dialog vs background indicator in the status bar). | Start a task: it appears in the status bar widget with a cancel button. |
| PLAT-CONC-041 | MUST | A **modal** progress runs the work on a background thread and shows a dialog on the UI thread; if a dialog cannot be shown (inside a write section, headless) the work runs synchronously on the calling thread. | Start a modal task from inside a write section: runs inline. |
| PLAT-CONC-042 | MUST | Progress reporting from analysis code goes through a **progress reporter** that is also the cancellation source; it supports fractions and nested sub-steps (a step with weight). | A 3-step task with weights 1:3:6 reports 10%, 40%, 100%. |
| PLAT-CONC-043 | MUST | A task that holds a read section **must not** wait for the UI thread (deadlock: the UI thread may be waiting for a write that waits for this read). Waiting for the UI thread is done outside read sections only. | Lint on blocking waits inside read sections. |
| PLAT-CONC-044 | SHOULD | Backgrounded work is **scheduled by priority**: interactive (completion, typed handlers, hover) over daemon (highlighting) over background (indexing, usages). One dedicated interactive lane exists so interactive work never queues behind indexing. | While indexing 100k files, completion starts within 50 ms (budget). |

## 7. Parallelism inside a read section

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CONC-050 | MUST | Parallel fork/join of read work (usage search, inspections over files) is allowed: each worker takes its own cancellable read section; on a pending write all workers stop, **wait for the write to finish**, then **resume with the unprocessed items only** (idempotent per item). | Run a parallel search; type during it; search resumes and does not repeat or skip files. |
| PLAT-CONC-051 | MUST | A per-item processor must not return "stop" because of cancellation; cancellation is signalled by the exception/result, so the framework can distinguish "caller wants to stop" from "write interrupted". | A processor wrongly returning false on cancellation is caught by a debug check. |
| PLAT-CONC-052 | SHOULD | Pre-read file contents **outside** the read section when possible (prefetch) to avoid stalling the UI thread on I/O. | Find in files over cold disk does not stall typing. |
| PLAT-CONC-053 | SHOULD | The work queue order for multi-file searches processes first the files with the highest hit probability: files that also contain a *container name*, files in the target's directory, open files. | A usage search lists same-file hits first (see PLAT-IDX-082). |

## 8. Smart-mode and commit requirements for features

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-CONC-060 | MUST | Every feature that reads indexes or index-backed resolve states its requirement: *smart mode required*, *works in dumb mode*, or *degrades*. The table lives in spec 04, section 6. | Review checklist item. |
| PLAT-CONC-061 | MUST | Every feature that reads trees in the background states whether it needs *committed documents*; if so it uses the committed constraint or reads the last committed text consistently (PLAT-VFS-114). | A highlighting pass reads tree and text from the same committed revision. |
| PLAT-CONC-062 | MUST | Typing and similar user input handlers run on the UI thread, commit the document synchronously first (PLAT-VFS-112), and never start long work. | Typing in a 10k-line file stays under the frame budget. |
| PLAT-CONC-063 | MUST | Any feature that starts as a result of an edit **cancels** the previous run for the same (feature, file) (coalescing) and discards results from a superseded run. | Type quickly: only the last highlighting result is applied. |

## 9. Mapping to the code-intelligence spec's snapshot model (D4, section 1.11)

The code-intelligence spec replaces locks with: a UI-thread-owned mutable `AnalysisHost`, immutable `Snapshot`s for background work, a `CancelToken`,
and `Result<_, Cancelled>`. That design **can** satisfy this spec; it changes *how*, not *what*. The following promises of the reference must still hold,
and each has a testable requirement above:

| Reference promise | Snapshot-model equivalent | Requirement |
|---|---|---|
| No torn reads (a reader never sees a half-applied change) | A snapshot is immutable and complete | PLAT-CONC-001, 024 |
| Pending write cancels readers | A new revision cancels jobs registered against the old revision (or marks their results stale) | PLAT-CONC-002, 023 |
| Re-run on cancel; idempotence | A job is a pure function of (snapshot, input) | PLAT-CONC-031 |
| Finish step sees no intervening write | Delivery step checks the revision/stamps of the inputs and **discards or range-maps** if changed | PLAT-CONC-032, 037 |
| Smart/committed constraints | The snapshot carries `is_dumb` and `committed_revision`; a job constrained to smart/committed is only started from a qualifying snapshot and discarded if the flag flips | PLAT-CONC-033 |
| Coalescing | Spawning with an equal key cancels the earlier job | PLAT-CONC-034 |
| Cancellation is not an error | `Cancelled` is a distinct variant | PLAT-CONC-020, 021 |
| Elements valid only within one read section | A snapshot-bound handle is valid for the snapshot's lifetime; **holding handles across revisions requires a smart-pointer/anchor** | PLAT-PSI-074, 075 |

Differences that the snapshot model must still handle explicitly:

* **Stale results by design**: because readers do not block writers, a result may be computed for revision N while the user is at N+3.
  The delivery rules (PLAT-CONC-032) must be enforced centrally, not per feature.
* **The write path still needs ordering**: PSI-modifying operations (quick fixes, refactorings) run as *read-then-write* (PLAT-CONC-037) against the
  latest revision with an explicit conflict check.
* **Blocking cost moves to snapshot creation**: creating a snapshot must be O(1) (cheap clones) or typing will stall. This is a budget in the earlier spec; keep it as a test.

## 10. Notes for implementers (non-binding)

* The most common failure in the reference ecosystem is swallowing cancellation. Make the error type `#[must_use]` and forbid `Cancelled -> String` conversions at the type level.
* Per-item idempotence (PLAT-CONC-050) is what lets parallel searches survive typing; design item processors as pure functions of (item, snapshot) and aggregate in the caller.
* "Wait for write to finish then resume" is not needed in a snapshot model; resume is a natural consequence of the job having its own immutable input. The *user-visible* effect (search continues while typing, no duplicates) must still be tested.

## 11. Open questions

1. Whether Archipel allows **background write sections** (PLAT-CONC-005) at all, or keeps all mutations on the UI thread with commit results applied there; the earlier spec chose the latter.
2. The 256-iteration / 1 ms cancellation granularity (PLAT-CONC-015) is an Archipel budget, not a reference fact; calibrate with profiling.
3. How to represent modality (PLAT-CONC-007) in an immediate-mode UI where there are no nested event loops; probably as an explicit "dialog layer id" on scheduled callbacks.
