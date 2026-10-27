# 02. Find in Files and Replace in Files

Status: specification, language independent. Requirement prefix `SRCH-FIP`.
Source basis: IntelliJ Community tag `idea/262.10968.63`. Paths are cited as `ic: ...`.
Priorities: `MUST` / `SHOULD` / `LATER`. Executable cases: `cases/02-find-in-files.cases.md`.

Scope: the *Find in Files* / *Replace in Files* popup, the scopes (project, module, directory, named scope), file masks, filters,
the live results list with preview, *Open in Find Window*, replace with confirmation, and what files are searched. The matching of
text itself (case, words, regex, replacement templates, preserve case) is identical to the editor bar and is defined in
`01-find-replace-in-file.md` sections 3 and 5; the Find tool window (usage view) is `03-find-usages.md` section 4.

ic: `platform/lang-impl/src/com/intellij/find/impl/{FindPopupPanel,FindPopupScopeUIImpl,FindPopupHeader,FindPopupResultsAutoloadHandler,
FindInProjectUtil,FindInProjectTask,FindManagerImpl}.java|kt`, `.../find/findInProject/FindInProjectManager.java`,
`.../find/replaceInProject/ReplaceInProjectManager.java`, `.../find/actions/{FindInPathAction,ReplaceInPathAction,FindSelectionInPathAction}`,
`platform/analysis-impl/resources/messages/FindBundle.properties`, `platform/util/src/com/intellij/util/PatternUtil.java`.

## 1. Actions and keys

| Action id | Text | Default keys | Notes |
|---|---|---|---|
| `FindInPath` | Find in Files… | `Ctrl+Shift+F` | project view, navigation bar, editor and Edit menu; context decides the preset scope |
| `ReplaceInPath` | Replace in Files… | `Ctrl+Shift+R` | same popup in replace mode |
| `FindSelectionInPath` | Find in Files | as `FindInPath` | editor context menu entry |
| `ActivateFindToolWindow` | Find | none | shows the Find tool window |
| `UsageView.ShowRecentFindUsages` | Recent Find Usages | `Alt+Down` in the results view | `03-find-usages.md` |

In the popup:

| Key | Effect |
|---|---|
| `Enter` | find mode: open the selected result in the editor and close the popup; replace mode: *Replace* the selected result(s) |
| `Ctrl+Enter` | *Open in Find Window* (find mode) or open the replace preview in the Find window (replace mode) |
| `Shift+Enter` | open the selected result in the right split |
| `F4` | navigate to the selected result (like `Enter`) |
| `Alt+Shift+Enter` | *Replace All* (replace mode) |
| `Alt+Shift+Backspace` | reset all filters |
| `F3` / `Shift+F3` | select the next / previous result in the list (the keys of *Find Next* / *Find Previous*) |
| `Up` / `Down` / `PageUp` / `PageDown` | move the selection in the result list while the focus stays in the search field |
| `Alt+C` `Alt+W` `Alt+X` `Alt+E` `Alt+K` | Match case, Words, Regex, Preserve case, File mask |
| `Alt+P` `Alt+M` `Alt+D` `Alt+S` | scope In Project, Module, Directory, Scope |
| `Esc` | hide an open sub-popup or combo list first; otherwise close the popup |
| `F1` | help |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-001 | MUST | The four actions of the table exist with the stated keys. `FindInPath` opens the popup in find mode, `ReplaceInPath` in replace mode. A running Find in Files (while the popup's search is still working) or replace does not block opening another popup. | keymap test |
| SRCH-FIP-002 | MUST | The popup is a non-modal popup centred in the project window, resizable and movable, with a *pin* toggle (tooltip `Pin Window`) that keeps it open when it loses focus or after navigating. It closes on `Esc`, on navigating (unless pinned) and when focus leaves (unless pinned). Size and position are remembered. | UI test |

## 2. Layout and controls

Top to bottom: the title (`Find in Files` or `Replace in Files`); the **search field** (multi-line, up to 3 rows, `Search` placeholder) with the embedded toggles
*Match case*, *Words*, *Regex* and, in replace mode, the **replace field** (`Replace` placeholder) with *Preserve case*; the **filter row**: the check box
*File mask* with an editable combo of recent masks, and the *Filter Search Results* button (context filter menu of `01` section 6, the button is highlighted
when a filter other than `Anywhere` is active); the **scope row**: toggle buttons `In Project`, `Module`, `Directory`, `Scope` (mnemonics underlined `P`, `M`, `D`, `S`),
followed by the details of the selected scope; the check box *Open results in new tab* (`Alt+B`, stored as a setting, default off); the **results area**: a list
(about four rows high initially, the preview below it takes the rest), the info label, then the **preview** (source of the selected result with all matches highlighted);
the bottom buttons: find mode `Open in Find Window`; replace mode `Replace` (text `Replace` for 0 or 1 selected rows, `Replace N Occurrences` for N >= 2 selected rows) and `Replace All` (tooltip `Alt+Shift+Enter`).
A hint line `Use F3 and Shift+F3 to select usages` is shown while the search text has several lines.

Scope details: **In Project**: nothing more. **Module**: a combo of all modules. **Directory**: an editable combo of recent directories with a browse button and the check box
*Recursively* (hint `Search recursively in subdirectories`, default on). **Scope**: a scope chooser combo (named scopes, default selection from the setting default scope name,
initially `All Project Classes`), with an *Edit Scopes* button.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-010 | MUST | **Preset from context.** Opened with a selection in an editor: search text = the selected text if it is single-line (regex-escaped when Regex is on) and shorter than 10000 chars; multi-line selections are not copied; without a selection the previous search text of the Find in Files model (or of the editor bar's last text) is used. Opened from a **directory** in the project view (or navigation bar): scope = Directory with that path. Opened from a file in the project view: Directory of its parent (the parent is only recorded in the recent directories when invoked from an editor). Opened from a **module** node: scope = Module with that module. Otherwise: scope = In Project. The setting `ide.remember.last.search.scope` (advanced, default off) restores the last scope instead. | cases FIP-010/1..5 |
| SRCH-FIP-011 | MUST | **Scope semantics.** *In Project*: files of the project's content (indexed and non-indexed content, excluding ignored, excluded and library files). *Module*: content of the module (and nothing from other modules). *Directory*: files below the directory (only its direct children when *Recursively* is off); excluded subdirectories of the chosen directory are skipped unless the directory itself is excluded (registry `find.search.in.excluded.dirs`, default off). *Scope*: the files of the chosen named scope (open files, current file, recently viewed files, test/production, custom pattern scopes, `Project and Libraries`, ...). | cases FIP-011/1..4 |
| SRCH-FIP-012 | MUST | **Files that are skipped.** Binary files (by file type), files in the VCS metadata and ignored folders, directories, and files above the *large file* threshold (files too large to load: reported as `1 too large file was ignored` / `N too large files were ignored`; per file `File <path> is too large and cannot be scanned`). Project configuration files (`.idea/`, `*.iml`, `*.ipr`, `*.iws`) are searched by default (registry `find.search.in.project.files`, default on); when off the popup shows `Occurrences in project configuration files are skipped.` with a link `Include them`. | cases FIP-012/1..2 |
| SRCH-FIP-013 | MUST | **Which text is searched** for an open file: the current (possibly unsaved) editor content, not the disk content. For other files: the content in the IDE's file cache or on disk. Search is by plain scan or regex on the whole file text, line breaks normalised to `\n`. | case FIP-013/1 |

## 3. File mask and filters

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-020 | MUST | The **file mask** field is a comma-separated list; each entry is trimmed. An entry starting with `!` is a **negative** entry. A file name (the name only, not the path; the entry has no `/`) is accepted when it matches at least one positive entry (when there is no positive entry every name matches) and matches no negative entry. Entries are case-insensitive and must match the **whole name**. Conversion of an entry to a regular expression: `.` becomes a literal dot; `*` any run (`.*`); `?` any single char (`.`); the characters `+ ( ) [ ] / ^ $ { } |` are literal; a `\` separates path segments (matches a backslash). | cases FIP-020/1..8 |
| SRCH-FIP-021 | MUST | **Mask validation** (shown under the field, search not started): empty entry list while the check box is on: `Empty file mask`; a semicolon in the text: `File masks should be comma-separated`; an unparsable entry: `Bad file mask "<mask>"`. The masks are saved in a per-application list of recents (most recent first, no duplicates). | cases FIP-021/1..3 |
| SRCH-FIP-022 | MUST | The **context filter** (`Anywhere`, `In Comments`, `In String Literals`, `Except Comments`, `Except String Literals`, `Except Comments and String Literals`) restricts results by token category as in `01` SRCH-FIL-050. *Reset filters* (`Alt+Shift+Backspace`) switches off the file mask check box, resets the context to `Anywhere` and turns off Match case, Words and Regex. | cases FIP-022/1..2 |

## 4. Validation of the search text

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-030 | MUST | Before any result is shown or any action runs, the popup validates and marks the offending field: empty text (and no file mask): the results area shows `Type search query to find in files`; empty text with a file mask only is allowed (lists the matching files); regex that does not compile: `Bad pattern "<text>": <reason>` (reason = the regex engine's description, for example `Unclosed group`); regex that matches the empty string and does not start with `^` nor end with `$`: `Regular expression matches empty string`; replace text malformed (regex on): `Malformed replacement string: <reason>` (reasons as in `01` SRCH-FIL-044); scope Directory whose path does not exist: `Directory is not found` (while verifying the path: warning `Validating the directory path…`). Pressing an action while invalid shows the message in an error dialog titled `Error`. | cases FIP-030/1..5 |
| SRCH-FIP-031 | MUST | In replace mode with Regex on, the first result of the list is used to dry-run the replacement template, and the template is parsed against the pattern's group count (group *names* are not checked in the dry run), so an invalid template is reported on the replace field before replacing. | case FIP-031/1 |

## 5. Running the search and the result list

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-040 | MUST | **Live results.** The search re-runs 100 ms after the last change of any input (text, option, scope, mask, filter); the previous run is cancelled. The results area shows `Searching…` while no row exists yet. The info label reads `N matches in M files` (`1 match in 1 file`, `0 matches in 0 files`), with `N+ matches in M+ files` while more results are being loaded. | cases FIP-040/1..2 |
| SRCH-FIP-041 | MUST | **Search order** (phases, observable through which rows appear first): open editor files and the files that matched in the previous run first; then files found through the indexes; then a walk of the remaining files of the scope. Because of the first phase, typing more letters keeps earlier results at the top. | case FIP-041/1 |
| SRCH-FIP-042 | MUST | **Paging.** The list shows at most the *usages page size* (advanced setting `ide.usages.page.size`, default 100) results; when the user scrolls to within half a viewport (at least 50 px) of the end, the next page is loaded and appended; the info label keeps counting with `+` until the search is exhausted. Selecting a row, scrolling and appending never move the selection. | case FIP-042/1 |
| SRCH-FIP-043 | MUST | **Row order.** Rows are ordered by file path (string comparison of the path), then line, then offset; the file in which the first result of the run was reported stays pinned at the top with its rows. Each row shows: the line text (leading blanks removed) with the matches highlighted, the file name (trimmed in the middle beyond 60 characters; dimmed when equal to the previous row's file) and the line number. The first row is selected automatically. | cases FIP-043/1..2 |
| SRCH-FIP-044 | MUST | **Preview.** The selected row is shown in the preview (after 50 ms of quiet): the source with every match of the same file highlighted, the caret line scrolled into view, a title with the file name and the directory. Several selected rows of one file show all of them; rows of several files show `Several files selected. Select occurrences from one file to preview.`; no selection shows `Select item to preview`; a row without a preview shows `There is no preview for the selected item`. | case FIP-044/1 |
| SRCH-FIP-045 | MUST | **Navigation.** `Enter`/`F4`/double-click opens the file in the editor (reusing an open tab), selects the match, centres it, and closes the popup unless pinned. `Shift+Enter` does the same in the right split. Navigation is added to the navigation history. | cases FIP-045/1..2 |
| SRCH-FIP-046 | MUST | **Nothing found.** With a valid non-empty query and no rows the results area shows `Nothing found` plus the hints of section 6. | cases FIP-046/1..4 |
| SRCH-FIP-047 | SHOULD | **Too many results**: when the Find window (not the popup) collects more results than the safe limit, a dialog titled `Too Many Results` asks `Too many results found. Are you sure you wish to continue?` with `Continue` and `Abort`. | LATER cases |

## 6. Empty-result hints

When no row is found and some narrowing options are active, the empty text lists them so the user can relax the search:

| Active options | Empty text |
|---|---|
| none | `Nothing found` |
| exactly one of: Match case, Words, Regex (enabled), file mask in use, context filter | `Nothing found.` then the line `Search option used: <option>` then the link `Clear option` followed by the key text of *Reset filters* |
| two or more | `Nothing found.` then `Search options used:` then the options on one line separated by two spaces, then the link `Clear all options` and the key text |
| the text contains a regex metacharacter (`. $ \| ( ) [ ] { } ^ ? * + \`), Regex is off, and the text compiles as a regex | `Nothing found.` then the link `Search with Regex` followed by the key text of the Regex toggle (suggestion replaces the lines above) |
| scope Directory, *Recursively* off | after `Nothing found.` a link `Search recursively in subdirectories` |

Option names are the texts `Match case`, `Words`, `Regex`, `File mask`, `Filter`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-050 | SHOULD | The empty texts of the table with working links: *Clear option(s)* resets the filters (as `Alt+Shift+Backspace`), *Search with Regex* switches Regex on (once; the suggestion is not shown again for the same popup), *Search recursively* ticks *Recursively*. | cases FIP-046 |

## 7. Open in Find Window

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-060 | MUST | `Open in Find Window` (button, `Ctrl+Enter`) closes the popup and runs the full search (no page limit, with the prompt of 047 for too many results) into the Find tool window usage view: tab `Occurrences of '<text>'` + context suffix (`01` SRCH-FIL-062); tool window title `Occurrences of '<text>' in <scope>` + suffix, scope text `Project`, `Module '<name>'`, `Directory <path>`, `File mask '<mask>'` appended as ` with mask '<mask>'`. The tab is reused for the next search unless *Open results in new tab* is on or the tab was pinned. If the search found exactly one usage and the setting *Skip results tab with one usage* is on, the result is opened directly. | cases FIP-060/1..2 |

## 8. Replace in Files

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-070 | MUST | **Replace (selected).** In replace mode `Enter` or the `Replace` button replaces the selected row(s) in the files, one occurrence at a time, removes the replaced rows from the list and selects the row now at the same index. The files are made writable first (read-only files are requested like any write: VCS check-out prompt or error). The operation is one command named `Replace`. A malformed template at this point: error dialog `Replace Error` with the message of `01` SRCH-FIL-044 and nothing replaced. | cases FIP-070/1..2 |
| SRCH-FIP-071 | MUST | **Replace All** (`Alt+Shift+Enter`, button): when the list holds more than one row and the matches are in more than one file, a confirmation dialog titled `Replace All` asks (HTML) `Replace <N> occurrences of '<find>'<br>across <M> files with '<replacement>'?` (when find or replacement is 400 characters or longer: `Replace <N> occurrences of<br>'<find>'<br><br>across <M> files with<br>'<replacement>'?`, each text middle-trimmed to 400 chars) with buttons `Replace` and `Cancel`. `N` and `M` are the counts of the info label. On `Replace` the popup closes and a background task `Replace all` runs over **all** matches of the scope (not only the loaded page). One match in one file replaces without a prompt. | cases FIP-071/1..3 |
| SRCH-FIP-072 | MUST | Replace All semantics per file are those of `01` SRCH-FIL-041 (left to right, no rescanning, regex templates, preserve case). Each changed file is one undo step in the editor; all files of one run form one *Local History* action labelled `Replace all '<find>' in <scope> with '<replacement>'` (`<scope>` as in 060). When done the status bar shows `N occurrences replaced`. | cases FIP-072/1..2 |
| SRCH-FIP-073 | MUST | Matches whose replacement template is malformed for that match are skipped; at the end a notification (group *Find in Files operation failed*, error type) `One or more malformed replacement strings` is shown. | case FIP-073/1 |
| SRCH-FIP-074 | SHOULD | **Replace in the Find window** (after `Ctrl+Enter` in replace mode): the usage view has buttons `Replace All` (`Alt+Shift+Enter`) and `Replace <N> Occurrences` (`Alt+Enter`, text `Replace` for 0 or 1 selected), `Exclude` (`Delete`), `Include` (`Insert`), `Remove`, `Rerun`; excluded usages are not replaced; the same multi-file confirmation applies to Replace All. | LATER cases |

## 9. Persistence

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FIP-080 | MUST | Remembered per application: the toggles Match case, Words, Regex, Preserve case; the file mask text and its recents; the last context filter; the *Recursively* flag; the *Open results in new tab* flag; the recent directories; the search and replace histories (300 entries each, shared with the editor bar); the popup size and location; the last scope name of *Scope* (default `All Project Classes`). | settings test |
| SRCH-FIP-081 | SHOULD | Search and replace fields complete from the history (`Alt+Down` shows the history popup of `01` SRCH-FIL-070). | covered |

## 10. Search Everywhere text tab

The *Text* tab of Search Everywhere (`Ctrl+Shift+Alt+E`, advanced setting `se.text.search`) runs the same engine in a different shell; see
`04-search-everywhere.md` SRCH-SE-060. Structural search: `06-structural-search.md`.

## 11. Open questions

1. Same-line occurrences: whether two matches on one line are one row or two in the popup (the Find window merges same-line usages by default, setting *Merge Usages
   from the Same Line*). The counts of the info label are occurrences, rows may be fewer.
2. The *too large file* threshold and the *too many results* limit are platform settings not seen in the sources read; choose 2.5 MB text file limit and 1000-row prompt (typical IDE
   defaults) and record them in `docs/component-map.md`.
3. Whether double-click navigates in the popup list (the code binds Enter, F4, Shift+Enter, Ctrl+Enter; mouse is handled by the list).
