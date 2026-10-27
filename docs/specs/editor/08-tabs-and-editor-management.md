# Editor spec 08: editor tabs, splits, history and navigation

Status: specification, language independent. Requirement prefixes `ED-TABS` (tabs, splits,
reopening) and `ED-NAV` (editor history: Back/Forward, change places, Recent Files and
Recent Locations).
Priorities: `MUST` / `SHOULD` / `LATER`.

How files open into editors, where their tabs go, which tab becomes active when one closes, the
tab limit and how tabs are trimmed, preview and pinned tabs, split windows, reopening closed tabs,
the per-file editor state that is restored when a file reopens, and the navigation history.
Go to Declaration and the other navigation commands themselves are in
`codeinsight/07-navigation.md`; this file only covers what they record and where they land.

Executable acceptance cases are in `cases/08-tabs-and-editor-management.cases.md`. In those cases the
tab strip is written `a.txt | [b.txt] | c.txt`: tabs left to right, brackets mark the active tab, a
suffix `(p)` marks a pinned tab and `~` a preview tab. Cases are normative.

ic: `platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/{EditorWindow,EditorsSplitters,
FileEditorManagerImpl,EditorComposite,EditorHistoryManager,IdeDocumentHistoryImpl,ReopenClosedTabAction,
NavigationHistoryContext}.kt`, `platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/text/
TextEditorState.java`, `platform/editor-ui-api/src/com/intellij/ide/ui/UISettingsState.kt`,
`platform/platform-impl/src/com/intellij/ide/actions/{RecentLocationsAction.java,RecentLocationsDataModel.kt}`,
`platform/platform-impl/resources/intellij.platform.ide.impl.xml` (advanced settings),
`platform/util/resources/misc/registry.properties`,
`platform/platform-resources/src/keymaps/$default.xml`,
`platform/platform-resources-en/src/messages/ActionsBundle.properties`,
`platform/ide-core/resources/messages/ApplicationBundle.properties`.

## 1. Defaults and shortcuts

Settings, Editor, General, Editor Tabs (labels from the message bundle; values are the shipped
defaults of `UISettingsState` and the advanced settings).

| Setting (label) | Default |
|---|---|
| Tab placement | Top (other values Left, Bottom, Right, None) |
| Show tabs in one row / hide tabs if there is no space | one row, scrolling (`Scroll the tabs panel`); `Hide tabs if there is no space` on; `Show pinned tabs in a separate row` off |
| Close button position | right |
| Show file icon / Show file extension | file icon on; known extensions shown (the setting hides them: off) |
| Mark modified / Mark modified (*) | modified marker on; asterisk variant off |
| Show full path on mouse hover | on |
| Tab limit | **30** |
| Enable preview tab | off (a product may turn it on) |
| Open declaration source in the same tab (reuse unmodified tabs) | off |
| Open new tabs at the end | off (new tabs open **right of the active tab**) |
| Sort tabs alphabetically | off |
| When the current tab is closed, activate | the tab on the left (alternatives: most recently opened tab; the tab on the right) |
| When tabs exceed the limit | close unused (alternative: close unchanged) |
| Keep pinned tabs on the left | on |
| Maximize or hide tool windows on double click of a tab | `Hide All Tool Windows` on double click on; `Maximize Editor` in splits off |
| Prefer selecting an existing tab in the inactive split pane when navigating | on |
| Recent files limit | 50 |
| Recent locations limit | 25 |
| Navigation history size | 150 places (registry `editor.navigation.history.stack.size`) |

| Action (ID) | Shortcut | Text |
|---|---|---|
| NextTab / PreviousTab | Alt+Right / Alt+Left | Select Next / Previous Tab |
| NextEditorTab / PreviousEditorTab | Alt+Shift+Right / Alt+Shift+Left | next / previous tab in a multi-editor file |
| CloseContent | Ctrl+F4 | Close Tab |
| CloseActiveTab | Ctrl+Shift+F4 | Close Active Tab |
| ReopenClosedTab | none | Reopen Closed Tab |
| Back / Forward | Ctrl+Alt+Left / Ctrl+Alt+Right (mouse buttons 4 and 5) | Back / Forward |
| JumpToLastChange | Ctrl+Shift+Backspace | Last Edit Location |
| RecentFiles | Ctrl+E | Recent Files |
| RecentLocations | Ctrl+Shift+E | Recent Locations |
| Switcher | Ctrl+Tab and Ctrl+Shift+Tab | the file and tool window switcher |
| NextProjectWindow / PreviousProjectWindow | Ctrl+Alt+] / Ctrl+Alt+[ | switch project window |
| OpenInRightSplit | Shift+Enter in a tree, Alt+double-click | open in the right split |
| PinActiveTabToggle, CloseAllEditors, CloseAllEditorsButActive, CloseAllUnmodifiedEditors, CloseAllUnpinnedEditors, CloseAllToTheRight, CloseAllToTheLeft, CloseAllReadonly, SplitVertically, SplitHorizontally, ChangeSplitOrientation, NextSplitter, PrevSplitter, MoveEditorToOppositeTabGroup | none | `Pin Active Tab`, `Close All Tabs`, `Close Other Tabs`, `Close Unmodified Tabs`, `Close All but Pinned`, `Close Tabs to the Right`, `Close Tabs to the Left`, `Close All Read-Only Tabs`, `Split Right`, `Split Down`, `Change Splitter Orientation`, `Go to Next Splitter`, `Go to Previous Splitter`, `Move to Opposite Group` |

## 2. Opening files

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TABS-001 | MUST | Opening a file already open in the target window selects its tab (no second tab in that window); otherwise a new tab is created. A file may be open in several split windows (one tab per window) and all those editors share one document, so text edits show in all of them while carets, selections, scroll position and folding are per editor. | Open the same file in two splits: typing in one appears in the other. |
| ED-TABS-002 | MUST | A new tab is inserted **right of the active tab** (index of the active tab plus one); with `Open new tabs at the end` at the end of the strip; a pinned tab and `Keep pinned tabs on the left` keep pinned tabs before unpinned ones; with `Sort tabs alphabetically` at the alphabetical position. The new tab becomes active unless the open request says otherwise (background open). | `a \| [b] \| c` open d: `a \| b \| [d] \| c`. |
| ED-TABS-003 | MUST | Opening restores the **editor state** stored for that file in the editor history (caret and selection, scroll position, folding state, the selected editor provider among text and designer views), unless the request carries an explicit position (navigation to a declaration or a line), which wins. Files never seen before open with the caret at the start (or at the position given) and default folding (`ED-FOLD-008`). | Close and reopen a file: caret and folds as left. |
| ED-TABS-004 | MUST | Opening focuses the editor and the text area gets keyboard focus (so typing works at once), unless the opener asked for no focus (a project tree preview). The window title and the status bar are refreshed. | Open a file and type. |
| ED-TABS-005 | MUST | **Preview tab** (`Enable preview tab`): a file opened by a single click in the project tree and some similar passive opens (and during debugging) opens in the window's single preview tab, shown with an italic title. Opening another passive file **replaces** the content of the preview tab (it stays at the same position, which is the position after the last preview tab). A preview tab becomes a normal tab when the user edits the file, double-clicks the tab, pins it, or opens the file by an explicit action (double click in the tree, Enter, Go to File). Preview tabs are closed first when trimming for the tab limit. | Click a.txt then b.txt in the tree: one preview tab showing b.txt. |
| ED-TABS-006 | SHOULD | `Open declaration source in the same tab` (default off): navigating to a declaration replaces the current tab if its file is unmodified and not pinned; with `Open declaration source called from a detached window in the main IDE window` (off) navigation from a detached window opens in the main window. | Setting on: Go to Declaration reuses the tab. |

## 3. Closing, the active tab, the tab limit

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TABS-010 | MUST | **Which tab becomes active** when the active tab is closed: a tab that was not active stays as it is; otherwise with `most recently opened tab` the most recent file of the editor history that is still open in this window; with `the tab on the right` the next tab if there is one; otherwise (default) the tab on the left of the closed one; when the closed tab was the first, the strip picks its default (the tab that now occupies the first position); with no tabs left the window shows the empty-editor state. | `a \| [b] \| c` Close: `[a] \| c`. |
| ED-TABS-011 | MUST | Close actions: `Close Tab` (Ctrl+F4) closes the active tab; `Close Other Tabs` closes all except the active one and pinned ones; `Close All Tabs` closes all unpinned tabs; `Close All but Pinned`; `Close Tabs to the Right/Left`; `Close Unmodified Tabs`; `Close All Read-Only Tabs`. Closing a modified file never asks to save (documents are saved by autosave or on frame deactivation, `platform/09-saving-and-autosave.md`); its edit history is kept for the project session. | Close modified tab: no dialog. |
| ED-TABS-012 | MUST | The **tab limit** (default 30, 1 to 100 as registry `ide.max.editor.tabs`) applies per split window: when opening a tab makes the count exceed the limit, tabs are closed until it fits, except pinned tabs, the file just opened and tabs the auto-closing rule excludes. The closing order is: preview tabs first; then files not in the editor history; with `close unchanged` the unmodified ones in history order first; then the remaining files **from the least to the most recently used**, the active file last. | Limit 3 with `a \| b \| [c]`, open d: a is closed. |
| ED-TABS-013 | MUST | **Reopen Closed Tab** (no default key): each window keeps a stack of the tabs closed in it (at most the tab limit entries, oldest dropped); the action reopens the most recently closed one at its former index and pin state and activates it; when the window has none it reopens the last file of the editor history; it is disabled when neither exists. | Close b, reopen: `a \| [b] \| c`. |
| ED-TABS-014 | MUST | A file that is deleted or moved while open: its tab closes (a deleted file) or stays with the new name (a rename or move); history entries of invalid files are dropped. | Delete the file in the tree: its tab disappears. |
| ED-TABS-015 | MUST | **Pin**: `Pin Active Tab` toggles; a pinned tab shows the pin icon instead of the close button, is skipped by the Close All and Close Other actions and by the tab limit's trimming, and with `Keep pinned tabs on the left` (default) moves to the left group (pinned tabs keep their relative order). Pinned state persists with the project. A preview tab that is pinned becomes a normal tab. | `a \| [b]`, pin b: `[b(p)] \| a`. |
| ED-TABS-016 | SHOULD | Tab context menu: the close actions above, `Pin Active Tab`, `Split Right`, `Split Down`, `Open in Right Split`, `Copy Path`, `Reveal in Project View`, `Show in Files`, `Rename File`, `Mark as Read-only`. Dragging a tab reorders it, dragging it out creates a window, dragging it onto an edge of the editor area splits (drag-to-split ratio 0.2 of the area), middle-click closes it. Double-click on the tab strip hides or restores tool windows (advanced setting on). | Middle-click closes. |
| ED-TABS-017 | MUST | Next/Previous Tab (Alt+Right/Left) selects the neighbour in strip order; at the ends the selection wraps (the product's cyclic scrolling setting `ide.cycle.scrolling` is on by default). `Select Next/Previous Tab in Multi-Editor File` cycles the text and designer views of one file. | `[a] \| b \| c` Alt+Right: `a \| [b] \| c`. |

## 4. Splits

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TABS-020 | MUST | `Split Right` (vertical divider) and `Split Down` (horizontal divider) split the active window: a new window opens next to it, showing the same file with the same editor state (exact state: caret, scroll, folds), the splitter at 50 percent (limits 10 percent and 90 percent), and focus moves to the new window (or not, per the invoker). A split with a file given opens that file in the new window. | Split Right on `[a]`: two windows each with a.txt. |
| ED-TABS-021 | MUST | Splits form a tree of nested splitters; `Change Splitter Orientation` flips the nearest splitter; `Go to Next/Previous Splitter` moves focus; `Unsplit` and `Unsplit All` merge the windows: the tabs of the removed window are moved into the sibling (while the sibling stays under the tab limit); closing the last tab of a split window removes the window. The advanced settings `Equalize proportions in nested splits` (default off) and `Maximize editor in splits on double click` (off). | Close the last tab of the right window: left window fills the area. |
| ED-TABS-022 | MUST | Opening a file from a navigation command while a file is already open in the **other** split selects that existing tab instead of opening a duplicate in the active split (`Prefer selecting an existing tab in the inactive split pane`, default on). | A open in the right split; Go to Declaration to A from the left: right tab selected. |
| ED-TABS-023 | SHOULD | Floating (detached) editor windows can be created from a tab; each has its own splitters and the last-closed-tab stack; when the project closes they close too. | Tab "Open in New Window". |

## 5. Editor state and history manager

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-NAV-001 | MUST | The **editor history** keeps, per file opened in the project (most recently used last, at most the recent-files limit plus one entries, default 51), the selected editor kind and the saved **full editor state** (carets and selections, scroll position, folding state) per editor kind. An entry moves to the top whenever the file is opened or its tab selected, and its state is refreshed when the tab is closed or the project closes. The history persists in the project's workspace data and is used to restore state, to order Recent Files, to choose the next active tab and the tab-closing order. | Close the project and reopen: states restored. |
| ED-NAV-002 | MUST | **Navigation history** (Back and Forward): a stack of *places* (file, editor kind, navigation-level state: caret positions and scroll position). A place is recorded when a command that is marked as **navigation** (Go to Declaration, Go to Line, jump to code block end, Ctrl+Home and Ctrl+End, switching tabs or files, Find in Path result opening, bookmarks, ...) finishes and it moved the caret to another line; the recorded place is the one at the **start** of the command. Plain caret movement by keys or mouse, typing and scrolling are not navigation. | Go to Line 50 then 100, Back: line 50. |
| ED-NAV-003 | MUST | **Merging**: a new place equal to the previous one (same file and the carets within **fewer than 4 lines** of each other) replaces it, so small hops do not fill the history; the stack is capped at 150 places (oldest dropped); a new navigation clears the forward stack; Back moves the current place onto the forward stack and goes to the previous one; Forward goes the other way and skips places equal to the current one; places of invalid files are removed lazily. | Two jumps of 2 lines record one place. |
| ED-NAV-004 | MUST | Back and Forward restore the editor state of the place (caret, scroll) and the tab (opening the file if it was closed, in the same window and preview state if possible), as a command that does not itself add navigation places. `Back` is disabled with an empty stack, `Forward` likewise; both are also on the toolbar and the mouse side buttons. | Back after closing the tab reopens it. |
| ED-NAV-005 | MUST | **Change places** (Last Edit Location, Ctrl+Shift+Backspace): a second list records the place of each command that modified a file's document (excluding changes marked external); each file appears once per run of commands; the list is capped at 150 and persists the paths of the most recently changed files (limit: the recent-files limit plus one). `Last Edit Location` goes to the most recent change place that differs from the current location; repeated invocation walks back through older change places; `Next Edit Location` the other way. A caret move that is not a change resets the walk. | Edit in two files, Ctrl+Shift+Backspace twice: both places visited. |
| ED-NAV-006 | SHOULD | The files-with-changes list (`Recently Changed Files`, an option of Recent Files) lists the changed paths, newest first. | Recent Files, toggle `Show changed only`. |

## 6. Recent Files, Recent Locations, Switcher

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-NAV-010 | MUST | **Recent Files** (Ctrl+E): a popup listing the most recently used files of the editor history, newest first, up to the recent-files limit (50), each with its icon and name and the open/changed marker; also tool windows on the right in the new layout; speed search filters by typing (camel-case matching on, matching anywhere), Enter opens the selected file, Delete removes it from the list. Pressing Ctrl+E again toggles `Show changed only` (the files changed recently, ordered by time). The previously used file is pre-selected so that Ctrl+E then Enter returns to it. | Ctrl+E lists files; type `ab` filters. |
| ED-NAV-011 | MUST | **Recent Locations** (Ctrl+Shift+E): a popup of at most 25 places (the recent-locations limit) from the navigation history, each shown as a code fragment around the place with the file name; the toggle `Show changed only` (same key) switches to the change places; a preview pane shows the code around the place; Enter navigates there. | Ctrl+Shift+E: fragments with file names. |
| ED-NAV-012 | SHOULD | **Switcher** (hold Ctrl, press Tab): a popup with two lists, tool windows and files; Tab or Shift+Tab move the selection, releasing Ctrl opens the selected item; the file list is ordered like Recent Files with the previously used file selected first; Delete closes the selected tab; Escape cancels. | Ctrl+Tab then release: previous file. |
| ED-NAV-013 | SHOULD | Opening a file from these popups uses the same tab placement and state restore as any open. | Same as ED-TABS-002 and ED-TABS-003. |

## 7. Open questions

* The complete list of commands that mark themselves as navigation (the mouse click, find next and
  incremental search behaviors are not confirmed from the sources read).
* The exact contents of the editor-level "auto-closing" exclusion used by the tab limit
  (`EditorAutoClosingHandler`), believed to protect pinned and some special editors.
* Whether Alt+Left at the first tab wraps in the default keymap (taken from `ide.cycle.scrolling`).
* The layout and keys of Recent Files in the new IDE layout (tool windows list).
* The tab context menu entries, drag gestures and middle-click behavior (ED-TABS-016) and the Recent Files
  popup details (ED-NAV-010) are product observations that were not confirmed from the sources read.
