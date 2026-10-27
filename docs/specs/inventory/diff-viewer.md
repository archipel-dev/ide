# Diff and merge viewer

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (62)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `CloseDiffEditor` | Close Diff Editor | `Diff.KeymapGroup` | `ESCAPE` | MISSING | `platform/diff-impl` |
| `CompareClipboardWithSelection` | Compare with Clipboard | `Diff.EditorPopupMenu`, `Diff.KeymapGroup`, `CompareActions` (+3) |  | MISSING | `platform/diff-impl` |
| `CompareFileWithEditor` | Compare File with Editor | `CommanderPopupMenu`, `HierarchyPopupMenu.Common`, `ProjectViewPopupMenu` (+2) |  | MISSING | `platform/diff-impl` |
| `CompareTwoFiles` | Compare Files | `CommanderPopupMenu`, `StructureViewPopupMenu`, `HierarchyPopupMenu.Common` (+3) | `control D` | MISSING | `platform/diff-impl` |
| `Diff.AppendLeftSide` | Append Left Side | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.AppendRightSide` | Append Right Side | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.ApplyLeftSide` | Accept Left Side | `Diff.KeymapGroup` | `control alt R` | MISSING | `platform/platform-api` |
| `Diff.ApplyNonConflicts` | Apply All Non-Conflicting Changes | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.ApplyNonConflicts.Left` | Apply Non-Conflicting Changes from the Left Side | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.ApplyNonConflicts.Right` | Apply Non-Conflicting Changes from the Right Side | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.ApplyRightSide` | Accept Right Side | `Diff.KeymapGroup` | `control alt A` | MISSING | `platform/platform-api` |
| `Diff.ComparePartial.Base.Left` | Left and Middle | `compare.contents` |  | MISSING | `platform/platform-api` |
| `Diff.ComparePartial.Base.Right` | Middle and Right | `compare.contents` |  | MISSING | `platform/platform-api` |
| `Diff.ComparePartial.Left.Right` | Left and Right | `compare.contents` |  | MISSING | `platform/platform-api` |
| `Diff.CompareWithBase.Left` | Base and Left | `compare.contents` |  | MISSING | `platform/platform-api` |
| `Diff.CompareWithBase.Result` | Base and Middle | `compare.contents` |  | MISSING | `platform/platform-api` |
| `Diff.CompareWithBase.Right` | Base and Right | `compare.contents` |  | MISSING | `platform/platform-api` |
| `Diff.FocusOppositePane` | Select Opposite Diff Pane | `Diff.KeymapGroup` | `control shift TAB` | MISSING | `platform/platform-api` |
| `Diff.FocusOppositePaneAndScroll` | Select Position in Opposite Diff Pane | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.IgnoreLeftSide` | Ignore Left Side | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.IgnoreRightSide` | Ignore Right Side | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.MagicResolveConflicts` | Resolve Simple Conflicts | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.NextChange` | Compare Next File | `Diff.KeymapGroup` | `alt shift RIGHT` | MISSING | `platform/diff-impl` |
| `Diff.NextConflict` | Next Conflict | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.OpenDiffInEditor` | Open Selected Diff Request as Editor Tab | `Diff.KeymapGroup` |  | MISSING | `platform/diff-impl` |
| `Diff.OpenInEditor` | (set in code) |  | as `EditSource` | MISSING | `platform/diff-impl` |
| `Diff.PrevChange` | Compare Previous File | `Diff.KeymapGroup` | `alt shift LEFT` | MISSING | `platform/diff-impl` |
| `Diff.PreviousConflict` | Previous Conflict | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.ResolveConflict` | Resolve Automatically | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.ShowDiff` | Show Diff | `ActivityView.Popup`, `Diff.KeymapGroup`, `ChangesViewToolbar` (+18) | `control D` | MISSING | `platform/diff-impl` |
| `Diff.ShowInExternalTool` | Show Diff in External Tool | `Diff.KeymapGroup` |  | MISSING | `platform/platform-api` |
| `Diff.ShowSettingsPopup` | Show Diff Settings Popup… | `Diff.KeymapGroup` | `control shift D` | MISSING | `platform/platform-api` |
| `Diff.ShowStandaloneDiff` | Show Diff in a New Tab or Window | `Diff.KeymapGroup`, `ChangesViewPopupMenu`, `ShelvedChangesPopupMenu` (+5) |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.CancelComparingNewFilesWithEachOther` | Cancel Comparing New Files with Each Other | `DirDiffMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.CompareNewFilesWithEachOtherAction` | Compare New Files with Each Other | `DirDiffMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.EnableEqual` | Show Equal Files | `DirDiffMenu` |  | MISSING | `platform/platform-api` |
| `DirDiffMenu.EnableLeft` | Show New Files on Left Side | `DirDiffMenu` |  | MISSING | `platform/platform-api` |
| `DirDiffMenu.EnableNotEqual` | Show Difference | `DirDiffMenu` |  | platform/01-vfs-and-documents.md PLAT-VFS-101 | `platform/platform-api` |
| `DirDiffMenu.EnableRight` | Show New Files on Right Side | `DirDiffMenu` |  | MISSING | `platform/platform-api` |
| `DirDiffMenu.Exclude` | Exclude from Results | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.MirrorToLeft` | Set Mirror to Left | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.MirrorToRight` | Set Mirror to Right | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.SetCopyToLeft` | Set Copy to Left | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.SetCopyToRight` | Set Copy to Right | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.SetDefault` | Set Default | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.SetDelete` | Set Delete | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.SetNoOperation` | Set Do Nothing | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `DirDiffMenu.SynchronizeDiff` | Synchronize Selected | `DirDiffMenu` | `ENTER` | MISSING | `platform/platform-api` |
| `DirDiffMenu.SynchronizeDiff.All` | Synchronize All | `DirDiffMenu` | `control ENTER` | MISSING | `platform/platform-api` |
| `DirDiffMenu.WarnOnDeletion` | Warn When Delete | `DirDiffOperationsMenu` |  | MISSING | `platform/diff-impl` |
| `Merge3Files` | Merge | `Internal1` |  | MISSING | `java/java-backend` |
| `MergeAllWindowsAction` | Merge All Project Windows | `OpenProjectWindows` |  | MISSING | `platform/platform-impl` |
| `MergeMenuWithWindowTitleAction` | Merge Main Menu with Window Title… | `ToolbarPopupActions.MainToolbarNewUI` |  | MISSING | `platform/platform-impl` |
| `NextDiff` | Next Difference | `Diff.KeymapGroup` | `F7` | MISSING | `platform/diff-impl` |
| `OpenBlankEditorInBlankDiffWindow` | Switch to Blank Editor | `BlankDiffViewerEditorPopupMenu` |  | MISSING | `platform/diff-impl` |
| `OpenFileEditorInBlankDiffWindow` | Switch to a File… | `BlankDiffViewerEditorPopupMenu` |  | MISSING | `platform/diff-impl` |
| `PreviousDiff` | Previous Difference | `Diff.KeymapGroup` | `shift F7` | MISSING | `platform/diff-impl` |
| `ShowBlankDiffWindow` | Open Blank Diff Window | `Diff.KeymapGroup` |  | MISSING | `platform/diff-impl` |
| `SwapSidesInDiffWindow` | Swap Sides | `Diff.ViewerToolbar`, `Diff.KeymapGroup` |  | MISSING | `platform/diff-impl` |
| `SwapThreeWayColorModeInDiffWindow` | Change Three-Side Viewer Mode | `Diff.ViewerToolbar`, `Diff.KeymapGroup` |  | MISSING | `platform/diff-impl` |
| `ToggleThreeSideInBlankDiffWindow` | Toggle Three-Side Viewer | `BlankDiffViewerEditorPopupMenu` |  | MISSING | `platform/diff-impl` |
| `openAssertEqualsDiff` | View assertEquals Difference | `TestTreePopupMenu` | as `Diff.ShowDiff` | MISSING | `platform/testRunner` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `diff.base` | Diff \\& Merge | tools |  | application | MISSING |
