# 08. Optimize imports framework and reformat hooks

Status: specification, framework level. Requirement prefix `CI-OPT`.
Priorities: `MUST` / `SHOULD` / `LATER`.

The formatter engine and code style model are specified in the editing/formatting work package
(`docs/analysis/code-intelligence-spec.md` 2.16); the Java import layout and star-import
thresholds belong to the Java spec. This file covers the framework that *invokes* them: the
import optimizer contract, the explicit and on-the-fly optimize flows, the processors that
combine reformat, optimize imports, rearrange and cleanup, "actions on save", and the editor hooks
that reformat as you type, paste and save.

ic: `platform/lang-api/.../lang/ImportOptimizer.java`,
`platform/lang-impl/src/com/intellij/codeInsight/actions/{AbstractLayoutCodeProcessor.kt,
OptimizeImportsProcessor.java,OptimizeImportsAction.java,ReformatCodeAction.java,
ReformatCodeProcessor.java,LayoutCodeInfoCollector.java}`,
`platform/analysis-impl/src/com/intellij/codeInsight/CodeInsightSettings.java`,
`platform/analysis-impl/.../codeInsight/daemon/ReferenceImporter.java`,
`java/java-analysis-impl/.../daemon/impl/analysis/{UnusedImportsVisitor,OptimizeImportRestarter}.java`,
`java/java-impl/.../intention/impl/config/QuickFixFactoryImpl.java` (`OptimizeImportsFix`,
`timeToOptimizeImports`), `java/java-impl/src/com/intellij/lang/java/JavaImportOptimizer.java`,
`java/java-frontback-impl/.../psi/codeStyle/JavaCodeStyleSettings.java`.

## 1. The `ImportOptimizer` contract

A language registers one or more *import optimizers* (extension, per language). The platform
calls, for a file:

1. `supports(file)`: may this optimizer process this PSI file (language root of the file's view
   provider)? Called for every root of a multi-language file; each supporting optimizer runs.
2. `process(file)` -> a *prepared change* (a task): the optimizer does all analysis here, in a
   read action, possibly slow and cancellable, and returns a small task that only *applies* the
   edit. The task runs in a write action on the UI thread; it must be quick.
3. Optionally the returned task can report a **user notification** text describing what it did
   (see 3.4).
4. Optionally the optimizer declares its **action mode**: `write command action` (default: the
   platform opens a write command around the task) or `UI thread` (the platform opens only the
   command; the task enters a write action itself and can show a cancellable progress). The second
   mode exists for optimizers that may resolve for a long time while adding unambiguous imports.
5. Optimizers are skipped in dumb mode (no change, no error).

The prepared change must be **idempotent** (running the optimizer twice changes nothing the
second time) and **semantics-preserving** (compiled meaning unchanged).

`CI-OPT-001` MUST: the optimizer extension with `supports`, two-phase `process` (analysis in read
action, apply in write action), notification text, and per-root application. Acceptance: a test
language registers an optimizer; the action calls it once per supporting root.

`CI-OPT-002` MUST: idempotence is testable: a conformance test helper runs the optimizer twice on
a fixture and asserts the second run produces an empty change. Acceptance: Java and Kotlin
optimizers pass.

## 2. What an optimizer does (language-independent rules)

An optimizer for a language with import statements performs, in this order:

1. **Remove unused imports**: imports whose names are not referenced in the file. An import that
   *cannot be resolved* is never removed (the library may simply not be on the classpath yet),
   and an import is removed only if resolve data for the whole file is reliable (no resolve in
   dumb mode).
2. **Add unambiguous imports** (explicit action only, and only when the language's setting `Add
   unambiguous imports on the fly` is on): for each unresolved reference in the file ask the
   language's reference importer; add the import when there is exactly one candidate (see
   `05-auto-import.md`). All of these are applied before sorting so the new imports are sorted
   too.
3. **Collapse and expand**: replace several single-class imports of one package by an
   on-demand import (`pkg.*`) when their count reaches the language's threshold (Java: **5**
   classes, **3** static members of one class), expand on-demand imports whose package is not
   in the "always use on-demand" list when only few names are used; never collapse if it would
   introduce an ambiguity or hide a name (a `*` import that would bring in a competing class of
   the same short name is not allowed).
4. **Sort and group** the remaining imports according to the language's *import layout* (an
   ordered table of entries: a package prefix with or without "with subpackages", a *blank line*
   entry, *all other imports*, *all other static imports*, and for Java *module imports*) and
   insert blank lines where the layout says. Within a group the order is alphabetical (case
   sensitive, package segments compared segment-wise). Default Java layout: `import all other
   imports`, blank line, `import javax.*`, `import java.*`, blank line, `import all other static
   imports` (module imports first when present).
5. Keep the position of the import block, leading comments of the file and header comments; keep
   comments attached to an import with that import where the language has such comments.
6. Do not touch the code outside the import block (except the collapse in step 3 which changes only
   the imports themselves).

Java examples (informative, see the Java spec): `import java.util.*` collapses when 5 or more
classes of `java.util` are used; `java.awt` and `javax.swing` are in the default list of
packages that use on-demand imports. A static import of a member is only kept if the member name is
used unqualified.

`CI-OPT-010` MUST: steps 1, 4, 5, 6 for Java and Kotlin; step 3 for Java (Kotlin has its own
collapsing rule). Acceptance: S-OPT-1, S-OPT-2.

`CI-OPT-011` MUST: unresolved imports are never removed. Acceptance: file with `import
com.missing.Foo;` unused-or-not: optimize keeps it.

`CI-OPT-012` MUST: the import layout is read from the code style (per project, per language) with
the Java default above; changing it changes the result. Acceptance: S-OPT-2.

`CI-OPT-013` SHOULD: step 2 (add unambiguous) in the explicit action when the setting is on.
Acceptance: S-OPT-3.

## 3. Explicit `Optimize Imports` (`Ctrl+Alt+O`)

### 3.1 Entry points and scopes

- In an editor: the current file (injected fragments map to the host).
- From the project tree or file chooser: the selected files (no dialog) or a directory: a dialog
  `Optimize Imports` with the scope text `Directory '<name>'`, the option `Include subdirectories`
  and, when VCS is enabled and the directory has changes, `Only process files with changes
  uncommitted to VCS`; project or module context: the scope text `Project '<path>'` / `Module
  '<name>'` with the VCS option.
- Not available when no language of the file supports it (the action is disabled).

### 3.2 Execution

1. Commit documents.
2. Run in the **background with a progress** `Optimizing imports…` for multi-file scopes; for a
   single editor file it runs without a visible progress unless slow.
3. For each file: take its optimizers (section 1), run `process` in a read action (non-blocking),
   then apply the returned tasks in a **single write command** named `Optimize Imports`. In a
   multi-file run the files can be undone together (one undo group) or one by one (a setting;
   default: **one group for all files of the run** when processed from the tree).
4. Skip: files not writable (a prompt to make writable once per run), non-physical files, files
   outside the project content unless they are scratches.
5. The caret and scroll position of the open editor are preserved.
6. After a single-file run the **result hint** appears at the caret (setting `Show notification
   after Optimize Imports action`, on by default): the first non-empty text of the optimizers'
   notifications, else `Imports optimized` if anything changed, else `No unused imports found`.
   Java notification texts: `Removed N import(s)` (singular `Removed 1 import`), followed by
   `, added M import(s)` if imports were added in step 2 of section 2; `Rearranged imports` when
   nothing was removed but the order changed.

`CI-OPT-020` MUST: single-file explicit optimize with one undo step named `Optimize Imports`, the
result hint of 3.2.6 and caret preservation. Acceptance: S-OPT-1.

`CI-OPT-021` SHOULD: directory/module/project scopes with the dialog texts and the VCS-changed
option, background progress and one undo group. Acceptance: S-OPT-4.

`CI-OPT-022` MUST: read-only files are not changed silently; a prompt appears once. Acceptance:
with a read-only file in the selection the run asks to make it writable or skips it.

## 4. Optimize imports on the fly

Setting (project level, default **off**): `Optimize imports on the fly` (Settings | Editor |
General | Auto Import). It is also offered as the LOW-priority quick fix `Enable 'Settings | Editor |
General | Auto Import | Optimize imports on the fly'` on the unused-import problem while it is off.

### 4.1 Trigger

At the end of a general-highlighting run on a file that has unused imports (or, when the "unsorted
imports" inspection is enabled, imports out of order), the visitor *requests* an optimize for that
file. The request records the document's modification stamp. When the daemon **finishes** (all
passes applied) the request is evaluated:

1. If the file text changed since the request, or any syntax-tree change happened, drop it (the
   daemon will restart and ask again).
2. If the file is not writable, drop it.
3. If analysis is not completely finished for the file, re-queue until it is.
4. Evaluate *time to optimize* (all must hold):
   - the project setting is on;
   - highlighting is available for the file and the language supports on-the-fly optimizing in this
     file kind (not in template-generated files that forbid it);
   - the file has **no errors** except (a) errors reported by inspections and (b) unresolved
     reference errors located inside the import list (so a file that is mid-edit and broken never
     loses imports);
   - the file may be changed silently (`05-auto-import.md` section 3).
5. Compute the optimize command in a non-blocking read action; apply it on the UI thread as an
   **undo-transparent** command (merged with the user's last typing in the undo history).
6. If the optimize also is allowed to add unambiguous imports (explicit action only), this flow
   never adds imports.

### 4.2 Interaction with other features

- While the user is typing inside the import list the file has syntax errors, so nothing happens.
- The request is cancelled by any PSI change (the daemon restarts and re-requests).
- After a silent import (`05-auto-import.md` 7) with the setting on, an optimize run follows in the
  same command (unused imports go, the new import is sorted).

`CI-OPT-030` MUST: the trigger and evaluation sequence of 4.1 (stamp check, daemon-finished
timing, error exceptions for inspection and import-list errors, silent-change predicate,
undo-transparent execution). Acceptance: S-OPT-5.

`CI-OPT-031` MUST: default off, project scope. Acceptance: see CI-IMP-002.

`CI-OPT-032` MUST: the on-the-fly flow never runs while the file has a real error elsewhere.
Acceptance: introduce `int x = ;` (syntax error) in the file; unused imports stay until it is
fixed.

## 5. Combined layout processing: Reformat Code, Optimize, Rearrange, Cleanup

### 5.1 Actions

| Action | Shortcut | Scope |
|---|---|---|
| Reformat Code | `Ctrl+Alt+L` | the selection if any; else the whole file in the editor; from the tree: the selected files/dir |
| Reformat Code... (dialog) | `Ctrl+Alt+Shift+L` | options below |
| Auto-Indent Lines | `Ctrl+Alt+I` | the selected lines or the current line |
| Optimize Imports | `Ctrl+Alt+O` | section 3 |
| Code Cleanup | no shortcut (`Code | Code Cleanup…`) | see `03-inspections.md` 9.1.7 |
| Rearrange Code | no shortcut | arrangement rules (LATER) |

### 5.2 The dialog `Reformat Code` (`Ctrl+Alt+Shift+L`)

For a file or selection: sections `Scope` and `Options`: `Selected text` (when a selection exists)
or `Whole file`; `Only changed lines` / `Only changes uncommitted to VCS` (when VCS has changes);
`Optimize imports`; `Rearrange entries`; `Cleanup code`; `Run` button (`Run`). For a directory
the dialog is titled `Reformat Files` with `Include subdirectories`, `Only changes uncommitted to
VCS`, file mask filter, `Optimize imports`, `Rearrange entries`, `Cleanup code`. The last
choices persist.

### 5.3 Execution order and undo

The processors form a chain executed **per file** in this order: **Reformat -> Optimize Imports ->
Rearrange -> Cleanup**; each stage is its own write command with the same *undo group id*, so a
single Undo reverts the whole sequence for that file (or for all files if "process all files as one
undo step" is set). A stage that fails or is cancelled stops the rest for that file. Stages that
need analysis run it in read actions before their write command. In dumb mode the optimize stage
does nothing.

### 5.4 Result messages

After a single-file run in an editor, one hint combines the stage notifications: reformat:
`Formatted N lines` (singular `Formatted 1 line`) or nothing when nothing changed; optimize imports as
in 3.2.6; rearrange: `Rearranged code`; cancellation: `Formatting canceled`. The hint is controlled
by the setting `Show notification after Reformat Code action` (default on). Messages are joined
with `, ` in the order of the stages (for example `Formatted 4 lines, Removed 2 imports`).

`CI-OPT-040` MUST: Reformat Code on selection/file with the chain order of 5.3 and one undo group.
Acceptance: S-OPT-6.

`CI-OPT-041` SHOULD: the dialogs of 5.2 and the persistent options. Acceptance: ticking `Optimize
imports` and `Run` formats and optimizes in one undo step.

`CI-OPT-042` SHOULD: the combined hint of 5.4. Acceptance: `Formatted 4 lines, Removed 2 imports`.

`CI-OPT-043` MUST: Auto-Indent Lines indents the selected lines according to the code style
without otherwise reformatting. Acceptance: pasted text with wrong indentation gets fixed per line.

## 6. Actions on save

Settings | Tools | Actions on Save lists, with checkboxes and per-action scope links:
- `Reformat code` (`Whole file` or `Changed lines`, file types: `All file types` or a selection;
  link `Configure scope…`),
- `Optimize imports` (file types),
- `Rearrange code` (with link `Configure arrangement rules…`),
- `Run code cleanup`,
- tool-specific entries registered by plugins.
They run on **explicit save** (`Ctrl+S` / Save All) for modified files; the order is the chain
of 5.3; each is part of the save command so Undo after save reverts the actions together. They do
not run on automatic background saves (frame deactivation, idle) unless the user enables that.

`CI-OPT-050` SHOULD: actions on save for `Reformat code` and `Optimize imports` with whole-file
scope, file-type filter, running on `Ctrl+S`. Acceptance: with `Optimize imports` on save ticked,
saving a file with an unused import removes it; `Ctrl+Z` brings both the import and the unsaved
state back.

## 7. Hooks that reformat while editing (high level)

These hooks belong to the typed-handler chain (editing work package); their *defaults* are listed
here because they are user-visible and shared.

| Hook | Setting (Settings | Editor | General | Smart Keys) | Default |
|---|---|---|---|---|---|
| Indent the new line after `Enter` | `Smart indent` | on |
| Reformat the block when typing the closing brace | (code insight setting) | on |
| Reformat pasted text | `Reformat on paste:` `None` / `Indent block` / `Indent each line` / `Reformat block` | `Indent each line` |
| Insert imports for pasted code | `Insert imports on paste:` `Always` / `Ask` / `Never` (Auto Import page) | `Always` |
| Surround selection on typing a quote or bracket | `Surround selection on typing quote or brace` | on |
| Insert matching quote / bracket / brace | `Insert pair quote`, `Insert paired brackets (), [], {}, <>`, `Insert pair '}'` | on |
| `Home` moves to first non-whitespace | `Home moves caret to first non-whitespace character` | on |
| Add closing comment/brace on Enter, Javadoc stub on Enter | (code insight settings) | on |
| Smart backspace (unindent) | (code insight setting) | on |

Paste behaviour: **None** inserts text verbatim; **Indent block** shifts the whole pasted block
so the first line is at the caret's indent; **Indent each line** re-indents each pasted line
according to the code style (the default); **Reformat block** runs the formatter on the pasted
range. A paste into a string or comment is never reformatted. The paste is one undo step together
with imports added by `Insert imports on paste`.

`CI-OPT-060` SHOULD: the hooks of the table with these defaults, implemented as typed/paste
handlers calling the formatter service. Acceptance: pasting four lines with 8-space indentation
into a 4-space block with `Indent each line` yields lines indented by the code style, one Undo
removes the paste.

## 8. Scenarios

**S-OPT-1, explicit optimize (Java).** File:
```
import java.util.List;
import java.util.ArrayList;
import java.io.File;
import static org.junit.Assert.*;

class A { ArrayList<String> xs = new ArrayList<>(); }
```
`Ctrl+Alt+O`: `List` and `File` are unused and removed; the result is a single import of
`java.util.ArrayList` and the static import after a blank line (layout: other imports, blank,
`javax`/`java`, blank, static)... with the default layout the order is: `import java.util.ArrayList;`,
blank line, `import static org.junit.Assert.*;` (the static import kept only if used; here it is
unused unless the file uses an `Assert` method, otherwise removed). The hint reads `Removed 2
imports` (or `Removed 3 imports` if the static import is unused too). Undo (one step) restores all.

**S-OPT-2, layout.** Project import layout set to put `org.*` before `java.*`: running optimize
reorders accordingly and `Rearranged imports` appears when nothing was removed.

**S-OPT-3, add unambiguous imports on optimize.** With `Add unambiguous imports on the fly` on,
a file that uses `Optional` without an import and has an unused import: `Ctrl+Alt+O` adds `import
java.util.Optional;`, removes the unused one, hint `Removed 1 import, added 1 import`.

**S-OPT-4, directory.** Right-click a package, `Optimize Imports`: the dialog `Optimize Imports`
with `Directory 'service'`, `Include subdirectories`; `Run`: a progress `Optimizing imports…`; every
file changed; one `Ctrl+Z` in the project reverts them all (setting default).

**S-OPT-5, on the fly.** With `Optimize imports on the fly` on, in a file already edited once the
user deletes the only use of `java.util.List` and waits: the import turns grey, then within one
daemon cycle after analysis finishes it disappears; Undo restores the import **together with** the
deleted usage (undo-transparent). If at that moment the file has a syntax error elsewhere, the
import stays grey.

**S-OPT-6, Reformat + Optimize.** `Ctrl+Alt+Shift+L`, tick `Optimize imports`, `Run`: indentation is
fixed and imports optimized; hint `Formatted 7 lines, Removed 1 import`; one `Ctrl+Z` undoes both.
