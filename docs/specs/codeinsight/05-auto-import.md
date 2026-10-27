# 05. Auto-import: hints, silent imports, and the Alt+Enter import fixes

Status: specification, framework and UX only. The language-specific candidate search (which
classes or members can be imported for an unresolved name, their filtering and ranking) belongs
to the Java and Kotlin specs. Requirement prefix `CI-IMP`.

ic: `platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/ShowAutoImportPass.java`,
`platform/analysis-impl/src/com/intellij/codeInsight/daemon/ReferenceImporter.java`,
`platform/analysis-api/src/com/intellij/codeInspection/HintAction.java`,
`platform/analysis-impl/src/com/intellij/codeInsight/CodeInsightSettings.java`,
`platform/analysis-impl/src/com/intellij/codeInsight/daemon/DaemonCodeAnalyzerSettings.java`,
`platform/platform-impl/src/com/intellij/codeInsight/hint/{HintManagerImpl,LocalHintManager}.java`,
`platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/DaemonListeners.java`
(`canChangeFileSilently`), `CanISilentlyChange.java`;
Java examples: `java/java-impl/.../daemon/impl/quickfix/ImportClassFixBase.java`,
`StaticImportMemberFix.java`, `ImportHintDismissalTracker.java`,
`java/java-impl/.../daemon/impl/JavaReferenceImporter.java`,
`java/java-impl/.../daemon/impl/actions/AddImportAction.java`,
`java/java-analysis-impl/.../daemon/impl/analysis/{UnusedImportsVisitor,OptimizeImportRestarter}.java`;
Kotlin: `plugins/kotlin/frontend-independent/.../KotlinReferenceImporter.kt`,
`plugins/kotlin/code-insight/impl-base/.../KotlinCodeInsightSettings.kt`.

## 1. The three behaviours and who does what

Auto-import is three user-visible behaviours that share one framework:

| Behaviour | Trigger | Result |
|---|---|---|
| **Import hint** | the daemon finished; an unresolved reference with import candidates is *visible* on screen | a small hint above the reference: `Import class... java.util.List?  Alt+Enter`; accepting imports |
| **Silent import** ("Add unambiguous imports on the fly") | same trigger, option on, exactly one candidate | the import is added without asking, undo-transparent |
| **Alt+Enter import fix** | user opens the intention popup on the reference | row `Import class` / `Import static method...`, chooser if several candidates |

Division of labour:
- The **language** supplies: the *import fix* (an intention marked as a *hint action*, attached
  to the unresolved-reference HighlightInfo), the *reference importer* (a service that, for a
  given caret offset, finds an unresolved reference and a function that performs the silent
  import), and its *settings*.
- The **framework** (this document) supplies: the pass that evaluates hints and silent imports,
  the hint UI and its lifecycle, the "can I change this file silently" rule, the dismissal
  memory, the Alt+Enter pre-step, and the settings page skeleton.

## 2. Settings (names, defaults, scope, UI)

Settings | Editor | General | Auto Import (one group per language; Java shown):

| UI label (exact) | Meaning | Default | Scope |
|---|---|---|---|
| `Show auto-import tooltip for:` `Classes` | enables the import **hint** for classes (the framework's "show import hints" switch) | on | application |
| `Show auto-import tooltip for:` `Static methods and fields` | enables the import **hint** for static members (Java: `ADD_MEMBER_IMPORTS_ON_THE_FLY`) | on | application |
| `Insert imports on paste:` `Always` / `Ask` / `Never` | what to do with imports for pasted code | `Always` | application |
| `Add unambiguous imports on the fly` | **silent import** of the only candidate. Help text: `While editing code, automatically add imports when there is only one importable declaration with matching name` | **off** | application |
| `Optimize imports on the fly` (shown with a project icon and a context-help text: `Remove unused imports and reorder imports according to the code style settings`) | after analysis, remove unused imports and reorder, see `08-optimize-imports-and-reformat.md` | **off** | **project** (workspace file) |
| `Include auto-import of static members in completion:` | list of classes/members whose static members are offered by completion with auto-import (examples: `java.util.Objects`, `java.util.Objects.requireNonNull`); columns Class or static members, Scope (IDE / project) | empty | IDE + project |
| `Exclude from auto-import and completion:` | list of package/class prefixes never imported or completed; wildcard `*` allowed | empty (a few built-in) | IDE + project |

Kotlin keeps its own two switches (`add unambiguous imports on the fly`, application level,
default off; `optimize imports on the fly`, workspace level, default off), the hint switch is
shared with the platform's `Show auto-import tooltip for: Classes`. Other settings that matter:

- the *reference-name filter* `NO_AUTO_IMPORT_PATTERN`: a regular expression (default `[a-z].?`,
  i.e. a lowercase letter optionally followed by one more character) matched against the whole
  *name as written at the reference* (including a qualifier if any). When it matches, the import
  fix produces **no candidates at all**: no hint, no silent import and no `Import class` row
  (a one- or two-letter lowercase name is far more likely a mistyped variable than a class).
  An invalid expression is ignored.
- "Import hints" are also disabled per file (per-file `import hints enabled` flag) and in files
  where the platform suppresses automatic hints (read-only decompiled files, files that report
  that auto hints are suppressed).

Changing any of these settings restarts the daemon for all files.

`CI-IMP-001` MUST: the settings of the table (labels, defaults, scope), persisted, with the page
placed under Settings | Editor | General | Auto Import. Acceptance: fresh profile: hint toggles
on, silent import off, optimize on the fly off; toggling each takes effect after the next daemon
cycle without restart of the IDE.

`CI-IMP-002` MUST: `Optimize imports on the fly` is stored per project, not per IDE.
Acceptance: enabling it in project A does not affect project B.

`CI-IMP-003` SHOULD: the exclude list and the "include static members" list with IDE and
project scopes and wildcard support. Acceptance: excluding `java.awt.*` removes `java.awt.List`
from candidates and from the Alt+Enter chooser.

`CI-IMP-004` MUST: the reference-name filter above. Acceptance: an unresolved lowercase name `ab`
has no import fix of any kind, while `Ab` does.

## 3. "Can I change this file silently?"

Any change the IDE makes *without the user asking* (silent import, optimize on the fly) is
allowed only if **all** hold:

1. the file has a real virtual file (not a null-file buffer);
2. the user has **already edited this file in this session**, i.e. an editor of this file has
   an available *undo* (never-touched files are never changed silently; scratch files follow the
   same rule);
3. for a normal file: it is inside the project content (not a library or excluded file);
   code fragments are always allowed;
4. no extension vetoes it (an extension point lets, for example, a read-only-VCS integration
   say no);
5. no *cut* operation just happened (right after Cut the file is in the middle of a
   cut-and-paste and imports must not be touched);
6. not in a modal dialog context;
7. not in dumb mode (for imports).

`CI-IMP-010` MUST: this predicate, evaluated on the UI thread when the change is applied, and
used by every silent change. Acceptance: open a file, do not edit it, introduce no changes by
other means: no silent import happens even with the option on; type one character anywhere:
silent imports start working for that file; right after Cut they pause until the next
command.

## 4. The daemon pass: when hints and silent imports are computed

The *show auto-import* pass runs after general highlighting and local inspections have been
applied (CI-DMN-005). Its two phases:

**Collect (background, read access).** Only if no inline AI prompt is shown and the file may be
changed silently (section 3 conditions 1 to 5). Take all HighlightInfos of the file that have at
least one quick fix and whose range does **not** contain the caret offset (a reference that
contains the caret is skipped so the user is not interrupted while typing it). For each of those
infos and each registered *reference importer* for which "add unambiguous imports on the fly" is
enabled for this file, ask the importer to compute, for the info's start offset, an *import
action* (a supplier executed later on the UI thread) or nothing. Collected actions are
kept for the apply phase.

**Apply (UI thread, short).** Do nothing unless the editor has focus, the project is not in dumb
mode, no inline prompt is shown, and the file and editor are still valid. Then:

1. **Silent imports**: if any collected action exists and "may auto-import now" holds
   (silent-change predicate true, option on, not in a modal context), run each action. Each
   action performs the import as an undo-transparent change and reports whether it did.
2. **Hints**: if the hint setting is on, hints are enabled for the file, the project-level
   `import hints enabled` flag is on, and the user did not just press Esc: take the infos *in
   the visible range* that carry a hint action and whose start is not inside a collapsed fold;
   **sort them by distance of their start to the caret** (nearest first); for each in order,
   if the hint action is available and its `show hint` returns true (a hint is now showing or
   something was imported), **stop**. At most one hint is visible at a time, and the nearest
   unresolved reference wins.

The pass runs again after every restart (caret move, edit, settings change), so moving the caret
or scrolling shows the hint of the new nearest reference.

`CI-IMP-020` MUST: the pass structure above, in particular the collect filter (infos with fixes
whose range does not contain the caret), the apply guards, and one hint at a time nearest to the
caret in the visible range. Acceptance: two unresolved names on screen, caret between: the hint
is for the nearer one; moving the caret next to the other switches the hint.

`CI-IMP-021` MUST: no hint and no silent import for a reference that contains the caret.
Acceptance: typing `Lis|` (caret at the end) with candidates shows nothing until the caret
leaves the identifier (the hint appears when you move away, unless the user triggers Alt+Enter).

`CI-IMP-022` MUST: no silent change and no hint when the editor has no focus or in dumb mode.
Acceptance: start indexing: hints disappear and do not reappear until smart mode.

## 5. The import hint (UI and lifecycle)

### 5.1 What it looks like

- A small *question hint* (light yellow/grey bubble, bold font) **above** the unresolved
  reference, anchored to the reference's range; the reference text is **underlined** with a
  line in the foreground colour while the hint is shown (a highlighter just above the error
  layer).
- Text, new UI: a link-styled `Import <kind>...` followed by the **qualified name of the first
  candidate**, a question mark, two spaces, and the Alt+Enter shortcut in grey, for example
  `Import class... java.util.List?  Alt+Enter`. For members `<kind>` is `static method`,
  `static constant`, `static field`, etc. When there are several candidates the same text is used
  (the name is that of the first). Classic UI variant: `java.util.List? Alt+Enter`, and
  `java.util.List? (multiple choices...) Alt+Enter` for several.
- The `<kind>` word for classes is `class` (message key `go.to.class.kind.text`).
- The shortcut text is the first keyboard shortcut of the action *Show Intention Actions* in
  the active keymap; its colour is the neutral "shortcut foreground" (light `818594`,
  dark `6F737A`).

### 5.2 Showing rules (inside the language's `show hint`)

A hint is shown only if all hold:
1. the reference is not qualified (no hint for `a.b.Foo` where `Foo` is after a dot);
2. the file and the reference are still valid and the PSI/modification stamp did not change;
3. the candidate list is non-empty after filtering (section 6) and the fix is still available;
4. the user did **not** dismiss the hint for this reference with Esc (see 5.4);
5. no visible hint is showing that would be hidden by another hint (for example a tooltip), and
   the currently shown *question* hint, if any, has a priority not higher than the import hint:
   the import hint has priority 0, the light bulb has priority -10, so an import hint replaces
   the light bulb but not another import hint of higher priority;
6. not in unit-test/headless mode;
7. for static members: the static-members hint switch is on, the call has no qualifier, and the
   member candidates pass the *hint candidates* filter (a signature that cannot match is not
   offered as a hint, though it stays in the Alt+Enter list), and it is not the case that the
   only candidate lies in the same file with static imports disabled.
If exactly one candidate exists, "add unambiguous imports on the fly" is on, the may-change
predicate holds and the caret is not at the end of the reference, the language imports it
*instead* of showing the hint (silent import, section 7).

### 5.3 Accepting the hint

- `Alt+Enter` while the hint is visible performs it (CI-ALT-060 step 5): **no popup** opens.
- Clicking the `Import class...` link performs it.
- Performing: one candidate: import it immediately; several candidates: the **class chooser**
  popup (section 8). The import is one command named `Add Import` (Undo shows `Undo Add Import`);
  the caret is not moved; after the import the daemon restarts and the red mark disappears.
- Statistics: using an import increases the use count of that class (feeds ranking, see 8).

### 5.4 Hiding and dismissal

- Default hide flags: **any key press**, **text change**, **scrolling** (it is repositioned when
  scrolling is by small steps and hidden when out of editor), **caret movement**, selection
  change, editor switch; it does **not** consume Esc.
- **Esc** while the hint is shown: the hint hides and the *reference + name* pair is remembered
  as dismissed for this editor (identity of the reference element and its text). A different
  occurrence of the same name still gets a hint; renaming the reference text re-enables it. The
  daemon's own "escape just pressed" flag only lasts until the next caret move, which is why the
  per-reference memory exists.
- The hint is also not shown while the user just pressed Esc (daemon flag) and while a template
  session or inline rename is active.

`CI-IMP-030` MUST: hint text, position, underline and shortcut text of 5.1. Acceptance
(screenshot): `Import class... java.util.List?  Alt+Enter` above `List`, `List` underlined.

`CI-IMP-031` MUST: showing rules 5.2 (1 to 5, and 7 for static members). Acceptance: no hint for
`x.Foo`; hint for `Foo` with candidates; no hint after Esc until the reference is edited.

`CI-IMP-032` MUST: accepting rules 5.3 including command name `Add Import`, no popup on
Alt+Enter, chooser for several candidates. Acceptance: S-IMP-2.

`CI-IMP-033` MUST: hide and dismissal rules 5.4. Acceptance: Esc on the hint; move caret away
and back: no hint for that reference; edit the name: hint returns.

`CI-IMP-034` SHOULD: the classic-UI hint text variant. Acceptance: setting toggle in tests only.

## 6. The import candidate set (framework requirements on the language)

A language's import fix must provide:

1. **Candidates**: the list of importable declarations for the unresolved name, computed on a
   background thread under a read action (it may be expensive), **filtered**:
   - by what the reference expects (class vs method vs annotation vs static member by required
     member name; type arguments when relevant);
   - by accessibility (drop inaccessible declarations if any accessible one exists; drop
     declarations in the same package; drop declarations from non-modifiable sources when an
     accessible one exists);
   - by the exclude list, by "never import classes of the default (unnamed) package" and by the
     rule that the qualified name must be a valid dotted name;
   - by the reference-name filter (section 2), which removes *all* candidates;
   - by *already imported but unresolved* names (do not offer a class whose short name is
     already imported but unresolved: the user must fix that import first);
   - by the package part of a qualified reference (`pkg.Foo` offers only classes in `pkg`);
   - by architectural dependency rules, when several candidates remain, dropping those that
     violate a rule (never dropping the last one).
2. **Order**: a deterministic order (Java: non-deprecated first, non-inner classes first, then
   proximity to the current file (same module, nearer directory...), then qualified name
   case-insensitive).
3. **Fix text**: `Import class` for classes; for static members `Import static method 'X.m'` /
   `Import static method...` (several), `Import static constant 'X.C'` / `...`.
4. **Priority** TOP (so it is the first error fix).
5. **Fix range**: wide (the whole line) so it is reachable with the caret anywhere on the line.
6. A *pure-command* version of the fix (`ChooseAction` over at most **25** candidates with
   title `Class to Import`), used for previews and batch.
7. `show hint`, `is available`, and a cheap "already imported" shortcut that avoids resolving
   when the import is obviously present.

`CI-IMP-040` MUST: the fix described above registered on every unresolved reference (class and
static member), TOP priority, wide fix range, text as in 3. Acceptance: S-ALT-1 and S-IMP-1.

`CI-IMP-041` MUST: candidate filtering rules 1 and order rule 2 are testable through a headless
function `import_candidates(file, offset) -> list`. Acceptance: unit tests on a fixture with
`java.util.List`, `java.awt.List` (deprecated?), an inner `Outer.List`, and a same-package
`List`: expected order and filtering match the rules.

## 7. Silent import (details)

When the pass collects an action for an unresolved reference (language importer):
- exactly **one** candidate (after filtering, section 6);
- the reference is not qualified;
- the caret is not at the end of the reference (the importer is told whether "caret near the
  reference" is allowed: false from the pass, true from Alt+Enter);
- for Java: importing a *member class* is skipped when the code style setting "insert imports for
  inner classes" is off (the import would add qualifying text the user did not ask for);
- no unresolved import statement in the file already carries the candidate's short name (that
  would just add a duplicate);
- the may-change-silently predicate holds (section 3).

The import is executed as an *undo-transparent* command (so it merges with the user's last
typing for undo purposes, and Undo removes it together with the typed text when appropriate),
without moving the caret. If "optimize imports on the fly" is on and the language's optimizer is
available, the import is followed by an optimize run in the same command (unused imports go,
imports are re-sorted).

If the language reports exactly one candidate but the importer cannot import it now (for
example the caret is next to it), the hint is shown instead (if enabled).

`CI-IMP-050` MUST: silent import conditions of section 7 and the undo-transparent execution.
Acceptance: S-IMP-3: with the option on, type `List<String> l;` in a touched file, then move to
the next line: `import java.util.List;`... (if only one candidate) appears without a hint; one
Undo removes the import together with the last typing step; for a reference
name matched by `NO_AUTO_IMPORT_PATTERN`, no import and no hint.

`CI-IMP-051` MUST: with the option off, a unique candidate shows the hint instead of importing.
Acceptance: S-IMP-1.

## 8. The class chooser popup (`Class to Import`)

Shown when accepting a hint or choosing `Import class` with several candidates.

- A list popup titled **`Class to Import`**. Rows: the candidate's icon and its **fully
  qualified name**; no sections. Order as in 6.2. The first row is selected. Speed search is
  on; automatic selection by typing is off.
- Enter imports the selected class (command `Add Import`, caret preserved, statistics updated).
- **Right arrow** on a row opens a submenu to exclude it from auto-import: rows
  `Exclude '<qualified name>' from auto-import`, then each shorter package prefix down to the
  second segment (`com.acme.util.List`, then `com.acme.util`, then `com.acme`), and choosing one
  opens Settings | Editor | General | Auto Import with the prefix added to the exclude list.
- The popup closes when dumb mode starts, when the editor loses focus, or on Esc.
- At most 25 candidates are listed in the pure-command form; the interactive form lists all.

`CI-IMP-060` MUST: the chooser of this section (title, rows, order, Enter, speed search).
Acceptance: S-IMP-2.

`CI-IMP-061` SHOULD: the exclude submenu rows. Acceptance: `Exclude 'java.awt.List' from
auto-import`, `Exclude 'java.awt' from auto-import` ... and the page opens with the entry.

## 9. Alt+Enter import fixes (what the rows do)

- The row is a normal error-group TOP row (CI-ALT). It does **not** use the hint's silent
  logic: choosing it always performs the import (several candidates: the chooser).
- Because Alt+Enter first runs the **auto-import pre-step** (CI-ALT-060 step 3), pressing
  Alt+Enter on a reference with a unique candidate while "Add unambiguous imports on the fly"
  is on imports it **immediately and no popup appears**; with the option off the popup opens
  with the row `Import class` first.
- Choosing the row for a *static member* offers the import of the member (`import static
  pkg.Class.member;`) and rewrites nothing else; when the language supports it, a second row
  `Add qualifier` / `Qualify static member` may appear (Java spec).
- Both paths record statistics and use the same import placement rules (import layout from the
  code style: groups, order, blank lines, star-import thresholds: `08-optimize-imports-and-reformat.md`).
- When the language's pure-command fix is used (previews, batch): the chosen candidate
  produces an `UpdateFileText` on the import list (and on the reference if it must be
  qualified).

`CI-IMP-070` MUST: Alt+Enter semantics of this section. Acceptance: S-IMP-1 and S-IMP-4.

`CI-IMP-071` MUST: import placement obeys the import layout (groups, ordering, blank lines) of
the language code style and never touches other code. Acceptance: importing into a file with a
custom layout `java, javax, *, blank, all other, static` yields the configured order.

## 10. Auto-import on completion and on paste (pointers)

- **Completion**: choosing a lookup item for a class that is not imported adds the import as
  part of the insert (see `06-completion.md` section 9). The same exclude and static-member
  lists apply.
- **Paste**: `Insert imports on paste` `Always` imports all needed classes for pasted code whose
  references resolve to a unique target; `Ask` shows a dialog listing the classes to import with
  checkboxes; `Never` does nothing. The copy side stores the *source file's* resolved
  references; the paste side adds imports for those that do not resolve in the target file.
  Priority `SHOULD` for Java/Kotlin.

`CI-IMP-080` SHOULD: insert imports on paste with the three modes. Acceptance: copy a method
using `Map<String, List<Foo>>` from file A, paste into file B that lacks the imports: with
`Always`, the imports for `Map`, `List`, `Foo` appear and the pasted code resolves.

## 11. Scenarios

**S-IMP-1, hint, then Alt+Enter, default settings.** Settings default (silent import off). File
`OrderService.java` has no import for `List`; the user types `List<Order> orders = null;` and
presses `Down` to move to another line.
Expected: after about 300 ms the red text `List` has a hint above it: `Import class...
java.util.List?  Alt+Enter` (two candidates `java.util.List`, `java.awt.List` in the project's
libraries: the same text, the first candidate shown); the text `List` is underlined. Pressing
Alt+Enter performs the hint: the chooser `Class to Import` appears with `java.util.List` first
(non-deprecated, nearer by proximity) and `java.awt.List`; Enter on the first: `import
java.util.List;` is inserted in the import block in layout order; undo label `Undo Add Import`;
the red mark is gone after the next cycle. If the user instead had clicked the red text and
pressed Alt+Enter *without* a visible hint (for example Esc was pressed earlier), the popup shows
`Import class` first (error group, TOP).

**S-IMP-2, ambiguous hint accepted by mouse.** As above; click the `Import class...` link: same
chooser; choose `java.awt.List` by typing `awt` (speed search) and Enter: `import java.awt.List;`.

**S-IMP-3, silent import.** Enable `Add unambiguous imports on the fly`. In a file already edited
once, type `Optional<String> o = null;` and press Enter for a new line. `Optional` has exactly one
candidate (`java.util.Optional`): while the caret was at the end of `Optional` nothing happened;
after the caret left the reference, `import java.util.Optional;` is added with no hint. One
Undo removes the import (undo-transparent merge with the typing).

**S-IMP-4, Alt+Enter with the option on.** Same file, option on, caret *inside* `Optional`
(unresolved, one candidate): Alt+Enter: the import is added immediately, no popup.

**S-IMP-5, name filter.** Type an unresolved lowercase name `ab` (one that would otherwise have
a candidate class `pkg.ab`): no hint, no silent import and no `Import class` row in Alt+Enter,
because the default pattern `[a-z].?` matches the written name. Rename it to `Ab`: the hint and
the row are back.

**S-IMP-6, untouched file.** Open a file, do not type anything, option on: silent import does not
run (no undo history); after typing one character it does.

**S-IMP-7, exclude.** In the chooser press Right on `java.awt.List`, choose `Exclude 'java.awt'
from auto-import`: settings open with `java.awt` added; the next hint for `List` has only
`java.util.List`.
