# 01. Local changes, changelists, file status, gutter markers

Status: specification, observable behavior of IntelliJ IDEA Community. Requirement prefix `VCS-CHG`.
Implementation language neutral. Git-specific change detection is in section 9; commit is in
`02-commit.md`.

ic: `platform/vcs-impl/src/com/intellij/openapi/vcs/changes/ChangeListManagerImpl.kt`,
`ChangeListWorker.java`, `VcsDirtyScopeManagerImpl.kt`, `VcsDirtyScopeVfsListener.kt`,
`UpdateRequestsQueue.java`, `ChangesViewManager.kt`, `UnversionedViewDialog.java`,
`IgnoredViewDialog.java`, `VcsIgnoreManagerImpl.kt`, `DefaultIgnoredFileProvider.java`
(all under `platform/vcs-impl/src/com/intellij/openapi/vcs/changes/`);
`platform/vcs-api/shared/src/com/intellij/openapi/vcs/{VcsConfiguration,VcsApplicationSettings}.java`;
`platform/editor-ui-api/src/com/intellij/openapi/vcs/FileStatus.java`;
`platform/platform-resources/src/themes/expUI/expUI_{light,dark}Scheme.xml`;
`platform/diff-impl/src/com/intellij/openapi/vcs/ex/*`,
`platform/diff-impl/src/com/intellij/openapi/diff/LineStatusMarkerDrawUtil.java`,
`platform/vcs-impl/src/com/intellij/openapi/vcs/ex/*`;
`platform/vcs-impl/resources/META-INF/VcsActions.xml`,
`platform/vcs-impl/shared/resources/intellij.platform.vcs.impl.shared.xml`;
`platform/vcs-api/vcs-api-core/resources/messages/VcsBundle.properties`;
`plugins/git4idea/backend/src/status/GitChangeProvider.java`,
`plugins/git4idea/backend/src/repo/GitUntrackedFilesHolder.kt`.

Related: saving documents before VCS operations is `PLAT-SAVE-040..045`
(`docs/specs/platform/09-saving-and-autosave.md`); document model and VFS events are
`PLAT-VFS-*` in `docs/specs/platform/01-vfs-and-documents.md`.

Priority legend: MUST = needed for a usable Git workflow; SHOULD = expected for parity;
LATER = specified for completeness. In tables the product's menu path separator `|` is
written `>`.

## 1. Concepts

- **Change**: a pair (before revision, after revision) for one path. Before absent = added;
  after absent = deleted; different paths = move/rename; both present and same path = modified.
- **Local changes**: the set of Changes of the working tree relative to the VCS base
  (for Git: HEAD plus, when the staging area is enabled, the index).
- **Changelist**: a named, ordered container of Changes. Fields: name (unique, non-empty),
  comment (free text), `active` flag (exactly one changelist is active), `readOnly`,
  `data` (opaque plugin data). The built-in changelist is named `Changes` (older projects may
  still carry the legacy name `Default Changelist`; it is shown as is, never renamed
  automatically).
- **Unversioned file**: a file in a VCS root that the VCS does not track and does not ignore.
- **Ignored file**: a file matched by the VCS ignore rules (Git: `.gitignore`, `.git/info/exclude`,
  global excludes file).
- **Dirty scope**: the set of paths whose status must be recomputed. The provider recomputes only
  the dirty scope, never the entire tree, except on explicit full refresh or VCS mapping change.
- **Gutter marker / line status**: per-line comparison of the editor document with the VCS base
  revision of the file, shown as colored bars in the editor gutter and optionally in the error
  stripe.

## 2. Changelists

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-CHG-001 | MUST | A project with an active VCS always has at least one changelist, and exactly one is active. The first changelist is named `Changes` and is active. | Fresh Git project with a modified file: Local Changes shows one group `Changes` containing the file; its node has the `Active` marker (bold). |
| VCS-CHG-002 | MUST | Every Change belongs to exactly one changelist. A newly detected change goes to the active changelist unless it was moved earlier (assignment survives refreshes and restarts, keyed by path). | Modify A with active list L1; make L2 active; modify B: A stays in L1, B appears in L2. Restart the IDE: assignments unchanged. |
| VCS-CHG-003 | MUST | Action `New Changelist...` (`ChangesView.NewChangeList`; no default shortcut) opens a dialog titled `New Changelist` with fields `Name:` and `Comment:` and a checkbox `Set active`. | Dialog fields present; OK with empty name disabled or rejected. |
| VCS-CHG-004 | MUST | Names must be unique within the project. A duplicate is rejected with the text `A changelist named '<name>' already exists`. A blank name is rejected. | Create `Fix` twice: second attempt shows the message and the dialog stays open. |
| VCS-CHG-005 | MUST | `Edit Changelist...` (`ChangesView.Rename`, shortcuts `F2` and `Shift+F6` in the changes tree) opens the dialog `Edit Changelist` with name and comment prefilled. Renaming the active list keeps it active. | Rename active list `Changes` to `Work`: the list stays active and keeps its changes. |
| VCS-CHG-006 | MUST | `Set Active Changelist` (`ChangesView.SetDefault`, `Ctrl+Space` in the changes tree) makes the selected list active. Disabled when the selected list is already active or more than one list is selected. | Select inactive list, press `Ctrl+Space`: it becomes bold/active, the previous one loses the mark. |
| VCS-CHG-007 | MUST | `Move to Another Changelist...` (`ChangesView.Move`, `Alt+Shift+M`) on selected changes or files opens a chooser popup listing existing changelists plus `New Changelist...`; picking one moves the selection. Also offered from the editor gutter popup of a line range (section 7) and as `Move Files to Another Changelist...` for unversioned files in the Unversioned dialog. | Select two changes in L1, `Alt+Shift+M`, choose L2: both appear in L2, L1 shows the remaining ones. |
| VCS-CHG-008 | MUST | Moving an unversioned file to a changelist adds it to the VCS (schedules an add), as the changelist can hold only versioned changes. | Move unversioned file `n.txt` into list L2: it is added (shown as new/added in L2). |
| VCS-CHG-009 | MUST | `Delete Changelist` (`ChangesView.RemoveChangeList`, also reachable with the `Delete` key on a changelist node) asks `Are you sure want to remove changelist '<name>'?` followed by `All changes will be moved to the active changelist.` (plural form `Are you sure want to remove N changelists?`). Title `Delete Changelist`. Deleting a non-active list moves its changes to the active list. | Delete list `Fix` holding one change: confirm, the change shows in the active list. |
| VCS-CHG-010 | MUST | Deleting the active changelist is allowed only when another list exists; the user is asked `Select the changelist to which the changes should be moved:` (dialog `Delete Active Changelist`) and the chosen list becomes active. If all lists are being deleted, the text is `All changes will be moved to the new default changelist.` and a fresh `Changes` list is created. | Delete active list with two others present: a chooser appears; choosing one activates it. |
| VCS-CHG-011 | SHOULD | `Delete Changelist` is disabled for a list marked read-only by a plugin. | Plugin sets read-only: action disabled with no error. |
| VCS-CHG-012 | MUST | Setting `Create changelists automatically` (Settings > Version Control > Changelists, checkbox `Create changelists automatically`, `VcsApplicationSettings.CREATE_CHANGELISTS_AUTOMATICALLY`, default off, application level). When on, integrations that request it create and activate a changelist per task or branch context; the flag must be stored and exposed. | Flag defaults off; stored in application settings. |
| VCS-CHG-013 | SHOULD | Setting `Allow putting changes within one file into different changelists` (`ENABLE_PARTIAL_CHANGELISTS`, default **on**) enables per-chunk assignment (section 7). It has no effect while the Git staging area is enabled (see `02-commit.md`, VCS-CMT-070). | Turning it off collapses chunk assignment: all chunks of a file follow the file's list. |
| VCS-CHG-014 | SHOULD | When an empty changelist stops being active, a prompt `The empty changelist '<name>' is no longer active. Do you want to remove it?` (plural form `Empty changelists<br/>... are no longer active. Do you want to remove them?`) is shown, with title `Remove Empty Changelist` and button `Remove`. The project-level option `remove empty inactive changelists` has values Ask (default), Remove silently, Do nothing (labels `Show options`, `Remove silently`, `Do nothing`). | Commit all changes of a non-default list: prompt appears on the next active switch; choosing `Remove silently` in settings suppresses the prompt. |
| VCS-CHG-015 | SHOULD | Changelist conflict handling for files modified while belonging to a non-active changelist: when the user starts typing in such a file, a dialog `Resolve Changelist Conflict` offers `Move changes to active changelist ('<name>')`, `Switch to changelist ('<name>')`, `Shelve changes` or `Ignore`. Settings (Changelists page): `Show dialog on attempt to edit file from inactive changelist` (off by default), `Highlight files with changelist conflicts`, `Highlight files from inactive changelists`. | With the dialog option on: editing a file of an inactive list shows the dialog with the four choices; `Ignore` adds the file to `Files with ignored conflicts`. |
| VCS-CHG-016 | MUST | Changelist data (names, comments, active flag, file assignments, unversioned/ignored exclusions) is stored in the project workspace state, not in the shared project files. | Changelists do not appear in version-controlled project files. |
| VCS-CHG-017 | LATER | A registry flag `vcs.disable.changelists` turns the changelist model into a single list; changelist UI is hidden when the Git staging area is enabled. | With staging area enabled the changelist toolbar group is absent. |

## 3. Changes view

The Local Changes view is a tab of the Commit tool window (default, advanced setting
`vcs.commit.tool.window` = on) or of the Version Control tool window.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-CHG-020 | MUST | Tree layout: root node per changelist (`<name>` plus count: `N files`), then directories/files; below them the nodes `Unversioned Files` (with count), and, when enabled, `Ignored Files`. Count text: `No files`, `1 file`, `N files`; for directories `1 directory`, `N directories`. An empty changelist name is shown as `<empty name>`. | Node titles and counts match the strings in a project with 3 changes and 2 unversioned files: `Changes 3 files`, `Unversioned Files 2 files`. |
| VCS-CHG-021 | MUST | Default grouping is by directory (`Group By > Directory`, `ChangesView.GroupBy.Directory`, `Ctrl+Alt+P`). Additional groupings: `Module` (`ChangesView.GroupBy.Module`, `Ctrl+Alt+M`), `Repository` (`ChangesView.GroupBy.Repository`), plus plugin-provided ones; groupings combine (directory within repository...) and are persisted per user. Option `Group By` is in the `View Options` popup of the toolbar (eye icon). | Toggle `Repository` in a two-root project: top level nodes are the roots. |
| VCS-CHG-022 | MUST | `Show > Ignored` toggle (`ChangesView.ShowIgnored`) adds the `Ignored Files` node; default off. The list of ignored files is requested from the VCS only while advanced setting `vcs.process.ignored` is on (label `Highlight ignored files`, default **on**). | With setting off, ignored files are not colored in Project view and `Ignored Files` is absent even if toggled. |
| VCS-CHG-023 | MUST | Toolbar actions in order: `Refresh` (`ChangesView.Refresh`), `Commit...` (`CheckinProject`, `Ctrl+K`), `Rollback...` (`Ctrl+Alt+Z`), `Show Diff` (`Ctrl+D`), `Changelists` group (`New Changelist...`, `Edit Changelist...`, `Delete Changelist`, `Set Active Changelist`, `Move to Another Changelist...`), `Shelve Silently` (`Ctrl+Shift+H`), then `View Options`, `Select Opened File` (`ChangesView.SelectFile`, locate icon), `Expand All`, `Collapse All`. | Toolbar tooltips and order correspond; hovering shows action name and shortcut. |
| VCS-CHG-024 | MUST | Context menu of a change: `Commit File` (for selected files), `Rollback...`, `Rollback File...`, `Move to Another Changelist...`, `Show Diff`, `Show Diff in a New Tab or Window`, `Jump to Source` (`ChangesView.EditSource`, shortcut of `EditSource`), `Copy Path/Reference...`, `Delete`, `Add to VCS` (`ChangesView.AddUnversioned`, `Ctrl+Alt+A`, only for unversioned; Git menu `Git.Add` text `Add`, GoToAction `Add to Git`), `Remove from VCS` (for locally deleted files), changelist actions, `Create Patch from Local Changes...`, `Copy as Patch to Clipboard`, `Shelve Changes...`, `Refresh`, then the VCS submenu. | Right-click on a modified file: all items except `Add to VCS` appear; on an unversioned file `Add to VCS` is enabled. |
| VCS-CHG-025 | MUST | Single click with preview enabled (`ChangesView.SingleClickPreview`, toggle `Preview Diff`) shows the diff in the details panel or editor tab; double click opens per the `Show on Double-Click` setting (actions `CommitView.ShowOnDoubleClick.EditorPreview` `Show Diff on Double-Click` and `CommitView.ShowOnDoubleClick.Source` `Show Source on Double-Click`): `Editor Preview` (`SHOW_EDITOR_PREVIEW_ON_DOUBLE_CLICK`, default on) or `Source`; `SHOW_DIFF_ON_DOUBLE_CLICK` default off. `Enter` opens the diff; `F4` jumps to source. | Default: double click opens the diff in an editor tab; with `Source` selected it opens the file. |
| VCS-CHG-026 | MUST | While a refresh is running the tree shows the placeholder text `updating...` on affected nodes; unversioned files are not shown during the initial scan and the node reads `Unversioned files are not available during file scanning`. | During a forced full refresh the `Unversioned Files` node shows the delayed message until the scan is done. |
| VCS-CHG-027 | SHOULD | Nodes show tooltips and colors per file status (section 5); renamed/moved files are annotated `- renamed from <path>` / `- moved from <path>` (also `copied from`, `replaced`, `renamed to`, `moved to`). | Rename a tracked file: new-path node shows ` - renamed from old/path`. |
| VCS-CHG-028 | SHOULD | Changes that only move a file without content change can be hidden (`Filter By > Hide Files > Moved without changes`, `With non-important changes`). | Toggle hides the matching nodes without changing counts of other nodes. |
| VCS-CHG-029 | MUST | While local changes are frozen by a running VCS operation (update, checkout), the view shows `Local changes are not available until <operation> is finished`. | Start a checkout: message appears and Commit is disabled until completion. |
| VCS-CHG-030 | MUST | Empty-project hints: no VCS: `To track changes to code:` with links `Share project on...` / `Create Git repository...` / `Version control integration`; commit window without VCS: `To commit changes, Create Git repository...` and `For recent changes, see Local History...`. | Open a plain folder: the strings appear. |

## 4. Unversioned and ignored files

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-CHG-040 | MUST | A new file inside a VCS root that the VCS does not know appears under `Unversioned Files`; it is added to the VCS by `Add to VCS` (`Ctrl+Alt+A`), drag to a changelist, or by checking it for commit (committing implies adding). | Create `n.txt`: it appears under Unversioned within the refresh delay (VCS-CHG-060); `Ctrl+Alt+A` moves it to a changelist as added. |
| VCS-CHG-041 | MUST | When the user creates a file in the IDE the question `Do you want to add the following file to Git?` (title `Add File to Git`, plural title `Add Files to Git`) follows the policy: `Ask` (default for Git), `Add silently`, `Do not add` (settings page Confirmation, option `When files are created`, labels `Ask`, `Add silently`, `Do not add`). Files matched by ignore rules are never offered; the check shows `Checking for ignored files...`. A checkbox `Apply to files created outside <IDE>` extends the policy to externally created files (`ADD_EXTERNAL_FILES_SILENTLY`, default off). | Create a file via New > File with policy Ask: dialog appears; `Cancel` leaves it unversioned. |
| VCS-CHG-042 | MUST | Deleting a tracked file in the IDE follows `When files are deleted`: `Ask`, `Remove silently`, `Do not remove`. Git prompt: `Do you want to delete the following file from Git?` with title `Delete File from Git` / `Delete Files from Git`. | Delete tracked file with `Ask`: prompt appears, confirming stages the deletion. |
| VCS-CHG-043 | MUST | A tracked file deleted outside the IDE appears as a locally deleted change; `Remove from VCS` (`ChangesView.RemoveDeleted`) schedules removal. | Delete file in a shell: it appears in the changelist as deleted (gray). |
| VCS-CHG-044 | MUST | Ignored files are matched by the VCS (not by the IDE); the IDE additionally treats a fixed list as ignored for add prompts (`Default ignored files`) and project excluded paths. | `.idea/workspace.xml` is never offered for add; a file listed in `.gitignore` is shown in the ignored color. |
| VCS-CHG-045 | MUST | `Add to .gitignore` / `Add to <ignore file>` actions (`vcs.add.to.ignore.file.action.group.text` = `Add to {0}`) on files in Project view and changes view; if the ignore file is missing the prompt `Create .gitignore file in <dir>?` (title `Create .gitignore`) appears. A Git-specific entry `Add to .git/info/exclude` and `Open .git/info/exclude` exist. | Choose `Add to .gitignore` on `build/`: a `build/` line is appended; the file disappears from Unversioned. |
| VCS-CHG-046 | SHOULD | The IDE can manage ignore files automatically: setting `Ignored files management policy:` with values `Always ask` (default), `Manage for all projects`, `Manage for this project only`, `Don't manage for this project only`, `Turn off app-wide`. Message: `<IDE> can automatically add files that are usually ignored to <ignore file>.` with buttons `In This Project`, `In All Projects`, `Do not Add`, `View`. Excluded directories are added to the ignore file only when `Add excluded directories to ignore file` is on, and ignored directories are excluded from analysis when `Automatically exclude ignored directories from the analysis` is on (`MARK_EXCLUDED_AS_IGNORED` default on, `MARK_IGNORED_AS_EXCLUDED` default off). | Fresh project with `.idea` and `out`: the prompt appears once per project under policy `Always ask`. |
| VCS-CHG-047 | SHOULD | Dialogs `Unversioned Files` and `Ignored Files` (opened from the node's `View` link) list files with actions `Add to VCS`, `Move to Another Changelist...`, `Delete`; Ignored view says `No ignored files` when empty. | Open dialog on a node with 3 files: all listed; delete removes from disk after confirmation. |

## 5. File status and colors

Status values (stable ids): `NOT_CHANGED`, `NOT_CHANGED_IMMEDIATE`, `NOT_CHANGED_RECURSIVE`,
`DELETED`, `MODIFIED`, `ADDED`, `MERGED`, `UNKNOWN`, `IDEA_FILESTATUS_IGNORED`, `HIJACKED`,
`IDEA_FILESTATUS_MERGED_WITH_CONFLICTS`, `IDEA_FILESTATUS_MERGED_WITH_BOTH_CONFLICTS`,
`IDEA_FILESTATUS_MERGED_WITH_PROPERTY_CONFLICTS`, `IDEA_FILESTATUS_DELETED_FROM_FILE_SYSTEM`,
`SWITCHED`, `OBSOLETE`, `SUPPRESSED`. Display names: `up to date`, `deleted`, `modified`,
`added`, `merged`, `unknown`, `ignored`, `merged with conflicts` (and `both conflicts`,
`property conflicts`), `deleted from file system`, etc.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-CHG-050 | MUST | The status of a file is computed by the VCS provider; a file whose editor document differs from disk (unsaved) and is not reported by the VCS is treated as modified. | Type in a tracked file without saving: its tab title and tree node turn the modified color. |
| VCS-CHG-051 | MUST | Status foreground colors are theme data, keyed `FILESTATUS_<ID>`. Values in the shipped new-UI schemes (light / dark): modified/renamed `0033B3` / `70AEFF`; added/copied `067D17` / `73BD79`; deleted and deleted-from-file-system `6C707E` / `6F737A`; unknown (unversioned) `B23247` / `E88F89`; merged `643CB8` / `CF84CF`; any conflict status `DE1B2E` / `DE6A66`; ignored `8C4F00` / `D69A6B`; directories that contain modified files (`NOT_CHANGED_IMMEDIATE`, `NOT_CHANGED_RECURSIVE`) use the modified color. Additional keys: `addedOutside`, `modifiedOutside`, `changelistConflict` (labels `Modified in not active changelist`, `Added in not active changelist`, `Changelist conflict`). | Each status renders the listed hex value in the matching theme (tolerance: exact). The light-scheme file defines the ignored color twice (`7d5020`, `8C4F00`); the later one wins (open question OQ-1). |
| VCS-CHG-052 | MUST | Status color applies to: editor tab title, Project view node, Changes view node, Recent Files and Search Everywhere rows, breadcrumbs of the file. Directory nodes show the modified color only when the setting `Highlight directories that contain modified files in the Project tree` (`SHOW_DIRTY_RECURSIVELY`, default off) is on. | Setting off: directory containing a modified file keeps the normal color; on: modified color. |
| VCS-CHG-053 | SHOULD | Color-blind palettes (deuteranopia / protanopia variants) override `MODIFIED_LINES_COLOR` (`9ca9bf`) and `FILESTATUS_MERGED` (`828eba`) in the light/dark schemes. | With a color-blind scheme variant selected those overrides apply. |
| VCS-CHG-054 | SHOULD | Settings page Version Control > File Status Colors is not part of the kit: colors are edited in the color scheme (Editor > Color Scheme > File Status Colors). | The scheme editor lists the same status names. |

## 6. Refresh triggers and timing

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-CHG-060 | MUST | Refresh is incremental: VFS events (create, delete, move, rename, content change, property change) in local files under an active VCS root mark paths dirty, recursively for directories. A rename marks the old path (with its parent directory) and the new path dirty; a move marks both paths. | Rename `a.txt` to `b.txt`: the old path vanishes and the new appears with `renamed` annotation within one refresh. |
| VCS-CHG-061 | MUST | Dirty marks coalesce; the provider run is scheduled with a debounce of 300 ms after the last mark. A mark arriving while a run is in progress schedules another run afterward (no lost update). | Burst of 100 file saves yields at most one run per 300 ms window and a final consistent state. |
| VCS-CHG-062 | MUST | Document edits mark a file dirty only when it is saved or when the editor first becomes modified, so unsaved edits show as modified in the view without waiting for disk. | After the first keystroke the file is listed under its changelist. |
| VCS-CHG-063 | MUST | Triggers beyond VFS events: VCS mapping change or root added/removed (full refresh), project open (full refresh after startup), IDE frame activation (re-check of external Git state via repository files watcher), manual `Refresh` (`ChangesView.Refresh`, menu entry `Refresh VCS Changes`, also `Ctrl+F5`-style global refresh in the toolbar), end of Git operations (branch change, pull, rebase, commit), and Git repository file changes (`HEAD`, refs, index) which mark the whole repository dirty. | `git commit` in an external terminal: after the IDE regains focus the committed changes disappear from the view. |
| VCS-CHG-064 | MUST | Provider results update status colors and notify listeners once per run, not per file. Callers needing consistent state use a `wait until refreshed` barrier; modal operations show `Performing VCS refresh...` and `Local Changes Refresh` as progress text. | Starting commit during a refresh shows `Synchronizing files...` then proceeds. |
| VCS-CHG-065 | SHOULD | Optional `Check for conflicts with the server every N minutes` (`CHANGED_ON_SERVER_INTERVAL` = 60, `CHECK_LOCALLY_CHANGED_CONFLICTS_IN_BACKGROUND` default off); applies to centralized VCS only. | Default off, no background polling for Git. |
| VCS-CHG-066 | MUST | The refresh never blocks the UI thread; a failure is reported as the text `Warning: not all local changes may be shown due to an error: <msg>` in the view and does not clear previous results. | Provider throws: previous changes stay and the warning row appears. |

## 7. Gutter line status markers

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-CHG-070 | MUST | Editors of files under VCS show markers for lines that differ from the base revision: added (green), modified (blue), deleted (small gray marker on the line boundary where lines were removed). Application flags `SHOW_LST_GUTTER_MARKERS` (default **on**) and `SHOW_LST_ERROR_STRIPE_MARKERS` (default on) control gutter bars and error-stripe ticks; the same-named text `Highlight modified lines in gutter` is the merge-tool option (VCS-CHG-081), the location of the editor toggles is OQ-5. | Edit a tracked file: bars appear; disabling the setting hides them. |
| VCS-CHG-071 | MUST | Marker colors (scheme keys): `ADDED_LINES_COLOR` `7fc784` light / `549159` dark; `MODIFIED_LINES_COLOR` `88adf7` light / `375fad` dark; `DELETED_LINES_COLOR` `767a8a` light / `868a91` dark; `WHITESPACES_MODIFIED_LINES_COLOR` `f7e2cb` light / `52433d` dark (lines whose only change is whitespace, shown when whitespace-only markers are enabled: `SHOW_WHITESPACES_IN_LST`, default on). Ignored-change borders use the same hue as an outline only: `IGNORED_ADDED_LINES_BORDER_COLOR`, `IGNORED_MODIFIED_LINES_BORDER_COLOR`, `IGNORED_DELETED_LINES_BORDER_COLOR`. | Pixel color of the marker interior equals the listed value for each state. |
| VCS-CHG-072 | MUST | Geometry in the new UI: the marker is a rounded bar of width 4 px by default (registry `gutter.vcs.changes.width`, range 4..6) placed after the line-marker-free painters offset plus 1 px (plus 2 px more when the width is below 5); a bar spans the changed lines minus 1 px at top and bottom; a deleted-lines marker is a rounded block of height `2*5-2` = 8 px centered on the boundary; hover widens the bar by an extra hover width and the block under the pointer is drawn after the others. Classic UI draws a rectangle across the whole free-painters area with a 1 px border for ignored changes and a triangle of height 4 for deletions. | Measured bar width is 4 px at 100% scale; deleted marker is 8 px high. |
| VCS-CHG-073 | MUST | Hit testing: a pointer is over a block if its y lies within the block extent; for an empty (deleted) range the extent is extended by one third of the line height above and below. | Hovering one third line above the deletion marker still selects it. |
| VCS-CHG-074 | MUST | Clicking a marker opens the line status popup (below). Keyboard: `Show Change Under Caret` (`VcsShowCurrentChangeMarker`) opens it at the caret; `Next Change` / `Previous Change` (`VcsShowNextChangeMarker` `Shift+Ctrl+Alt+Down`, `VcsShowPrevChangeMarker` `Shift+Ctrl+Alt+Up`, menu Navigate) scroll to the neighbor change and reopen the popup there. | `Shift+Ctrl+Alt+Down` at the last change is disabled (no wrap). |
| VCS-CHG-075 | MUST | The popup shows a mini toolbar and, for modified/deleted ranges, an embedded read-only editor with the base text of the range (word-level differences highlighted when `Highlight Words` is on, `SHOW_LST_WORD_DIFFERENCES` default on, toggled from the popup). Toolbar actions in order: `Rollback Lines` / `Rollback` (`Vcs.RollbackChangedLines`, `Ctrl+Alt+Z`), `Next Change`, `Previous Change`, `Copy` (copies the base text of the range), `Show Diff for Lines` (`Vcs.ShowDiffChangedLines`, `Ctrl+D`), `Highlight Words` toggle, and, if changelists with partial assignment are enabled, `Move Lines to Another Changelist...` (`Vcs.MoveChangedLinesToChangelist`, `Alt+Shift+M`; popup text `Move lines to another changelist (<list>)`). | Open the popup on a modified line: toolbar and base text appear; `Esc` or click outside closes it. |
| VCS-CHG-076 | MUST | `Rollback Lines` replaces the range in the document with the base text (or removes added lines) as one undoable command named `Rollback Change`; the caret stays near the range. | Rollback then `Ctrl+Z` restores the edit. |
| VCS-CHG-077 | MUST | `Show Diff for Lines` opens a diff titled `Diff for Range` with side titles `Base revision` and `Current`; the range is expanded by one line before and after when available. | Opened diff shows range plus one context line. |
| VCS-CHG-078 | SHOULD | Context menu of the gutter has `Annotate` (see `07-annotate-and-history.md`) and a line-status group; the gutter also supports sound signals (`Line added`, `Line removed above`, `Line changed`) for accessibility when enabled. | Screen-reader sound group present in settings. |
| VCS-CHG-079 | MUST | Trackers: one tracker per open document, created when the file has a VCS base revision of at most 500 000 bytes (`ourMaximumFileForBaseRevisionSize`); binary or oversized files get none. The base revision is the last committed one (Git: HEAD, or with staging area on, index and HEAD markers can both be shown, registry `git.enable.stage.line.status.tracker` default on). The tracker re-diffs after each document change with whitespace ignoring applied and merges adjacent ranges. | A 600 KB file shows no markers; a 10 KB file does. |
| VCS-CHG-080 | MUST | Partial changelists: with `ENABLE_PARTIAL_CHANGELISTS` on, each range is assigned to a changelist and an extra tooltip/icon indicates ranges not in the active list (the message `Part of the changes were moved to active changelist` with action `View Changes...` appears if damaged by an external edit). | Move a chunk to L2: marker has L2's identification; commit L1 leaves the chunk uncommitted. |
| VCS-CHG-081 | SHOULD | Markers in merge editors are controlled by `MERGE_LST_GUTTER_MARKERS` (default on). Diff views use the same renderer for the changes against the base. | Merge tool shows bars when on. |

## 8. Settings summary (defaults)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-CHG-090 | MUST | Project workspace settings `VcsManagerConfiguration`: `SHOW_FILE_HISTORY_DETAILS` on, `SHOW_DIRTY_RECURSIVELY` off, `LIMIT_HISTORY` on with `MAXIMUM_HISTORY_ROWS` 1000, `SHOW_ONLY_CHANGED_IN_SELECTION_DIFF` on, `LOCAL_CHANGES_DETAILS_PREVIEW_SHOWN` on, `SHELVE_DETAILS_PREVIEW_SHOWN` off, `RELOAD_CONTEXT` on (label `Restore workspace when switching branches`, comment `A workspace is a set of opened files, the current run configuration and breakpoints associated with a branch.`), `MAKE_NEW_CHANGELIST_ACTIVE` off, `PRESELECT_EXISTING_CHANGELIST` off, `DEFAULT_PATCH_EXTENSION` `patch`. | Fresh state serializes exactly these defaults. |
| VCS-CHG-091 | MUST | Application settings `VcsApplicationSettings` (file `vcs.xml`): `SHOW_WHITESPACES_IN_LST` on, `SHOW_LST_GUTTER_MARKERS` on, `SHOW_LST_ERROR_STRIPE_MARKERS` on, `DETECT_PATCH_ON_THE_FLY` off, `CREATE_CHANGELISTS_AUTOMATICALLY` off, `ENABLE_PARTIAL_CHANGELISTS` on, `MANAGE_IGNORE_FILES` off, `MARK_EXCLUDED_AS_IGNORED` on, `COMMIT_FROM_LOCAL_CHANGES` on, `SHOW_EDITOR_PREVIEW_ON_DOUBLE_CLICK` on, `SHOW_DIFF_ON_DOUBLE_CLICK` off. | Fresh state matches. |
| VCS-CHG-092 | SHOULD | `Limit history to N rows` (`Version Control` settings; default on, 1000) and `Highlight files changed in the last N days` (comment `Modified files will be highlighted in the external stack trace and while debugging.`). | Setting labels exact. |

## 9. Git change detection

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-CHG-100 | MUST | The Git provider reports for each dirty root: staged and unstaged status per path (from the index and working tree), renames detected by Git's rename detection, merge-conflict entries, submodule changes, and unversioned files from an incrementally maintained untracked-files holder per repository (full scan on startup, then updates from VFS events). | Edit and stage a file externally: the change appears in the changelist after refresh. |
| VCS-CHG-101 | MUST | Unsaved editor documents under the dirty scope that the status command did not report are added as modified changes with the HEAD revision as base. | Typed-but-unsaved file appears modified even though `git status` is clean. |
| VCS-CHG-102 | MUST | Submodules: a change in a submodule marks its parent repository dirty when the submodule's emptiness changes (clean to dirty or back) unless the parent already has a staged record; submodule roots are separate repositories. | Modify a file in a submodule: the submodule path appears modified in the parent changelist. |
| VCS-CHG-103 | SHOULD | Git reports ignored files only when the ignore list is requested (VCS-CHG-022); results come from `git status --ignored`-equivalent data held by the ignored-files holder. | With `Show Ignored` on, a file in `.gitignore` is listed under `Ignored Files`. |
| VCS-CHG-104 | MUST | When the experimental staging area is enabled (Git setting `Enable staging area`, default off, comment `This will disable changelists support. Only for non-modal commit interface.`) the view shows `Staged`/`Unstaged` groups instead of changelists. | Option on: changelist toolbar group missing; Staged group present. |

## Open questions

- OQ-1: the light scheme lists `FILESTATUS_IDEA_FILESTATUS_IGNORED` twice; verify the effective
  color in the shipped light theme by screenshot.
- OQ-2: exact hover extra width of the gutter bar (`getHoveredMarkerExtraWidth`) and the exact
  colors of the `Ctrl+click` hover state need measurement with `tools/reference/`.
- OQ-3: default of `Show dialog on attempt to edit file from inactive changelist` and of the two
  highlight options should be confirmed in a running IDE (source-derived value is off).
- OQ-5: the settings page hosting `Highlight lines in gutter`-style toggles for `SHOW_LST_GUTTER_MARKERS`
  and `SHOW_LST_ERROR_STRIPE_MARKERS` was not found in the sparse clone; verify the label and page.
- OQ-4: which of the status refresh triggers fire on frame activation (versus file-watcher
  events on `.git`) was inferred from repository updater classes; confirm by experiment.
