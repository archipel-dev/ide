# Version control core (commit, changelists, shelve, annotate, history)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (114)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ActivateCommitToolWindow` | (set in code) | `ActivateToolWindowActions` | `alt 0` | MISSING | `platform/vcs-impl` |
| `Annotate` | Annotate | `Diff.EditorPopupMenu`, `Diff.EditorGutterPopupMenu`, `EditorGutterVcsPopupMenu` (+5) |  | java/07-inspections.md JAVA-INS-011 | `platform/vcs-impl` |
| `AnnotateStackTraceAction.show.files.modification.info` | Show Files Modification Info | `AnalyzeStacktraceToolbar` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.AddUnversioned` | Add to VCS | `ChangesViewPopupMenu`, `Unversioned.Files.Dialog`, `Unversioned.Files.Dialog.Popup` (+4) | `control alt A` | MISSING | `platform/vcs-impl` |
| `ChangesView.ApplyPatch` | Apply Patch… | `VcsGlobalGroup`, `ChangesView.ApplyPatch.LangGroup`, `Patch.MainMenu` (+2) |  | MISSING | `platform/vcs-impl` |
| `ChangesView.ApplyPatchFromClipboard` | Apply Patch from Clipboard… | `VcsGlobalGroup`, `Patch.MainMenu`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.Browse` | Browse Changes… | `VcsActions.KeymapGroup`, `hg4idea.file.menu`, `SubversionGroup` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.CreatePatch` | Create Patch from Local Changes… | `VcsGlobalGroup`, `Patch.MainMenu`, `ChangesViewPopupMenu` (+1) |  | MISSING | `platform/vcs-impl` |
| `ChangesView.CreatePatchFromChanges` | Create Patch… | `ShelvedChangesPopupMenu`, `VcsHistoryInternalGroup.Popup`, `CommittedChangesToolbar` (+7) |  | MISSING | `platform/vcs-impl` |
| `ChangesView.CreatePatchToClipboard` | Copy as Patch to Clipboard | `ChangesViewPopupMenu`, `ShelvedChangesPopupMenu`, `VcsActions.KeymapGroup` (+1) |  | MISSING | `platform/vcs-impl` |
| `ChangesView.Edit` | Check Out | `ChangesViewPopupMenu`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.EditSource` | (set in code) | `ChangesViewPopupMenu` | as `EditSource` | MISSING | `platform/vcs-impl` |
| `ChangesView.GroupBy.Directory` | Directory | `ChangesView.GroupBy` | `control alt P` | MISSING | `platform/vcs-impl` |
| `ChangesView.GroupBy.Module` | Module | `ChangesView.GroupBy` | `control alt M` | MISSING | `java/vcs` |
| `ChangesView.Move` | Move to Another Changelist… | `ChangesView.Changelists`, `ChangesViewPopupMenu`, `Unversioned.Files.Dialog.Popup` (+1) | `alt shift M` | MISSING | `platform/vcs-impl` |
| `ChangesView.NewChangeList` | New Changelist… | `ChangesView.Changelists`, `ChangesViewPopupMenu`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.Refresh` | Refresh | `VcsGlobalGroup`, `ChangesViewToolbar`, `ChangesViewPopupMenu` (+2) |  | MISSING | `platform/vcs-impl` |
| `ChangesView.RemoveChangeList` | Delete Changelist | `ChangesView.Changelists`, `ChangesViewPopupMenu`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.RemoveDeleted` | Remove from VCS | `ChangesViewPopupMenu`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.Rename` | Edit Changelist… | `ChangesView.Changelists`, `ChangesViewPopupMenu`, `VcsActions.KeymapGroup` | `F2`, `Shift F6` | MISSING | `platform/vcs-impl` |
| `ChangesView.Revert` | Rollback… | `ChangesViewToolbar`, `ChangesViewPopupMenu`, `VcsToolbarActions` (+9) | `control alt Z` | MISSING | `platform/vcs-impl` |
| `ChangesView.RevertFiles` | Rollback File… | `ChangesViewPopupMenu`, `VcsActions.KeymapGroup`, `Git.ContextMenu` (+1) |  | MISSING | `platform/vcs-impl` |
| `ChangesView.SaveToShelve` | Save To Shelf | `Shelve.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.SelectFile` | Select Opened File in Changes View | `ChangesViewToolbar.Shared` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.SetDefault` | Set Active Changelist | `ChangesView.Changelists`, `ChangesViewPopupMenu`, `VcsActions.KeymapGroup` | `control SPACE` | MISSING | `platform/vcs-impl` |
| `ChangesView.Shelve` | Shelve Changes… | `VcsGlobalGroup`, `ChangesViewPopupMenu`, `Shelve.KeymapGroup` (+1) |  | MISSING | `platform/vcs-impl` |
| `ChangesView.ShelveSilently` | Shelve Silently | `ChangesViewToolbar`, `Shelve.KeymapGroup` | `shift control H` | MISSING | `platform/vcs-impl` |
| `ChangesView.ShowCommitOptions` | Show Commit Options | `VcsActions.KeymapGroup` | `control O` | MISSING | `platform/vcs-impl` |
| `ChangesView.ShowIgnored` | Ignored Files | `ChangesView.ViewOptions` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.SingleClickPreview` | (set in code) | `ChangesViewToolbar.Shared.Other` |  | MISSING | `platform/vcs-impl` |
| `ChangesView.ToggleCommitUi` | Toggle Commit UI… | `SegmentedVcsActionsBarGroup`, `VcsTouchBarGroup`, `VcsGlobalGroup` (+4) | as `CheckinProject` | MISSING | `platform/vcs-impl` |
| `ChangesView.UnshelveSilently` | Unshelve Silently | `ShelvedChangesToolbar`, `Shelve.KeymapGroup` | `control alt U` | MISSING | `platform/vcs-impl` |
| `CheckStatusForFiles` | Check Status | `VcsActions.KeymapGroup`, `SubversionGroup` |  | MISSING | `platform/vcs-impl` |
| `CheckinFiles` | Commit File | `ChangesViewPopupMenu`, `VcsActions.KeymapGroup`, `Vcs.Operations.Popup.VcsAware` (+3) |  | MISSING | `platform/vcs-impl` |
| `CheckinProject` | Commit… | `SegmentedVcsActionsBarGroup`, `VcsTouchBarGroup`, `VcsGlobalGroup` (+7) | `control K` | MISSING | `platform/vcs-impl` |
| `CloseAllUnmodifiedEditors` | Close Unmodified Tabs | `CloseEditorsGroup`, `EditorTabsEntryPoint`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `CommitView.CloseWindowedFloatingTwOnCommit` | Close the Commit Tool Window in Window/Float Mode After Committing | `CommitView.GearActions` |  | MISSING | `platform/vcs-impl` |
| `CommitView.ShowOnDoubleClick.EditorPreview` | Show Diff on Double-Click | `CommitView.ShowOnDoubleClick`, `LocalChangesView.ShowOnDoubleClick` |  | MISSING | `platform/vcs-impl` |
| `CommitView.ShowOnDoubleClick.Source` | Show Source on Double-Click | `CommitView.ShowOnDoubleClick`, `LocalChangesView.ShowOnDoubleClick` |  | MISSING | `platform/vcs-impl` |
| `CommittedChanges.Clear` | Clear | `CommittedChangesToolbar`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `CommittedChanges.Details` | Show Details | `CommittedChangesToolbar`, `IncomingChangesToolbar`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `CommittedChanges.Filter` | Filter | `CommittedChangesToolbar`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `CommittedChanges.Refresh` | Refresh | `CommittedChangesToolbar`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `CommittedChanges.Revert` | Revert Changes | `CommittedChangesToolbar`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Compare.LastVersion` | Compare with Latest Repository Version | `Diff.CompareWith.KeymapGroup`, `SubversionGroup` |  | MISSING | `platform/vcs-impl` |
| `Compare.SameVersion` | Compare with Same Repository Version | `VcsToolbarActions`, `Diff.CompareWith.KeymapGroup`, `Vcs.Operations.Popup.VcsAware` (+3) | `control D` | MISSING | `platform/vcs-impl` |
| `Compare.Selected` | Compare with Revision… | `VcsActions.KeymapGroup`, `Git.FileActions`, `hg4idea.file.menu` (+1) |  | MISSING | `platform/vcs-impl` |
| `Compare.Specified` | Compare with Specified Revision… | `VcsActions.KeymapGroup`, `Diff.CompareWith.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `FileHistory.AnnotateRevision` | (set in code) | `VcsHistoryInternalGroup.Popup`, `FileHistory.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `IncomingChanges.Refresh` | Refresh | `IncomingChangesToolbar`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `IntegrateFiles` | Integrate | `VcsActions.KeymapGroup`, `SubversionGroup` |  | MISSING | `platform/vcs-impl` |
| `MarkFileDirty` | Mark File Dirty in VCS | `Internal.VFS` |  | MISSING | `platform/vcs-impl` |
| `ShelveChanges.UnshelveWithDialog` | Unshelve… | `ShelvedChangesPopupMenu`, `Vcs.Shelf.Operations.ContextMenu`, `Shelve.KeymapGroup` | `control shift U` | MISSING | `platform/vcs-impl` |
| `ShelvedChanges.ImportPatches` | (set in code) | `ShelvedChangesPopupMenu`, `Vcs.Shelf.Operations.ContextMenu`, `Shelve.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `ShelvedChanges.Rename` | Rename… | `ShelvedChangesPopupMenu`, `Shelve.KeymapGroup` | `F2`, `Shift F6` | MISSING | `platform/vcs-impl` |
| `ShelvedChanges.Restore` | Restore Applied Shelved Change | `ShelvedChangesPopupMenu`, `Vcs.Shelf.Operations.ContextMenu`, `Shelve.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `ShelvedChanges.ShowHideDeleted` | Show/Hide Applied Shelved Changes | `ShelvedChangesToolbar.ViewOptions`, `Shelve.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Show.Current.Revision` | Show Current Revision | `VcsActions.KeymapGroup`, `Git.ContextMenu`, `SubversionGroup` |  | MISSING | `platform/vcs-impl` |
| `Start.Use.Vcs` | Enable Version Control Integration… | `Vcs.MainMenu`, `VcsActions.KeymapGroup`, `Vcs.Operations.Popup.NonVcsAware` (+1) |  | MISSING | `platform/vcs-impl` |
| `UpdateFiles` | Update | `VcsActions.KeymapGroup`, `SubversionGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.ApplySelectedChanges` | Cherry-Pick Selected Changes | `Vcs.RepositoryChangesBrowserMenu`, `VcsActions.KeymapGroup`, `Git.Stash.ChangesBrowser.ContextMenu` |  | MISSING | `platform/vcs-impl` |
| `Vcs.CombinedDiff.CaretToNextBlock` | Next File |  | `RIGHT`, `PAGE_DOWN` | MISSING | `platform/vcs-impl` |
| `Vcs.CombinedDiff.CaretToPrevBlock` | Previous File |  | `LEFT`, `PAGE_UP` | MISSING | `platform/vcs-impl` |
| `Vcs.CombinedDiff.ToggleCollapseAllBlock` | Collapse All Files | `Vcs.Diff.EditorTabs.Group` |  | MISSING | `platform/vcs-impl` |
| `Vcs.CombinedDiff.ToggleCollapseBlock` | Collapse File |  | `control ESCAPE` | MISSING | `platform/vcs-impl` |
| `Vcs.Commit.CloseDialog` | Close Commit Dialog |  | as `EditorEscape` | MISSING | `platform/vcs-impl` |
| `Vcs.CopyCommitSubjectAction` | Copy Commit Subject |  |  | MISSING | `platform/vcs-impl` |
| `Vcs.CopyRevisionNumberAction` | Copy Revision Number | `VcsSelectionHistoryDialog.Popup`, `VcsActions.KeymapGroup`, `Vcs.Log.ContextMenu` (+2) | as `CopyReference` | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.EnableDisableCombinedDiff` | Show All Files in One Diff View | `Vcs.Diff.KeymapGroup`, `Diff.DiffModes`, `EditorTabPopupMenu` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.ExcludeChangedLinesFromCommit` | Split Chunks and Exclude Selected Lines from Commit | `Vcs.Diff.Commit.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.IncludeChangedLinesIntoCommit` | Split Chunks and Include Selected Lines into Commit | `Vcs.Diff.Commit.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.IncludeWholeChangedLinesIntoCommit` | Include Lines into Commit | `Vcs.Diff.Commit.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.ResolveConflictsInImports` | Resolve Conflicts in Import Statements | `Vcs.Diff.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.ShowCombinedDiff` | Show Combined Diff |  |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.ShowDiffInEditorTab` | Show Diff in Editor Tab | `Vcs.Diff.KeymapGroup`, `Diff.DiffModes`, `Diff.Binary.Settings` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.ShowDiffInNewWindow` | Show Diff in Separate Window | `Vcs.Diff.KeymapGroup`, `Diff.DiffModes`, `Diff.Binary.Settings` (+1) |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.ToggleDiffAligningMode` | Align Changes In Side-by-Side Diff | `Vcs.Diff.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Diff.ToggleSearchInChanges` | Search In Changes | `Editor.Search.Filter.Results` |  | MISSING | `platform/vcs-impl` |
| `Vcs.EditSource` | Edit Source | `Vcs.RepositoryChangesBrowserMenu`, `VcsActions.KeymapGroup`, `Git.Stash.ChangesBrowser.ContextMenu` | as `EditSource` | codeinsight/07-navigation.md | `platform/vcs-impl` |
| `Vcs.GetVersion` | Get | `VcsHistoryInternalGroup.Popup`, `Vcs.RepositoryChangesBrowserMenu`, `FileHistory.KeymapGroup` (+1) |  | MISSING | `platform/vcs-impl` |
| `Vcs.IntegrateProject` | Integrate Project | `VcsGlobalGroup`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.MoveChangedLinesToChangelist` | Move Lines to Another Changelist… | `Vcs.Diff.KeymapGroup`, `VcsActions.KeymapGroup` | `alt shift M` | MISSING | `platform/vcs-impl` |
| `Vcs.OpenRepositoryVersion` | (set in code) | `Vcs.RepositoryChangesBrowserMenu`, `VcsActions.KeymapGroup`, `Git.Stash.ChangesBrowser.ContextMenu` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Operations.Popup.Annotate` | Annotated Line Operations | `VcsGeneral.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.QuickListPopupAction` | VCS Operations Popup… | `VcsGlobalGroup`, `VcsActions.KeymapGroup`, `Git.MainMenu` | `alt BACK_QUOTE` | MISSING | `platform/vcs-impl` |
| `Vcs.ReformatCommitMessage` | (set in code) |  | as `ReformatCode` | MISSING | `platform/vcs-impl` |
| `Vcs.RefreshFileHistory` | (set in code) | `VcsHistoryInternalGroup.Toolbar` | as `Refresh` | MISSING | `platform/vcs-impl` |
| `Vcs.RevertSelectedChanges` | Revert Selected Changes | `Vcs.RepositoryChangesBrowserToolbar`, `Vcs.RepositoryChangesBrowserMenu`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.RollbackChangedLines` | Rollback Lines | `Vcs.Diff.KeymapGroup`, `VcsActions.KeymapGroup` | `control alt Z` | MISSING | `platform/vcs-impl` |
| `Vcs.RunCommitChecks` | Run Commit Checks | `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Shelf.Apply` | (set in code) | `Vcs.Shelf.Operations.ContextMenu` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Shelf.Drop` | (set in code) | `Vcs.Shelf.Operations.ContextMenu` | as `$Delete` | MISSING | `platform/vcs-impl` |
| `Vcs.Shelf.Pop` | (set in code) | `Vcs.Shelf.Operations.ContextMenu` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Shelf.UnshelveChanges` | (set in code) | `Vcs.Shelf.ChangesBrowser.ContextMenu` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Shelf.UnshelveChangesAndRemove` | (set in code) | `Vcs.Shelf.ChangesBrowser.ContextMenu` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Show.Local.Changes` | Show Local Changes | `Vcs.Show.Toolwindow.Tab` |  | MISSING | `platform/vcs-impl` |
| `Vcs.Show.Shelf` | Show Shelf | `Vcs.Show.Toolwindow.Tab`, `Git.MainMenu.LocalChanges` |  | MISSING | `platform/vcs-impl` |
| `Vcs.ShowConsoleTab` | Show VCS Console Tab | `Vcs.Log.ToolWindow.TabActions.DropDown` |  | MISSING | `platform/vcs-impl` |
| `Vcs.ShowDiffChangedLines` | Show Diff for Lines | `Vcs.Diff.KeymapGroup`, `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.ShowDiffWithLocal` | Compare with Local | `ShelvedChangesPopupMenu`, `Vcs.SavedPatches.ContextMenu`, `Vcs.SavedPatches.ChangesBrowser.ContextMenu` (+8) |  | MISSING | `platform/vcs-impl` |
| `Vcs.ShowDiffWithLocal.Before` | Compare Before with Local | `Vcs.RepositoryChangesBrowserMenu`, `VcsActions.KeymapGroup`, `Diff.CompareWith.KeymapGroup` (+1) |  | MISSING | `platform/vcs-impl` |
| `Vcs.ShowHistoryForBlock` | Show History for Selection… | `VcsActions.KeymapGroup`, `Git.FileActions`, `hg4idea.file.menu` (+1) |  | MISSING | `platform/vcs-impl` |
| `Vcs.ShowHistoryForRevision` | History Up to Here | `Vcs.RepositoryChangesBrowserToolbar`, `Vcs.RepositoryChangesBrowserMenu`, `RepositoryChangesBrowserToolbar` (+1) |  | MISSING | `platform/vcs-impl` |
| `Vcs.ShowMessageHistory` | Commit Message History | `Vcs.MessageActionGroup`, `VcsActions.KeymapGroup` | `control M` | MISSING | `platform/vcs-impl` |
| `Vcs.ShowTabbedFileHistory` | Show History | `VcsToolbarActions`, `VcsNavBarToolbarActions`, `VcsActions.KeymapGroup` (+6) |  | MISSING | `platform/vcs-impl` |
| `Vcs.ToggleAmendCommitMode` | Amend Commit | `ChangesView.CommitToolbar`, `VcsActions.KeymapGroup` | `alt M` | MISSING | `platform/vcs-impl` |
| `Vcs.Toolbar.ShowMoreActions` | Version Control | `SegmentedVcsActionsBarGroup` |  | MISSING | `platform/vcs-impl` |
| `Vcs.UpdateProject` | Update Project | `SegmentedVcsActionsBarGroup`, `VcsTouchBarGroup`, `VcsGlobalGroup` (+6) | `control T` | MISSING | `platform/vcs-impl` |
| `Vcs.VcsClone` | Get from Version Control… | `JavaNewProjectGroup`, `ProjectWidget.Actions`, `Vcs.MainMenu` (+2) |  | MISSING | `platform/vcs-impl` |
| `VcsHistory.ShowAllAffected` | Show All Affected Files | `Vcs.History`, `VcsSelectionHistoryDialog.Popup`, `VcsActions.KeymapGroup` | `alt shift A` | MISSING | `platform/vcs-impl` |
| `VcsShowCurrentChangeMarker` | Change Under Caret | `VcsActions.KeymapGroup` |  | MISSING | `platform/vcs-impl` |
| `VcsShowNextChangeMarker` | Next Change | `GoToChangeMarkerGroup`, `VcsActions.KeymapGroup` | `shift control alt DOWN` | MISSING | `platform/vcs-impl` |
| `VcsShowPrevChangeMarker` | Previous Change | `GoToChangeMarkerGroup`, `VcsActions.KeymapGroup` | `shift control alt UP` | MISSING | `platform/vcs-impl` |
| `VcsToolbarLabelAction` | VCS Label | `VcsToolbarActions`, `VcsNavBarToolbarActions` |  | MISSING | `platform/vcs-impl` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `project.propVCSSupport.Mappings` | Version Control | root |  | project | MISSING |

## Tool windows (2)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Commit` | Commit | left |  | CommitToolWindowFactory | MISSING |
| `Version Control` | Version Control | bottom |  | ChangeViewToolWindowFactory | MISSING |

## Status bar widgets (1)

| Id | Factory | Order | Spec |
|---|---|---|---|
| `IncomingChanges` | `IncomingChangesIndicatorFactory` |  | MISSING |
