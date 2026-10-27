# 01. The highlighting daemon (`DaemonCodeAnalyzer`)

Status: specification, language independent. Requirement prefix `CI-DMN`.
Source basis: IntelliJ Community tag `idea/262.10968.63`. Paths are cited as `ic: ...`.
Priorities: `MUST` (needed for the Java/Kotlin/Spring MVP), `SHOULD` (expected by a user
coming from IntelliJ), `LATER` (not needed before the MVP is usable).

## 1. Purpose and mental model

The daemon is a per-project service that keeps the markup of every *visible editor* up to date
with the results of background analysis. It never edits text. It produces **HighlightInfo**
records (see `02-highlight-info.md`) and attaches them to the document as range markers, so they
move with edits until the next analysis replaces them.

Three ideas carry the whole design:

1. **Dirty scopes, not whole-file reruns.** For every open document and every pass, the daemon
   remembers which part of the text still needs analysis (the *dirty scope*). An edit marks only
   a small scope dirty when the language can say that the change is local, otherwise the whole
   file.
2. **A graph of passes.** Analysis is split into independent *passes*. Each pass has a stable
   numeric id and declares which other passes it must wait for. Passes run in the background;
   their results are applied to the editor in small steps, each on the UI thread.
3. **Cancel fast, reuse a lot.** Typing, a write action or a settings change cancels everything
   in flight and restarts after a short quiet period. Old results stay on screen, shifted with
   the edits, until a new result replaces them. Identical results are recycled rather than
   removed and re-added, so nothing flickers.

ic: `platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/DaemonCodeAnalyzerImpl.java`,
`.../PassExecutorService.java`, `platform/analysis-impl/.../daemon/impl/FileStatusMap.java`.

## 2. Vocabulary

| Term | Meaning |
|---|---|
| Pass | A unit of analysis with an id, a display name (shown in the status widget), a collect step (background, may be cancelled) and an apply step (UI thread, short). |
| Dirty scope | Per document and per pass: none (clean), a text range, or the whole file. Stored as a range marker so it follows edits. |
| Visible range | From the first fully visible line below the sticky-lines panel to the end of the last visible line. |
| Priority range | The range a pass must finish first. For the main passes this is the visible range. |
| Daemon progress | One cancellable progress object per open editor for the current run. Cancelling it stops all passes of that editor. |
| Main passes | General highlighting and local inspections: what "Inspect on the fly" means. Used by headless callers that need "all errors of a file now". |
| Dumb mode | Indexing is running. Only passes and tools that declare themselves usable in dumb mode run. |

## 3. Pass registry and order

### 3.1 Registration contract

- A pass factory is registered through an extension point (`highlightingPassFactory`). The
  factory can create a pass for an (editor, file) pair, or decline (return nothing).
- Registration gives the pass an id. Core passes use fixed ids; plugin passes get the next free
  id above the highest core id.
- Registration states two dependency lists. They are different relations:
  - **runs after completion of** P: this pass starts only when P has finished *and its result has
    been applied to the editor*.
  - **runs after start of** P: this pass starts as soon as P has been started (so they run in
    parallel, but P is guaranteed to get its slot first).
  - The same id must not be in both lists, and a pass cannot depend on itself.
  - Unknown ids that are not registered are ignored.
- A flag "needs an additional intentions pass" asks the daemon to schedule the intentions pass
  after this pass too (used by local inspections and external annotators).
- In dumb mode a factory is asked to create a pass only if the factory is dumb-aware; a created
  pass is dropped again if the pass itself is not dumb-aware. A factory that declines marks its
  pass clean for that file so the daemon does not restart endlessly.
- If highlighting is switched off for the file (see 7) the daemon creates one empty pass, so
  that "nothing to do" is distinguishable from "creation failed".

### 3.2 Core passes (fixed ids) and their dependencies

| Id | Pass | Dependency | Dumb-aware | Produces |
|---|---|---|---|---|
| 1 | Code folding update | none | yes | fold regions (out of scope here) |
| 4 | **General highlighting** ("UPDATE_ALL") | after start of 1 | yes | syntax errors, `HighlightVisitor` results, `Annotator` results; the "error found" flag |
| 7 | **Local inspections** | after start of 4 | only if the tool is dumb-aware | inspection problems as HighlightInfo |
| 8 | **External annotators** | after completion of 4 | only if the annotator is dumb-aware | results of slow linters and language servers |
| 9 | Problem-file tracking ("wolf") | after completion of 4, after start of 7 | yes | which files contain errors (feeds the project tree and the "files with problems" notion) |
| 11 | **Line markers** (fast) | after start of 4 | yes | gutter icons that need only cheap data |
| 13 | Slow line markers | after completion of 4 | no | gutter icons that need searches (overridden, implemented, ...) |
| 14 | Injected-language general highlighting | after start of 4 | yes | highlighting inside injected fragments |
| 3 | **Show intentions** ("POPUP_HINTS") | after completion of 4 | yes | recomputes the Alt+Enter cache and the light bulb |
| dyn. | **Show auto-import** | after completion of 4 and 7 | no | import hints and silent imports (see `05-auto-import.md`) |
| dyn. | Inlay hints, code vision, sticky lines, indent guides, doc rendering | none, or relative to 1 | mostly yes | decorations (specified elsewhere) |

`CI-DMN-001` MUST: the pass registry is an extension point with the two dependency relations
of 3.1 (after start, after completion). Acceptance: a test registers pass B "after completion of
A"; B never starts before A's apply step ran; with "after start of A", B starts while A is still
collecting.

`CI-DMN-002` MUST: the core passes exist with the dependency edges of the table above (folding
may be a no-op). Acceptance: dumping the pass graph for an open Java file yields exactly these
edges.

`CI-DMN-003` MUST: General highlighting and local inspections run in parallel (inspections start
when general highlighting starts), external annotators and slow line markers wait for general
highlighting to complete. Acceptance: with an artificially slow annotator, inspection problems
appear before the annotator finishes; external-annotator results never appear before general
highlighting is applied.

`CI-DMN-004` MUST: the *show intentions* pass runs after general highlighting and also after
every pass flagged "needs additional intentions pass" (local inspections, external annotators).
Acceptance: after an inspection with a quick fix reports at the caret line, the light bulb
appears without any caret movement.

`CI-DMN-005` MUST: *show auto-import* runs after both general highlighting and local
inspections have completed and applied. Acceptance: an unresolved reference reported by general
highlighting gets its import hint only after local inspections are applied.

`CI-DMN-006` MUST: every pass has a stable id and a short display name. Acceptance: the status
widget (section 9) lists running passes by these names ("Inspections", "Checking red files",
and so on).

`CI-DMN-007` SHOULD: plugin passes can be registered at runtime; adding or removing a pass
extension restarts the daemon for all files. Acceptance: loading a plugin that adds a pass
causes all open editors to re-highlight.

### 3.3 What is not a pass any more: caret-driven highlighting

Identifier highlighting ("highlight usages of the symbol at the caret"), brace matching and
"highlight occurrences of the selection" are **not** passes of the graph. A separate service
listens to caret and selection changes, computes the ranges in the background, and applies them
as markup with a dedicated severity above ERROR (so they paint on top). Details in
`07-navigation.md`. They are cancelled by the next caret move and are independent of daemon
restarts.

`CI-DMN-008` MUST: caret-driven highlights are computed off the UI thread, cancelled by the next
caret move or document change, and never delay or restart the analysis passes. Acceptance:
holding the Down arrow key in a 5000-line file does not trigger analysis restarts (the daemon
restarts only after edits or settings changes; see 5.2 for the caret rule).

## 4. Scheduling

### 4.1 One run

A **run** is the daemon analysing all *active editors* of the project (the editors that are
showing, the selected one of each split). For each editor:

1. Create an editor-bound daemon progress.
2. Ask the registry for passes (3.1). Passes whose dirty scope is empty produce an empty pass
   (nothing to do, marked clean).
3. Submit the passes in dependency order. Passes without predecessors start at once on a pool
   of background threads. When a pass's collect step finishes, its apply step is queued on the
   UI thread; the "after completion" successors are released only after that apply step.
4. When all passes of all editors have applied, fire the *daemon finished* event.

`CI-DMN-010` MUST: the daemon only analyses editors that are actually showing, plus the selected
tab of each editor group. Closed or background tabs are not analysed. Acceptance: opening 20
files via a search does not start 20 analyses; switching to a tab starts its analysis.

`CI-DMN-011` MUST: every pass follows the two-phase contract: collect (background, read access,
cancellable, must not mutate anything) then apply (UI thread, short). Acceptance: no analysis
code runs on the UI thread longer than one frame budget; cancelling during collect leaves the
markup unchanged.

`CI-DMN-012` MUST: the daemon publishes the lifecycle events *starting* (with the list of
editors), *finished* (with the list), and *cancelled* (with a human-readable reason string).
Other features (status widget, tests, "wait until highlighted") subscribe to them.
Acceptance: a subscriber receives `finished` exactly once per uninterrupted run.

### 4.2 Visible-range-first

General highlighting and local inspections process the file in two stages **within one pass**:
the PSI/syntax elements whose text lies in the priority range (visible range) are processed
first, then the rest of the dirty scope. Results are applied to the markup after the first stage
so the user sees red code on screen long before the file ends. Line markers use the visible
range rounded out to whole lines as their priority range.

`CI-DMN-020` MUST: for a dirty scope larger than the visible range, results for the visible
range are applied before the rest is processed. Acceptance: in a 20k-line file with one error
on screen and one 15k lines below, the first error is applied first (observable through the
order of apply events), and the pass's progress is below 100% at that moment.

`CI-DMN-021` MUST: elements that are parents of both ranges (the file, a class) are processed
last, after both stages, so a class-level problem does not delay the first stage.
Acceptance: a class-level "does not implement method" error appears after method-body errors on
screen.

`CI-DMN-022` SHOULD: the visible range is re-read at the start of each run; scrolling does not
cancel a run but the next run uses the new range, and scrolling to code that was analysed while
off-screen shows existing results immediately. Acceptance: after scrolling down in a fully
analysed file nothing is recomputed.

### 4.3 Dirty scopes and what an edit invalidates

When a document change is committed to the syntax tree, the daemon computes the scope to mark
dirty:

1. Start from the changed element (or its parent when the change was whitespace-optimised, see
   rule 3).
2. Ask the language's *change locality* hook whether this element (or an ancestor) is a scope
   that can be analysed alone (a method body, a function body, a statement in a script). The
   first answer walking upward wins. If nothing answers, the whole file is dirty.
3. **Default locality rule** (applies to all languages): an edit confined to a whitespace token
   or to a comment is its own scope. Exceptions: a comment containing the suppression marker
   `noinspection` is not local, because it changes which problems are reported. The whitespace
   shortcut is disabled for a document once the user typed *inside* an existing highlighter
   range, because the range's text meaning may have changed.
4. If another scope is already dirty and the new scope does not contain it, the scopes are
   unioned rather than analysed separately (so code between two dirty places is not left
   half-highlighted).
5. Files that are not in the project content (excluded directories, project metadata files)
   never mark anything dirty.
6. Non-physical files (scratch buffers without a real file, code fragments) mark the whole file
   dirty.

Each pass has its own dirty scope; a pass that finishes marks only its own scope clean. A
"defensive" mark sets a whole-file dirty state for all passes without waiting for a tree event.
It is used when an edit might be reverted before the tree changes (fast type-and-backspace).

`CI-DMN-030` MUST: per-document, per-pass dirty scopes exist, are range markers, and are cleared
by the pass that finished them. Acceptance: editing the body of one Java method re-runs general
highlighting and inspections only over that method's range (measured by the elements visited).

`CI-DMN-031` MUST: the change-locality hook is a per-language extension; the default rule treats
whitespace and ordinary comments as local scopes. Acceptance: typing a space between two tokens
does not remove any highlight outside that whitespace and re-analyses only the whitespace scope.

`CI-DMN-032` MUST: a comment edit that adds or removes `noinspection` re-analyses the enclosing
declaration or statement scope (not only the comment). Acceptance: adding the suppression comment
above a flagged statement removes its warning within one daemon cycle.

`CI-DMN-033` MUST: a project-wide invalidation ("mark all files dirty") exists and is used for
the triggers marked "all" in 5.1.

### 4.4 Cancellation

- A cancelled pass throws away its partial collect results and applies nothing. Passes already
  applied keep their results.
- Cancellation points: before every element visited, and in every long loop of a collector.
  Collectors must call a "check cancelled" function.
- A pass that discovers that a write action is pending cancels the whole run and asks for a
  restart (so an edit is never blocked by analysis).
- A run is also cancelled when its editor stops being active, except during the very first
  highlighting of a just-opened editor (see 5.3).
- Cancelling must be cheap and must not wait for the pass to notice, except in tests.

`CI-DMN-040` MUST: any edit, write action, or listed restart trigger cancels all runs within one
frame, and the UI thread never waits for a running pass. Acceptance: with a pass that loops for
10 s checking cancellation, typing a character in the editor is not delayed.

`CI-DMN-041` MUST: when a collector finds a pending write action it cancels the run and the
daemon restarts it afterwards. Acceptance: a refactoring that starts a write action during
highlighting is not blocked, and highlighting resumes after it ends.

`CI-DMN-042` MUST: cancelled results are never applied. Acceptance: a unit test cancels a pass
between collect and apply; the markup is identical before and after.

### 4.5 Reuse of results and flicker avoidance

- Existing highlighters are **range markers**: they follow edits. A text change that touches the
  inside of a highlighter's range removes or recomputes it (typing inside an error underline
  makes it vanish at once unless the annotator says it "needs update on typing", in which case
  it stays until the next analysis).
- A new result for the same tool and the same element **recycles** the old highlighter when
  range, severity, message and attributes are equal. A new result for a *different* range
  creates a new one. Old highlighters for elements that were re-visited and produced nothing
  are removed. Highlighters whose PSI element became invalid are removed *last*, at the end of
  the pass, hoping that an equal result is produced for the re-created element (incremental
  parsers often recreate nodes).
- Highlighters of tools that no longer ran (tool disabled in the profile, tool no longer
  applicable) are removed at the end of the pass.
- In dumb mode, highlights of tools that are not dumb-aware are *kept* (not removed) so the
  editor does not lose all red code when indexing starts.
- A warning-level highlight that lies *inside* an error highlight is removed (so only the
  error shows) unless it is a "deprecated" or similar text-attribute-only highlight.
- After the last pass the daemon dedupes identical (range, severity, text) infos that two tools
  produced.

`CI-DMN-050` MUST: highlighters are stored with per-tool ownership (tool id: an inspection's
short name, an annotator class, a visitor class, an external tool id). A pass replaces exactly
its own tool's infos in its range. Acceptance: re-running one inspection does not touch another
inspection's highlights.

`CI-DMN-051` MUST: identical new/old results are recycled in place (same highlighter identity).
Acceptance: re-running analysis on an unchanged file produces zero add/remove events on the
markup.

`CI-DMN-052` MUST: results are shifted by edits until replaced. Acceptance: inserting a line
above an error moves its underline down; it is not removed during the next 300 ms.

`CI-DMN-053` MUST: in dumb mode, highlights of tools that did not run because they are not
dumb-aware are retained. Acceptance: start indexing; red squiggles of unresolved symbols stay.

`CI-DMN-054` SHOULD: a per-file and per-tool cap on the number of infos (the Archipel proposal
is 2000 per file and 1000 per tool) collapses the surplus into a single file-level
"too many problems" info. IntelliJ itself protects large files through the highlighting-level
switch to "essential"/"syntax only" (section 7) instead of a cap, so this is an Archipel
addition and must not change results for normal files.

`CI-DMN-055` MUST: unrelated tools never clear each other's highlights when one tool's result is
empty. Acceptance: disabling one inspection removes only its highlights after the next cycle.

## 5. Restart triggers and timing

### 5.1 Triggers

The daemon registers listeners and, on each, either (a) *stops and restarts for all files* ("all"),
(b) *stops and restarts for the affected document*, or (c) *stops without a restart* (the restart
comes later from another event).

| Trigger | Effect |
|---|---|
| Document about to change | stop, no restart (the tree commit that follows restarts) |
| Tree/PSI change committed | mark dirty scope (4.3), restart |
| Document bulk update starts / finishes | stop / restart |
| Write action starts / finishes | stop / restart (unless the tree commit will restart) |
| Typing (before an editor typing action) | stop, no restart |
| Any shortcut triggered | stop and restart (a command may change state) |
| Caret moves, documents committed | restart (only so the show-intentions and auto-import passes recompute for the new caret); skipped while uncommitted changes exist, to avoid cancelling twice per keystroke |
| Command finished after Esc pressed with analysis unfinished | restart (so Esc never leaves a document half-highlighted) |
| Project roots changed, libraries changed, SDK changed | all |
| Dumb mode entered / left | all |
| Inspection profile changed, activated or profiles initialised | all |
| Severity registry changed (custom severity, order) | all |
| Editor colour scheme changed | all (attributes are resolved at apply time) |
| File renamed / file type changed / file property changed | all (rename) or document |
| Power-save mode toggled | all |
| Todo patterns changed | all |
| Extension point change for annotators, inspections, passes, suppressors | all |
| Facet / module settings changed | all |
| A heavy background operation (e.g. a VCS operation, build sync) finished | restart (daemon pauses during "heavy" processes) |
| Modal dialog opened or closed | stop; restart when leaving a modal state |
| Fold region expanded by the user | restart after the current run finishes |
| Highlighting settings changed (`settingsChanged()`) | all |
| External-tool results arrive (LSP diagnostics published) | restart only the external-annotator pass for that file (Archipel addition, see 8) |

`CI-DMN-060` MUST: each trigger of the table exists with the stated effect. Acceptance: a test
per row calls the trigger and observes the cancel event with a reason string containing the
trigger's name.

`CI-DMN-061` MUST: every restart carries a human-readable reason (string or object), logged and
included in the *cancelled* event. Acceptance: the daemon log shows "Caret moved",
"Dumb mode started", and so on.

`CI-DMN-062` MUST: a restart request while a run is pending only moves the scheduled time; at
most one update is queued at any time. Acceptance: 1000 restart requests within 100 ms produce
one run.

### 5.2 Delay

- Setting `autoReparseDelay` (default **300 ms**, application level). It is edited in
  Settings | Editor | General, group "Error Highlighting", field "Autoreparse delay:" with the
  suffix "milliseconds". Restart requests schedule the next run after this delay. A stored
  value of 0 is treated as suspicious and reset to the default once.
- The delay is **not** applied for the *first* run after the daemon was idle. The delay becomes
  active only after a pass of an open file has been cancelled (typical: the user typed). It is
  switched off again when a run starts successfully for an open file. Effect: opening a file
  starts highlighting at once, typing waits 300 ms of quiet.
- A forced zero delay exists for tests and for explicit actions ("rehighlight now").
- Caret movement triggers a restart with the same rule, so the intentions pass tracks the caret
  after the delay without cancelling a finished analysis (a restart with all dirty scopes clean
  only reruns the passes that depend on the caret: show intentions and auto-import).

`CI-DMN-070` MUST: default restart delay is 300 ms and is configurable (0 to 3000 ms).
Acceptance: after the last keystroke, passes start 300 ± 50 ms later; the first highlighting of
a just-opened file starts without delay.

`CI-DMN-071` MUST: the delay applies only after a cancellation of an open file's pass.
Acceptance: opening a file, waiting for completion, then moving the caret restarts the caret
passes immediately; after typing, the restart waits the delay.

`CI-DMN-072` SHOULD: a restart with no dirty scope re-runs only passes that depend on the caret
(show intentions, auto-import). Acceptance: moving the caret in a fully analysed file does not
re-run inspections (check pass-start events).

### 5.3 First highlighting of a new editor

When a file is opened, its editor is the first active editor and the focus event itself would
normally cancel the daemon ("active editor changed"). The daemon keeps the unfinished first run
when the first run has zero delay and every running editor is still active; otherwise a stop with
restart would activate the 300 ms delay and postpone the first result.

`CI-DMN-073` SHOULD: active-editor change does not cancel an unfinished first pass of an editor
that stays active. Acceptance: open a file in a tab group; time to first highlight equals
analysis time, not analysis time plus 300 ms.

### 5.4 Pausing and disabling

- **Power-save mode** switches the daemon off for code analysis (status text below).
- **Heavy processes** (a VCS update, project sync, a large refactoring) pause analysis; the
  status widget shows "Paused…" and the process name.
- An editor-level "update by timer" switch exists (used by dialogs and headless tools).
- Per-file switches: highlighting disabled for a file (`setHighlightingEnabled`), import hints
  disabled for a file (`setImportHintsEnabled`), "highlighting available" false for binary
  files, decompiled/compiled files, and files whose PSI is not physical.

`CI-DMN-080` MUST: files for which `isHighlightingAvailable` is false (binary, compiled,
non-physical, explicitly disabled) are never analysed, and the status widget shows OFF with a
reason (see 9.3). Acceptance: opening a `.class` file or an image shows no analysis and the
reason text.

`CI-DMN-081` SHOULD: power-save mode and heavy processes pause the daemon and show it.
Acceptance: enabling power-save mode removes the analysis passes and shows the
"Code analysis is disabled in power save mode" status.

## 6. Annotation producers feeding the general and external passes

### 6.1 `HighlightVisitor` (language core)

A visitor is a language-specific object run by the *general highlighting* pass over each element
in the two-stage order of 4.2. It reports HighlightInfos for syntax errors, unresolved
references, type errors, unused symbols and similar. It is *not* an inspection: it has no entry
in the inspection profile, cannot be disabled one by one, and runs whenever the highlighting
level (section 7) is at least "syntax". For Java, the "Cannot resolve symbol", incompatible
types, missing return and unreported exception errors come from here.

Contract:
- Visitors are cloned per run (no shared mutable state between runs).
- A visitor says whether it is applicable to a file (language, level).
- A visitor may be dumb-aware.
- Syntax-error elements (parser error nodes) are reported by a default visitor that always
  exists: each error element becomes an ERROR highlight whose text is the parser's message.
  Empty error elements are drawn at the end of the previous line.

### 6.2 `Annotator` (per language, many)

- `annotate(element, holder)` is called for each element of the file (both stages), cheap, no
  index access in dumb mode unless declared dumb-aware.
- The holder builds annotations: severity, message, range (defaults to the element), tooltip,
  text attributes key or enforced attributes, problem highlight type, fixes (plain, lazily
  computed, or tied to an inspection key), "after end of line", "file level", gutter icon,
  problem group (for suppression submenus), "needs update on typing".
- A "silent" annotation has no message (used for pure colouring, e.g., colouring keywords).
- Annotator exceptions are caught and logged; one failing annotator never breaks the pass.

### 6.3 `ExternalAnnotator` (slow, optional)

Three phases:

1. **collect** (read access, fast, UI-safe): look at the file and editor, return whatever the
   tool needs (a command line, a copy of the text, a document version), or nothing to skip. The
   flag "file already has errors" is passed; by default an external annotator is skipped when
   the file has syntax or resolve errors found by the general pass.
2. **annotate** (no read lock, may run processes or talk to a server, long): returns a result.
3. **apply** (read access): turns the result into annotations through the holder.

Rules:
- Runs only after the general pass completed (3.2) and only if the "essential highlighting
  only" mode is off.
- Requests go through a merging queue with a **300 ms** quiet period so a fast typist does not
  start a process per keystroke.
- If the document changed between collect and apply, the result is discarded (not shifted).
- An annotator can be tied to a batch inspection (paired inspection): if that inspection is
  disabled in the profile, the annotator does not run. This is how "linter X" is switched off.
- Per-file filters can veto an annotator (extension point).
- A pass waits up to 60 s for the queued job in tests/headless callers.

`CI-DMN-090` MUST: `HighlightVisitor`, `Annotator` and `ExternalAnnotator` exist as three
separate extension points with the contracts above. Acceptance: three sample producers, one of
each kind, show up at different pass milestones (visitor and annotator in general pass; external
after it).

`CI-DMN-091` MUST: an annotator exception is logged and does not cancel the pass or other
annotators. Acceptance: an annotator that throws on every element leaves other highlights.

`CI-DMN-092` MUST: external-annotator results are discarded when the document changed after
collect. Acceptance: type while a slow annotator runs; its result does not appear, and a new
run starts.

`CI-DMN-093` MUST: external annotators are skipped by default when the file has errors from the
general pass. Acceptance: a Java file with a syntax error shows no javac-lint style warnings.

`CI-DMN-094` SHOULD: a quiet period of 300 ms merges requests. Acceptance: 10 edits in 200 ms
start one external tool run.

`CI-DMN-095` MUST: dumb-mode behaviour: collectors that are not dumb-aware are skipped and their
highlights retained (4.5). Acceptance: see CI-DMN-053.

### 6.4 Where an LSP client fits (Archipel)

Diagnostics from a language server are an **external annotator**: collect = "which file and
version", annotate = wait for or read the latest published diagnostics for that version, apply =
convert to HighlightInfo with tool id `lsp:<server>`. They are deduplicated against native
results (see `docs/analysis/code-intelligence-spec.md` 1.10), and a code action list for the
same range becomes Alt+Enter rows (see `04-intentions-and-alt-enter.md` CI-ALT-060).
Pushed diagnostics trigger a restart of the external-annotator pass only.

`CI-DMN-096` MUST: LSP diagnostics enter the daemon through the external-annotator contract,
with the same stale-result rule. Acceptance: diagnostics for an older document version are
dropped.

## 7. Highlighting level per file

The user (or the platform for big or generated files) can lower the analysis per file. Levels
(user-facing names in the widget and the hector popup): **None**, **Syntax**, **Inspections**
(= all, the default), and an internal **Essential** level.

| Level | Behaviour |
|---|---|
| None (skip highlighting) | No passes except cheap lexical colouring. Status OFF, reason "Highlighting level is none" (UI text: "Highlighting disabled"). |
| Syntax (skip inspections) | General pass runs (syntax errors, visitors, annotators). Local inspections and external annotators do not. |
| Essential | General pass runs. Inspections and external annotators run only on "Save All" (explicit save). Status "Essential analysis completed. Only essential highlighting results shown. Press Ctrl-S for all". |
| Inspections (force highlighting) | Everything runs. |

The level is per file *and per language root* of a multi-language file (for example HTML with
embedded script). The effective level of the file is the minimum over its roots. Changing the
level restarts the daemon and re-creates the traffic-light renderer.

`CI-DMN-100` MUST: the three user levels None, Syntax, Inspections exist per file, stored in
workspace (not shared) state, switchable from the status widget popup ("Highlight level:",
"In the current file:") and from a hector-style popup titled "Highlighting Level" with slider
labels "None", "Syntax", "Inspections". Acceptance: choose Syntax; inspection warnings
disappear, syntax errors stay.

`CI-DMN-101` SHOULD: the Essential level exists for huge files and is chosen automatically for
files above a size threshold; status text as above. Acceptance: a 5 MB generated file opens
with the "Essential analysis completed" status.

## 8. Differences and additions relative to the existing Archipel documents

These are deliberate corrections; the implementation team should follow this file.

1. `code-intelligence-spec.md` 1.7 rule 3 describes "each pass is run twice (visible, then
   rest)". IntelliJ runs one pass and processes the visible elements first inside it, applying
   results in stages (4.2). Observable behaviour is the same; the single-pass model avoids
   running tool setup twice.
2. 1.7 and `daemon.md` model dependencies as one relation (`Pass::after`). IntelliJ has two
   (after start and after completion, 3.1). The difference is why inspections start before
   general highlighting ends.
3. 1.7 rule 2 says "debounce non-syntax passes by 300 ms". Refine: the delay applies only after
   a cancellation of an open file's pass; the first run is immediate (5.2). Archipel's "Syntax
   runs inline, no debounce" matches the IntelliJ lexer-level colouring, which is not part of
   the daemon at all.
4. `daemon.md` lists `IdentifierHighlighter` as a pass. In current IntelliJ it is a
   caret-driven service outside the pass graph (3.3). Implementing it as a pass is acceptable
   only if it keeps the cancel and caret semantics of CI-DMN-008.
5. Severity numbers in 1.7 are correct for the public severities, but the daemon also uses
   internal severities (see `02-highlight-info.md` section 3). Do not leak them into profiles.
6. `daemon.md` budgets (1000 per tool, 2000 per file) are an Archipel invention (CI-DMN-054).
7. `daemon.md` suppression: "trailing on its first line" is not an IntelliJ form; see
   `03-inspections.md` section 7.

## 9. Analysis status: traffic light and inspection widget

IntelliJ calls the editor's top-right indicator the *inspection widget* (new UI) or *traffic
light* (classic). It shows the analysis state and counts, and gives access to navigation and
the highlighting level.

### 9.1 Inputs

- Error counts per severity, maintained incrementally by listening to the file's markup model:
  each added highlighter with severity above TEXT_ATTRIBUTES increments the count of its
  severity; each removed one decrements it. Severities are indexed by the severity registry
  order. Counts are per file (and per code-insight context).
- The set of passes currently running, with a name and a progress fraction each (0..1).
- Whether *all* analysis is finished for the file: the document stamp equals the tree stamp, and
  the dirty scope of the general pass is empty (`isErrorAnalyzingFinished`).
- Global modifiers: power-save mode, dumb mode, heavy process running, daemon paused, level.

### 9.2 Decision table (first matching row wins; evaluated off the UI thread, cached)

| # | Condition | Icon family | Title (tooltip headline) | Details line | Text next to icon |
|---|---|---|---|---|---|
| 1 | Power-save mode | "power save" | Code analysis is disabled in power save mode | empty | OFF state |
| 2 | no file / project disposed / highlighting unavailable / level None | "traffic off" | No analysis has been performed | the reason (see 9.3) | `OFF` |
| 3 | heavy process other than sync is running | "pause" | Code analysis has been suspended | the process display name | the process type, or `Paused…` |
| 4 | daemon paused (update by timer off) | "pause" | Code analysis has been suspended | Highlighting is paused temporarily | `Paused…` |
| 5 | analysis finished, dumb mode | "pause" | Shallow analysis completed | Complete results will be available after project analysis is completed | `Indexing…` (or `Analyzing…` when the registry flag asks) |
| 6 | analysis finished, level Essential | OK | Essential analysis completed | Only essential highlighting results shown. Press Ctrl-S for all | counts |
| 7 | analysis finished, counts all zero | OK (outlined if level is not Inspections) | No problems found | empty | none |
| 8 | analysis finished, some counts > 0 | icon of the worst severity | empty (counts speak) | empty | counts |
| 9 | analysis running, some counts > 0 | icon of the worst severity | Performing code analysis | pass list with percentages | counts, "partial" state |
| 10 | analysis running, no counts | "eye" | No problems found | pass list | `Analyzing…` |

Counts are displayed as one chip per severity that has at least one problem, in descending
severity order, each with the severity's icon, the number, and the severity's count message
(for example "2 errors"). Severity count messages come from the severity definition
(`HighlightSeverity`: error, warning, weak warning, ... singular and plural).

`CI-DMN-110` MUST: the status indicator exists in the top right of every editor with at least
the states of rows 2, 7, 8, 9, 10 (OFF, no problems, counts, in progress with counts, analysing).
Acceptance: opening a clean Java file shows "analysing" then the "no problems" check; adding an
error shows one red chip "1" and, on hover, "1 error".

`CI-DMN-111` MUST: counts are updated incrementally from markup add/remove events, and the
status is recomputed when the daemon starts, finishes or is cancelled. Acceptance: fixing the
last error updates the widget in the same cycle as the underline disappears.

`CI-DMN-112` MUST: while analysis is in progress, the hover tooltip lists the running passes by
name with a percentage. Acceptance: with an artificially slow annotator named "My tool", the
list shows "My tool 40%".

`CI-DMN-113` MUST: the indicator is never "finished" while the document stamp differs from the
tree stamp or the general pass's dirty scope is non-empty. Acceptance: type a character; the
widget goes back to analysing at the moment of the edit.

`CI-DMN-114` SHOULD: rows 1, 3, 4, 5, 6 as in the table. Acceptance per row by simulating the
condition.

`CI-DMN-115` MUST: clicking the indicator, or invoking the navigation actions, works as
specified in 9.4. Acceptance: see below.

### 9.3 Reasons shown for OFF

"No file", "Project is disposed", "File is generated" (non-physical), "File is decompiled",
"File is binary", "Highlighting is disabled for this file", "Highlighting level is none" (the
title in the widget popup is "Highlighting disabled").

### 9.4 Widget popup and navigation

- The popup has a header "Highlight level:", a section "In the current file:" with the level
  choices (None, Syntax, Inspections), a "Configure inspections…" link, and a "Show:" section
  with "Inlay Hints" toggles.
- A cog button has the tooltip "Configure highlighting level".
- The widget's expanded row shows "{Severity} in File" and the shortcuts: "Navigate to
  next/previous: F2 / Shift+F2", "Keyboard shortcuts navigate through all types of problems.
  Next/Previous: F2 / Shift+F2"; a link "Show all problems" opens the Problems tool window on the
  current file.
- **Next / Previous Highlighted Error** (F2 / Shift+F2). Severities are indexed in registry
  order: 0 information, 1 text attributes, 2 server problem, 3 info, 4 weak warning, 5 warning,
  6 error (custom severities are inserted by order). Navigation considers indexes from 2 upward
  (the "shown" severities). The setting "The 'Next Error' action goes through:" (Settings |
  Editor | General | Error Highlighting) has two values: "The problems with the highest priority"
  (default) and "All problems". In the default mode the search first looks for the highest
  severity only (the next error after the caret, wrapping around the file), and only if the file
  has none, repeats with the next lower severity, down to index 2. In "All problems" mode it
  walks all shown severities together in text order. Purely informational infos (index 0) are
  never navigation targets. When several infos start at one offset the one with the highest
  severity is chosen. After navigating, the caret goes to the info's navigation offset, the
  selection is cleared, the target is scrolled to the centre of the viewport (centre-down when
  moving forward, centre-up when moving backward) and the error tooltip is shown at the caret.
  If nothing is found: when analysis has finished, an information hint "No errors found in this
  file"; otherwise a hint "Error analysis is in progress..." that disappears on any key, edit
  or scroll, and, when the daemon finishes while the hint is visible, the search is repeated.

`CI-DMN-120` MUST: F2 and Shift+F2 navigate across highlight infos of severity "server problem"
and above using the two modes and the two hint messages above, default mode "highest priority
first", with wraparound. Acceptance: file with a warning at line 5 and an error at line 50,
caret on line 1; F2 goes to the error at line 50 (not the warning); F2 again stays on that
error (it is the only one; wrap-around lands on it); with the mode "All problems", F2 from
line 1 goes to line 5, then line 50.

`CI-DMN-121` MUST: the editor has a vertical *error stripe* next to the scrollbar showing one
mark per highlight with colour from the info's severity (details in `02-highlight-info.md`
section 8). Clicking a mark moves the caret to it and shows its tooltip.

`CI-DMN-122` SHOULD: the widget popup of 9.4. Acceptance: choose Syntax from the popup; passes
restart with the new level.

## 10. Line markers (gutter), at the framework level

- A **line marker provider** is a per-language extension: for a given element it returns a
  marker (icon, tooltip, click action, optional navigation targets) or nothing. Two kinds:
  *fast* providers run in the fast line-marker pass; *slow* providers (they search for
  inheritors or usages) run in the slow pass after general highlighting. Both only process
  elements in the dirty scope, and expand ranges to whole lines.
- A marker for an element is replaced by the next run's marker of the same provider.
- Markers can be globally or per-provider disabled in Settings | Editor | General | Gutter Icons.
- A dumb-mode-unaware provider is skipped while indexing.

`CI-DMN-130` MUST: the two line-marker passes (fast, slow) exist with the dependencies of 3.2
and a provider extension point for each. Acceptance: an "overrides method" marker appears after
general highlighting even when the provider needs an index search.

`CI-DMN-131` SHOULD: markers are replaced per provider and per line; disabling a provider
removes its markers in the next cycle.

## 11. Observability and tests (Archipel requirement)

`CI-DMN-140` MUST: the daemon exposes for tests and the scenario harness: "run until finished
for this file", "all HighlightInfos of this file with severity, range, message, tool id, fix
texts", "pass graph", and cancel reasons. Acceptance: the scenario corpus can assert
"`assertThat` is red with message *Cannot resolve method 'assertThat'*" without sleeping.

`CI-DMN-141` SHOULD: a debug panel shows per-pass timings and cancel counts, and CI asserts
budgets: first result for the visible range of a 5k-line file within 300 ms of the end of
parsing; cancel latency below one frame.

## 12. UX acceptance scenarios (daemon level)

**S-DMN-1, first open.** Open `OrderService.java` (Spring Boot project, indexed). Expect: no
delay before analysis; red wavy underline on unresolved symbols appears in the visible range
first; the widget goes from "analysing" to a chip with the number of errors; line markers for
`@Autowired`/overrides appear after the main passes.

**S-DMN-2, typing.** Type `x` inside a method body. Expected timeline: t=0 stop, old highlights
stay (shifted); t=300 ms passes restart for the method body scope only; widget flips to
"analysing" at t=0 and back after the pass applies; no other method's highlights flicker
(zero add/remove events outside the method).

**S-DMN-3, caret only.** Move the caret with the arrow keys over an error. No analysis pass
restarts for the file (pass-start counters unchanged for passes 4 and 7); the light bulb and
import hint recompute; identifier highlighting updates.

**S-DMN-4, indexing.** Start Gradle sync; indexing begins. Expect: non-dumb-aware inspections
stop running; their old warnings stay; widget shows "Shallow analysis completed" with
"Indexing…"; when indexing ends, a full restart runs and warnings refresh.

**S-DMN-5, slow external annotator.** A linter takes 2 s. Expect: native errors are visible at
once; linter results appear after about 2.3 s (300 ms queue plus 2 s); typing during the run
discards the stale result and restarts.

**S-DMN-6, Syntax level.** Switch the file to Syntax via the widget. Expect: all inspection
warnings (unused import, etc.) disappear within one cycle; unresolved-symbol errors (general
pass) stay; widget shows the outlined "OK" icon when clean.
