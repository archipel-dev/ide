# Workbench spec 04: status bar and its widgets

Status: specification (clean-room). ID prefix `WB-SB`. Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`.
Executable cases: `cases/04-status-bar.cases.md`.

Scope: the status bar frame, the widget model (ids, order, enable/disable, context menu), and every standard widget: position, line separator,
encoding, indentation, read-only attribute, column selection, notifications, memory, power save, file-system sync, large-file encoding, internal error,
settings entry point, plus the placeholders for VCS branch, background progress and the analysis status. Background progress and the notification flows are
detailed in `07-notifications-and-background-tasks.md`; encoding and line-separator **conversion** in `08-file-operations-and-scratches.md`.

Sources studied: `ic: platform/platform-impl/src/com/intellij/openapi/wm/impl/status/` (`PositionPanel.kt`, `LineSeparatorPanel.java`, `EncodingPanel.java`, `MemoryUsagePanel.java`,
`ColumnSelectionModeWidgetFactory.java`, `ReadOnlyAttributeWidgetFactory.java`, `PowerSaveStatusWidgetFactory.java`, `MemoryIndicatorWidgetFactory.java`,
`VfsRefreshIndicatorWidgetFactory.java`, `FatalErrorWidgetFactory.java`, `widget/StatusBarWidgetsManager.kt`), `ic: platform/platform-api/src/com/intellij/openapi/wm/StatusBarWidgetFactory.java`,
`ic: platform/lang-impl/src/com/intellij/psi/codeStyle/statusbar/CodeStyleStatusBarWidget.kt`, `ic: platform/core-api/.../IndentStatusBarUIContributor.java`,
`ic: platform/platform-impl/src/com/intellij/notification/impl/widget/NotificationWidgetFactory.java`, `ic: platform/platform-impl/src/com/intellij/ide/util/GotoLineNumberDialog.java`,
`EditorGotoLineNumberDialog.java`, `ic: platform/platform-impl/src/com/intellij/openapi/vfs/encoding/ChangeFileEncodingAction.java`,
`ic: platform/platform-api/resources/messages/UIBundle.properties`, `IdeBundle.properties`, `ic: platform/platform-impl/resources/idea/PlatformActions.xml`, inventory `settings-keymap.md` (14 widget rows).

## 1. The status bar

The status bar is the strip at the bottom of the main frame. Left: the **status text** (hints, short messages) followed by the progress of background tasks
(see 07). Right: the **widgets**, ordered left to right as in section 3. A widget is either text, an icon, or a custom component.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SB-001 | MUST | The status bar is shown by default (`showStatusBar` = true) and toggled by View > Appearance > Status Bar (`ViewStatusBar`, text "Status Bar"). It is per frame. | Toggle it off: strip gone, no panic; on: restored with the same widgets. |
| WB-SB-002 | MUST | Widgets are registered by id with a **display name** (used in menus), **order constraints** (`first`, `last`, `after X`, `before Y`) and defaults: *available* (is it offered at all for this project/editor), *enabled by default*, *configurable*. Ordering resolves constraints deterministically; unknown ids in constraints are ignored. | Register A `after Position` and B `before A`: order Position, B, A. |
| WB-SB-003 | MUST | The context menu of the status bar (right click on empty space or on a widget) lists every **configurable and available** widget by display name as a checkbox (menu text of the toggle action: "Toggle '{name}' Widget" in Search Everywhere; "Hide '{name}'" on the widget's own menu). Toggling creates or disposes the widget at once and persists the choice per **application** (not per project). | Right-click, uncheck "Line:Column Number": the position text vanishes; restart: still hidden. |
| WB-SB-004 | MUST | A widget disabled by the user stays disabled across restarts; a widget never touched by the user follows `enabled by default`. | A widget with default off appears only after the user checks it. |
| WB-SB-005 | MUST | Editor-based widgets show only while the selected editor is a text file editor of a real file; with no editor (welcome, image, empty), they are hidden (not just empty). They update when the selected file changes, when the caret moves (Position) or when the underlying property changes (encoding, separator, indent settings). | Close the last editor: position, separator, encoding, indent widgets disappear; open a file: they come back. |
| WB-SB-006 | MUST | Widget text updates are debounced (Position: 100 ms) and never block the UI thread. | Hold Down in a long file: the position text follows within 100 ms of the last caret move. |
| WB-SB-007 | MUST | Every widget has a tooltip (named per widget below) and an accessible name; widgets with a click action show a hand/pointer cursor; widgets that are not enabled (`action disabled`) do nothing on click. | Hover the position: tooltip "Go to Line (Ctrl+G)". |
| WB-SB-008 | SHOULD | "Show Status Text" (`StatusTextModeAction`, menu "Status Text" under View > Appearance > Status Bar Widgets) controls whether the left text area shows messages. | Toggle off: hints no longer appear. |

## 2. Widget catalog (what exists, defaults)

| Id | Display name | Shown when | Enabled by default | Text / content | Tooltip |
|---|---|---|---|---|---|
| `Position` | Line:Column Number | text editor open | yes | `L:C`, selection suffix, multi-caret form (section 4) | "Go to Line" (+ shortcut " (Ctrl+G)") |
| `LineSeparator` | Line Separator | file with a document | yes | `LF`, `CRLF` or `CR` | "Line Separator: \n" (the separator escaped: `\n`, `\r\n`, `\r`) |
| `Encoding` | File Encoding | any file (not directory) | yes | charset display name, for example `UTF-8` | "File Encoding: UTF-8" plus " (reason)" when the encoding is fixed, for example by BOM or content |
| `PowerSaveMode` | Power Save Mode | always | **no** | eye icon, power-save icon while on | "Power save mode: enabled" / "Power save mode: disabled" |
| `InsertOverwrite` / `ColumnSelectionMode` (id `InsertOverwrite`, display name "Editor Selection Mode") | Editor Selection Mode | editor in column selection mode | yes (but only offered in column mode) | text "Column" | "Column selection mode" |
| `CodeStyleStatusBarWidget` | Indentation | writable file with language settings | yes | `Tab` or `N spaces` | "Indent: 4 spaces" with a grey hint of the source (for example "EditorConfig", "Detected") |
| `ReadOnlyAttribute` | Read-Only Attribute | file with a document | yes | lock icon (locked/unlocked) | file read-only state |
| `Notifications` | Notifications | **only when tool window bars are hidden or in presentation mode** | yes (when available) | bell icon with unread badge | "{n} notification(s) pending" / "No new notifications" |
| `FatalError` | Internal Error | only while internal errors exist | not configurable | red icon | error summary |
| `Memory` | Memory Indicator | always | **no** | `{used} of {max}M` over a bar | heap/process breakdown |
| `VfsRefresh` | File System Sync | while a VFS refresh runs | **no** | refresh progress icon | "File system sync is not running" when idle |
| `largeFileEncodingWidget` | Large File Encoding | large-file editor | yes | encoding text | "File Encoding: {x}" |
| `settingsEntryPointWidget` | (settings entry point) | always | yes | gear icon | opens the settings entry menu (see 05) |
| `git` (VCS branch), `light.edit.git`, `IncomingChanges` | Git branch | project under Git | yes | branch name | VCS area (`LATER` here, reserve slot) |
| `JSONSchemaSelector`, `LanguageServiceStatusBarWidget`, `McpServerStatusBarWidget`, `SmartModeIndicator`, `WriteThread`, `IndexesAndVfsFlushIndicator`, `EditorAnimationCacheStatistics`, `light.edit.large.file.encoding.widget` | plugin / internal widgets | | | | LATER (other areas or internal) |

Fixed **order** between the widgets that exist together (left to right at the right end of the bar): `VfsRefresh` (first), `Position`, `LineSeparator`, `Encoding`, `PowerSaveMode`,
`InsertOverwrite` (Column), `CodeStyleStatusBarWidget` (Indentation), `git` (branch), `ReadOnlyAttribute`, `Notifications`, `FatalError`, `Memory` (last), `settingsEntryPointWidget` (last).
A language-service widget sits after Position and before LineSeparator; plugin widgets use their own constraints.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SB-010 | MUST | The widgets of the catalog exist with these ids, display names and defaults; PowerSaveMode, Memory and VfsRefresh are **off by default**; Notifications only when stripes are hidden (WB-TW-006) or presentation mode. | Fresh profile with a Java file open: right side shows `L:C`, `LF`, `UTF-8`, `4 spaces`, lock; no memory bar, no power-save icon. |
| WB-SB-011 | MUST | The order is as above and stable across sessions. | Enable Memory and PowerSave: order is ... Encoding, PowerSave, ..., Memory (last). |
| WB-SB-012 | MUST | "Internal Error" (`FatalError`) is not user-configurable and appears only while unseen internal errors exist; clicking opens the error dialog (Archipel: error report). | Cause an internal error: red icon appears; clear errors: icon disappears. |
| WB-SB-013 | MUST | The widget display names in the context menu are exactly: "Line:Column Number", "Line Separator", "File Encoding", "Editor Selection Mode", "Read-Only Attribute", "Notifications", "Internal Error", "Indentation", "Memory Indicator", "File System Sync", "Power Save Mode", "Large File Encoding". | The context menu lists these names (those available in the current state). |

## 3. Position widget (`Position`)

Text rules (computed for the **selected text editor**, primary caret):

1. One caret, no selection: `{line}:{column}` where `line` and `column` are the **1-based logical** line and column of the caret. The logical column counts every character cell with tabs expanded to their tab stops and ignores soft wraps and folded regions.
2. One caret, non-empty selection: the text of rule 1, then ` (` + `{n} char` / `{n} chars` (n = Unicode **code points** in the selection) + optionally `, {m} line break` / `, {m} line breaks` (m = line of the selection end minus line of the selection start, shown only if m > 0) + `)`.
   Example: caret at `2:3` after selecting `ab\ncd` completely: `2:3 (5 chars, 1 line break)`.
3. If the selection is 500 000 chars or longer the count is computed asynchronously; the text shows `... chars` until the count arrives, then the number replaces the dots.
4. More than one caret: `{k} carets` (no line, column or selection info).
5. Empty (no text editor, or position panel disabled for that editor): the widget is hidden.

Click or Ctrl+G (`GotoLine`, menu "Line:Column...") opens the dialog **Go to Line:Column** (title `Go to Line:Column`): label `[Line] [:column]:` and one text field of width 200 px, prefilled with the current `line:column` (1-based) and fully selected; OK/Cancel.

Parsing of the input (field content): optional line number, then an optional separator `:` or `,` (spaces allowed), then an optional column number.
* line missing: the caret's current line; line `0` is invalid; column missing or `0` or `1`: column 1 (index 0).
* In an editor, text of the form `+N` or `-N` is a **relative** line move from the caret line by N logical lines (result clamped to `[first, last]` line), column 1.
* Text that does not match the pattern (letters, a second colon, ...) does nothing and **keeps the dialog open**. Empty text matches: current line, column 1.
* On OK: secondary carets are removed, the caret moves to the logical position, the view scrolls the caret to the **center**, the selection is removed, focus returns to the editor and the position is recorded as a navigation entry.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SB-020 | MUST | Position text follows rules 1-5. | Cases WB-SB-020 case 1..5. |
| WB-SB-021 | MUST | Tooltip "Go to Line" with " (Ctrl+G)" appended when a first keyboard shortcut for `GotoLine` exists and help tooltips are off. | Default keymap: tooltip "Go to Line (Ctrl+G)". |
| WB-SB-022 | MUST | The dialog and its parsing as above. | Cases WB-SB-022 case 1..6. |
| WB-SB-023 | MUST | Column in the dialog is clamped to the line's length (past the end places the caret at the end of the line); line past the end places the caret on the last line. | 3-line file, `99:1`: caret on line 3. |
| WB-SB-024 | SHOULD | Pressing Enter in the field confirms, Escape cancels with no caret change. | Escape: caret unchanged. |

## 4. Line separator widget (`LineSeparator`)

* Text: `LF`, `CRLF` or `CR`, the separator of the file as the document manager reports it (the one the file will be saved with).
* Click: a popup titled **"Line Separator"** (speed search enabled) with the group `ChangeLineSeparators` = three entries in this order: **CRLF - Windows (\r\n)**, **LF - Unix and macOS (\n)**, **CR - Classic Mac OS (\r)**; the current one is checked. The wording after the dash is the reference's `Windows (\r\n)`, `Unix and macOS (\n)`, `Classic Mac OS (\r)`. The same group is File | File Properties | Line Separators ("Line Separators").
* Choosing an entry converts the **document text's** line separators for that file and marks the document modified; undoable.
* Hidden for read-only files in the sense that the click does nothing when the file is not writable.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SB-030 | MUST | Text and tooltip as above; tooltip `Line Separator: \n` with the separator escaped. | File with CRLF: text `CRLF`, tooltip `Line Separator: \r\n`. |
| WB-SB-031 | MUST | Popup contents and order as above, current entry checked. | LF file: popup shows the three rows, second checked. |
| WB-SB-032 | MUST | Choosing a different separator rewrites the file's separator: after Save the file on disk contains the new separator for every line break. New files created by the IDE use the separator of the code-style setting for new files (see 08). | LF file, choose CRLF, Save: file bytes contain `\r\n` only. |
| WB-SB-033 | SHOULD | The choice is a single undo step ("Ctrl+Z" restores the old separator and widget text). | Ctrl+Z: widget back to `LF`. |

## 5. Encoding widget (`Encoding`)

* Text: the charset's display name (`UTF-8`, `windows-1252`, `ISO-8859-1`, ...). If the charset cannot be determined the text is `N/A`.
* Tooltip: `File Encoding: {name}` with ` ({reason})` appended when the file's encoding is **fixed** (for example, it has a BOM, or it is `utf-8` declared in content); with a reason the widget's click action is disabled.
* Click: popup titled "File Encoding" listing charsets (favorites and a `More` submenu), speed search on; an entry may be "System Default"-style items for the default encoding. Each entry's text is **`Change encoding to '{charset}'`** (reference message `action.text.change.encoding`).
* Choosing a charset **c** for a file:
  1. If converting to `c` and reloading in `c` are both "absolutely safe" (the text is identical either way), the file's encoding setting changes silently (stored per project, as the encoding for that file) and the editor shows the same text; undoable as one step named "Change Encoding for '{file}'".
  2. Otherwise the dialog **"Incompatible Encoding: {file}"** (also titled `{file}: Reload or Convert to {charset}`) appears: message `The encoding you've chosen ('{charset}') may change the contents of '{file}'. Do you want to 1. Reload the file from disk in the new encoding '{charset}' and overwrite editor contents or 2. Convert the text and overwrite file in the new encoding?`, buttons **Reload**, **Convert**, Cancel.
  3. **Reload** discards editor contents and reads the file's bytes with `c`. **Convert** keeps the text and saves it with `c`; refused with "Encoding '{charset}' does not support some characters from the text." if characters would be lost; the check "Please do not convert to '{charset}'." applies to charsets that must not be a target (for example ones that cannot represent text).
* Entries are disabled when neither convert nor reload is possible (for example, file has no document, or is binary).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SB-040 | MUST | Text, tooltip, `N/A` fallback as above. | UTF-8 file: text `UTF-8`, tooltip `File Encoding: UTF-8`. |
| WB-SB-041 | MUST | The click popup lists charsets and applies the flow above; the change persists per file in project settings (see 05, File Encodings). | Pick `windows-1252` for an ASCII-only file: no dialog; widget `windows-1252`. |
| WB-SB-042 | MUST | The Incompatible Encoding dialog has exactly Reload / Convert / Cancel with the message above; Cancel changes nothing. | File with "é" read as UTF-8; choose `ISO-8859-1`: dialog; Cancel: unchanged. |
| WB-SB-043 | SHOULD | Add BOM / Remove BOM entries (`AddBom`, `RemoveBom`) appear in the encoding popup for UTF encodings. | UTF-8 file without BOM: popup offers "Add BOM". |

## 6. Indentation widget (`CodeStyleStatusBarWidget`)

* Visible only for **writable** files of languages with indent options.
* Text: **`Tab`** when the effective indent options use tab characters, otherwise `{n} space` / `{n} spaces` with n the indent size (1 -> `1 space`).
* Tooltip: `Indent: {text}` plus, in grey, the source of the setting (for example EditorConfig or the detected indent).
* Click: a popup titled `{Language} Code Style` with, in this order: provider actions (for example "Indents Detection" actions: `Enable Indents Detection`, `Disable Indents Detection`, `Indent New Lines with {x}`, `Reindent File with {x}`), then **`Configure Indents for {Language}...`**, then provider-specific disable/show-all entries. "Configure Indents" opens Settings | Editor | Code Style | {Language}.
* Updates when code style settings change.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SB-050 | MUST | Text `Tab` / `N spaces` from the effective options of the file (code style plus EditorConfig plus detected indent). | Java file, default settings: `4 spaces`; a `.editorconfig` with `indent_style = tab`: `Tab`. |
| WB-SB-051 | MUST | Tooltip `Indent: {text}` (+ source hint). | Tooltip for default: `Indent: 4 spaces`. |
| WB-SB-052 | MUST | Click opens the popup with `Configure Indents for {Language}...` which opens the code style page of that language. | Java: entry "Configure Indents for Java...". |
| WB-SB-053 | SHOULD | Hidden for read-only files and files without a language. | A read-only file: widget hidden. |

## 7. Read-only attribute, column selection, power save, memory, file-system sync, notifications

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-SB-060 | MUST | **Read-Only Attribute** (`ReadOnlyAttribute`): an icon showing the file's read-only (VFS write) attribute: locked padlock for read-only, open padlock for writable; tooltip names the state; click toggles the attribute (`ToggleReadOnlyAttribute`, text "Toggle Read-Only Attribute"; menu variants "Make file read-only" / "Make file writable") after which the editor becomes read-only / writable. Disabled for files that cannot change attributes (jar contents, non-local). Enabled by default (unless the simplified status bar registry flag is on). | Click on a writable file: padlock locks; typing in the editor is rejected with the read-only hint; click again: writable. |
| WB-SB-061 | SHOULD | **Column selection mode** widget: appears only while the editor is in column selection mode (`EditorToggleColumnMode`, text "Column Selection Mode"); text "Column", tooltip "Column selection mode"; clicking it leaves column mode (assumed, open question 2). | Alt+Shift+Insert: widget "Column" appears; toggle again: disappears. |
| WB-SB-062 | SHOULD | **Power Save Mode** (default off): icon (eye normal, power-save icon when on); tooltip "Power save mode: enabled" or "Power save mode: disabled"; click toggles power save mode (analysis off; see PLAT/daemon). Changing the mode from the menu updates the icon. | Enable widget, click: tooltip "Power save mode: enabled". |
| WB-SB-063 | SHOULD | **Memory indicator** (default off): text `{used} of {max}M` over a two-shade bar (used, allocated; background panel colour) sized for the widest value; single click triggers a GC and refresh, double click a deeper full cleanup; tooltip lists Heap (Used, Committed, Max), direct buffers, internals and a total estimate. Archipel reports its own process: Used = resident set, Max = configured budget; tooltip rows are "Used", "Committed", "Max". Accessible name "Memory Usage: {text}". | Enable: text like `412 of 2048M`; click: value refreshes within one second. |
| WB-SB-064 | SHOULD | **File System Sync** (default off, available only while the VFS refresh indicator is relevant): shows an animated icon while a VFS refresh runs; tooltip "File system sync is not running" when idle. | Enable, run Reload All from Disk: icon animates, then idle tooltip. |
| WB-SB-065 | MUST | **Notifications** widget: available only when tool window bars are hidden or in presentation mode; bell icon with a badge for unread notifications; tooltip `{n} notification(s) pending` or `No new notifications`; click opens the Notifications tool window (see 07). | Hide stripes, a warning notification arrives: badge 1, tooltip "1 notification pending"; click: window opens and badge clears. |
| WB-SB-066 | SHOULD | **Large File Encoding** widget for the large-file editor: text encoding; tooltip `File Encoding: {x}`; encoding can not change while a search runs ("Search is in progress.") or when the file has a BOM ("The file has a BOM (Byte order mark)."). | Large file editor open: widget shown; others hidden. |
| WB-SB-067 | MUST | The VCS branch widget occupies its order slot (after Indentation, before Read-Only) once VCS areas register it; text is the branch name; click opens the branch popup. | Placeholder: ordering test with a stub widget `git`. |
| WB-SB-068 | MUST | There is **no** status-bar widget for analysis status in Archipel; the daemon status (errors count, analysis progress) is the editor's top-right inspection widget (`docs/specs/codeinsight/01-daemon.md` section 9). Problems counts also appear in the Problems window (`03-structure-problems-todo.md`). | The status bar has no inspection icon. |
| WB-SB-069 | LATER | Plugin and internal widgets of the catalog table (language services, JSON schema selector, MCP server, smart-mode, write thread, index flush). | n/a |

## 8. Inventory coverage

All 14 widget rows of `inventory/settings-keymap.md` (`largeFileEncodingWidget`, `CodeStyleStatusBarWidget`, `light.edit.large.file.encoding.widget`, `VfsRefresh`, `Position`,
`LineSeparator`, `Encoding`, `PowerSaveMode`, `InsertOverwrite`, `ReadOnlyAttribute`, `Notifications`, `FatalError`, `Memory`, `settingsEntryPointWidget`) appear in section 2; `light.edit.*` (LightEdit mode) is LATER.
Other-area rows (`git`, `IncomingChanges`, `JSONSchemaSelector`, `LanguageServiceStatusBarWidget`, `McpServerStatusBarWidget`, `SmartModeIndicator`, `WriteThread`, `IndexesAndVfsFlushIndicator`, `EditorAnimationCacheStatistics`) are LATER or owned elsewhere.

## 9. Open questions

1. Click behavior and exact list contents of the encoding popup's charset entries (item texts, favorites, "More"): the item text (`Change encoding to '{charset}'`) and the flow were read; the order of charsets, the favorites section and the `More` submenu need a reference run.
2. Column selection widget click action: not read (`ColumnSelectionModePanel`).
3. Exact text of the line-separator menu entries: texts are produced by the actions' code, not bundles; the form `CRLF - Windows (\r\n)` is expected.
4. Read-only widget tooltip texts and icons (UI strings from the toggle panel were not read).
5. Whether the Position widget is hidden in diff/console editors (flag `positionPanel.disableForEditor`: the editor can opt out; list of opting-out editors not enumerated).
