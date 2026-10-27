# Workbench spec 03: Structure, Problems, TODO and Bookmarks

Status: specification (clean-room). ID prefixes `WB-STR` (Structure view and File Structure popup), `WB-PRB` (Problems view), `WB-TODO` (TODO window and
settings), `WB-BKM` (Bookmarks). Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`.
Executable cases: `cases/03-structure-problems-todo.cases.md`. Tool window frame rules: `01-tool-window-framework.md`.

Sources studied: `ic: platform/structure-view-impl/` (`StructureViewWrapperImpl.kt`, `StructureViewFactoryImpl.java`, `StructureViewState.kt`, `frontend/src/FileStructurePopup.kt`,
`resources/messages/StructureViewBundle.properties`), `ic: platform/problemsView/ui/resources/messages/ProblemsViewBundle.properties`,
`ic: platform/todo/` (`TodoView.java`, `TodoToolWindowFactory.java`, `resources/messages/TodoBundle.properties`), `ic: platform/editor-ui-ex/src/com/intellij/ide/todo/TodoConfiguration.java`,
`TodoDefaultPatternProvider.kt`, `ic: platform/lang-api/src/com/intellij/ide/todo/TodoPanelSettings.java`, `ic: platform/bookmarks/src/com/intellij/ide/bookmark/` (`BookmarksManagerImpl.kt`, `actions/*`, `ui/BookmarksViewState.kt`),
`ic: platform/lang-api/src/com/intellij/ide/bookmark/BookmarkType.kt`, `ic: platform/lang-api/resources/messages/BookmarkBundle.properties`,
`ic: platform/platform-resources-en/src/messages/ActionsBundle.properties`, `ic: platform/platform-api/resources/messages/IdeBundle.properties`, `ic: platform/platform-resources/src/keymaps/$default.xml`.

---

## Part A. Structure tool window and File Structure popup (`WB-STR`)

`Structure` (id `Structure`, title "Structure", anchor `left`, split / secondary, Alt+7) shows the structure tree of the **file currently selected in the editor**.
Tree contents (nodes, icons, grouping of members) come from each language's structure builder (language extension points, `platform/08-language-extension-points.md` PLAT-LANG-084).
Some languages offer two tabs: **Physical** and **Logical** (texts `Physical`, `Logical`); the selected tab persists per project.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-STR-001 | MUST | The window follows the selected editor: when the selected file editor changes, the tree is rebuilt for it. Change detection runs every 100 ms and rebuild requests are merged for 100 ms. | Switch tab from `A.java` to `B.java`: within about 300 ms the tree shows B's structure. |
| WB-STR-002 | MUST | With no editor, or a file without structure support, the tree shows the empty text "No structure". While a rebuild is running the empty text is the standard loading text. | Close all editors: "No structure". |
| WB-STR-003 | MUST | When the window is hidden the tree is cleared (nothing is built for hidden windows) and rebuilt when it is shown. | Hide, change file, show: tree is for the new file. |
| WB-STR-004 | MUST | A "View Options" button (gear, text "View Options") groups: **Sort** ("Sort"), **Show** filters ("Show"), **Group** ("Group") sub-sections as the language supplies: Alphabetically, By Visibility, By Kind, By Type; Fields, Inherited; Members by Defining Type. Their state persists per project (`ACTIVE_ACTIONS`). | Toggle "Alphabetically": members sort alphabetically; persists after restart. |
| WB-STR-005 | MUST | Two navigation toggles: **Always Select Opened Element** (autoscroll from source: the tree selects the element at the editor caret) default off; **Navigate with Single Click** (autoscroll to source) default per project state (reference state class default true for the "mode" flag; see open question 1). Double-click and Enter always navigate. | Always Select on: moving the caret into method `foo` selects `foo`. |
| WB-STR-006 | MUST | Enter and F4 navigate to the selected element and move focus to the editor; Escape moves focus to the editor (WB-TW-041). | Select `foo`, Enter: caret at `foo`, focus editor. |
| WB-STR-007 | SHOULD | Speed search works as in the Project view (WB-PV-070). | Type `fo`: first node containing `fo` selected. |
| WB-STR-008 | SHOULD | Invalid nodes show `<invalid>`, empty nodes `<empty>`. | n/a (error handling of language builders). |
| WB-STR-009 | LATER | Logical structure tab (`Logical`/`Physical` tabs), "Dump Logical Structure". | n/a |

### File Structure popup (`FileStructurePopup`, Ctrl+F12, text "File Structure")

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-STR-020 | MUST | Ctrl+F12 opens a popup (preferred size 540 x 500 px) with the structure tree of the current editor file, the tree has focus and **typing filters immediately** (speed search without a prefix key). | Press Ctrl+F12, type `ge`: only nodes matching `ge` (and their parents) remain visible. |
| WB-STR-021 | MUST | The popup shows toggles for the same sorters/filters as the tool window plus **"Narrow down on typing"** (default on; persisted under the property `FileStructurePopup.narrowDown`): on = non-matching nodes are hidden; off = only highlighted. A footer or first row for "Inherited members" appears for languages with inheritance (Ctrl+F12 pressed again toggles it). | Narrow down off, type `ge`: all nodes remain, matches highlighted. |
| WB-STR-022 | MUST | Enter or F4 or a single mouse click on a node navigates to it and closes the popup; Escape closes the popup, **but first closes the speed-search field** if it is active. | Type `ge`, Escape: filter cleared, popup still open. Escape again: popup closes. |
| WB-STR-023 | MUST | Status text when the filter matches nothing: `'prefix' not found`; when the file has no structure: `Structure is empty`; before loading: standard loading text. | Type `zzz`: `'zzz' not found`. |
| WB-STR-024 | SHOULD | Opening the popup selects the element at the caret and scrolls it into view. | Caret in method `foo`: `foo` is selected when the popup shows. |
| WB-STR-025 | MUST | Disabled (action not available) when no editor file with structure support is current. | In an image editor Ctrl+F12 does nothing. |

---

## Part B. Problems tool window (`WB-PRB`)

Tool window id `Problems View`, title "Problems", anchor `bottom`, Alt+6. Tabs:

| Tab | Title | Content |
|---|---|---|
| Current file | "File" (reference text `problems.view.highlighting`; the task and docs call it "Current File") | highlighting problems of the file selected in the editor |
| Project errors | "Project Errors" | errors found by the IDE in the whole project (`problems.view.project`) |

Texts: no file selected: "Open file in editor to see problems"; analyzing: "Analyzing {file}"; clean: "No problems in {file}"; filtered: "Problems could be filtered out" with link "Show Other Problems"; power save: "Code analysis is disabled in power save mode"; project tab empty: "No errors found by the IDE" followed by "or" and the action link(s); several files selected for the preview: "Several files selected.\nSelect problems from one file to preview."; count label: "{n} problem" / "{n} problems".

Toolbar / options (action ids `ProblemsView.*`): **Show Quick-Fixes** (Alt+Enter), **Open Editor Preview** (`ShowPreview`, default off), options group **Sort by**: Severity, Name, **Folders Always on Top**; **Group by Inspection** (`GroupByToolId`); secondary: **Open Files with Single Click** (`AutoscrollToSource`: "Open the editor for the selected file and navigate to the selected problem when the selection changes"), **Enable Preview Tab**. Tree popup: **Jump to Source** (`EditSource`, F4), **Copy Problem Description** (Ctrl+C), **Show Quick-Fixes**.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-PRB-001 | MUST | The window has the two tabs above in this order; "File" is selected by default; the selection persists per project. | Open Alt+6: tab "File". |
| WB-PRB-002 | MUST | The File tab shows, for the **selected editor file**, the problems of the highlighting daemon (see `docs/specs/codeinsight/01-daemon.md`) as a tree grouped by severity (errors, then warnings, then weaker), each leaf with message, location `line:column` and description; the tab updates as the daemon updates. Counts: label "{n} problem(s)". | A file with two errors: two leaves, label "2 problems"; fix one: "1 problem". |
| WB-PRB-003 | MUST | Empty and transient states: nothing open "Open file in editor to see problems"; running "Analyzing {name}"; no problems "No problems in {name}"; power-save "Code analysis is disabled in power save mode". | Close all files: "Open file in editor to see problems". |
| WB-PRB-004 | MUST | Enter / double-click / F4 on a problem opens the file at the problem; "Open Files with Single Click" (default off) navigates on selection; preview tab per option. Alt+Enter on a problem shows the quick fixes of that problem. | Select error row, F4: caret on the error range. |
| WB-PRB-005 | MUST | Sort options: **Severity** (default) orders siblings by severity, most severe first; **Name** orders siblings alphabetically by node name, case-insensitively; **Folders Always on Top** (default on) applies to the directory/file tree of the Project Errors tab. Exact tie-breaking: open question 2. | Project Errors with `b.java` (error) and `A.java` (warning): Severity: `b.java`, `A.java`; Name: `A.java`, `b.java`. |
| WB-PRB-006 | SHOULD | "Group by Inspection" regroups leaves under the inspection that reported them. | Toggle on: nodes by inspection short name. |
| WB-PRB-007 | SHOULD | The editor preview (default off) shows the code around the selected problem in a split below the tree; with several files selected: "Several files selected.\nSelect problems from one file to preview." | Toggle preview, select a problem: preview shows its line. |
| WB-PRB-008 | MUST | Project Errors tab lists files with errors for the whole project (not only open files): the wolf list of the daemon (problem files, `docs/specs/codeinsight/01-daemon.md` section 3.2 pass 9) plus build errors where available; empty text "No errors found by the IDE". | A project with one broken file that is not open: its error appears here. |
| WB-PRB-009 | SHOULD | The window title tooltip / stripe icon shows an error badge while errors exist (LATER detail). | n/a |
| WB-PRB-010 | LATER | Security problems sub-tabs, "Turn on Split Problems View Implementation" (`SwitchImplementation`). | n/a |

Inventory (`problems-analyze.md`, window items): `ProblemsView.*` and `ProblemsView.Frontend.*` pairs are one UI; inspection, scope and analyze actions belong to the code-insight area (`docs/specs/codeinsight/03-inspections.md`); settings pages `Errors` (Inspections) and `project.scopes` (Scopes) are covered by `05-settings-dialog-and-keymap.md` (names, defaults) and CI-INS.

---

## Part C. TODO tool window and settings (`WB-TODO`)

Window id `TODO`, title "TODO", anchor `bottom`, no default shortcut. Tabs (in this order): **Project** (all TODOs in the project; title is the project term "Project"), **Current File** ("Current File"), **Scope Based** ("Scope Based"), **Changes** (TODOs in files of the active changelist, "Changes", only with VCS).

**Patterns.** A TODO item is a match of a *pattern* (a regular expression with options) inside a **comment** of a file (not in strings or code). The defaults are two patterns, both case-insensitive, with the default TODO text attributes:
`\btodo\b.*` and `\bfixme\b.*`. A pattern may carry custom icon and attributes. A **multi-line** option (default on, "Treat indented text on the following lines as part of the same TODO") continues the item over indented comment continuation lines.

**Filters.** A filter has a name and a subset of patterns. The window's filter menu ("Show All", "Edit Filters") selects one; "Show All" shows every pattern.

**Panel options** (per tab, persisted per project): arePackagesShown (group by packages) default off, areModulesShown (group by modules) default off, flatten packages default off, autoscroll to source default off, preview shown default on, selected filter none. Actions with default keys: Group by Modules = Ctrl+Alt+M (`TodoViewGroupByShowModules`), Group by Packages = Ctrl+Alt+P (`TodoViewGroupByShowPackages`), Flatten Packages = Ctrl+Alt+C (`TodoViewGroupByFlattenPackage`), Jump to Source (`TodoViewEditSource`, as EditSource F4; description "Open an editor for the selected item and give focus to it"), Preview Source ("Preview Source").

Tree texts: summary node "Found {n} TODO item(s) in {m} file(s)"; group nodes "{n} items" / "{n} item"; truncated lists end with "...more...". Item rows show the TODO text with the matched part highlighted and the location `(line:column)`. Empty states: settings table "No patterns configured", "No filters configured".

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TODO-001 | MUST | The default patterns are exactly `\btodo\b.*` and `\bfixme\b.*`, both case-insensitive. | In a comment, `// TODO fix`, `// todo fix`, `// FixMe later` match; `// TODOS` and `// autotodo` do not (word boundary). |
| WB-TODO-002 | MUST | A pattern matches only inside comments of the file's language; the matched region from the keyword to the end of the line (for `.*`) is the item text. | A string literal "TODO" is not an item; `// TODO: x` item text `TODO: x`. |
| WB-TODO-003 | MUST | With multi-line on, comment lines that continue the TODO by indentation (indented further than the start of the comment text, no new pattern match) belong to the same item. | `// TODO a` + `//   b` → one item `TODO a b`; with the option off, item `TODO a`. |
| WB-TODO-004 | MUST | The Project tab lists all matches in project files (content and optionally scopes) grouped by file; Current File lists only the selected editor's file and follows editor selection; Scope Based lets the user choose a scope; Changes the changed files. Counts appear in the summary. | Add `// TODO x` in two files: Project: "Found 2 TODO items in 2 files". |
| WB-TODO-005 | MUST | Index-based: results exclude files not in the TODO index scope and update after file changes without reopening the window; in dumb mode the window shows the dumb-mode text (`PLAT-DUMB`). | Type a new `// TODO y` in an open file: item appears within one second of the commit. |
| WB-TODO-006 | MUST | Enter / double-click / F4 / "Jump to Source" opens the file at the item; autoscroll (default off) navigates on selection; preview (default on) shows the source in a split. | Select an item, F4: caret at the TODO. |
| WB-TODO-007 | MUST | Group By actions: Modules (Ctrl+Alt+M), Packages (Ctrl+Alt+P), Flatten Packages (Ctrl+Alt+C); defaults off; they persist. | Toggle Packages: file nodes nest under package nodes. |
| WB-TODO-008 | MUST | Editor highlighting: TODO items are highlighted in the editor with the TODO attributes (color scheme key "Code // TODO defaults") unless a pattern says otherwise; a TODO stripe mark appears on the error stripe. | A `// TODO x` comment is bold/blue in the default scheme. |
| WB-TODO-009 | SHOULD | Settings page **TODO** (id `preferences.toDoOptions`, parent Editor, application scope, search keywords "todo", "fixme"): sections "Patterns" (table, columns "Icon", "Case Sensitive", "Pattern"; buttons Add, Edit, Remove; the multi-line checkbox), "Filters" (table columns "Name", "Patterns"). Add Pattern dialog titled "Add Pattern", fields "Pattern:", "Case sensitive", "Icon:", "Use color scheme TODO default colors". Errors: "Pattern should be specified", "Same pattern already exists"; filters: "Filter name should be specified", "Filter with the same name already exists", "Filter should contain at least one pattern". | Add pattern with empty text: error "Pattern should be specified" and OK disabled. Two identical patterns: "Same pattern already exists". |
| WB-TODO-010 | SHOULD | Changing patterns re-indexes: the window updates and highlighting follows. | Add pattern `\bhack\b.*`: `// hack x` shows up. |
| WB-TODO-011 | LATER | "Next TODO" / "Previous TODO" navigation, per-pattern icons rendering in the gutter. | n/a |

---

## Part D. Bookmarks (`WB-BKM`)

### D.1 Model

* A **bookmark** points to a place: a **line** (file + line, tracked through edits), a **file or directory**, a **module**, or a root-type node (scratches). A bookmark that cannot be resolved is **invalid** (shown struck/greyed, kept).
* A bookmark has a **type**: `DEFAULT` (no mnemonic) or a **mnemonic** `0-9`, `A-Z` (36 types). A mnemonic is unique **per project** (assigned to at most one bookmark).
* A bookmark has an optional **description** (default: trimmed text of the line for line bookmarks; the selected text when added with a selection).
* Bookmarks live in **lists** (groups, "Bookmark list"). One list may be the **default** list (marked "Default"). A bookmark can be in several lists. Lists and order persist per project (workspace file). When no state exists the project gets one list named like the project.
* Tool window id `Bookmarks`, title "Bookmarks", anchor `left`, split, Alt+2.

### D.2 Actions and shortcuts

| Action (id) | Text | Default shortcut (Win/Linux) | macOS keymap |
|---|---|---|---|
| `ToggleBookmark` | Toggle Bookmark (context menu: "Add Bookmark" / "Delete Bookmark") | F11 | F3 |
| `ToggleBookmarkWithMnemonic` | Toggle Bookmark Mnemonic... | Ctrl+F11 | Alt+F3 |
| `ShowBookmarks` | Show Bookmarks... (popup "Bookmarks") | Shift+F11 | Cmd+F3 |
| `ShowTypeBookmarks` | Go to Mnemonic... (popup "Go to Mnemonic") | Ctrl+Shift+F11 | (inherits) |
| `ToggleBookmark0..9` | Toggle Bookmark 0..9 | Ctrl+Shift+0..9 | same |
| `GotoBookmark0..9` | Go to Bookmark 0..9 | Ctrl+0..9 | same |
| `ToggleBookmarkA..Z`, `GotoBookmarkA..Z` | Toggle Bookmark A..Z / Go to Bookmark A..Z | none | none |
| `GotoNextBookmark`, `GotoPreviousBookmark` | Next / Previous Line Bookmark | none | none |
| `GotoNextBookmarkInEditor`, `GotoPreviousBookmarkInEditor` | Next / Previous Line Bookmark in Editor | none | none |
| `EditBookmark` | Rename Bookmark... (edit description) | none | none |
| `DeleteMnemonicFromBookmark` | Remove Mnemonic | none | none |
| `AddAnotherBookmark` | Add Bookmark to Another List | none | none |
| `BookmarkOpenTabs` | Bookmark Open Tabs... | none | none |
| `ShowLineBookmarks` | Show Line Bookmarks... | none | none |

The macOS rows use Ctrl+digits where the default keymap does (inherited, `GotoBookmark1` = Ctrl+1).

Bookmarks tool window actions (`BookmarksView.*`): Create Bookmark List... (Alt+Insert), Edit (rename / description, F2 as tree edit), Delete (Delete), Move Up / Move Down (same keys as Next/Previous Occurrence: Ctrl+Alt+Down/Up on the default keymap), Mark List as Default / Unmark List as Default, Change Mnemonic... (Ctrl+F11), Remove Mnemonic, Sort Bookmarks by Type & Name, Open All Bookmarked Files (F4 / EditSource), Open Editor Preview (`ShowPreview`, default off); gear: **Group Line Bookmarks by File** (default on), **Always Select Opened Element** (default off), **Navigate with Single Click** (default: the global autoscroll default, off), **Enable Preview Tab**, **Ask Before Deleting Multiple Bookmarks** (default on), **Ask Before Rewriting Mnemonic** (default on).

### D.3 Requirements

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-BKM-001 | MUST | **Toggle Bookmark** (F11) in an editor toggles a **line bookmark** on the caret line: if the line has one it is removed, otherwise one is added. In the Project view (or any view with a context) it toggles a file / directory / module bookmark on the selection. With several selected items the action reads "Add Bookmarks..." and asks for a list. | Caret on line 10, F11: gutter bookmark icon on line 10; F11 again: icon gone. |
| WB-BKM-002 | MUST | On add, the target list is: the **default list** if there is one; else, if exactly one list exists, that list; else (several lists, none default) a **Select Bookmark List** dialog (button "Select", label "Bookmark list:"); if no list exists, a **Create Bookmark List** dialog first. A new project without state starts with one list named like the project, so the first F11 adds directly to it. | Fresh project: F11 adds to list "<project name>" without any dialog. Two lists, none default: dialog "Select Bookmark List". |
| WB-BKM-003 | MUST | Adding with a non-empty text selection stores the selected text as description; otherwise the default description is the trimmed text of the line (line bookmarks). | Select `foo(bar)` on a line, F11: description `foo(bar)`. No selection on `  return x;`: `return x;`. |
| WB-BKM-004 | MUST | **Line bookmark tracking**: the bookmark follows its line through edits above it (lines inserted or removed), and is kept (at the nearest valid line, flagged if the text changed) on reload. | Bookmark on line 10, insert 2 lines above: bookmark on line 12. |
| WB-BKM-005 | MUST | **Mnemonic toggle with digit** (Ctrl+Shift+N, N = 0-9) toggles the bookmark on the caret line **with type N**: no bookmark: added with mnemonic N; existing bookmark with another type: its type becomes N; same type: removed. Ctrl+N (Go to Bookmark N) navigates to the bookmark holding mnemonic N (disabled when none). Letters have no default shortcuts and go through the chooser popups. | Caret line 5, Ctrl+Shift+1: bookmark with "1" on line 5. Move to line 20, Ctrl+1: caret jumps to line 5. |
| WB-BKM-006 | MUST | **Toggle Bookmark Mnemonic...** (Ctrl+F11) opens a chooser: title "Add Mnemonic Bookmark" (no bookmark yet), "Assign Mnemonic" (bookmark without mnemonic), "Change Mnemonic" (bookmark with mnemonic); a grid of 0-9 then A-Z, legend "Already used" and "Current", an optional text field "Description (Optional)", hint "Quickly set: type or double-click mnemonic / Provide note: select mnemonic, add description, and press Enter". Typing a character or double-clicking a cell assigns at once; Enter after selecting a cell and typing a description applies both. | Ctrl+F11, type `a`: bookmark with mnemonic "A". |
| WB-BKM-007 | MUST | A mnemonic already held by another bookmark asks "Rewrite Mnemonic": `'X' mnemonic is already taken by 'File.java:12'. Do you want to rewrite it?` (for a line bookmark `'{file}:{line}'`, file bookmark `'{file}'`, otherwise the short form `'X' mnemonic is already taken. Do you want to rewrite it?`), button "Rewrite", with a do-not-ask-again checkbox that sets "Ask Before Rewriting Mnemonic" off. On confirm: a **line** bookmark that held it is removed entirely; other bookmarks fall back to type `DEFAULT`. Cancel: nothing changes. | "1" on line 5; on line 20 press Ctrl+Shift+1, confirm: line 5 bookmark removed, line 20 holds "1". Cancel: unchanged. |
| WB-BKM-008 | MUST | **Show Bookmarks...** (Shift+F11) opens a popup list titled "Bookmarks" of all bookmarks (default order: list order, then bookmark order), each with its mnemonic icon, description, and `{file}, line {n}` location; typing filters; Enter navigates; Delete removes; the advanced setting "Show only line bookmarks in popup" restricts to line bookmarks. Status line shows tips: "{shortcut} to toggle", "{shortcut} to jump to". | Three bookmarks: three rows; Enter on the second jumps to it. |
| WB-BKM-009 | MUST | **Go to Mnemonic...** (Ctrl+Shift+F11): popup "Go to Mnemonic": pressing a mnemonic key navigates to that bookmark. | Ctrl+Shift+F11, A: caret at the bookmark with "A" (nothing if none). |
| WB-BKM-010 | MUST | **Next / Previous Line Bookmark** navigate through all line bookmarks ordered by **file path (string order) then line**; next after the current caret position (or after the bookmark under it), previous accordingly; navigation **wraps cyclically** at either end. Disabled when no line bookmark exists. "In Editor" variants only consider the current editor's file. | Bookmarks `a.txt:3`, `a.txt:9`, `b.txt:1`; caret `a.txt:9`: Next goes to `b.txt:1`; Next again to `a.txt:3`. |
| WB-BKM-011 | MUST | Bookmarks tool window tree: top level are lists (default list marked "Default" and always first); under them bookmarks. With "Group Line Bookmarks by File" (default on), line bookmarks nest under a file node. Order of lists and bookmarks is the user's (Move Up/Down, drag and drop), or Sort Bookmarks by Type & Name. Empty state: "No bookmarks added." with tips "Add Bookmark ({shortcut}) to code lines," / "symbols, files, and directories." | New project, open Alt+2: empty text lines above. |
| WB-BKM-012 | MUST | **Create Bookmark List...** dialog "Create Bookmark List": field "Bookmark list:" prefilled "New List", checkbox "Use as default list" (help tooltip: "New bookmarks will be added here automatically. You can change the default list at any time in the Bookmarks tool window"), button "Create". Duplicate names are rejected: "A list with the specified name already exists". Blank names are rejected (no list). **Rename Bookmark List...** dialog titled "Rename Bookmark List" (button "Rename") with the same validation. | Create "Todo" twice: second time error text and the Create button disabled. |
| WB-BKM-013 | MUST | **Delete** in the tree: with "Ask Before Deleting Multiple Bookmarks" on (default): a confirmation: one list: `Are you sure you want to delete 'X' bookmark list? This action can't be undone.` (dialog "Delete Bookmark List"); several lists: `Are you sure you want to delete the selected bookmark lists? This action can't be undone.` ("Delete Bookmark Lists"); a node with bookmarks: `Are you sure you want to delete {n} bookmarks in 'X'? ...` ("Delete Bookmark Node"); several nodes: `... the selected bookmark nodes? ...` ("Delete Bookmark Nodes"); buttons "Delete" / "Cancel". Single bookmark delete needs no confirmation. | Delete list "Todo" with the option on: dialog "Delete Bookmark List" with the text above. |
| WB-BKM-014 | MUST | **Mark List as Default** moves the list to the top and marks it "Default"; there is at most one default; "Unmark List as Default" clears it. The default list cannot be moved by drag or Move Up/Down. | Mark list 3 default: it becomes first and "Default". |
| WB-BKM-015 | MUST | **Edit / Rename Bookmark...** (`EditBookmark`) edits the description in a dialog "Bookmark Description" with message "Enter a short bookmark description". | Rename: description updated in tree and popup. |
| WB-BKM-016 | SHOULD | **Add Bookmark to Another List** adds the bookmark under the caret to a list chosen in the Select dialog; a line bookmark exists in each list at most once. | Add the same line to a second list: bookmark listed under both. |
| WB-BKM-017 | SHOULD | **Bookmark Open Tabs...** creates a new list (dialog "Create Bookmark List") with a file bookmark for every open tab. | Three tabs open: new list with three file bookmarks. |
| WB-BKM-018 | MUST | A bookmarked line shows a gutter icon: the plain bookmark icon (DEFAULT) or the mnemonic character in a bookmark shape. Tooltip "Bookmarked" (and the mnemonic). A mouse click on that icon with Ctrl (Cmd on macOS) toggles the bookmark; other clicks are ignored by the toggle action. | Ctrl+click the icon: bookmark removed. |
| WB-BKM-019 | SHOULD | **Open All Bookmarked Files** (list node, F4) opens the files of all bookmarks of the list. | List with bookmarks in 3 files: 3 editor tabs. |
| WB-BKM-020 | SHOULD | **Group**: persists "Group Line Bookmarks by File", autoscroll options, preview, "Ask Before Deleting Multiple Bookmarks", "Ask Before Rewriting Mnemonic" in the workspace file; view proportions (popup 0.3, view 0.5). | Change an option, restart: restored. |
| WB-BKM-021 | MUST | Deleting or renaming a bookmarked file keeps the bookmark but marks it invalid (name kept); invalid bookmarks cannot navigate. | Delete `a.txt`: its bookmark row greyed, navigation disabled. |
| WB-BKM-022 | LATER | "Restore bookmark context on branch switching" (advanced setting, tasks/context integration). | n/a |

---

## Open questions

1. Structure view: the state class of the reference stores `AUTOSCROLL_MODE = true` and `AUTOSCROLL_FROM_SOURCE = false`; how these map to the two toolbar toggles ("Navigate with Single Click" vs "Always Select Opened Element") must be confirmed by a reference run (UI default of "Navigate with Single Click").
2. Problems view: exact severity group captions and tab titles in the Project Errors tab beyond the strings quoted; error badge behavior on the tool window icon.
3. TODO: the pattern scanner treats comments by lexer token; multi-line continuation rule details (exact indentation comparison) need a reference run. Default of "Scope Based" scope selection is open.
4. Bookmarks: group-name suggestion for the first list when the project is renamed; ordering inside the "Bookmarks" popup beyond "list then bookmark order"; whether a mouse click on the gutter icon without Ctrl opens a popup (not read).
