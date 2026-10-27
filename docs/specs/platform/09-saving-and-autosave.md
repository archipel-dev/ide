# Platform spec 09: saving documents and autosave

Status: specification (clean-room, written from IntelliJ IDEA Community behavior). ID prefix `PLAT-SAVE`.
Priorities: `MUST` (needed for the first Java/Kotlin/Spring code-intelligence release), `SHOULD` (needed
for parity, schedule after the MUSTs), `LATER` (document now, build later, but do not design it out).

This document answers "when does the IDE write unsaved editor text to disk, which documents, what can stop
it, what the user sees when it fails, and why highlighting does not care". It builds on, and does not repeat:

* PLAT-VFS-092: what "unsaved" means and what a save applies (separators, charset, BOM, strip trailing spaces, final newline);
* PLAT-VFS-093: the three defaults (save on deactivation on, save after idle off with 15 s in the range 1 to 300 s, safe write on);
* PLAT-VFS-094: the "from save" content-change event and its consumers;
* PLAT-VFS-095: refusing edits to read-only files;
* PLAT-VFS-040..046 and 100..103: refresh, "sync external changes" defaults, silent reload, the "File Cache Conflict" dialog and automatic merge.

Source files studied: `ic: platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/FileDocumentManagerImpl.java`,
`ic: platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/MemoryDiskConflictResolver.java`,
`ic: platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/LargeFileSavingVetoer.java`,
`ic: platform/core-impl/src/com/intellij/openapi/fileEditor/impl/FileDocumentManagerBase.java`,
`ic: platform/core-api/src/com/intellij/openapi/fileEditor/FileDocumentManager.java`,
`ic: platform/core-api/src/com/intellij/openapi/fileEditor/FileDocumentManagerListener.java`,
`ic: platform/platform-api/src/com/intellij/openapi/fileEditor/FileDocumentSynchronizationVetoer.java`,
`ic: platform/configuration-store-impl/src/SaveAndSyncHandlerImpl.kt`,
`ic: platform/platform-api/src/com/intellij/ide/SaveAndSyncHandler.kt`,
`ic: platform/ide-core/src/com/intellij/ide/GeneralSettings.kt`,
`ic: platform/platform-impl/src/com/intellij/ide/GeneralSettingsConfigurable.kt`,
`ic: platform/platform-api/resources/messages/IdeBundle.properties`,
`ic: platform/platform-api/resources/messages/UIBundle.properties`,
`ic: platform/ide-core/src/com/intellij/openapi/vfs/SafeWriteRequestor.java`,
`ic: platform/platform-impl/src/com/intellij/openapi/editor/impl/TrailingSpacesStripper.java`,
`ic: platform/platform-impl/src/com/intellij/ide/actions/SaveAllAction.kt`,
`ic: platform/platform-impl/src/com/intellij/ide/actions/SaveDocumentAction.java`,
`ic: platform/platform-impl/src/com/intellij/ide/actionsOnSave/impl/ActionsOnSaveFileDocumentManagerListener.kt`,
`ic: platform/lang-impl/src/com/intellij/codeInsight/lookup/impl/LookupDocumentSavingVetoer.java`,
`ic: platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/DaemonListeners.java`,
`ic: platform/lang-impl/src/com/intellij/codeInsight/daemon/impl/EssentialHighlightingRestarter.kt`,
`ic: platform/platform-impl/src/com/intellij/openapi/application/impl/ApplicationExit.kt`,
`ic: platform/platform-impl/src/com/intellij/openapi/project/impl/ProjectManagerImpl.kt`,
`ic: platform/platform-impl/src/com/intellij/application/options/editor/EditorTabsOptionsModel.kt`,
`ic: platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/EditorsSplitters.kt`,
`ic: platform/ide-core-impl/src/com/intellij/openapi/editor/ex/EditorSettingsExternalizable.java`,
`ic: platform/execution/src/com/intellij/execution/runners/DefaultProgramRunner.kt`,
`ic: platform/execution-impl/src/com/intellij/execution/runners/DefaultRunProgramRunner.kt`,
`ic: java/execution/impl/src/com/intellij/execution/impl/DefaultJavaProgramRunner.java`,
`ic: java/execution/impl/src/com/intellij/compiler/options/CompileStepBeforeRun.java`,
`ic: java/compiler/impl/src/com/intellij/compiler/impl/CompileDriver.java`,
`ic: java/compiler/impl/src/com/intellij/compiler/server/BuildManager.java`,
`ic: platform/external-system-impl/src/com/intellij/openapi/externalSystem/util/ExternalSystemUtil.java`,
`ic: platform/external-system-impl/src/com/intellij/openapi/externalSystem/service/execution/ExternalSystemRunnableState.java`,
`ic: plugins/maven/src/main/java/org/jetbrains/idea/maven/execution/MavenRunner.java`,
`ic: platform/execution-impl/src/com/intellij/tools/Tool.java`,
`ic: platform/vcs-impl/src/com/intellij/vcs/commit/SaveCommittingDocumentsVetoer.kt`,
`ic: platform/vcs-impl/src/com/intellij/vcs/commit/AbstractCommitWorkflow.kt`,
`ic: platform/vcs-impl/src/com/intellij/openapi/vcs/changes/VcsFreezingProcess.kt`,
`ic: platform/vcs-impl/src/com/intellij/openapi/vcs/update/VcsUpdateTask.kt`,
`ic: platform/vcs-impl/src/com/intellij/openapi/vcs/ex/LineStatusTracker.kt`,
`ic: plugins/git4idea/backend/src/branch/GitBranchOperation.java`,
`ic: plugins/terminal/src/org/jetbrains/plugins/terminal/block/BlockTerminalView.kt`,
`ic: platform/lang-impl/src/com/intellij/execution/console/ConsoleExecutionEditor.java`,
`ic: platform/platform-impl/src/com/intellij/ide/lightEdit/LightEditTabs.java`.

## 1. Vocabulary

| Term | Meaning in this spec |
|---|---|
| **Unsaved set** | The documents of physical files that were changed since their last save or reload. A document enters the set on every change (including a change that is later undone). It leaves the set on save, on reload from disk, or when a save finds nothing to write (PLAT-SAVE-071, 075). |
| **Explicit save** | A save requested by the user directly (Save All, Save Document) or indirectly through an action that needs the files on disk (run, build, commit). Everything an autosave trigger does is **non-explicit**. The flag is passed to vetoers; it changes only the veto outcome (PLAT-SAVE-073). |
| **Autosave trigger** | A situation in which the IDE saves although the user did not ask: frame deactivation, idle timeout, focus into a terminal or console, the build server's periodic task, batch overflow. |
| **Save point** | A place where a feature saves before it reads files from disk (run, build, VCS, close). |
| **Veto** | A policy that makes one document skip the current save without an error. |
| **Daemon** | The background highlighting and inspection pass of the editor (specified in the code-insight set; its input is the committed document text, PLAT-VFS-110..114). |

## 2. Core contract

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-001 | MUST | The platform offers: `save_all(explicit)`, `save_filtered(predicate)` (always explicit), `save_one(doc, explicit)`, `save_as_is(doc)` (saves without stripping trailing spaces or adding a final newline, for that call only), `unsaved_documents()`, `is_unsaved(doc)` and `is_file_modified(file)` (true only when the file has an unsaved document whose stamp differs from the file's). Default calls are explicit. | A test calls each; `save_as_is` on a document with trailing spaces leaves them in the file and in the document. |
| PLAT-SAVE-002 | MUST | A listener contract exists with these events, in order for one `save_all`: `before_all_documents_saving`, then for each document that is written `before_document_saving(doc)` (only when a write will happen; listeners may edit the document) and `after_document_saved(doc)`. A single-document save additionally fires `before_any_document_saving(doc, explicit)` first, even when the document turns out unchanged; `save_all` does not fire it per document. Also `unsaved_document_dropped` when a document leaves the unsaved set without a write. Listeners can be added by extensions and may modify the document inside the `before` events. | A recording listener sees the sequence for a two-file save; an editing listener's change is part of what is written. |
| PLAT-SAVE-003 | MUST | The document of a **non-physical** file (copy, fragment, preview, scratch buffer used only for analysis) is not tracked: it never enters the unsaved set and saving it is a no-op. An opt-in flag on the file makes a non-physical file tracked. | Edit a code fragment document: `is_unsaved` is false. Set the flag: it becomes true. |
| PLAT-SAVE-004 | MUST | "Autosave cannot be disabled completely": with both autosave settings off, every other save point of sections 4 to 6 still saves, and Save All still works. No setting may disable them. | Turn both options off; run a configuration: files are saved. |

## 3. Settings and their texts

Location: Settings > Appearance & Behavior > System Settings (page title "System Settings"), group "Files". Rows in this order: the optional "delete to bin" row, the idle row, the deactivation row, the safe-write row, then the sub-group "Sync external changes:", then one comment line.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-010 | MUST | Idle row: a checkbox "Save files if the IDE is idle for", then an integer field (4 columns), then the label " seconds". The field is enabled only while the checkbox is on. Range 1 to 300, default 15, checkbox default off. A stored or typed value outside the range is clamped into it. | Type 999: it becomes 300. Uncheck: the field is disabled. |
| PLAT-SAVE-011 | MUST | Deactivation row: checkbox "Save files when switching to a different application or a built-in terminal". Default on. | The text matches; default on after a clean profile. |
| PLAT-SAVE-012 | MUST | Safe-write row: checkbox "Back up files before saving", default on. Meaning: when on, a save that fails part way (disk full, quota, I/O error) must not destroy the previous content of the file; the write goes to a temporary sibling and replaces the target only after it completed (PLAT-VFS-093). When off, the file is overwritten in place. Only writes requested by the document save path honor it. | Inject a failing writer at 50 percent: with the option on the original bytes are intact; with it off they may be truncated. |
| PLAT-SAVE-013 | MUST | Under the page the comment "Autosave cannot be disabled completely." followed by a link "How it works" that opens the help topic "autosave". | The line is shown; the link is wired to help. |
| PLAT-SAVE-014 | SHOULD | The sub-group "Sync external changes:" has "When switching to the IDE window or opening an editor tab" (default on) and "Periodically when the IDE is inactive (experimental)" (default on). Behavior is PLAT-VFS-040..044. When the first is **off**, an editor banner "File was changed on disk." with an action "Reload" appears for an open file whose disk timestamp or length differs from the cached values, on window activation and on file events; "Reload" refreshes that file. | Turn the option off, touch an open file externally, activate the window: banner appears. |
| PLAT-SAVE-015 | MUST | Settings apply immediately, without restart: switching the idle checkbox on arms the idle timer; changing the timeout re-arms it with the new value; switching off cancels it. | Enable idle with 2 s, type, wait 2 s: file saved. Change to 60 s: no save at 2 s. |
| PLAT-SAVE-016 | SHOULD | Editor-tab option "Mark modified (*)" (classic UI text; "Mark modified" in the new UI), in Settings > Editor > General > Editor Tabs. Stored default: off. When on, a tab whose editor has unsaved changes shows a modified marker: classic UI a small dot over the file icon (or in its place when file icons are hidden); new UI a dot in the place of the close button, and when the close button is hidden and the tab is not pinned the close action for that tab is restricted. The marker disappears as soon as the document is saved or reloaded. The option has no effect on saving. | Toggle on, type: marker appears. Save: gone. Toggle off: never shown. |

## 4. Autosave triggers

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-020 | MUST | **Frame deactivation.** When the application loses focus (the user switches to another application; moving focus between IDE windows does not count) and the deactivation option is on, a non-explicit `save_all` runs on **all** unsaved documents of all open projects, then pending file writes are flushed, then a save of application and project settings is queued (settings are specified elsewhere). Not done while a modal dialog or a modal progress is active. | Edit two files in two projects, switch to another app: both saved; a modal dialog open: nothing saved. |
| PLAT-SAVE-021 | MUST | **Idle timeout.** While the idle option is on, after N seconds with no user input event in the IDE (key, mouse; background tasks do not count as activity), a non-explicit `save_all` runs on all unsaved documents. The countdown restarts at the next input and a save repeats after the next quiet period; with nothing unsaved the save is a no-op. | Type, stop for N seconds: saved once. Keep typing every N-1 seconds: never saved by idle. |
| PLAT-SAVE-022 | SHOULD | The two triggers above are armed about 15 seconds after application start, not earlier. | A change made 3 s after start and a deactivation then does not save (observable only in tests; do not rely on it for product behavior). |
| PLAT-SAVE-023 | SHOULD | **Focus into a built-in terminal or console.** When the terminal prompt becomes active, or the input editor of a language console gains focus, and the deactivation option is on: save all (console: later, outside the focus event; terminal: now, then flush pending writes). When the terminal loses focus, and when a terminal command finishes, an external-change refresh is scheduled. This is the "or a built-in terminal" of PLAT-SAVE-011. There is no save before each terminal command beyond this focus rule. | Edit a file, click into the terminal: file saved before any command is typed. |
| PLAT-SAVE-024 | MUST | **Save All** (default shortcut Ctrl+S, label "Explicit save (Ctrl+S)" in the Actions on Save page): explicit `save_all`, then settings are saved immediately (no queue delay). If the focused editor's document has "strip trailing spaces" on and "keep on caret line" off, that document is stripped first (PLAT-SAVE-093). **Save Document** saves only the focused editor's document and is the only single-document save that triggers Actions on Save for it (PLAT-SAVE-102). | Press Save All with two dirty files: both clean, no delay. |
| PLAT-SAVE-025 | SHOULD | **Batch overflow.** When the unsaved set grows beyond 100 documents or beyond the content loading limit (PLAT-VFS-035) of total text, a non-explicit `save_all` is scheduled for later on the UI thread outside modal state, so batch operations do not pile up unsaved documents (same threshold as PLAT-VFS-118). | A script edits 150 documents in one command: afterwards the unsaved set is drained shortly after, without user action. |
| PLAT-SAVE-026 | LATER | Registry-style hidden switch (default off): save project files 1.5 s after any document change so the automatic build can start sooner. | Off by default; not exposed in settings. |

## 5. Save points before execution and build

All explicit unless stated. Each saves **all** unsaved documents (not only the project's, not only the focused file) and runs on the UI thread in a write-safe context.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-030 | MUST | **Run and debug.** The default program runners save all documents immediately before the process is started, i.e. after before-launch tasks completed; the default (non-Java) runner also flushes pending file writes. | Edit a script, run: the process sees the new content. |
| PLAT-SAVE-031 | MUST | **Tests.** Running tests is running a run configuration, so PLAT-SAVE-030 and 033 apply; there is no separate test trigger. (Inferred from the runner structure, see open question 3.) | Edit a test, run it: the new test runs. |
| PLAT-SAVE-032 | MUST | **Before-launch "Build" step.** After the build task is prepared and before it starts, a **non-explicit** `save_all` runs, then pending file writes are flushed. Because it is non-explicit, a document with an open completion popup is skipped (PLAT-SAVE-073). | Run with a build step and a lookup open in the edited file: the build uses disk content of the other files. |
| PLAT-SAVE-033 | MUST | **Build, make, rebuild, compile (user-started) and custom compile tasks.** An interactive build first commits all documents to their syntax trees, then saves all (explicit). A build started automatically (no user context) does not force a save by itself. Automatic build ("build project automatically") is postponed while the unsaved set is not empty and while the IDE has been idle for less than 3 seconds. | Edit, Build project: compiled output reflects the edit. With auto build on and a dirty document, no build starts until it is saved. |
| PLAT-SAVE-034 | MUST | **External build systems (Gradle, Maven, other external-system plugins).** Saving all happens before: executing a task of an external-system run configuration, importing or refreshing a project (reload buttons, "refresh all"), the Maven runner and the Maven project task runner, and the Gradle actions that re-read dependencies or download sources. | Edit `build.gradle` text, press Reload: the import reads the edit. |
| PLAT-SAVE-035 | MUST | **External tools** (Tools menu entries): save all before the tool starts. | Edit, run an external tool that cats the file: new text printed. |
| PLAT-SAVE-036 | SHOULD | Other whole-project readers save all first: Inspect Code and code cleanup (interactive and silent), analysis-scope actions, Replace in Path (before applying), Ant targets, "Open in Browser", and the Synchronize action. | Edit, Inspect Code: results match the edited text. |

## 6. Save points around version control

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-040 | MUST | **Commit.** Documents are saved (explicit): when the commit starts after an update, before the commit session executes, and after the checks that may change text (reformat, optimize imports, cleanup) so the committed bytes equal the text the user sees. Committing a change list also saves all first. | Reformat-on-commit on: the committed file contains the reformatted text and the editor shows no unsaved marker afterwards. |
| PLAT-SAVE-041 | MUST | **Update/pull, checkout and branch switch, merge, reset, create worktree, rollback, shelve and unshelve (silent and with dialog), silent stash, apply patch, "compare with" branch or local, and every change-list refresh (which saves documents and project settings before refreshing):** save all before the operation. Update also blocks project reload on external change and saves settings before it starts. | Edit, switch branch: the dirty text takes part in the checkout's local-change handling instead of being lost. |
| PLAT-SAVE-042 | MUST | **Freeze.** Operations that freeze the change list manager (update, branch operations and similar) first save all, then, until they finish, suppress the queued settings save and the external-change sync; both are counted and re-entrant, and resume when the last holder releases, running a pending save at that point. Observed limit: document autosave by deactivation or idle is **not** gated by this suppression (open question 5). | During a long update, deactivate the window: settings are not written until the update ends. |
| PLAT-SAVE-043 | MUST | **Commit-in-progress veto.** While files are being committed their documents carry a "committing" mark. Saving such a document (explicit or not) asks the user: title "Save Files During Commit", text "The following file is currently being committed to the VCS. Saving now could cause inconsistent data to be committed." (plural form for several), then the file paths on separate lines, then "Save the file now?"; buttons "Save Now" and "Postpone Save". In a save-all the question is asked once for all marked files and the answer sticks for the rest of the commit: "Save Now" clears the mark and the save proceeds, "Postpone Save" vetoes it and every later save-all skips those files silently until the commit ends. A single-document save asks again each time and remembers nothing. | Start a commit, edit a committed file, deactivate window: the question appears; Postpone: file stays unsaved. |
| PLAT-SAVE-044 | SHOULD | The VCS and the compiler treat unsaved documents as changes: a file with in-memory-only edits appears in the change list and marks its module dirty before it is saved. | Type in a tracked file without saving: it is listed as modified. |
| PLAT-SAVE-045 | SHOULD | **Back to unchanged.** When the line status tracker finds the document equal to the VCS base again (the user reverted by typing or undo) and the deactivation option is on, that single document is saved at once (later, outside the change event). | Edit a line then retype the original: the file is no longer reported modified without any trigger. |

## 7. Project close, exit, and where nothing is saved

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-050 | MUST | **Project close.** A user-initiated close first runs an explicit `save_all`; if any document cannot be saved the close is cancelled (the project stays open, documents stay unsaved) and the failure dialog (PLAT-SAVE-077) is shown with "Ignore Changes" as the confirm button. After a successful save the project settings are saved, then the project closes. Closing without the veto option (programmatic close that requests saving) still saves all and shows the failure dialog but does not abort. During the close, autosave settings saves are suppressed. | Make a file read-only on disk after editing, close the project: the project stays open and the dialog is shown. |
| PLAT-SAVE-051 | MUST | **Application exit.** The same close check runs for every project; then, for exit, all documents are saved (a failure is logged and does not by itself stop a confirmed exit), then application and project settings are saved under a modal progress, then projects close. | Edit, quit: next start shows the edit on disk. |
| PLAT-SAVE-052 | MUST | **Closing an editor tab does not save and does not prompt** in the main IDE. The document stays in the unsaved set and is written by the next trigger (deactivation, idle, a save point, Save All). (Only the lightweight single-file editor mode prompts "save or discard" on tab close, and saves silently when its own autosave mode is on: LATER.) | Edit, close the tab, wait for nothing: file on disk unchanged until deactivation, idle or another save point. |
| PLAT-SAVE-053 | MUST | **No save from editing features.** Typing, undo and redo, completion, intentions, and moving focus between editors or tool windows inside the IDE do not save. Exceptions are exactly PLAT-SAVE-023, 025, 045 and the autosave settings. | Type a line, stay in the IDE, wait less than the idle timeout: disk unchanged. |
| PLAT-SAVE-054 | SHOULD | Reformat, optimize imports and refactorings are ordinary document edits and do not themselves save; the user-visible save happens at the next trigger. (Not observed saving in the studied source; see open question 4. Refactorings that also write other files do so through the VFS.) | Reformat a file: it is marked modified, not saved. |
| PLAT-SAVE-055 | LATER | The lightweight single-file editor mode has its own "Autosave: on/off" status widget with the popup title "Save changes automatically" and text "All open files are saved ..." (every N seconds, on tab or window close, or on window deactivation); implicit saves there are allowed only when that mode is on. | Not required for the first release. |

## 8. The save algorithm and its edge cases

`save_all(explicit)` in observable steps:

1. Fire `before_all_documents_saving` (trailing-space leftovers, Actions on Save scheduling, commit confirmation run here). If the unsaved set is empty, return success.
2. Loop over a snapshot of the unsaved set, skipping documents rejected by the filter, documents that already failed in this call, and documents already vetoed in this call. Documents changed by listeners during the pass are picked up by repeating the loop until a pass handles no document.
3. For each document, the per-document procedure (PLAT-SAVE-071 to 076) runs. An I/O failure is recorded and the loop continues. A veto is recorded and the loop continues.
4. After the loop, if there were failures, show the failure dialog once for all of them and return failure; otherwise return success.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-070 | MUST | Steps 1 to 4 above, including: save runs with a progress titled "Save All Documents" whose secondary text is the path of the file being written, and the UI stays responsive during it. A document that is vetoed or fails stays in the unsaved set. | Trace test with one failing, one vetoed, one good document: good one saved, others still unsaved, one dialog. |
| PLAT-SAVE-071 | MUST | Per document, first: if it has no file, or its file is not tracked (PLAT-SAVE-003), or its file is valid and `is_file_modified` is false (stamps equal), drop it from the unsaved set without writing. If the file is no longer valid, drop it. | Edit, undo to original, save: no write happens (mtime unchanged). |
| PLAT-SAVE-072 | MUST | Then, if the file's cached timestamp differs from the timestamp on disk, refresh that file before anything else. A resulting reload or conflict is handled by PLAT-VFS-100..103. If the document left the unsaved set because of the refresh, stop. | Touch the file on disk, then save an unrelated edit of it: a conflict or reload happens first; no silent overwrite of unseen disk content. |
| PLAT-SAVE-073 | MUST | Then ask the vetoers. A veto leaves the document unsaved, raises no error and no dialog of its own, and is not retried in the same call. Built-in vetoes: (a) the file is in the external-change conflict queue (dialog pending); (b) the file is larger than the content loading limit (never written from a document); (c) a completion lookup is active for this document in any open project, **non-explicit saves only**; (d) the file is marked committing and the user chose to postpone (PLAT-SAVE-043); (e) in lightweight single-file mode, an implicit save while its autosave mode is off. Extensions may add vetoes; they receive the document and the explicit flag; an extension that throws is isolated and treated as no veto. | With a lookup open, deactivate: document skipped; press Save All: written. |
| PLAT-SAVE-074 | MUST | After the vetoes, the write happens in a write section: if the file turned invalid, drop it; if the document's file binding changed, rebind it. | Delete the file on disk while its document is dirty, save: no error, document dropped. |
| PLAT-SAVE-075 | MUST | **Content-equal short-circuit.** For a text file whose document has at most 1,000,000 characters, the document text is compared with the decoded disk bytes; if equal, nothing is written, the document stamp is set to the file's stamp, the document leaves the unsaved set, and modified markers are refreshed. A binary file, a file whose document is longer, or a document marked "force save" is always written. A failure to read for the comparison counts as an I/O failure of that document after the stamp bookkeeping. | Edit then restore the text by typing: save writes nothing and the tab marker clears. |
| PLAT-SAVE-076 | MUST | The write: fire `before_document_saving` (strip trailing spaces and similar edits happen here), convert line separators to the file's style, encode with the file's charset and BOM (PLAT-VFS-030..032, 092), write through the VFS with the safe-write request (PLAT-SAVE-012), tagging the event as "from save" and the new file stamp equal to the document stamp (PLAT-VFS-094), remove the document from the unsaved set, clear its changed-line flags except lines that kept their trailing spaces because of the caret (PLAT-SAVE-091), then fire `after_document_saved`. | Save a CRLF file with trailing spaces on modified lines: CRLF kept, those lines stripped, stamps equal. |
| PLAT-SAVE-077 | MUST | **Failure.** Failures are logged and shown, once per save call, in an application-modal dialog opened outside the write section, titled "Cannot Save Files", with the message "Following errors occurred on attempt to save files:" and a read-only scrolling text with the error messages of all failed documents joined by newlines. Buttons: confirm "Revert Changes" and cancel "Close" (normal case); confirm "Ignore Changes" and the default cancel (during project close). Confirm reloads **each failed document from disk**, discarding its memory text; cancel keeps the documents unsaved so a later trigger retries (and shows the dialog again on the next failure). The save call returns failure. | Make the target directory read-only, save: dialog with the I/O message; "Close" keeps the text; "Revert Changes" restores the disk text. |
| PLAT-SAVE-078 | MUST | **Read-only files.** A document's read-only state follows its file's writable flag and is updated when that flag changes (property event). Editing is refused by PLAT-VFS-095. If a document is unsaved and its file is read-only at save time, the write fails and PLAT-SAVE-077 applies; trailing-space stripping is skipped for read-only documents. The "toggle read-only attribute" action saves all before changing the attribute. | Edit, flip the file read-only externally, deactivate: dialog "Cannot Save Files". |
| PLAT-SAVE-079 | MUST | **Changed on disk while dirty.** Detection and resolution are PLAT-VFS-100..103; the save side adds only: while a file's conflict dialog is pending, the save of that file is vetoed (PLAT-SAVE-073a), and if the user keeps the memory text the next save overwrites the disk content; if the user loads disk changes the document is reloaded and leaves the unsaved set. | Edit, change the file externally, refresh: dialog; choose "Keep Memory Changes", save: disk equals memory text. |
| PLAT-SAVE-080 | MUST | **Suppression tokens.** Callers can suppress, for a scope, the queued settings save (and external sync) with a counted token; suppressing while a settings save is running interrupts it and re-queues it for when the count returns to zero. The settings save of Save All, project close and exit bypasses the delay. | Nested suppressions: save resumes only after the outermost ends. |
| PLAT-SAVE-081 | SHOULD | **Settings saves are debounced.** A queued settings save starts after a 300 ms delay unless it is forced (Save All, close), is deferred while a modal progress is running, and coalesces (a task is kept queued, not dropped, while a modal progress runs): a request for all projects replaces queued per-project requests, and a request for a project already covered is dropped. | Fire 10 deactivations in 200 ms: one settings write. |

## 9. Trailing spaces, final newline, and the caret line

Editor settings (Settings > Editor > General) read by the save path and their defaults:

| Setting | Text | Default |
|---|---|---|
| Strip trailing spaces | "Remove trailing spaces on:" with values None, "Modified lines", "All lines" | Modified lines |
| Keep on caret line | "Keep trailing spaces on caret line" | on |
| Final newline | "Ensure every saved file ends with a line break" | off |
| Trailing blank lines | "Remove trailing blank lines at the end of saved files" | off |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-090 | MUST | The stripping and final-newline edits run in `before_document_saving` of **every** save path: autosave triggers, Save All, Save Document, and save points. There is no difference between explicit and non-explicit saves except as stated in PLAT-SAVE-093. They are skipped for read-only documents, for files with the per-file disable flag (set by `save_as_is`), and for documents whose file is invalid. Per-file providers (for example editor config) may override each of the four settings, first answer wins. They are undo-transparent: no separate undo step. | Autosave on deactivation strips trailing spaces on edited lines; undo does not offer a "strip" step. |
| PLAT-SAVE-091 | MUST | **Caret rule.** With "keep on caret line" on, trailing spaces on a line that contains a caret of the editor that has focus for this document are not removed, and that line stays flagged as modified after the save so that the next save strips it once the caret has left. Carets are restored to their previous visual position after stripping. With "Modified lines" and no virtual space, the flags of lines holding carets are kept; with virtual space enabled they are cleared. If the last line is only whitespace and "ensure final newline" is on: with the caret on it and "keep" on, a newline is appended and the spaces stay; otherwise the whitespace is deleted. | Type `x ` at the end of a line, deactivate while the caret is there: the file has the space; move the caret away and deactivate again: the space is gone. |
| PLAT-SAVE-092 | SHOULD | The same caret rule applies to autosave and to explicit saves. The "active editor" is the focused editor on the document; when none is focused (for example the window is deactivated), there are no caret lines to keep. Remote client editors count when focused. | Expected, to be confirmed (open question 6): deactivate the window with the caret on a line with trailing spaces; the editor has lost focus, so the spaces are stripped. |
| PLAT-SAVE-093 | SHOULD | Save All (the user action) first strips the focused editor's document when stripping is on and "keep on caret line" is off, including its caret lines, before the general save. It is the only place where explicit differs from non-explicit for stripping. | With keep off, Save All strips the caret line. |
| PLAT-SAVE-094 | SHOULD | If a document cannot be stripped at that moment, it is remembered and stripped at the beginning of the next `save_all`. The memory is dropped when the document is deleted or the unsaved set is discarded. | Not observable without a test hook. |

## 10. Actions on Save (interplay only)

Actions on Save (format, optimize imports, rearrange, code cleanup; Settings > Tools > Actions on Save, link "Configure autosave options..." back to the page of section 3) are specified in the code-insight set. Only the interplay with saving is fixed here.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-100 | SHOULD | Every `save_all` schedules the enabled Actions on Save for the project's unsaved documents that are in project content, after the normal save, in the background with the progress title "Running Actions on Save" and per file "<action>: processing <file>". Individual `save_one` calls by features do **not** trigger them, to avoid surprising runs (for example a VCS-driven save while typing). | Reformat on save on: Save All reformats and saves. A feature that calls `save_one` does not reformat. |
| PLAT-SAVE-101 | SHOULD | A user edit of a document cancels the pending Actions on Save for that document only; that document is then not saved by them. When all actions finish for a document it is saved. | Press Save All, type during the run: that file keeps the edit and stays unsaved. |
| PLAT-SAVE-102 | SHOULD | The Save Document action marks the current action as "explicit single save" so that PLAT-SAVE-100 applies to that one document. The "Activated on" column offers "Any save (including autosave)" or "Explicit save (Ctrl+S)" for actions that choose to distinguish. | Choose "Explicit save": autosave never reformats. |

## 11. Analysis does not depend on saving

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| PLAT-SAVE-110 | MUST | The highlighting daemon and inspections analyze the **document text** (as committed to the syntax tree, PLAT-VFS-110..114), never the disk. A change to a physical document: before it is applied, the daemon is stopped and highlighters damaged by the edit are dropped; the file scope is marked dirty; after the commit of the tree the daemon restarts for the visible file. Therefore errors and warnings appear while typing, without saving. A bulk update defers the restart until it finishes. | Type a syntax error in a Java file and wait: the red mark appears with the file still unsaved. |
| PLAT-SAVE-111 | MUST | A save does not restart analysis and does not change highlights: the save path changes no text except trailing-space and final-newline edits, which are ordinary document changes. The "from save" event is ignored by reload logic (PLAT-VFS-094). | Wait for a clean daemon state with no "Essential only" files (PLAT-SAVE-112), deactivate the window: no new analysis run starts. |
| PLAT-SAVE-112 | SHOULD | Exception: whenever a queued settings-save task runs (Save All, frame deactivation, project close), a full highlighting restart is requested for projects with open files whose highlighting level is "Essential only", so the remaining diagnostics appear after saving. A registry-style switch (default on) turns this off. | Set a file to "Essential only", Save All: full inspections run. |
| PLAT-SAVE-113 | MUST | Tools that read files from disk (build, external processes, VCS commands, run) are the reason for sections 5 and 6; they must not be given a silent exception to saving, and they must save at the points listed, not rely on autosave. | Disable both autosave options and run PLAT-SAVE-030, 033, 040: all pass. |

## 12. Implementation notes (non-binding)

* Keep the "explicit" flag as a plain boolean on the save call and on the veto contract; it is the only difference the rest of the system sees.
* The unsaved set can be driven by a document-change listener that runs first (lowest priority number), so any change, including a programmatic one, marks the document. External reloads must not mark it.
* A single `SaveCoordinator` service can own: the idle timer, the deactivation hook, the suppression counter and the debounced settings queue. Document writing belongs to the document manager.
* The failure dialog is shown from an event-loop callback, never inside the write section.

## 13. Open questions

1. **Granularity of "frame deactivation".** The observed hook is the application-level activation event (whole app). Whether switching between two IDE windows of the same app counts was not found in the studied source; the spec assumes it does not.
2. **Rebase and stash with dialog.** Silent stash, branch checkout, merge, reset, worktree creation, update, rollback and shelve were found saving. A dedicated rebase action and the stash-with-dialog path were not found calling a save themselves; they may rely on the freeze (PLAT-SAVE-042) or on a shared repository-action base. Needs a behavior check.
3. **Tests.** No test-specific save was found; PLAT-SAVE-031 is inferred from tests running as run configurations through the runners that save. The JUnit and TestNG plugins were not scanned in detail.
4. **Refactoring and reformat.** No save was found in the platform refactoring and reformat code that was checked out; the Java refactoring tree is not fully present in the sparse clone, so PLAT-SAVE-054 is "not observed", not "proved".
5. **Suppression versus document autosave.** The suppression token stops the queued settings save and external sync, but the deactivation and idle paths call `save_all` directly without consulting it. The update code even comments that a save "can still occur". Decide whether Archipel gates document autosave during VCS operations; if it does, record it as a deliberate deviation.
6. **Caret line on deactivation.** PLAT-SAVE-092 follows from "focused editor only", but the focus owner at the moment of deactivation was not checked by running it. Verify with the reference IDE: does trailing whitespace on the caret line survive a deactivation autosave?
7. **Explicit Save All pre-strip.** With "keep on caret line" off, the regular strip already removes caret-line spaces; why Save All strips once more before the general save (PLAT-SAVE-093) is unclear (possibly to handle the deferred-strip case). Observable difference not found.
8. **Default of the tab marker.** The stored default of "Mark modified" is off, but the new UI may show a dot regardless through other paths (the file icon decoration returns the plain icon in the new UI). Verify in the reference IDE which defaults the shipped profile has.
9. **Failure dialog repetition.** After "Close" a persistent failure repeats the dialog at every idle period or deactivation. Whether the reference throttles it was not found.
10. **Content loading limit default.** PLAT-VFS-035 says 20 MB; PLAT-SAVE-025 and 073b reuse it without restating a number.
11. **Save before "Open in Browser", Ant, Inspect Code.** Listed from the source (PLAT-SAVE-036) but marked SHOULD because they are secondary tools; promote if Archipel ships them.
12. **Callers of the unchanged-save.** PLAT-SAVE-045 rests on the helper that saves a document that became equal to its VCS base; its call sites were not located in the checked-out tree, so the exact trigger (every change event or only line-status updates) is unverified.
13. **Actions on Save and autosave.** At the platform layer every `save_all`, including autosave, schedules Actions on Save (PLAT-SAVE-100); whether format-on-save itself restricts to explicit saves through its own "Activated on" option was not checked and belongs to the code-insight spec.
14. **Remote development.** One branch saves documents in a background write on deactivation when allowed by a registry flag, another on the UI thread; remote-host mode always uses the UI thread. The spec requires only the result, not the thread.
