# 09. Local History

Status: specification. Requirement prefix `VCS-LH`. Executable cases: `cases/09-local-history.cases.md`.
Local History is the IDE's own change journal; it is independent of any VCS and is the safety net
behind rollbacks, refactorings, applying patches and VCS operations (VCS-LH-050).

ic: `platform/lvcs-api/src/com/intellij/history/**`,
`platform/lvcs-impl/src/com/intellij/history/{core,core/changes,core/tree,core/revisions,
integration,integration/revertion,integration/ui}/**`,
`platform/lvcs-impl/src/com/intellij/platform/lvcs/impl/**`,
`platform/lvcs-impl/resources/intellij.platform.lvcs.impl.xml`,
`platform/lvcs-impl/resources/messages/LocalHistoryBundle.properties`,
`platform/vcs-api/vcs-api-core/resources/messages/VcsBundle.properties` (activity names),
`platform/platform-resources/src/keymaps/$default.xml`.

## 1. What is recorded

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-LH-001 | MUST | The journal is a sequence of change sets, each a list of structural changes: create file, create directory, content change (old content stored), rename, move, delete (the deleted subtree is stored), read-only status change, and labels (user label, system label). Change sets are formed by grouping all VFS changes made inside one IDE command/activity (undoable command, refactoring, VCS operation) and carry an optional name and activity id. An empty change set is discarded. | Rename a class: one change set named after the refactoring contains the file renames and content edits. |
| VCS-LH-002 | MUST | Versioned files are: local file system files under a project content root (not excluded and not ignored); files outside every open project are also versioned unless they match the ignored-file patterns (scratch files and external files you edit are covered); files excluded by an open project and outside its content are not. Never versioned: files named `workspace.xml`, files ending `.iws` or `.class`, files not on the local file system unless the file system opts in, files when the system property `lvcs.disable.local.history` is true. | Edit `a.iws` and `Main.class`: no entry. Edit a file in an excluded folder: no entry. |
| VCS-LH-003 | MUST | Content changes are stored only for text-typed files (file type not binary) and scratch-root files; binary files record structure only. Content of a deleted binary file is stored only up to the registry limit `lvcs.store.binary.file.content.on.deletion.mb` (default 0 MB = never). Large text files obey the platform's content limit (store skipped for very large files; the history shows `File content is not available`). | Binary png edit: entries exist but `File content is not available` in the diff. |
| VCS-LH-004 | MUST | Content changes are recorded when the document is saved to disk (see `PLAT-SAVE`), not on each keystroke; external modifications are recorded as an activity named `External change`. | Type, save: one `Modify` entry; edit in an external program: entry `External change`. |
| VCS-LH-005 | MUST | Retention: change sets older than N days are purged, N from the advanced setting `localHistory.daysToKeep` (group IDE, default **5** days). Purging keeps activities that are closer to each other than the activity interval together (related change sets are never split). Storage lives in the IDE system directory under `LocalHistory`; corrupted storage is rebuilt with notification `Local History is broken` / `Local History storage file has become corrupted and will be rebuilt`. Fallback to an in-memory implementation if storage cannot be created. `Validate Local History Storage` (`ValidateLocalHistory`, internal) checks consistency. Registries: `lvcs.store.write.cache.in.memory` default true, `lvcs.store.entry.file.id` default true. | A change from 6 days ago is gone after startup purge; one from 4 days ago remains. |
| VCS-LH-006 | MUST | Labels: user labels are put by `Put Label...`; system labels are put by the IDE around VCS operations (see VCS-LH-050). Labels appear as their own entries and survive until purged; a label can be used to revert files to the state at the label (API `revertToLabel`). | Put a label, edit, revert to label: content restored. |

## 2. Actions and shortcuts

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-LH-010 | MUST | Group `Local History` (menu VCS > Local History; text `Local _History`): `Show History...` (`LocalHistory.ShowHistory`, GoToAction text `Show Local History...`), `Show History for Selection...` (`LocalHistory.ShowSelectionHistory`; synonyms `Show Local History for Method...` / `...for Class...`), `Show Project History...` (`LocalHistory.ShowProjectHistory`; `Show Local History for Project...`; widget text `Local History...`), `Put Label...` (`LocalHistory.PutLabel`), `Recent Changes` (`RecentChanges`, `Alt+Shift+C`; also in `View > Recent Changes`). `Show History...` also appears in `Vcs.Operations.Popup` as `Show Local History...`. | Menu lists the five entries in that order. |
| VCS-LH-011 | MUST | `Show History...` is enabled for a single file or directory (or several selected files); the history dialog/tab is titled `Local History: <name>` (activity dialog) and shows revisions of that path. For a directory with a project-level scope it shows the aggregated entries of everything below. | Select a folder: all child changes appear. |
| VCS-LH-012 | MUST | `Show History for Selection...` is enabled in a text editor with a non-empty selection (or the caret line); it shows revisions that changed the selected line range and the diff between consecutive revisions limited to that range; the title reads `Local History: <file>` with the selection range tracked across revisions (line mapping via the diff of consecutive contents). | Select a method, run it: only entries touching it list. |
| VCS-LH-013 | MUST | `Put Label...` opens an input dialog titled `Put Label` with the prompt `Label name:` (mnemonic N), empty names rejected (OK disabled); on success the notification `Label <name> created` with action `View local history` (opens the project directories' history); on failure `Could not create label <name>`. | Enter `before-refactor`: notification text exact. |
| VCS-LH-014 | MUST | `Recent Changes` shows the activity view scoped `Recent` in the `Local History` tool window (tab `Recent`) or, in the classic popup, a list `Recent Changes` of the last changes (load text `Loading Recent Changes`, empty `There are no changes`). Recent entries cover the last `Last <n> Hours` period; older entries are grouped `Old Changes` / `Older`. | `Alt+Shift+C` opens the list with the newest on top. |

## 3. History view

The activity view (default) is a split window: left a list of activity items grouped by time, right
the changed files/diff for the selected item.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-LH-020 | MUST | List items (newest first) show a presentation text by kind: `Create <path>`, `Delete <path>`, `Rename <old> to <new>`, `Move <path> to <new>`, `Modify <path>`, `Change <path>`, `Change <path> + <n> more`, the activity name when present (`Revert`, `Revert <date>`, `Revert '<label>' at <date>`, `Commit changes: <msg>`, `Amend commit: <msg>`, `Update from VCS`, `Get from VCS`, `Get from <vcs>`, `Shelve changes`, `Rollback`, `Apply patch`, `Unshelve`, `External change`), user labels by their name, system labels with their color. Group headings: `Last <n> Hours`, then dates, `Old Changes`/`Older`. Item secondary text `<n> file(s)`. List accessible name `Activity History`. | An item for a rename shows `Rename A.java to B.java`. |
| VCS-LH-021 | MUST | Filter field with placeholder `Search by content` (file scope) or `Search by file name` (directory/project scope); matching restricts items. Empty states: loading `Loading...`, `No recent activity detected`, `No matching recent activity detected`, `No activity in <scope> detected`, `No matching activity in <scope> detected`. | Typing `Foo` in a directory history keeps items whose file names contain it. |
| VCS-LH-022 | MUST | The right pane shows the changed files of the selected item as a tree (counts `<n> file(s)`); empty `No changes detected`, no selection `Select activity to view changes`. Diff mode toggle: `Diff with the local directory` (default) / `Diff with the next revision` (link `Switch to diff with the next revision`/`... the local directory`; setting `diffMode` default `WithLocal`). Selecting files opens a diff tab titled `Local History Diff`, `Recent Local History Diff` or `Local History: <file>`; content title `Before <date>`; diff side names `Current` for the working copy. `Show Difference` (`Show Diff`), `Create Patch...`, `Revert` actions in the toolbar/popup. `Show System Events` (`ActivityView.ShowSystemLabelsAction`, view options group `View Options`) toggles system labels (default **on**). | Select an item: the tree lists its files; double click opens the diff. |
| VCS-LH-023 | MUST | The diff of text files shows the content before the selected change on the left and (depending on the mode) local content or next revision on the right; for missing content `File does not exist` / `File content is not available`. Editing is disabled on the historical side. | Selecting a deleted file shows `File does not exist` on the right in next-revision mode. |

## 4. Revert and patches

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-LH-030 | MUST | `Revert Selected and Later Changes` (`ActivityView.Revert`; button/menu `Revert`) undoes the selected item and every later change set for the files of the scope (file/directory/project): content restored, renamed/moved back, deleted re-created, created removed; it is one undoable VFS operation recorded as activity `Revert`, `Revert <date>` or `Revert '<label>' at <date>`. A confirmation dialog titled `Revert` with the text `<details>` and `Do you want to proceed?` appears when the revert is destructive. | Revert to the change before the last edit: the file content equals the old text and a `Revert` item appears at the top. |
| VCS-LH-031 | MUST | Revert prerequisites: all affected files must be writable (`Cannot revert because some files are read-only`); missing content: `<n> file(s) cannot be restored:` plus list; errors `Error during revert: <msg>`. Revert asks the read-only status handler to make files writable first. | Read-only target: dialog with the read-only reason. |
| VCS-LH-032 | MUST | `Revert Selection` (`ActivityView.RevertDifferences`; `Revert Selection` text in the file-level changes browser; also `Revert` in selection history) applies only the chosen differences of one file (diff chunks selected in the viewer) to the current content. | Select two chunks in the diff and revert: only those chunks change in the file. |
| VCS-LH-033 | SHOULD | `Create Patch...` (`ActivityView.CreatePatch`; dialog `Create Patch` with file path/clipboard choice as VCS-PATCH-001) creates a patch of the selected activity items against the current state; messages `Patch file has been created`, `Patch copied to clipboard`, `Collecting Differences`, `Cannot create patch because content of some of the selected files was not stored due to its big size.`, `Error during patch creation: <msg>`. | Create patch for one item: file contains that change. |
| VCS-LH-034 | SHOULD | Revert/patch progress strings: `Processing revisions...`, `Processing left revision...`, `Processing right revision...`. | Seen during long history loads. |

## 5. System labels and integration points

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| VCS-LH-050 | MUST | The IDE puts system labels around risky operations so the user can find the state: before and after VCS update (`Before update`, `After update`), before and after push (`Before push`, `After push`), checkin/commit activities, and patch application; rollback, shelve, unshelve and update are recorded as named activities (`Rollback`, `Shelve changes`, `Unshelve`, `Update from VCS`). The `Before update`/`After update` labels carry colors used in the list. | After a Git update the history shows both labels around the update's file changes. |
| VCS-LH-051 | MUST | Deleting files from the IDE keeps their content in the journal so that `Show History` on the parent directory reveals them (`LocalHistoryFilesDeletionHandler`), and a directory history can restore a deleted file via revert. | Delete a file, open its parent's history, revert: the file returns. |
| VCS-LH-052 | SHOULD | Empty project hints and the VCS widget offer `Local History...` (`LocalHistory.ShowProjectHistory` in the empty-VCS widget group `Vcs.ToolbarWidget.CreateRepository.LocalHistory`); commit tool window text for no-VCS: `For recent changes, see Local History...`. | In a plain project the widget lists `Local History...`. |
| VCS-LH-053 | LATER | Third-party API: `LocalHistory.startAction`, `putUserLabel`, `putSystemLabel`, `revertToLabel`, labels by id with validity checks (`isLabelValid`); `ByteContent` for file content at label. The kit should expose the same operations to plugins. | Not scheduled for plugins. |
| VCS-LH-054 | LATER | Internal: `Validate Local History Storage`, trace limit registry `lvcs.trace.changes.persistence.limit` (100). | Internal only. |

## Open questions

- OQ-1: the exact maximum size of text content stored per file (platform content limit constant)
  is outside the sparse clone; measure by editing a very large file.
- OQ-2: the interval between "related" change sets used by the purge (`DEFAULT_INTERVAL_BETWEEN_
  ACTIVITIES_MILLISECONDS`) was not read; confirm before implementing purge.
- OQ-3: whether the classic `Recent Changes` popup or the activity view is the default is
  controlled by `ActivityView.isViewEnabled`; assume the activity view.
