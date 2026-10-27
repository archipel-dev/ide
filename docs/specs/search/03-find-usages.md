# 03. Find Usages: user interface and behaviour

Status: specification, language independent. Requirement prefix `SRCH-USG`.
Source basis: IntelliJ Community tag `idea/262.10968.63`. Paths are cited as `ic: ...`.
Priorities: `MUST` / `SHOULD` / `LATER`. Executable cases: `cases/03-find-usages.cases.md`.

This file specifies the **user interface and user-visible behaviour** of Find Usages: commands and keys, how the target is chosen,
the options dialog, the Show Usages popup contents, the Find tool window tree (grouping, filters, node texts, actions) and the
persistent highlighting commands. The code-level semantics (what a usage is, searchers, access detection, Java options such as
derived classes, the hints of the immediate cases) are in `docs/specs/codeinsight/07-navigation.md` sections 5 to 8 and are
referenced as `CI-NAV-xxx`; nothing here relaxes them. Text search in files is `02-find-in-files.md`.

ic: `platform/lang-impl/src/com/intellij/find/actions/{ShowUsagesAction,ShowUsagesTable,ShowUsagesHeader,UsageOptionsDialog,FindUsagesAction,
FindUsagesInFileAction,ShowTargetUsagesActionHandler,UsageNavigation}.*`, `platform/lang-impl/src/com/intellij/find/findUsages/*Dialog.java`,
`platform/analysis-impl/src/com/intellij/find/findUsages/FindUsagesOptions.java`, `platform/usageView/resources/messages/UsageViewBundle.properties`,
`platform/usageView/src/com/intellij/usages/UsageViewSettings.kt`, `platform/usageView-impl/src/com/intellij/usages/impl/{UsageGroupingRuleProviderImpl,rules/*}`,
`platform/analysis-impl/resources/messages/FindBundle.properties`, `platform/lang-api/resources/messages/CodeInsightBundle.properties`.

## 1. Commands and keys

| Action id | Text | Default keys | Behaviour |
|---|---|---|---|
| `FindUsages` | Find Usages | `Alt+F7` | tool window; runs with the saved options |
| `ShowUsages` | Show Usages | `Ctrl+Alt+F7` | popup |
| `ShowSettingsAndFindUsages` | Find Usages Settings… | `Ctrl+Alt+Shift+F7` | options dialog, then the search |
| `FindUsagesInFile` | Find Usages in File | `Ctrl+F7` | highlights the usages in the file (alias of `HighlightUsagesInFile` while registry `ide.find.in.file.highlight.usages` is on, default on) |
| `HighlightUsagesInFile` | Highlight Usages in File | `Ctrl+Shift+F7` | toggles persistent highlighting |
| `GotoNextElementUnderCaretUsage` / `GotoPrevElementUnderCaretUsage` | Next / Previous Highlighted Usage | none | step through highlighted usages |
| `UsageView.Rerun` | Rerun | as `Rerun` (`Ctrl+F5`) | usage view toolbar |
| `UsageView.ShowRecentFindUsages` | Recent Find Usages | `Alt+Down` | in the usage view |
| `UsageView.Include` / `Exclude` / `Remove` | Include / Exclude / Remove | `Insert` / `Delete` / `Ctrl+X` | |
| `UsageFiltering.ReadAccess` / `WriteAccess` / `Imports` / `Comments` / `GeneratedCode` | Show Read Access / Show Write Access / Show Import Statements / Show Usages in Comments / Show Results In Generated Code | `Ctrl+R` / `Ctrl+W` / `Ctrl+I` / none / none | filter toggles (checked = shown); keys apply inside the usage view and the popup |
| `UsageGrouping.UsageType` / `Module` / `FlattenModules` / `Directory` (alias `Package`) / `DirectoryStructure` / `FileStructure` / `Scope` / `ShortFilePath` | grouping toggles | `Ctrl+Alt+T` / `Ctrl+Alt+M` / `Ctrl+Alt+O` / `Ctrl+Alt+P` / `Ctrl+Alt+D` / `Ctrl+Alt+F` / none / none | |
| `ShowUsageFeatures`, `SliceBackward`, `SliceForward` | Show Usage Features; Analyze Data Flow to/from Here | none | LATER (SRCH-USG-090) |
| `NextOccurence` / `PreviousOccurence` | Next / Previous Occurrence | `Ctrl+Alt+Down` / `Ctrl+Alt+Up` | in the usage view: select the next / previous usage and show it in the editor |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-001 | MUST | All actions of the table exist (except those marked LATER) with the keys given, are enabled when the caret or the context holds a searchable target and disabled in the gutter, in virtual space past the end of a line, and when the project is in dumb mode without index-free search. | keymap test |
| SRCH-USG-002 | MUST | **Target choice.** The target is the element at the caret: the declaration the reference at the caret resolves to, or the declared element when the caret is on a name. If several targets apply (for example an overloaded call that resolves to several declarations, or a name that declares and references) a list popup appears first, titled `Find Usages Of` for `Alt+F7` and `Show Usages Of` for `Ctrl+Alt+F7`, one row per target (icon, name, grey location), `Enter` selects (speed search on). No target: for editors a red hint under the caret, otherwise an error dialog titled `Error`, with the text `Cannot search for usages from this location.` + line break + `Place the caret on the element to find usages for and try again.` | cases USG-002/1..3 |
| SRCH-USG-003 | MUST | **Stale target.** When the element disappeared before the search could start: dialog `Cannot Search for Usages` with `Searched elements have been changed.` + line break + `Cannot search for usages.` | case USG-003/1 |
| SRCH-USG-004 | MUST | **Dumb mode.** While indexes are not ready the commands are disabled or answer with the notification `'Find Usages' will be available once project analysis is finished`. | covered by CI-NAV |
| SRCH-USG-005 | MUST | `Ctrl+Alt+Shift+F7` always shows the options dialog; `Alt+F7` and `Ctrl+Alt+F7` never show it and run immediately with the saved options of the target's kind (`Alt+F7` from the popup's gear does show it). | case USG-005/1 |

## 2. The options dialog

Title `Find Usages` (`Find Usages in File` when restricted to one file). First line: the presentable name of the target (`Class Foo`,
`Method foo(int)`, `Field count`, …). Then a group titled `Find:` with check boxes; a group `Scope:` with the scope chooser; in the footer
the check box `Open results in new tab` (when a previous tab could be reused; default off) and the buttons `Find`, `Cancel`, `Help`.

| Option | Text | Default |
|---|---|---|
| search for usages | `Usages` | on |
| search for text occurrences | `Text occurrences` | the value saved for the kind of target (the base option defaults to on; Java and Kotlin dialogs show the saved value, initially off, see open question 1) |
| skip results tab with one usage | `Skip results tab with one usage` | off (a global setting) |
| scope | `Scope:` combo | `Project Files`; the combo lists the predefined scopes (`Project Files`, `Project and Libraries`, `All Places`, `Module '<name>'`, `This Module`, `Open Files`, `Current File`, `Selection`, `Recently Viewed Files`, `Recently Changed Files`, named custom scopes) and `Edit Scopes…` |

Language specific options (Java examples, reference): for classes `Usages of classes and interfaces`, `Methods`, `Fields`, `Derived classes`,
`Derived interfaces`, `Implementing classes`; for methods `Usages`, `Overriding methods`, `Implementing methods`, `Overloaded methods`, `Search for
accessors`, `Search for base method usages`, `Implicit calls`; for fields `Usages`, `Search for accessors`, `Search for base methods of accessors`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-010 | MUST | `Find` is enabled only when at least one of `Usages` / `Text occurrences` (or a language option that produces results) is checked, and the scope is valid. The dialog commits its values to the saved options for the kind of target and runs the search. The chosen scope name becomes the default scope for later searches. | cases USG-010/1..2 |
| SRCH-USG-011 | MUST | The saved options are per kind of target (class, method, field, …), per application. The *Skip results tab with one usage* flag is global. | case USG-011/1 |
| SRCH-USG-012 | SHOULD | Keyboard: `Alt+<underlined letter>` toggles the option; `Enter` runs `Find`; `Esc` cancels. | UI test |

## 3. The Show Usages popup

The popup structure and the immediate cases (zero usage, one usage, all usages on one line) are `CI-NAV-040` to `CI-NAV-042`. This section adds the UI
details.

Popup content, top to bottom: the **header** (title with a spinner while searching, a status text, a pin toggle); the **toolbar** (the five filter toggles of
section 1, the `Show` menu with `Preview Source` and `Short File Path`, a scope chooser button unless the scope is one file, a gear `Settings…` with the key of
`ShowSettingsAndFindUsages` shown as `Settings…(Ctrl+Alt+Shift+F7)`); the **table**; the **preview** (a code view of about five or more lines under the table, shown when `Preview Source` is on, default on, splitter
proportion 0.6); a footer line.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-020 | MUST | **Appearance delay.** The popup is shown as soon as the first usage is found, or after 300 ms of searching, whichever is earlier; if the search ends sooner with a decisive result (immediate cases) the popup is never shown. | case USG-020/1 |
| SRCH-USG-021 | MUST | **Header status.** While searching, or when more usages exist than were collected: `N+ usages` (`0 usages`, `1 usage`; the plus is part of the text for every N >= 2). When finished and some usages are hidden by filters: `V of T usages`. Otherwise `N usages` (`0 usages`, `1 usage`). | cases USG-021/1..3 |
| SRCH-USG-022 | MUST | **Rows.** One row per usage: icon of the usage kind, the usage line with the reference highlighted (leading whitespace trimmed, long lines clipped around the usage), the location in grey (`FileName.ext`, plus the container when grouping by file structure is off). The row of the usage at the caret is marked `Current` in the location column. Same-line usages are merged into one row (`Merge Usages from the Same Line`, default on). | cases USG-022/1..2 |
| SRCH-USG-023 | MUST | **Order.** Rows are grouped by the active grouping rules (section 4: defaults usage type, module when several, directory/package, file structure) and inside a group ordered by file path then offset. Separator rows come last, in this order: the `… more usages` row, the `N usages are out of scope '<scope>'` row (`One usage is out of scope`), the `N usages were filtered out` row. | cases USG-023/1..2 |
| SRCH-USG-024 | MUST | **Page limit.** At most the usages page size (advanced setting `ide.usages.page.size`, default 100) usages are collected; then the search stops and a row `more usages` is added; activating it (Enter or click) continues the search for another page. The popup grows with the number of rows up to 30 rows without preview (20 with preview) and then scrolls. | case USG-024/1 |
| SRCH-USG-025 | MUST | **Keys in the table.** `Up`/`Down`/`PageUp`/`PageDown` move; `Enter` navigates to the selected usage (or opens the `Find Options…` / filtered-out / more row) and closes the popup; `Shift+Enter` navigates in the right split; `Alt+F7` opens all results in the Find window; `Ctrl+Alt+F7` again searches in the widest scope (next bullet); `Ctrl+Alt+Shift+F7` opens the options dialog; `Esc` closes; typing a letter filters the rows by speed search. Double-click navigates. | cases USG-025/1..2 |
| SRCH-USG-026 | MUST | **Scope widening.** The popup shows the advertisement `Press Ctrl+Alt+F7 again to search in '<scope>'` where `<scope>` is the widest scope the handler offers (for example `Project and Libraries`) as long as the current scope is narrower; pressing the key restarts the search in that scope and stores it as the default scope. | case USG-026/1 |
| SRCH-USG-027 | MUST | **Filtered out.** When filters hide rows, the row `N usages were filtered out` (singular `1 usage was filtered out`) with the tooltip `Some filters are configured to not show these usages. Click or press Enter to restart with filters toggled` appears; activating it switches the off filters on and reruns. | case USG-027/1 |
| SRCH-USG-028 | SHOULD | The `Preview Source` toggle (tooltip `Preview Source`) is remembered per project; the preview follows the selection (after 50 ms), shows the whole file region around the usage with all usages of the file highlighted, and shows `Select item to preview` when nothing is selected. | UI test |
| SRCH-USG-029 | MUST | Row click selects, double-click navigates. A navigated-to usage is selected in the editor (range of the usage selected, caret at its start, centred if it was off screen) and recorded in the navigation history. | case USG-029/1 |

## 4. The Find tool window (usage view)

A tab per search in the *Find* tool window, titled `Usages of '<name>'` plus `in <scope>` when the scope is not the default (the
full title text of the tool window panel is `<what> in <scope>`, for example `Usages of Foo in Project Files`). The view has the toolbar on the left (`Rerun`,
`Next Occurrence`, `Previous Occurrence`, `Expand All`, `Collapse All`, `Group By` menu, filters, `Preview Source`, `Export…`, settings), the tree, and an optional preview below
(split proportion 0.5).

### 4.1 Tree structure and grouping

The tree root shows `Usages` (`<N> results` in the counter) and has up to these levels, from the outside in. Each level exists only when its rule
is active; levels are ordered by the **rank** of their rule (low first):

| Rank | Rule (action) | Group node text | Default |
|---|---|---|---|
| 0 | non-code usages | `In Strings, Comments, and Text` (code usages under `In Code`; dynamic usages under `Dynamic`) | on when any non-code usage exists |
| 100 | scope (`UsageGrouping.Scope`) | the scope description | off |
| 200 | usage type (`UsageGrouping.UsageType`) | the usage type label (section 4.2) | on |
| 300 | module (`UsageGrouping.Module`), `Flatten Modules` | `Module:<name>` / `Module group:<name>` | on (the level has an effect only when the project has more than one module); flatten on |
| 400 | directory structure (`DirectoryStructure`) or package (`Directory`/`Package`) | package or directory path (compact middle directories off) | package on, directory structure off |
| 500 | file structure (`FileStructure`) | the file name, then the enclosing declarations (class, method) of the language, then the usage rows | on |

A node shows its text and, in a secondary colour, the number of usages below it (for example `Foo.java 3`). Leaf rows show the line number, the code text with the usage
highlighted. The `Short File Path` toggle (default on; effective only while the directory structure and package grouping are both off) prefixes the file name with a shortened path.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-030 | MUST | The grouping levels, ranks, defaults and node texts above. Each grouping toggle immediately regroups the tree without a new search, keeps the selection when possible and is saved in the application settings (`GROUP_BY_*`, `Flatten Modules`, `Compact middle directories`, `Short file path`). Turning all rules off shows a flat list of usage rows sorted by file path and offset. | cases USG-030/1..4 |
| SRCH-USG-031 | MUST | **Counters and status.** The tab and the root show the live count `N results` (`1 result`); while searching a spinner shows and the status text of the footer reads `Searching…`; after the search it reads `<N> usages found` (`no usages found`, `1 usage found`) and, when a page limit or filter applies, the filtered counts of section 3. | case USG-031/1 |
| SRCH-USG-032 | MUST | **Expansion.** After the first results arrive the tree auto-expands to the first usage and selects it unless the setting `Expand` (default off) is on, in which case all nodes expand. | case USG-032/1 |
| SRCH-USG-033 | MUST | **Selection syncing.** With *Autoscroll to Source* (`Navigate with Single Click`, default off) selecting a usage opens it in the editor without moving focus; with the preview on the preview follows the selection. `Enter`/`F4`/double-click always navigate and focus the editor. | case USG-033/1 |

### 4.2 Usage type labels

The usage type of a usage is, in order: `Usage in comments` when the element lies in a comment; the label from the language's provider; else `Value write` /
`Value read` from the access (writes win over reads); else `Unclassified`. Labels the Java provider produces (reference): `Usage in instanceof`, `Usage in import`, `Usage in cast target type`,
`Usage in extends/implements clause`, `Usage in permits clause`, `Class static member access`, `Nested class access`, `Method throws list`, `Usage in .class`, `Field declaration`,
`Local variable declaration`, `Method parameter declaration`, `Catch clause parameter declaration`, `Method return type`, `Usage in string constants`, `New instance creation`,
`New anonymous inheritor creation`, `New array creation`, `Annotation`, `Type parameter`, `Pattern`, `Recursion`, `Delegate to another instance method`, `Delegate to super method`
(with `, parameters changed` variants). Groups at one level are ordered by label ascending (open question 2).

### 4.3 Filters

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-040 | MUST | `Show Read Access` / `Show Write Access` hide usages that only read / only write the target (a read+write usage is shown while either is on, hidden when both are off); `Show Import Statements` hides usages inside import/using/use statements; `Show Usages in Comments` hides usages inside comments and documentation; `Show Results In Generated Code` hides usages in generated files. The toggles filter the existing results (no new search); a hidden usage does not count in the status text. The view's *Filtered out* summary (popup only, 027) lets the user undo. | cases USG-040/1..3 |
| SRCH-USG-041 | SHOULD | `Merge Usages from the Same Line` (default on) shows one row per source line with all usages of the line highlighted; with it off each usage is a row. | case USG-041/1 |
| SRCH-USG-042 | SHOULD | `Sort Alphabetically` (view setting, default off) sorts the rows of the file-structure level by name instead of source order. | LATER cases |

### 4.4 Actions on usages

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-050 | MUST | `Exclude` (`Delete`) marks the selected usage(s) or group(s) as excluded: the rows are drawn struck through / dimmed with the word `excluded`; `Include` (`Insert`) reverts; excluded usages are skipped by refactorings and by *Replace*. `Remove` (`Ctrl+X`) removes the rows from the view (they come back after Rerun). Each acts on a group's whole subtree. | cases USG-050/1..2 |
| SRCH-USG-051 | MUST | `Rerun` repeats the search with the saved options and replaces the results. When the target was invalidated the dialog `Cannot Re-Run Search` / `Targets have been invalidated` appears. After a refactoring or text changes that invalidated usages the view asks `Would you like to rerun the search now?` (title `Changes Detected`). | case USG-051/1 |
| SRCH-USG-052 | MUST | `Next Occurrence` / `Previous Occurrence` select the next/previous usage row in tree order (wrapping off: at the ends nothing happens) and open it in the editor; `Expand All` / `Collapse All` act on the tree. | case USG-052/1 |
| SRCH-USG-053 | SHOULD | **Recent Find Usages** (`Alt+Down` in the view): a list popup titled `Recent Find Usages` of earlier searches in the form `<kind> '<name>' in <scope>` (for example `Method 'foo' in Project Files`), newest first; choosing one reruns it; empty: `No Recent Find Usages Entries`. | case USG-053/1 |
| SRCH-USG-054 | SHOULD | **Export**: writes the tree as text to a file chosen in a dialog (default name `report.txt`) or the clipboard. | LATER cases |
| SRCH-USG-055 | MUST | A search that finishes with exactly one usage and the option `Skip results tab with one usage` on navigates directly and shows no tab; with no usages a balloon/hint `Usages of <kind> <name> not found` (or `… not found in '<scope>'` when the scope is not the default) is shown instead of an empty tab. | cases USG-055/1..2 |
| SRCH-USG-056 | MUST | **Tabs.** A new search replaces the content of the current Find tab unless the tab is pinned or `Open results in new tab` is on, in which case a new tab is created; tabs close individually; the last search of a closed tab is kept in Recent Find Usages. | case USG-056/1 |

## 5. Highlighting commands

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-060 | MUST | `Ctrl+Shift+F7` and (by default) `Ctrl+F7` behave as `CI-NAV` 7.2 and 7.3: highlight the usages in the current file with the search-result attributes, mark them on the scrollbar stripe, show the status bar / hint `N usage(s) of <name> found (press Ctrl+Shift+F7 again to remove the highlighting, Escape to remove all highlighting)`, `No usages of '<name>' found` or `No usages found` when there is no name. Pressing the key again on the same element removes the highlighting; `Esc` removes all; typing removes them. | cases USG-060/1..3 |
| SRCH-USG-061 | SHOULD | Stepping through highlighted usages: `GotoNextElementUnderCaretUsage` / `GotoPrevElementUnderCaretUsage` (no default keys) move the caret to the next/previous highlighted usage; at the last one the hint `No more highlights found` appears (no wrap). | case USG-061/1 |

## 6. Settings

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-070 | MUST | Persisted usage-view settings with defaults (`UsageViewSettings`): group by file structure on, module on, package on, usage type on, scope off, directory structure off; *Flatten modules* on; *Compact middle directories* off; *Short file path* on; *Preview usages* on; *Replace preview usages* on; *Filter duplicated line* (merge same line) on; *Sort alphabetically* off; *Autoscroll to source* off; *Expanded* off; preview splitter proportion 0.5. | settings test |
| SRCH-USG-071 | MUST | Advanced settings: `ide.usages.page.size` (default 100; text `Maximum number of results to show in Find in Files/Show Usages preview`, description `Increasing this limit can significantly increase IDE memory usage`); registry `ide.usages.popup.show.options.string` (default on: a text with the search options is shown in the popup header). | settings test |

## 7. Inventory coverage (`find-usages.md`)

`HighlightUsagesInFile`, `ShowUsages`, `ShowSettingsAndFindUsages`: SRCH-USG-001/060/025/005; `ShowUsageFeatures`, `SliceBackward`, `SliceForward`: LATER
(SRCH-USG-090, data-flow analysis and usage clustering are separate features); `UsageFiltering.*` (5): 040; `UsageGrouping.*` (8): 030; `UsageView.Exclude`,
`Include`, `Remove`: 050; `UsageView.Rerun`: 051; `UsageView.ShowRecentFindUsages`: 053.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-USG-090 | LATER | *Similar Usages* (usage clustering), `Show Usage Features`, `Analyze Data Flow to/from Here`. Reason: separate analysis features, not part of the first release. | n/a |

## 8. Open questions

1. Default of `Text occurrences`: the base options class defaults to on while `CI-NAV` 5.1 says off in the UI; the Java and Kotlin dialogs persist the last choice. Confirm
   the first-run value with a recording (this file assumes off for code targets).
2. Order of groups inside one grouping level (for example the usage-type groups). The group classes order themselves by their own comparison; this file assumes label
   ascending with `Unclassified` and non-code groups last. Confirm.
3. The exact popup title text of Show Usages (`Usages of <name>` plus the scope in the status text). The title in the reference is built by the target handler.
4. Whether `Ctrl+X` is the key of `Remove` (the inventory says "as `$Cut`" which maps to `Ctrl+X` in the default keymap).
