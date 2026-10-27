# 07. Navigation: go to declaration, usages, highlight usages

Status: specification, language independent. Requirement prefix `CI-NAV`.
Priorities: `MUST` / `SHOULD` / `LATER`.

Resolve itself (which element a reference means) is specified in the analysis docs
(`docs/analysis/code-intelligence-spec.md` 1.5). This file specifies the *behaviours built on
resolve*: what each navigation command does in each situation, which popups and messages
appear, and how usage search and highlighting are structured. The Java-specific search options
(overriders, derived classes, getters/setters) belong to the Java spec.

ic: `platform/lang-impl/src/com/intellij/codeInsight/navigation/actions/`
(`GotoDeclarationOrUsageHandler2.kt`, `GotoDeclarationOnlyHandler2.kt`,
`GotoTypeDeclarationHandler2.kt`, `GotoSuperAction.java`, `GotoImplementationAction.java`),
`.../navigation/impl/gtdu.kt`, `.../navigation/{CtrlMouseHandler,GotoImplementationHandler,
GotoTargetHandler,ImplementationSearcher}`, `platform/lang-impl/src/com/intellij/find/actions/
ShowUsagesAction.java`, `platform/analysis-impl/src/com/intellij/find/findUsages/FindUsagesOptions.java`,
`platform/usageView/` (`UsageViewSettings.kt`, `UsageViewBundle.properties`),
`platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/IdentifierHighlighting*.kt`,
`.../highlighting/HighlightUsagesHandler*.java`, `platform/core-api/.../usageView/UsageInfo.java`.

## 1. Commands and default shortcuts

| Command | Shortcut | Notes |
|---|---|---|
| Go to Declaration or Usages | `Ctrl+B`, `Ctrl+Click` (macOS `Cmd+B`, `Cmd+Click`, middle click) | at a reference: go to its declaration; at a declaration: show usages |
| Go to Declaration Only | none | never falls back to usages |
| Go to Implementation(s) | `Ctrl+Alt+B`, `Ctrl+Alt+Click` | subclasses, overriders, implementors |
| Go to Type Declaration | `Ctrl+Shift+B`, `Ctrl+Shift+Click` | type of the element at caret |
| Go to Super Method | `Ctrl+U` | overridden method or super class |
| Find Usages | `Alt+F7` | tool window with all usages |
| Show Usages | `Ctrl+Alt+F7` | popup |
| Find Usages in File | `Ctrl+F7` | highlights usages in the current file |
| Highlight Usages in File | `Ctrl+Shift+F7` | persistent highlighting |
| Find Usages Settings | `Ctrl+Alt+Shift+F7` | options dialog before searching |
| Quick Definition | `Ctrl+Shift+I` | popup with the source of the target |
| Back / Forward | `Ctrl+Alt+Left` / `Ctrl+Alt+Right`, mouse back/forward | navigation history |
| Go to Class / File / Symbol | `Ctrl+N` / `Ctrl+Shift+N` / `Ctrl+Alt+Shift+N` | name-based, specified in the search spec |

## 2. Go to Declaration or Usages (`Ctrl+B`)

### 2.1 Algorithm (in order)

1. If a completion lookup is showing and an item is selected: navigate to the *item's* target.
2. If the caret is in virtual space (past the end of line): do nothing.
3. If the file is in an *untrusted/safe* mode (opened from an untrusted location): ask once per
   file and session for confirmation ("navigate").
4. Under a short modal progress ("Resolving reference…", cancellable, shows only if slow):
   a. **Direct navigation providers** (extension, per language): custom rules, for example a path
      in a string literal navigates to the file, an `import` path to the module file, a build
      script reference to its declaration. First provider with an answer wins.
   b. Otherwise compute the *target data* at the caret offset (injected fragment first, then
      host): `referenced` symbols (what the reference at the caret resolves to) and `declared`
      symbols (what is *declared* at the caret: a name identifier of a declaration).
   c. **Go to declaration** if the referenced symbols exist and at least one is navigable.
   d. Else, if the referenced symbols exist but nothing can be navigated to (for example the target
      is the element itself or has no navigable source), **show usages** of the referenced
      symbols.
   e. Else, if declared symbols exist: **show usages** of the declared symbols.
   f. Else: nothing to go to.
5. Outcomes:
   - *nothing*: an information hint at the caret: `Cannot find declaration to go to`;
   - *go to declaration*: see 3;
   - *show usages*: see section 8, at the caret position with the *default scope*;
6. If an `IndexNotReady` condition arises: notification `Navigation is not available here during
   index update` and nothing else.

"Navigable" means the target has a source location the IDE can open: a source file, a library
class with sources, a decompiled class (decompile on demand; asks nothing), or a file.

### 2.2 Go to declaration, single and multiple targets

- **One target:** open its file in the current editor tab group (reusing a tab when the file is
  already open), place the caret at the *target offset* (the name identifier for declarations),
  scroll so the target is visible (centre if it was off-screen), and add a navigation history
  entry (the previous caret position is stored so *Back* returns).
- **Several targets:** a popup titled **`Choose Declaration`** (list popup, speed search on) lists the
  targets with icon, name, and a grey location (container class or file and module). The first row is
  selected. `Enter` navigates; the popup loads lazily if the target list is still being computed (shows a
  loading row).
  While the popup is open the *target element is highlighted* in the editor if it lies in the current
  file.
- Targets in the **same file** navigate in place without opening a new tab.
- Navigation to a **library class without sources** opens the decompiled view with the
  *Decompiled* banner (specified in the editor spec).

### 2.3 Go to Declaration Only

Same as 2.1 steps 1 to 4.c, but steps 4.d and 4.e do not run: on a declaration it reports
`Cannot find declaration to go to`... except that at a *declaration* it first tries 4.c for the
declared element's own super/implementation links where the language registers them (rare; LATER).

### 2.4 Ctrl+Click and Ctrl+hover

- Holding the modifier (`Ctrl`; `Cmd` on macOS) over a symbol turns the identifier into a
  hyperlink (`HYPERLINK_ATTRIBUTES`: underlined in the link colour) when the symbol has a
  navigation outcome (2.1), and shows a tooltip with the short documentation of the target
  (kind, signature, first paragraph of the doc comment, defining file). On a *declaration* the hyperlink stands for the
  *Show Usages* outcome.
- Click while the hyperlink is shown runs the same outcome as `Ctrl+B` for the clicked position
  (the caret moves to the click position first).
- Right/middle button variants: middle click is the same as `Ctrl+B` on macOS.

`CI-NAV-001` MUST: the algorithm of 2.1 with steps 4.a to 4.f in this order and the outcomes of 5.
Acceptance: S-NAV-1 to S-NAV-3.

`CI-NAV-002` MUST: the single-target navigation of 2.2 (reuse tab, caret at the name, history
entry) and the multi-target popup `Choose Declaration` with speed search and `Enter`. Acceptance: S-NAV-2.

`CI-NAV-003` MUST: hint text `Cannot find declaration to go to` and the index-not-ready
notification. Acceptance: `Ctrl+B` on whitespace shows the hint.

`CI-NAV-004` SHOULD: Go to Declaration Only (2.3). Acceptance: on a declaration the command
reports the hint, `Ctrl+B` shows usages.

`CI-NAV-005` SHOULD: Ctrl+hover hyperlink and tooltip, Ctrl+Click (2.4). Acceptance: hovering
with Ctrl over a method call underlines it and shows the signature and the first doc sentence.

## 3. Navigation history

Every navigation command, every `Ctrl+B`, every popup choice and every jump of more than a
few lines records the *previous* caret position in the history (editor, file, offset). `Back`
and `Forward` move through it and also reopen closed files. The history keeps a bounded number of positions (Archipel: at least 100, configurable);
consecutive records for the same line collapse.
`Navigate | Last Edit Location` (`Ctrl+Shift+Backspace`) jumps to the last change.

`CI-NAV-010` MUST: history for declaration jumps with Back/Forward (including reopening a closed
tab). Acceptance: `Ctrl+B` on a call, then `Ctrl+Alt+Left` returns the caret to the call.

## 4. Go to Implementation, Type Declaration, Super Method

- **Implementation** (`Ctrl+Alt+B`): finds the *definitions* of the symbol at the caret in
  inheritance terms (subclasses and sub-interfaces of a class or interface; overriders and
  implementors of a method; Kotlin/Java equivalents). The search is incremental. If exactly one
  result: navigate. If several: a popup (speed search, loading row) with title text from the kind
  of element: `Choose Implementation of <name> (<N> Methods Found)`, `Choose Overriding Method of
  <name> (<N> Methods Found)`, `Choose Subclass of <name> (<N> Classes Found)` where the count is
  shown while the search runs and updated. On a declaration inside an interface the command lists implementors; on a
  *call* it lists the implementations of the *resolved* method. If nothing is found: hint `No implementations
  found`.
- **Type Declaration**: the declaration of the *type* of the expression or variable at the caret;
  language hook `type declaration of(element)`. Multiple (union, intersection) types show a
  chooser. On a type name it navigates to the type itself.
- **Super Method** (`Ctrl+U`): from a method: its overridden/implemented method(s): one direct
  super method navigates; several (multiple interfaces) show a popup `Choose Super Method of
  <name>`; from a class the chooser is titled `Choose super class or interface`. When there is
  nothing to go to the command does nothing visible (no popup).
  The gutter icons for *overrides*/*implements* (`01-daemon.md` 10) use the same search and
  popups.
- **Related symbol** (`Ctrl+Alt+Home`): language-registered related elements (test <-> class).

`CI-NAV-020` SHOULD: Go to Implementation with single-result navigation and the popup titles
above, in Java/Kotlin first. Acceptance: `Ctrl+Alt+B` on an interface method with 3 implementors shows
`Choose Implementation of foo (3 Methods Found)`; one implementor navigates directly.

`CI-NAV-021` SHOULD: Go to Super Method and Type Declaration. Acceptance: `Ctrl+U` on an
overriding method jumps to the super method; `Ctrl+Shift+B` on a variable of type `Foo` opens `Foo`.

## 5. Find Usages framework

### 5.1 Concepts

- **Target(s)**: *primary* elements (what the user asked for) and *secondary* elements searched
  together (for example a field and its getter/setter, an enum constant and its name in a `match`,
  a re-exported item). A **find-usages handler** per language/element kind supplies them, the
  options type, and special processing.
- **Options**: *search for usages* (default on), *search for text occurrences* (in comments and
  strings; default off in the UI), *scope* (default **Project Files**; the scope list has `Project
  Files`, `Project and Libraries`, `Module`, `Directory`, `Open Files`, `Current File`, `Selection`,
  `Recently Viewed Files`, `Recently Changed Files`, custom scopes) and language-specific flags
  (Java: derived classes, implementing methods, overriding methods, overloaded methods, read/write
  access filters, usages of the class in `new`...). The options persist per kind of element.
- **Usage record** (`UsageInfo`): file, range (a smart range that survives edits), flag *is
  non-code usage* (found by text), the element at the usage, and a *usage type* for grouping.
  Usages may be *potential* (reference not exactly resolved) or *dynamic*.
- **Searcher**: `search(target, options, scope)` -> stream of usages (index-assisted: word index
  prefilter by the target's name and aliases, then resolve each candidate to confirm). Results
  stream into the view as found; cancellable.
- **Access detection** for variables: each usage is classified *read*, *write* or *read+write*
  by a language-provided detector (assignment target = write; `x++` = read+write; call
  argument or expression = read). Declarations may count as writes when they have initialisers.

### 5.2 The Find Usages tool window (`Alt+F7`)

- `Alt+F7` runs the search **immediately** with the options last used for this kind of element (`Find Usages Settings`
  `Ctrl+Alt+Shift+F7` shows the options dialog first with `Find Usages` title and the options of 5.1,
  and an `OK` that runs it). If the search target is ambiguous (several elements at the
  caret) a popup titled `Find Usages Of` lets the user choose.
- Results open in the **Find** tool window as a tab named after the search (`Usages of '<name>'`
  in a panel titled `<name> in <scope>`). Search runs in the background with the label
  `Searching…` and a progress icon in the tab; a counter `N results` updates live.
- **Tree**: root `Usages (N)`; groups (toggleable, defaults): by *usage type* (on), *module* (on),
  *package* (on), *file structure* (on), directory structure (off), scope (off), "Production/Test";
  then file nodes with a count; then usage rows `line number text-with-highlighted-reference`.
  The non-code usages go to a separate node `In Strings, Comments, and Text`; potential ones to
  `Potential usages`. Toolbar: `Rerun`, `Next Occurrence` / `Previous Occurrence`, `Expand all`,
  `Collapse all`, `Group By` toggles, `Show Read Access` / `Show Write Access` filters,
  `Show Import Statements`, `Show Usages in Comments`, `Preview Source` (on by default; a split
  with a read-only editor showing the usage in context), `Merge Usages from the Same Line`.
- **Usage type labels** (Java examples; the language supplies the set): `Value read`, `Value write`,
  `Usage in import`, `Usage in extends/implements clause`, `Method return type`, `Field
  declaration`, `Local variable declaration`, `Method parameter declaration`, `New instance
  creation`, `Annotation`, `Usage in comments`, `Usage in string constants`, `Unclassified`.
- Too many results: a confirmation titled `Too Many Results` with the text `Too many results
  found. Are you sure you wish to continue?` and buttons `Continue`/`Abort` (the threshold is an
  advanced setting).
- `Esc`/`Enter`/double-click: `Enter` and double-click navigate to the usage in the editor;
  `F4` (Edit Source) too.

### 5.3 Search result rules

1. The *declaration itself* is not a usage (a method's own name in its declaration is excluded);
   comments are included only when `search for text occurrences` is on.
2. Usages in the same file are included; usages in generated files follow the generated-code
   filter.
3. Results are sorted by file path then offset by default (`Sort Alphabetically` is a view
   setting).
4. A search started on an element that disappears or changes (edits during search) is restarted
   or reports `Targets have been invalidated` on rerun.
5. In dumb mode: notification `Usage search is not available until indexes are ready` (no
   partial results).

`CI-NAV-030` MUST: the usage record and searcher contract of 5.1 (streaming, cancellable, index
prefilter then resolve confirmation), with scopes `Project Files` (default), `Project and
Libraries`, `Module`, `Directory`, `Current File`. Acceptance: searching for a method used in 3
files returns exactly those usages in file/offset order, none for the declaration.

`CI-NAV-031` MUST: the Find tool window of 5.2 with tab name, live counter, groups by usage
type/module/package/file, navigation by Enter/double-click, rerun. Acceptance: S-NAV-4.

`CI-NAV-032` SHOULD: read/write classification and filters. Acceptance: for `count++; int y =
count; count = 0;` usages are classified read+write, read, write; the filter `Show Write
Access` leaves two rows.

`CI-NAV-033` SHOULD: preview split, `Merge Usages from the Same Line`, export. Acceptance: with
preview on, selecting a usage scrolls the read-only editor to it.

`CI-NAV-034` MUST: Find Usages Settings dialog (`Ctrl+Alt+Shift+F7`) with options and scope.
Acceptance: unchecking `Search for text occurrences` removes comment hits.

`CI-NAV-035` SHOULD: usage grouping rules as data (file, module, package, usage type,
read/write, language-defined container) feeding both the tool window and the popup.
Acceptance: adding a grouping rule needs no change to the view.

`CI-NAV-036` LATER: `Similar Usages` (clustering), `Recent Find Usages`.

## 6. Show Usages popup (`Ctrl+Alt+F7`, and the result of `Ctrl+B` on a declaration)

- The search starts at once; the popup opens near the caret and shows results as they stream in.
  It is a **table**: one row per usage with the *usage text* (the line, reference highlighted) and
  the *location* (`FileName.ext:line`, grey), grouped under separator rows by file (default group
  by file structure and usage type, as the tool window's defaults). A row of the current file is
  marked `Current` in the location column.
- **Immediate cases** (the popup is not shown): zero usages: hint `No usages found in <scope
  name>` (scope name default `Project Files`); exactly one usage: navigate there and hint `It's the only usage in
  <scope name>`; several usages all on one line: navigate to the first and hint `All <N> usages in
  <scope name> are in this line`.
- **Header:** `Usages of <name>` with the count (`N usages`, shown as `N+ usages` while more
  exist, `N of M usages` while a limit applies), a settings gear (tooltip `Settings…` plus its
  shortcut), a button `Open in Find Tool Window`, and a hint line `Use <shortcut> and <shortcut> to
  select usages`.
- **Page size:** at most **100** usages are loaded; a final row `more usages` loads the next 100
  on `Enter`/click.
- **Keys:** `Up`/`Down` select; `Enter` navigates to the usage and closes the popup; `Alt+F7` (find usages
  shortcut) or the link `Open in Find Tool Window` opens all results in the tool window; `Esc` closes; typing filters rows (speed search over usage text and file name); a
  preview panel toggle shows the code around the usage.
- **Scope advertisement:** `Press <Ctrl+Alt+F7> again to search in '<scope>'` (the next wider scope,
  for example `Project and Libraries`); pressing the shortcut again restarts the search in that
  scope.
- **Filtered results:** when filters (comments, imports, generated code) hide results the footer says
  `<N> usages were filtered out` with a link to re-search without filters.
- **Out of scope:** when usages exist outside the scope, a separator row reads `<N> usages are out
  of scope 'Project Files'`.

`CI-NAV-040` MUST: the immediate cases and their exact hint texts (`No usages found in <scope>`,
`It's the only usage in <scope>`, `All <N> usages in <scope> are in this line`). Acceptance: S-NAV-3 and a test
per case.

`CI-NAV-041` MUST: the popup table, live counts, 100-row paging with `more usages`, keys and
navigation. Acceptance: S-NAV-3.

`CI-NAV-042` SHOULD: scope advertisement and re-search, filtered-out footer, out-of-scope row.
Acceptance: pressing `Ctrl+Alt+F7` twice widens the scope.

## 7. Highlight usages

### 7.1 Automatic: identifier under the caret

- The setting `Usages of element at caret` (Settings | Editor | General | Appearance, group
  "Highlight on caret movement", default on; siblings `Matched brace` and `Current scope`) enables
  the automatic highlighting; a second setting `Highlight occurrences of selected text` (default on)
  highlights other occurrences of the selection instead.
- When the caret rests on an identifier or keyword, a *caret-driven* background computation
  (`01-daemon.md` 3.3) finds the symbol at the caret, then:
  1. If a **keyword/structure handler** applies (exit points of a method for `return`/`throw`,
     `break`/`continue` targets, `if/else` chains, `try/catch/finally`, `case` labels,
     `yield`...), highlight the keywords of the structure with the **structural** attribute
     (`MATCHED_BRACE_ATTRIBUTES` style) and, when the handler says so, stop.
  2. Else find all references to the symbol **in this file** (target = the declaration plus every
     reference resolving to it; read-write access known) and paint them: reads with
     `IDENTIFIER_UNDER_CARET_ATTRIBUTES`, writes with `WRITE_IDENTIFIER_UNDER_CARET_ATTRIBUTES`;
     the declaration is included (read or write by access rule).
  3. Highlight ranges are exactly the name identifier ranges (not the whole expression).
- The ranges are applied as markup of the *element under caret* severity (above ERROR, never
  counted); they are removed when the caret leaves the set of highlighted ranges **for a symbol
  that no longer applies** and recomputed lazily: if the caret moves *within* a range that is
  already known to belong to the current result no recomputation happens (the result is cached
  against the "target ranges").
- Not computed: in dumb mode (unless the language says reference search is index-free), in
  one-line editors embedded in dialogs, while a template or inline rename is active, for compiled
  files, when the selection is non-empty and `Highlight occurrences of selected text` is on, or when
  the setting is off.
- Typing (document change) clears and recomputes after the edit settles; scrolling does not.

### 7.2 Explicit: `Highlight Usages in File` (`Ctrl+Shift+F7`)

- Highlights usages of the element at the caret **persistently** (with the search-result
  attributes `SEARCH_RESULT_ATTRIBUTES`, writes with `WRITE_SEARCH_RESULT_ATTRIBUTES`), also
  marks them on the error stripe.
- If several targets are possible at the caret the user is asked to choose (popup of target names).
- Status/hint text (information hint at the caret, also in the status bar):
  `<N> usage(s) of '<name>' found (press <shortcut> again to remove the highlighting, Escape to
  remove all highlighting)`; none found: `No usages of '<name>' found`; exit points handler:
  `<N> exit point(s) highlighted (press <shortcut> again to remove the highlighting, Escape to
  remove all highlighting)`.
- Pressing the shortcut again on the same element removes the highlighting; `Esc` removes all;
  editing text removes them.
- `F3` / `Shift+F3` (next/previous occurrence) step through the highlighted usages and wrap.

### 7.3 `Find Usages in File` (`Ctrl+F7`)

Runs the search restricted to the current file, highlights the usages like 7.2 and positions the
caret at the first usage after the caret; `F3`/`Shift+F3` cycle. A hint with the count appears like
7.2.

`CI-NAV-050` MUST: automatic identifier highlighting of 7.1 (steps 1 to 3, read vs write
attributes, declaration included, ranges are identifiers, caching by target ranges, exclusions,
setting and its default). Acceptance: S-NAV-5.

`CI-NAV-051` MUST: it never blocks typing and does not restart the daemon (CI-DMN-008).
Acceptance: holding an arrow key does not start analysis passes.

`CI-NAV-052` SHOULD: `Highlight Usages in File` (7.2) with the three hint texts, repeat
to remove, `Esc`, and `F3` stepping. Acceptance: S-NAV-6.

`CI-NAV-053` SHOULD: keyword structure highlighting (return/exit points, if/else, try/catch,
loops) for Java and Kotlin. Acceptance: caret on `return` highlights all exit points of the method.

`CI-NAV-054` SHOULD: `Find Usages in File` (7.3). Acceptance: `Ctrl+F7` on a local variable
highlights all its uses and `F3` cycles.

## 8. Show usages from `Ctrl+B` on a declaration

`Ctrl+B` on a declaration (2.1 step 4.e, or 4.d) calls the Show Usages popup of section 6 with
the declared symbols as targets and the *default scope*. Identical popup, hints and shortcuts.
This is why `Ctrl+B` on a method name shows its callers.

## 9. LSP mapping (Archipel)

| Command | LSP request | Notes |
|---|---|---|
| Go to Declaration | `textDocument/definition` (and `declaration`) | multiple locations -> `Choose Declaration` popup |
| Go to Type Declaration | `textDocument/typeDefinition` | |
| Go to Implementation | `textDocument/implementation` | |
| Go to Super Method | `typeHierarchy/supertypes` or server extension | |
| Find / Show Usages | `textDocument/references` (`includeDeclaration: false`) | grouping by file only; usage type missing -> `Unclassified` unless native classification applies |
| Highlight usages | `textDocument/documentHighlight` | kind Read/Write/Text maps to read/write/neutral |
| Quick Definition | `definition` + file read | |
Native results win and are merged with LSP results by (file, range); duplicates are dropped.
Native resolve is preferred; the LSP is the fallback for tiers G0-G2.

`CI-NAV-060` SHOULD: LSP fallback of the table with de-duplication by (file, range). Acceptance:
with the native resolver disabled, `Ctrl+B` uses `definition`; with both, one result per target.

## 10. UX acceptance scenarios

**S-NAV-1, reference to declaration.** In `OrderService.java` the caret is on `repository` in
`repository.findAll()`. `Ctrl+B`: the caret moves to the field declaration `private final
OrderRepository repository;` in the same file (no new tab); the previous position is stored;
`Ctrl+Alt+Left` returns.

**S-NAV-2, several targets.** Caret on the call `process(order)` where the argument list matches
two overloads resolved ambiguously (or an interface method with a default and a generic super
method): `Ctrl+B` opens a popup titled `Choose Declaration` with two rows (method signature in the
name, grey container). `Down`, `Enter` navigates to the second.

**S-NAV-3, declaration shows usages.** Caret on a method declaration `findByName` used in two files.
`Ctrl+B`: the Show Usages popup opens at the caret, rows grouped by file, header `Usages of
findByName` with `3 usages`; `Enter` on a row navigates to that call. If the method has one
usage the popup does not appear: the editor jumps to it and a hint `It's the only usage in
Project Files` appears; with none, the hint `No usages found in Project Files`.

**S-NAV-4, Find Usages window.** `Alt+F7` on the same method: the Find tool window opens with a tab
`Usages of 'findByName'`, panel title `findByName in Project Files`, groups `Method call`/`Usage in
import` (usage type), module, package, file; double-click on a row opens the editor at the usage;
toolbar `Rerun` repeats the search.

**S-NAV-5, caret highlighting.** Caret in a local variable `total` used 4 times (2 writes, 2 reads):
all 5 name occurrences (declaration, 2 writes, 2 reads) are highlighted; the writes use the write
background; moving the caret to `total` in another place keeps the highlights without flicker; moving
to another variable switches them; typing in the method clears and recomputes after the daemon delay.

**S-NAV-6, explicit highlight.** `Ctrl+Shift+F7` on `total`: usages are highlighted persistently
and the hint reads `5 usages of 'total' found (press Ctrl+Shift+F7 again to remove the highlighting,
Escape to remove all highlighting)`; `F3` steps through them.

**S-NAV-7, nothing to go to.** `Ctrl+B` on a blank line: hint `Cannot find declaration to go to`.
