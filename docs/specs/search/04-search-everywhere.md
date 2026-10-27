# 04. Search Everywhere and the Go to commands

Status: specification, language independent. Requirement prefix `SRCH-SE`.
Source basis: IntelliJ Community tag `idea/262.10968.63` (the *split* Search Everywhere, `search.everywhere.new.enabled` default on, and the
classic contributors it adapts). Paths are cited as `ic: ...`.
Priorities: `MUST` / `SHOULD` / `LATER`. Executable cases: `cases/04-search-everywhere.cases.md`.

Search Everywhere (SE) is one popup with several *tabs*; `Go to Class`, `Go to File`, `Go to Symbol` and `Go to Action` are the same popup opened on the matching tab.
The name matching and the scores of classes, files and symbols are in `07-matching-and-ranking.md`; this file specifies the popup, the tabs, the mixing and ordering of results,
the action search, history, navigation and the Text tab. The code-level semantics of Go to Declaration are in `docs/specs/codeinsight/07-navigation.md`.

ic: `platform/lang-impl/src/com/intellij/ide/actions/{SearchEverywhereAction,SearchEverywhereBaseAction,GotoActionBase,GotoFileItemProvider}.*`,
`platform/lang-impl/src/com/intellij/ide/actions/searcheverywhere/*`, `platform/searchEverywhere/{frontend,backend,shared}/src/**`,
`platform/lang-impl/src/com/intellij/ide/util/gotoByName/{GotoActionModel,ActionAsyncProvider,ActionSearchUtil,ChooseByNamePopup,DefaultChooseByNameItemProvider}.*`,
`platform/lang-impl/src/com/intellij/find/impl/TextSearchContributor.kt`, `platform/platform-api/resources/messages/IdeBundle.properties` (keys `searcheverywhere.*`,
`search.everywhere.*`), `platform/platform-resources-en/src/messages/ActionsBundle.properties`, `platform/util/resources/misc/registry.properties`.

## 1. Commands and keys

| Action id | Text | Default keys | Opens |
|---|---|---|---|
| `SearchEverywhere` | Search Everywhere | `Shift Shift` (two quick presses) | tab *All* |
| `GotoClass` | Go to Class… | `Ctrl+N` | tab *Classes* |
| `GotoFile` | Go to File… | `Ctrl+Shift+N` | tab *Files* |
| `GotoSymbol` | Go to Symbol… | `Ctrl+Alt+Shift+N` | tab *Symbols* |
| `GotoAction` | Find Action… | `Ctrl+Shift+A` | tab *Actions* |
| `TextSearchAction` | Text… | `Ctrl+Alt+Shift+E` | tab *Text* (when enabled) |
| `GotoRelated` | Related Symbol… | `Ctrl+Alt+Home` | a list popup of related elements (language feature, see CI-NAV) |
| `GotoLine` | Go to Line:Column… | `Ctrl+G` | `05-recent-and-structure-popups.md` |

Inside the popup:

| Action id | Text | Keys |
|---|---|---|
| `SearchEverywhere.NextTab` / `PrevTab` | Next Tab / Prev Tab | `Tab` / `Shift+Tab` (also the platform next/previous tab keys) |
| `SearchEverywhere.SelectItem` | Select Item | `Enter` (any of `Shift`, `Ctrl`, `Alt` added: the modifier is passed to the action) |
| `SearchEverywhere.NavigateToNextGroup` / `PrevGroup` | Next / Prev Group Element | `PageDown` or `Ctrl+Down` / `PageUp` or `Ctrl+Up` |
| `SearchEverywhere.CompleteCommand` | Complete Command | `Tab` while a `/command` prefix is typed |
| `ShowSearchHistory` (in the field) | Show Search History | `Alt+Down` (older) / `Alt+Up` (newer) |
| `OpenInRightSplit`, `EditSourceInNewWindow` | open in right split, in new window | `Shift+Enter`, `Shift+F4` |
| `Se.IndexFiles`, `SeNewSearchToggleAction`, `SeSetJevKeyAction`, `SeSetOpenAiKeyAction`, `SearchEverywhereComponent`, `SearchEverywhereNewToolbarAction`, `ToggleSearchEverywhereMlSorting`, the ML feature/typo debug actions | internal and experimental | none (LATER, section 12) |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-001 | MUST | The actions of the first table exist with those keys, are available without a project too (on the welcome screen only *All*, *Actions* and project-independent tabs), and are listed in the *Navigate* menu and the main toolbar search button (tooltip `Search Everywhere` + the shortcut, `Searches for: Classes, files, tool windows, actions, and settings`). The three navigation tabs need a project and are disabled in dumb mode with the empty text of SRCH-SE-024. | keymap test |
| SRCH-SE-002 | MUST | **Double Shift.** Two presses of `Shift` within the platform double-click interval, with no other key and no mouse click in between, open SE on the *All* tab. Holding Shift or Shift with another key never counts. The advanced setting `ide.suppress.double.click.handler` turns the gesture off. A third quick press does not toggle. | case SE-002/1 |
| SRCH-SE-003 | MUST | **Tab shortcuts while open.** Pressing the shortcut of the tab that is already selected toggles that tab's *everywhere* filter (the *Include non-project items* switch of section 3) instead of reopening; pressing the shortcut of another tab switches to that tab, keeping the typed text. | cases SE-003/1..2 |
| SRCH-SE-004 | MUST | **Initial text.** Priority: a predefined text from the context; the editor selection if single-line (selected in the field); the text typed in a focused speed search; otherwise empty. When no text results and the history of the tab is not empty, the field is filled with the **most recent history entry of that tab**, selected (so typing replaces it). History can be switched off by the system property `idea.searchEverywhere.noHistory` or, for the All tab, by registry `search.everywhere.disable.history.for.all`. | cases SE-004/1..3 |

## 2. The popup

Layout: a rounded popup centred above the editor with: the **header** (the tabs row and, at its right, the filter controls of the selected tab); the **search field** (placeholder
`Search Everywhere` on All; clear button; a hint line shown below for the first uses: `Press Shift+Enter to open the file in the right split`, `Press Shift+F4 to open the file in a new window`,
`Press Alt+Up or Alt+Down to navigate through the search history`); the **result list**; an optional **preview** (the *Preview* toggle, default on for All and Files); the **footer** with
the advertisement and, for the Actions tab and the All tab, the *extended info* line (the description of the selected action).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-010 | MUST | **Tabs.** In header order: `All`, `Classes`, `Files`, `Symbols`, `Actions`, and `Text` when the text search is enabled (advanced setting `se.text.search`, `Show text search results in Search Everywhere`; the *All* tab then also contains text matches). The first tab's name for a language is the language's term for *class* (`Classes` for Java; for example `Types`), plural, from the same source as the Go to Class menu text. `Tab` / `Shift+Tab` cycle through the tabs with wrap-around; clicking a tab selects it; the typed text stays. The selected tab is remembered per opening (the popup always opens on the tab of the invoking action). | cases SE-010/1..2 |
| SRCH-SE-011 | MUST | **Filters per tab.** *All*: the scope switch `Include non-project items` (default off; Alt mnemonic `Alt+P`), a *Preview* toggle (`Preview`, description `Show Preview`) and a *type filter* menu listing the providers (the entries are the provider names `Classes`, `Files`, `Symbols`, `Actions`, `Top Hit`, `Recent Files`, `Text`...) with check marks, unchecked providers are excluded from All (persisted). *Classes*, *Symbols*: the scope chooser (default `Project Files`; entries as in `03-find-usages.md` section 2, with `All Places`) or the switch `Include non-project items`. *Files*: scope chooser and a file-type filter (menu of file types with check marks, `Directory` first). *Actions*: the check box `Include disabled actions` (mnemonic `Alt+D`, default off). *Text*: section 11. | cases SE-011/1..3 |
| SRCH-SE-012 | MUST | **Auto-widening.** If a search on a tab with the *everywhere* filter off ends with an empty list for a non-empty pattern, and the user has not toggled the filter by hand, the filter switches **on** automatically and the search reruns; the pattern that failed is remembered. When the user later edits the text so it no longer contains the remembered pattern (the remembered pattern must be longer than one character), the filter switches back off. A manual toggle disables the automatism for the popup. | cases SE-012/1..2 |
| SRCH-SE-013 | MUST | **Compact mode.** With an empty field the popup shows only the header, the field and (on All and Files) the recent files list; typing expands it to the full height. Minimum width 700 px (registry `search.everywhere.new.minimum.width`); size and position are remembered. | UI test |
| SRCH-SE-014 | MUST | **Closing.** `Esc` closes (first `Esc` clears an open sub-popup); a click outside or a window switch closes (except on Wayland); selecting an item closes the popup unless the contributor keeps it open (actions that open a dialog close first); the search is cancelled, the text and the selected item are saved for the next opening (SRCH-SE-026). | case SE-014/1 |
| SRCH-SE-015 | SHOULD | **History.** Per tab the last 50 distinct texts are kept (newest last, a repeated text moves to the end); the *All* tab shows the last 50 distinct texts of all tabs. A text is saved when the popup closes with a non-empty field. `Alt+Down` puts the older entry in the field (selected), cycling to the oldest and wrapping; `Alt+Up` the newer. The history lives for the IDE session (not persisted). | cases SE-015/1..2 |

## 3. Include non-project items

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-016 | MUST | The scope has two named states for the navigation tabs: **project** (default: files in project content, classes and symbols of project sources and their test sources; the `GlobalSearchScope` project scope) and **everywhere** (adds libraries, SDK classes, and non-project files such as excluded dirs unless ignored). For *Files* the *everywhere* state also lets an absolute path or a `~/` path typed in full point to files outside the project (registry `search.everywhere.absolute.path.outside.project`). The *Files* tab and the *All* tab share the file scope. The state of the switch is remembered per tab for the project. | cases SE-016/1..2 |

## 4. Running a search and ordering the results

Each tab has a set of **providers** (the *All* tab: all of them; *Classes*: classes; *Files*: files; *Symbols*: symbols; *Actions*: actions and options; *Text*: text). Every provider emits found
items, each with a **weight** (an integer; for name searches the matching degree of `07`), asynchronously. The list merges them.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-020 | MUST | **Restart.** Every change of the text, the tab or a filter cancels the running search and starts a new one; the previous list stays until the first new item arrives (then it is replaced). While nothing is found yet the list shows `Searching…`. An empty text on a tab that needs a pattern shows no list. Rapid typing is coalesced (searches for intermediate texts may be skipped). | cases SE-020/1..2 |
| SRCH-SE-021 | MUST | **Order of the mixed list** (the *All* tab and any tab with several providers): (1) pinned providers first, in this order: calculator, completion command, commands, top hit (client), top hit, and *Recent Files* when the advanced setting `search.everywhere.recent.at.top` is on (default off); (2) among items of the same provider an **exact match** (the item's name equals the typed text ignoring case, when registry `search.everywhere.exact.match.priority` is on, default on) before others; (3) higher weight first; (4) equal weights: lower provider sort weight first (`Recent Files` 70, `Top Hit` 50, `Classes` 100, `Files` 200, `Symbols` 300, `Run Configurations` 350, `Actions` 400, `Text` 1500), then provider emission order. A single-provider tab uses (2) to (3) only, then the order of the provider's own list (`07` SRCH-MAT-041). | cases SE-021/1..4 |
| SRCH-SE-022 | MUST | **No duplicates.** Two items are the same when they are the same file / the same class element / the same action id / the same settings option; the later or weaker duplicate is dropped: an action found both as an action and as an option stays once as the action; a file found as *Recent File* and as *File* stays once (the *File* item replaces the recent one unless `search.everywhere.recent.at.top` is on). | cases SE-022/1..2 |
| SRCH-SE-023 | MUST | **Stability and paging.** After 800 ms of a search the list is **frozen**: items that arrive later are inserted only below the rows already shown (exact matches excepted when they have the empty-offset rule), so selecting and clicking never meets a moving row. The list loads further items while the user scrolls to the end (a `… loading…` row marks the end while loading, `… more` if the provider has more beyond its section limit; the classic section limits are 15 per provider on All and 30 on a single-provider tab). Items from one coalesced batch (10 first, then 20) are added together. | case SE-023/1 |
| SRCH-SE-024 | MUST | **Empty list texts.** Search still running: `Searching…`. Single-provider tab with pattern: `No <group> with such a name found` (group in lower case: `No classes with such a name found`, `No files with such a name found`, `No symbols…`, `No actions…`). All tab: `Nothing found`. In dumb mode, a navigation tab: `Search in '<tab>' will be available once project analysis is finished` (and `Search in '<tab>' results may be incomplete, analyzing the project…` when partial). A line `No exact matches found. Here are similar results` heads a list that contains only fuzzy matches. When the *everywhere* filter is off the text adds `Set scope to <scope>` (a link); when filters are active `Reset filter`; on All and Files with text search available `Use Find in Files or use Text tab to perform full-text search` style links (`Find in Files`, `to perform full-text search`). | cases SE-024/1..3 |
| SRCH-SE-025 | MUST | **Selection.** After the first results arrive the first row is selected; selection never changes on its own afterwards (no jump while results arrive), except when the previous selection (SRCH-SE-026) reappears among the visible rows. `Up`/`Down` move by row, `Ctrl+Up/Down` or `PageUp/PageDown` jump between the first rows of provider groups (the next row whose provider differs from the current's), `Shift+Up/Down` extend the selection (multi-select). Mouse hover does not select. | cases SE-025/1..2 |
| SRCH-SE-026 | SHOULD | **Restoring.** Reopening the same tab with the same text re-selects the item that was selected when the popup closed, if it is among the visible results. | case SE-026/1 |

## 5. Providers

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-030 | MUST | **Classes.** Items are classes (language dependent: classes, interfaces, enums, records, objects, type aliases). Candidate names come from the project's name index. Matching, qualified patterns (`pkg.Name`, `Outer#member`, `Outer$1`), line suffix and ordering: `07` SRCH-MAT-040 to 044. Row: icon, class name with matched fragments bold, the container/package in grey (`(com.acme.util)`), the module/library in the far right in grey for non-project items. Scope: project / everywhere. Duplicate short names appear as separate rows ordered by SRCH-MAT-044. | cases SE-030/1..3 |
| SRCH-SE-031 | MUST | **Files.** Items are files and directories. Pattern rules: `07` SRCH-MAT-045/046 (directory part, name part, tiers). Additional rules: a typed text that is an **absolute path** (or a path relative to the project directory, or `~/…`, or one starting `./`) that exists inside the project content or libraries (or anywhere with `search.everywhere.absolute.path.outside.project`) gives that file with the **exact** weight (15000) first; matching trailing `/` selects directories only; names are compared case-insensitively. Directories sort after files of the same weight and show `Directory` as file type. Row: file icon, name with fragments, the path relative to the project in grey. The *Files* tab accepts an empty pattern and then lists recent files (SRCH-SE-033). The `File type` filter limits the types shown. | cases SE-031/1..4 |
| SRCH-SE-032 | MUST | **Symbols.** Items are classes' members and top-level symbols (methods, fields, functions, properties, constants, enum constants, type parameters excluded). `Class#member` and `Class.member` qualified forms and the line suffix follow `07` SRCH-MAT-043/044. The row shows the symbol name with its signature/type in grey and the container `Class` in grey right. Scope as Classes. | cases SE-032/1..2 |
| SRCH-SE-033 | MUST | **Recent Files.** On the *All* and *Files* tabs the editor history (most recently used first; duplicates removed; files currently selected in an editor window excluded, except on the welcome screen) supplies items filtered by the typed text as a name matcher `"*" + text` with prefer-start; the weight is (name match degree, then recency: newer higher). With an empty text the whole history appears in recency order (the first recent file that is not the current one is selected by default). Group name `Recent Files`. | cases SE-033/1..2 |
| SRCH-SE-034 | SHOULD | **Top Hit.** On *All* the settings pages and options that match the text as an action-style search (section 7, names from the settings search index) and commands appear as `Top Hit`; their weights are reduced by 100 (settings penalty). Selecting an option row toggles it (boolean options show a check box) or opens the settings page at that option. | cases SE-034/1..2 |
| SRCH-SE-035 | LATER | *Run Configurations* provider (`/run name`, `/debug name` commands to start a configuration), *Calculator* (`2+2` shows `Result: 4`, `Enter` or `Ctrl+C`-style shortcut copies it, text `{0} to copy to the clipboard`), *Autocompletion* of commands. | n/a |

## 6. Navigating with the result

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-050 | MUST | `Enter` on the selected row (several rows when multi-selected) runs the provider's action: classes, files and symbols **open in the editor** (an already open file's tab is reused, the navigation is added to the history) and close the popup; actions run (SRCH-SE-045); options toggle or open settings; recent files open. `Shift+Enter`: the file opens in the **right split**; `Shift+F4`: in a **new window**; `Ctrl` or `Alt` with `Enter` are passed to the action (for example run vs debug in run configurations). Double-click equals `Enter`. Several selected rows open each. | cases SE-050/1..3 |
| SRCH-SE-051 | MUST | **Line and column.** If the typed text carries a position suffix (`07` SRCH-MAT-042), the first selected item's **file** opens with the caret at that line and column (1-based in the text; line clamped to the document; a missing column is 1; a column without a line goes to line 1), for classes and symbols as well as files. | cases SE-051/1..3 |
| SRCH-SE-052 | MUST | **Member navigation.** For `Class#member`, after opening the class the caret goes to the first member of that class whose name matches the member part (prefix, case-insensitive). | case SE-052/1 |
| SRCH-SE-053 | SHOULD | **Preview.** With the preview on, the selected result is shown in a read-only code view below the list (for files the file start or, for symbols and text results, the region around the target with the match highlighted); it updates 50 ms after the selection stops moving and hides for rows without a preview. | UI test |
| SRCH-SE-054 | SHOULD | **Open in Find window.** The footer action `Open in Find Window` (and `Ctrl+Alt+Enter`-style key shown in the advertisement) shows all results of the tab in the Find tool window with the title `Matches for '<text>' in [<tab names>]` (or `Unsorted results` for unsorted sources). | LATER cases |

## 7. The Actions tab and action search

An **action** row shows: the action's icon (or a check box for toggle actions: checked state shown), its text, the group path in grey (`Find in Files…   Edit | Find`: the best group name), and the shortcut text at the
right; a disabled/unavailable action (not enabled in the current context) is drawn grey with a trailing marker and is listed only when *Include disabled actions* is on or when it is the only kind of result.
Invisible actions (presentation not visible) are hidden when the registry `search.everywhere.hide.invisible.actions` is on (default off).

**Name match for filtering** (`actionMatches`), a candidate action passes when **one** of these matches, tried in order, and the first that matches sets its *match mode*:

| Mode (rank) | Rule |
|---|---|
| `NAME` | the action text (with search-place text overrides) matches the pattern: pattern without a space: `07` matcher with `"*" + pattern`, IGNORE_CASE; pattern with a space: *word-prefix* matching: split the pattern and the name at whitespace, `-` and `/`; every pattern word must be a case-insensitive prefix of at least one name word (any order) |
| `SYNONYM` | any synonym of the action matches by the same rule |
| `DESCRIPTION` | the description (when it differs from the text) matches by word-prefix |
| `GROUP` | `<group> <text>` or `<text> <group>` of any of its menu groups matches (for toggle actions the first form counts as `NAME`) |
| `NON_MENU` / `INTENTION` | actions that are not in menus; intention actions (when included) |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-040 | MUST | **Candidates.** All registered actions (including children of non-popup groups), their abbreviations (user-defined and keymap abbreviations: an abbreviation that equals the pattern ignoring case matches), and when on *All*: intentions and settings options (`Top Hit`). Actions without text, and (default registry `go.to.action.filter.out.inspections` on) inspection toggles, are excluded. | cases SE-040/1..2 |
| SRCH-SE-041 | MUST | **Weight** of a match = the matching degree of the **text with leading and trailing non-alphanumeric characters removed** (so `Find…` is `Find`) using the matcher `"*" + pattern`, IGNORE_CASE, with the prefer-start bonus (`07` SRCH-MAT-034); if the degree is zero or no match, the degree of the action's original English text is used; `+100` when the pattern, trimmed, contains a space; settings options: `-100` (not below 0); when no weight can be computed (word-prefix match without a camel-hump match) the weight falls back to a rank: `3` when the text without trailing `...`/`…` equals the pattern ignoring case, `2` when it starts with it, `1` when it contains it, `0` otherwise, plus `1` for an action that is not a group. | cases SE-041/1..4 |
| SRCH-SE-042 | MUST | **Order of equal weights** on the Actions tab (and the Actions provider on All): (1) available (enabled) actions before boolean options before unavailable actions before non-boolean options; (2) match mode order `NAME`, `DESCRIPTION`, `GROUP`, `NON_MENU`, `SYNONYM`; (3) action text ascending ignoring case and a trailing `…`; (4) shorter text first; (5) the group name ascending. Then SRCH-SE-021. | cases SE-042/1..2 |
| SRCH-SE-043 | MUST | **Abbreviations** come first: an action whose abbreviation equals the typed text (ignoring case) is listed with the abbreviation as its text and the weight of the abbreviation match. | case SE-043/1 |
| SRCH-SE-044 | MUST | **Row details.** Shortcut text uses the platform's text for the active keymap (`Ctrl+Shift+A`; macOS symbols). The group column shows the first menu group path that contains the action (`Edit | Find`), omitted when it equals the action's own text. Toggle actions show a check box that reflects the current state and toggles it without closing the popup. | cases SE-044/1..2 |
| SRCH-SE-045 | MUST | **Executing** an action row: the popup closes first, then the action runs in the data context the popup was opened from (the focused editor, project view selection, ...). If the action is disabled in that context a hint `Action <name> is disabled` is not shown; the row for a disabled action is only listed with *Include disabled actions* and does nothing but run the action's update to display the platform's own refusal. An action that opens a popup or dialog runs after the close. A toggle action toggles. | cases SE-045/1..2 |
| SRCH-SE-046 | MUST | **Include disabled actions** (mnemonic `Alt+D`, default off, remembered): off hides unavailable actions; on lists them after the available ones of equal weight (rule 042 (1)). | case SE-046/1 |

## 8. History, restore, and the field

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-056 | MUST | The field is a single-line text field; `Ctrl+A` selects all; Esc closes; pasting multi-line text keeps the first line. A leading and trailing whitespace is not trimmed for matching except that a trailing space switches to *full word* matching (`07` SRCH-MAT-020). The field's context menu offers the standard edit actions. | UI test |

## 9. Commands in the field

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-060 | SHOULD | Typing the **top hit accelerator** (`/`) as the first character lists the available commands (rows with the command name and description, ordered by whether the name starts with the typed text, then ascending); `Tab` completes the selected command; `/<command> <args>` limits the search to that command's provider (`/plugins <text>` lists plugin commands, text `Plugins: {0}`; run configuration commands start a configuration, descriptions `Start Run Configuration in 'Run' mode` / `… in 'Debug' mode`). The field hint reads `Type / to see commands`. | cases SE-060/1..2 |

## 10. Settings (advanced and registry)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-070 | MUST | Advanced settings (group *Search Everywhere*): wait for all contributors before showing results (`search.everywhere.wait.for.contributors`, default off; when on, rows do not move; timeout `search.everywhere.contributors.wait.timeout`, ms); show notifications above results (`search.everywhere.show.results.notification`); show recent files at the top (`search.everywhere.recent.at.top`, default off); show text search results (`se.text.search`). Registry: `search.everywhere.new.enabled` (default on), `search.everywhere.new.minimum.width` (700), `search.everywhere.exact.match.priority` (on), `search.everywhere.qualified.symbols.exact.match.priority` (on), `search.everywhere.hide.invisible.actions` (off), `search.everywhere.prioritise.actions.with.shortcuts` (off; when on, +100 weight to actions whose text starts with the pattern and that have a shortcut), `ide.goto.middle.matching` (on), `ide.goto.file.include.directories` (on), `search.everywhere.fuzzy.*` (off). | settings test |

## 11. The Text tab

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SE-061 | SHOULD | **Text tab.** The tab searches file contents with the engine of `02-find-in-files.md`. The text field has the three toggles Match case, Words, Regex (inside the field; icon buttons as in the find bar); the header has a **file type** combo (`All file types` and the file types) and the scope chooser; a *Preview* toggle. Rows show the line text with the match highlighted, the file name and `:line`, grouped by nothing (flat), sorted by file path then offset; weight `1500` base so text results sort after name results on *All*; they are not shown on *All* when the typed text is shorter than 3 characters (to avoid noise; see open question). Selecting opens the file at the match and selects it. A footer action `Open in Find Window`. | cases SE-061/1..2 |

## 12. Not specified here (LATER) and open questions

LATER with reasons: the split-mode remote variants, ML sorting and features (`ToggleSearchEverywhereMlSorting`, feature dumps), typo/spelling correction, Lucene index
(`Se.IndexFiles`), AI rankers (`SeSetOpenAiKeyAction`, `SeSetJevKeyAction`): experimental and product specific; Run Anything (`Ctrl Ctrl`): separate feature; the calculator and
autocompletion providers; Pinyin and fuzzy file search.

Open questions:

1. Double-shift timing: the platform double-click interval or a fixed 500 ms.
2. Group headers: the classic SE showed a header row per provider group; the split SE shows a flat mixed list. This file specifies the flat list; confirm with a recording.
3. Whether the Text results are suppressed for patterns shorter than 3 characters (the reference excludes them on All through the provider's own minimum; confirm the number).
4. The exact row layout widths (icon column, location column) belong to the design measurements in `docs/component-map.md`.
