# 05. Recent files, switcher, locations, history, file structure, navigation bar, bookmarks, Go to Line

Status: specification, language independent. Requirement prefixes `SRCH-REC` (recent files, switcher, recent locations), `SRCH-HIS` (back/forward and edit locations),
`SRCH-GTL` (Go to Line), `SRCH-FST` (file structure popup), `SRCH-NVB` (navigation bar), `SRCH-BKM` (bookmarks).
Source basis: IntelliJ Community tag `idea/262.10968.63`. Paths are cited as `ic: ...`.
Priorities: `MUST` / `SHOULD` / `LATER`. Executable cases: `cases/05-recent-and-structure-popups.cases.md`.

Code-level navigation (Go to Declaration, Back after it) is `docs/specs/codeinsight/07-navigation.md` section 3; speed-search matching in these popups is `07-matching-and-ranking.md`.

ic: `platform/recentFiles/{frontend,shared}/src/**` (`Switcher.kt`, `SwitcherActions.kt`), `platform/platform-impl/src/com/intellij/ide/actions/{RecentLocationsAction,RecentLocationsDataModel,GotoLineAction,JumpToLastEditAction}.*`,
`platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/IdeDocumentHistoryImpl.kt`, `platform/platform-impl/src/com/intellij/ide/util/{GotoLineNumberDialog,EditorGotoLineNumberDialog}.java`,
`platform/structure-view-impl/src/com/intellij/ide/util/FileStructurePopup.java`, `java/java-structure-view/src/**`, `platform/navbar/{backend,frontend}/src/**`,
`platform/bookmarks/src/com/intellij/ide/bookmark/**`, `platform/lang-api/resources/messages/BookmarkBundle.properties`, `platform/platform-api/resources/messages/IdeBundle.properties`,
`platform/editor-ui-api/src/com/intellij/ide/ui/UISettingsState.kt`, `platform/util/resources/misc/registry.properties`.

## 1. Recent Files (`Ctrl+E`) and Switcher (`Ctrl+Tab`)

| Action id | Text | Keys |
|---|---|---|
| `RecentFiles` (`RecentFilesFallback`) | Recent Files | `Ctrl+E` |
| `RecentChangedFiles` (`RecentChangedFilesFallback`) | Recently Changed Files | none |
| `SwitcherRecentEditedChangedToggleCheckBox` | Toggle Changed Only Files | `Ctrl+E` while the popup is open |
| `SwitcherIterateItems` | Iterate Recent Files | `Ctrl+E` while the popup is open |
| `Switcher` | Switcher | `Ctrl+Tab`, `Ctrl+Shift+Tab` |
| `SwitcherForward` / `SwitcherBackward` | Iterate Switcher Forward / Backward | internal (the repeated `Tab` / `Shift+Tab`) |
| `SwitcherNextProblem` / `SwitcherPreviousProblem` | Select Next / Previous Problem File | `F2` / `Shift+F2` in the popup |
| `DeleteRecentFiles` | Delete from Recent Files | `Delete` (as `$Delete`) in the popup |
| `*Fallback` variants | same | used when the frontend implementation is unavailable (LATER) |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-REC-001 | MUST | **Recent Files popup** (`Ctrl+E`): a pinned (non-closing on modifier release) popup titled `Recent Files` with a file list in the centre and a list of tool windows on the left (registry `ide.recent.files.tool.window.list`, default on); in the header the check box `Show edited only` and the key text of the toggle (`Ctrl+E`). `Ctrl+E` again toggles the check box. The list holds files from the editor history, newest used first (at most 50: `recentFilesLimit`), without duplicates; with the check box on, only files edited in this session (the `Recently Changed Files` variant opens with it on). Empty list: `No recent files`; loading: `Fetching recent files…`; empty tool window list: `No tool windows`. | cases REC-001/1..3 |
| SRCH-REC-002 | MUST | **Initial selection:** the first file of the list, but when it is the file open in the focused editor the second one. If the list has exactly one entry that entry is selected. | cases REC-002/1..2 |
| SRCH-REC-003 | MUST | **Keys in Recent Files:** `Up`/`Down`/`PageUp`/`PageDown` move; `Left`/`Right` move between the file list and the tool window list (the selected index is carried over, clamped); `Enter` opens the selected file(s) (or activates the tool window) and closes; `Shift+Enter` opens in the right split; `Delete` closes the selected editor tab / removes the file from the list (the selection moves to the row that took its place) or hides the tool window; `Esc` closes (a first `Esc` closes the speed search); multi-selection with `Shift`/`Ctrl` click or `Shift+Up/Down` opens all. Typing starts **speed search** (registry `ide.recent.files.speed.search`, default on): the typed text filters the file names by the camel-hump matcher of `07` (not anchored at the start, humps on, registry `...speed.search.camel.case` default on); no match shows `Press 'Enter' to search in Project` (Enter then opens Search Everywhere with the text). | cases REC-003/1..3 |
| SRCH-REC-004 | MUST | **Row.** File icon, file name with matched fragments, the project-relative directory in grey, a marker for files with problems (error icon / red text, used by `F2`); the bottom status line shows the full path of the selected file (trimmed on the left with `…`). Tool window rows show the tool window name and, with mnemonics enabled (registry `ide.recent.files.tool.window.mnemonics`, default off), a digit. | UI test |
| SRCH-REC-005 | MUST | **Switcher** (`Ctrl+Tab` held): a non-pinned popup titled `Switcher`, listing the same files (and tool windows); the first `Ctrl+Tab` selects the second file (the previous one); each further `Tab` while `Ctrl` is held selects the next (wrapping from the last file to the tool windows list and back); `Shift+Tab` goes backwards; releasing `Ctrl` opens the selected file or activates the selected tool window and closes the popup; `Esc` cancels; `Delete`/`Backspace` close the selected tab. Arrow keys also move. A quick press and release of `Ctrl+Tab` toggles between the two latest files. | cases REC-005/1..3 |
| SRCH-REC-006 | MUST | **Order of the recent files:** a file moves to the top when it is selected in an editor (opened or tab-switched); closing a tab keeps the file in the list; moving to a file by Back/Forward counts. Files that do not exist any more are dropped on display. The list is shared by Recent Files, the Switcher and the *Recent Files* provider of Search Everywhere (`04` SRCH-SE-033). | cases REC-006/1..2 |
| SRCH-REC-007 | SHOULD | The *Switcher* and *Recent Files* popups open centred and non-resizable (Switcher) / resizable and movable (Recent Files, size remembered under `SwitcherDM`); minimum size 250 x 100 for the Switcher. | UI test |

## 2. Recent Locations (`Ctrl+Shift+E`)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-REC-010 | MUST | `RecentLocations` opens the popup `Recent Locations` (`Recently Edited Locations` for the edited variant) with a check box `Show edited only` and its key text. Each entry is a **code snippet**: the caret line with 2 lines before and 2 after (the window slides to keep five lines near file boundaries; trailing and leading blank lines trimmed; each line cut at 1000 chars), a breadcrumb header of the enclosing declarations (` > ` separated) or the file name, the matching line highlighted. Entries come from the navigation history (back places) newest first, de-duplicated by *same place* (same file and the same navigation state), at most 25 (`recentLocationsLimit`); the *edited* variant uses the change places. Empty: `No recent locations found`; an empty file shows the text `Empty lines`. | cases REC-010/1..3 |
| SRCH-REC-011 | MUST | Keys: `Up/Down` move; `Enter`/double-click navigate to the location (caret on the saved offset) and close; `Delete` removes the selected entries from the history (and the list); typing starts speed search over breadcrumb and snippet text; `Esc` closes. Position and size are remembered. | case REC-011/1 |

## 3. Navigation history

| Action id | Text | Keys |
|---|---|---|
| `Back` / `Forward` | Back / Forward | `Ctrl+Alt+Left` / `Ctrl+Alt+Right`, mouse buttons 4 / 5 |
| `JumpToLastChange` / `JumpToNextChange` | Last Edit Location / Next Edit Location | `Ctrl+Shift+Backspace` / none |
| `NextTab` / `PreviousTab` | Select Next / Previous Tab | `Alt+Right` / `Alt+Left` |
| `NextEditorTab` / `PreviousEditorTab` | Select next / previous tab in multi-editor file | `Alt+Shift+Right` / `Alt+Shift+Left` |
| `ShowContent` | Show List of Tabs | `Alt+Down` (as `MethodDown` in editors it moves to the next method) |
| `NextSplitter` / `PrevSplitter`, `NextProjectWindow` | Go to next/previous splitter; Next Project Window | `Ctrl+Alt+]` for the project window |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-HIS-001 | MUST | **Back stack.** Navigation commands (go to declaration, find/replace selections, search results, recent files/locations, structure popup, bookmarks, Ctrl+click) and editor switches push the *place* (file, editor type, caret offset and view state) of where the user was. At most 150 entries (registry `editor.navigation.history.stack.size`); the oldest is dropped. A new place equal to the last (same file and the same navigation-level state) replaces it instead of adding. Plain typing and arrow-key caret moves do not push. | cases HIS-001/1..3 |
| SRCH-HIS-002 | MUST | **Back / Forward.** `Back` pops the last place, pushes the current place on the forward stack, and navigates (opening the file if closed, restoring caret and scroll). `Forward` re-applies the forward stack (skipping entries equal to the current place). A new navigation after Back clears the forward stack. Places of deleted files are removed. Both are disabled when their stack is empty. Back/Forward themselves do not push. | cases HIS-002/1..3 |
| SRCH-HIS-003 | MUST | **Last Edit Location** (`Ctrl+Shift+Backspace`): navigates to the position of the most recent change, then to older change places on repeated invocation (skipping places equal to the current caret); *Next Edit Location* goes the other way. A change place is recorded per command that modified a document (merged when consecutive changes are in the same place), at most 150. | cases HIS-003/1..2 |
| SRCH-HIS-004 | SHOULD | `Alt+Right` / `Alt+Left` select the next / previous editor tab (wrapping); `Alt+Down` in a non-editor context shows the list of tabs. | n/a |

## 4. Go to Line:Column (`Ctrl+G`)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-GTL-001 | MUST | A modal dialog titled `Go to Line:Column` with the label `[Line] [:column]:` and one text field (200 px wide) prefilled with `<line>:<column>` of the caret (1-based) and all selected; `Enter` accepts, `Esc` cancels; the dialog is available in any editor. | case GTL-001/1 |
| SRCH-GTL-002 | MUST | **Syntax:** optional whitespace, an optional line number, optionally a separator `,` or `:` (with spaces) and an optional column. `12` line 12 column 1; `12:5` or `12,5` line 12 column 5; `:5` the current line, column 5; `12:` line 12; blank text: the current line. `+N` / `-N` (relative, with optional spaces) go N logical lines down/up from the caret line (column 1), clamped to the file. Line 0 or an unparsable text keeps the dialog open (OK does nothing). | cases GTL-002/1..6 |
| SRCH-GTL-003 | MUST | On OK: secondary carets are removed, the caret moves to the logical position (line and column clamped to the document: a line past the end goes to the last line; a column past the line end goes to the line end), the selection is removed, the editor scrolls so the caret is centred, and focus returns to the editor. The previous position is added to the navigation history. | cases GTL-003/1..3 |
| SRCH-GTL-004 | LATER | In internal mode an `Offset:` field shows the character offset in sync with the line:column field. | n/a |

## 5. File Structure popup (`Ctrl+F12`)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-FST-001 | MUST | `Ctrl+F12` opens a popup (about 540 x 500 px, resizable, size remembered) showing the structure tree of the current editor's file (declarations of the language: classes, methods, fields, properties, sections), with the element at the caret preselected. The tree is expanded to the selected element. For a file without structure the text `Structure is empty`. | cases FST-001/1..2 |
| SRCH-FST-002 | MUST | **Search.** Typing anything starts speed search over the node texts (the matcher of `07` with the separators space and parentheses); the tree filters to matching nodes and their parents (setting *Narrow down on typing*, text `Narrow down on typing`, default **on**; off only highlights and selects the best match). The best match is selected; with no match the text `Structure is empty` is replaced by `not found` after the typed text. `Esc` clears the search first, then closes. | cases FST-002/1..3 |
| SRCH-FST-003 | MUST | `Enter`, `F4` and double-click navigate to the selected element (the editor caret moves to the element's name, the popup closes, the navigation history gets an entry). `Up/Down`, `Left/Right` (collapse/expand), `Home/End` as in a tree. | case FST-003/1 |
| SRCH-FST-004 | MUST | **Toggles** (check boxes in the popup header, remembered): per language; for Java: `Inherited members` (shows members inherited from super classes and interfaces, repeated `Ctrl+F12` toggles it), `Anonymous classes` (`Ctrl+I`), `Lambdas` (`Ctrl+L`), `Fields`, `Non-public`, `Properties`; the key of each toggle is shown in the label. Toggling rebuilds the tree and keeps the selection. | cases FST-004/1..2 |
| SRCH-FST-005 | MUST | **Order** inside a class is source order unless a sorter is active (`Sort alphabetically`, `By Visibility`, `By Type`/`Kind`). Kind sorter weights (Java): inner classes 10 (53 in the popup), initializers 15, super-type groups 20, constructors 30, methods 35, property groups 40, fields 50, anonymous classes 55, others 60. | LATER cases |
| SRCH-FST-006 | LATER | The `Structure` tool window (secondary tool window, title `Structure`, tabs `Logical`/`Physical`), `Select In > File Structure`: same tree in a docked view. Reason: docked view shares the model, scheduled with tool windows. | n/a |

## 6. Navigation bar

| Action id | Text | Keys |
|---|---|---|
| `ShowNavBar` | Jump to Navigation Bar | `Alt+Home` |
| `NavBar-selectLeft` / `selectRight` / `selectHome` / `selectEnd` | Select previous / next / first / last item | `Left` / `Right` / `Home` / `End` |
| `NavBar-selectUp` / `selectDown` | Open Corresponding Popup Menu | `Up` / `Down` |
| `NavBar-return` | Perform Action on Selected Item | `Enter` |
| `ViewNavigationBar`, `NavBarLocationTop`, `NavBarLocationBottom`, `NavBarLocationHide`, `ViewMembersInNavigationBar`, `ShowMembersInNavigationBar`, `ViewObsoleteNavBarAction` | `Navigation Bar` (View), location `Top` / `In Status Bar` / `Don't Show`, `Members in Navigation Bar`, `Show Members` | none |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-NVB-001 | MUST | The bar shows the **path of the focused element** from the project root to the element: project, module/content root, directories, file, and (setting *Members in navigation bar*, default on) the enclosing declarations of the caret (class, method). Items are separated by a chevron. A long path scrolls/collapses from the left so that the last item stays visible. The path follows the focused editor and the project view selection. Location setting: `Top` (default, under the main toolbar), `In Status Bar`, `Don't Show`; the *View* menu entry `Navigation Bar` toggles visibility; default shown (`showNavigationBar` true). | cases NVB-001/1..2 |
| SRCH-NVB-002 | MUST | **Keyboard.** `Alt+Home` focuses the bar with the last item selected; `Left`/`Right` move (wrapping), `Home`/`End` jump; `Down`/`Up` (or `Enter` on a directory/ container) open the **child popup** of the selected item: a list of its children (directories, files, members) ordered by weight (directories before files) and then by name in natural order; `Enter` on a file or member navigates and closes; `Esc` returns focus to the editor. Typing in the popup speed-searches. Selecting a child that has children and pressing `Right` drills down. | cases NVB-002/1..2 |
| SRCH-NVB-003 | SHOULD | Click on an item opens its child popup; click on the last item navigates to it; the context menu offers the project view actions of the item. | UI test |

## 7. Bookmarks

| Action id | Text | Keys |
|---|---|---|
| `ToggleBookmark` | Toggle Bookmark | `F11` |
| `ToggleBookmarkWithMnemonic` | Toggle Bookmark Mnemonic… | `Ctrl+F11` |
| `ToggleBookmark0` … `ToggleBookmark9`, `ToggleBookmarkA` … `ToggleBookmarkZ` | Toggle Bookmark 0 … Z | `Ctrl+Shift+0` … `Ctrl+Shift+9`; letters unbound |
| `GotoBookmark0` … `GotoBookmark9`, `GotoBookmarkA` … `GotoBookmarkZ` | Go to Bookmark 0 … Z | `Ctrl+0` … `Ctrl+9`; letters unbound |
| `ShowBookmarks` | Show Bookmarks… | `Shift+F11` |
| `ShowTypeBookmarks` | Go to Mnemonic… | `Ctrl+Shift+F11` |
| `GotoNextBookmark` / `GotoPreviousBookmark` (`…InEditor`) | Next / Previous Line Bookmark (in Editor) | none |
| `EditBookmark`, `AddAnotherBookmark`, `BookmarkOpenTabs`, `DeleteMnemonicFromBookmark`, `OpenBookmarkGroup` | Rename Bookmark…, Add Bookmark to Another List, Bookmark Open Tabs…, Remove Mnemonic, Open All Bookmarked Files | none |
| `BookmarksView.*` (Create, Delete, Rename, MoveUp/Down, DefaultGroup, ChooseType, DeleteType, SortGroupBookmarks, GroupLineBookmarks, AutoscrollToSource/FromSource, OpenInPreviewTab, ShowPreview, AskBeforeDeletingLists, RewriteBookmarkType) | tool window and popup actions | per inventory |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-BKM-001 | MUST | **Toggle** (`F11`): in an editor adds a *line bookmark* at the caret line (a gutter icon at that line) to the **default bookmark list** (created with the project name as its name if no list exists); on a line that already has a bookmark it removes it. The bookmark description is the selected text if non-blank, else empty (a default description from the line). Bookmarks follow edits (the line moves with the text). On a project view / other selection with a file or directory it bookmarks that item; with several selected nodes it asks for the list (`Add Bookmarks…`). Context menu text: `Add Bookmark` / `Delete Bookmark`. | cases BKM-001/1..3 |
| SRCH-BKM-002 | MUST | **Mnemonics.** A bookmark may carry one mnemonic (`0`-`9`, `A`-`Z`; 36 types). `Ctrl+Shift+<digit>` toggles the numbered bookmark at the caret line: if the line has no bookmark it is added with that mnemonic; if it has the same mnemonic it is removed; if it has another type the mnemonic is changed. Setting a mnemonic that is already used elsewhere asks `‘X’ mnemonic is already taken by ‘<file>:<line>’. Do you want to rewrite it?` (file bookmarks: `…taken by ‘<name>’…`; plain: `‘X’ mnemonic is already taken. Do you want to rewrite it?`), title `Rewrite Mnemonic`, button `Rewrite`; accepting moves the mnemonic (the old bookmark loses it); the dialog is skipped when *Ask Before Rewriting Mnemonic* is off. | cases BKM-002/1..3 |
| SRCH-BKM-003 | MUST | `Ctrl+F11` opens a popup titled `Add Mnemonic Bookmark` (`Assign Mnemonic` / `Change Mnemonic` for an existing bookmark) with a 4-column grid of the 36 mnemonic buttons: already-used ones coloured *assigned* (legend `Already used`), the current one coloured *current* (`Current`), a field `Description (Optional)`; typing a letter/digit key or double-clicking a button chooses it; `Enter` in the description saves; the hint text `Quickly set: type or double-click mnemonic`, `Provide note: select mnemonic, add description, and press Enter`. | case BKM-003/1 |
| SRCH-BKM-004 | MUST | **Go to** `Ctrl+<digit>` jumps to the bookmark with that mnemonic (opens the file, caret at the line; no effect when absent); `Ctrl+Shift+F11` lists mnemonic bookmarks in a popup titled `Go to Mnemonic`, where typing the mnemonic jumps. `GotoNextBookmark` / `Previous` iterate all line bookmarks ordered by file path then line, cycling at the ends (registry `ide.bookmark.occurrence.cyclic.iteration.allowed`, default **on**); the `…InEditor` variants iterate the bookmarks of the current file by line (no cycling past the file's ends unless cyclic, then to the first/last, never staying on the same line) and place the caret at column 0 centred. | cases BKM-004/1..3 |
| SRCH-BKM-005 | MUST | `Shift+F11` shows the **Bookmarks popup** (title `Bookmarks`; `Show Line Bookmarks…` when the advanced setting `Show only line bookmarks in popup` is on): the same tree as the Bookmarks tool window (lists, bookmarks with mnemonic icons, descriptions, `file, line N`), size large, remembered; typing a mnemonic key jumps to that bookmark; `Enter`/double-click navigates and closes. | case BKM-005/1 |
| SRCH-BKM-006 | SHOULD | **Lists.** Bookmarks live in named lists; one list is the *default* (new bookmarks go there; marker text `Default`); `Create Bookmark List…` (dialog `Create Bookmark List`, field `Bookmark list:` default `New List`, check box `Use as default list`, error `A list with the specified name already exists`), rename (`Rename Bookmark List…`), `Mark List as Default` / `Unmark List as Default`, delete with confirmation (`Are you sure you want to delete '<name>' bookmark list? This action can't be undone.`; several: `…the selected bookmark lists?…`). The default list cannot be moved. | cases BKM-006/1..2 |
| SRCH-BKM-007 | SHOULD | The Bookmarks tool window (stripe `Bookmarks`): tree of lists and bookmarks with the toolbar (add list, edit, move up/down, sort by type and name, group line bookmarks by file, auto-scroll to/from source, preview), empty text `No bookmarks added.` with the hint `Add Bookmark (F11) to code lines, symbols, files, and directories.`; `Delete` removes (with `Delete Bookmark Node(s)` confirmation unless disabled). | LATER cases |
| SRCH-BKM-008 | LATER | `Bookmark Open Tabs…` (adds every open tab), `Open All Bookmarked Files`, module and root-type bookmarks, restoring bookmark context on branch switching (`Restore bookmark context on branch switching`), breakpoints in the Bookmarks view (registry `ide.bookmark.show.all.breakpoints`, off). | n/a |

## 8. Inventory coverage

`navigation.md` actions: `Back`/`Forward`, `JumpToLastChange`/`JumpToNextChange`, `RecentFiles*`, `RecentLocations`, `RecentChangedFiles*`, `Switcher*`, `DeleteRecentFiles`, `GotoLine`, `NextTab`/`PreviousTab`/`NextEditorTab`/`PreviousEditorTab`,
`ShowContent`, `NextSplitter`/`PrevSplitter`, `NextProjectWindow`, `ShowNavBar`, `NavBar-*`, `NavBarLocation*`, `View*NavigationBar*`, `ShowMembersInNavigationBar`: sections 1 to 6.
`GotoDeclaration`, `GotoDeclarationOnly`, `GotoImplementation`, `GotoTypeDeclaration`, `GotoSuperMethod`, `QuickImplementations`, `PeekDefinition`, `EditSource`, `ViewSource`, `OpenInRightSplit`, `EditSourceInNewWindow`,
`EditSourceNotInEditor`, `GotoTest`, `GotoNextError`/`GotoPreviousError`, `ShowErrorDescription`, `MethodUp`/`MethodDown`, `GotoCustomRegion`, `NextOccurence`/`PreviousOccurence`,
`SelectIn`/`SelectInProjectView`, `GotoChangedFile`, `OpenFile`, `GotoNextElementUnderCaretUsage`/`GotoPrevElementUnderCaretUsage`: owned by `codeinsight/07-navigation.md`, the editor, project-view and VCS specs (listed there or LATER in this area for the popups only).
Hierarchies (`CallHierarchy*`, `MethodHierarchy*`, `TypeHierarchy*`): **LATER**, separate hierarchy-view feature. `structure-view.md`: `FileStructurePopup` section 5; the `Structure` tool window: SRCH-FST-006 LATER. `bookmarks.md`: section 7 (all `ToggleBookmark*`/`GotoBookmark*` ids by the digit/letter rule; `BookmarksView.*` SRCH-BKM-006/007).

## 9. Open questions

1. Switcher mnemonics and the left tool window list are experimental registry options; the default (list shown, digits hidden) is specified.
2. The inherited-members shortcut in the Java File Structure popup: the sources define `Ctrl+I`/`Ctrl+L` for anonymous classes and lambdas; for *Inherited members* the key is the popup action's own (`Ctrl+F12`). Confirm.
3. `Alt+Down` is bound to both `ShowContent` and `MethodDown`: the editor binding wins in editors; confirm context rules.
