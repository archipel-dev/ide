# VCS log and branches (generic)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (37)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `Vcs.Log.AlignLabels` | References on the Left | `Log.Unsorted.KeymapGroup`, `Vcs.Log.PresentationSettings` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.AnnotateRevisionAction` | Annotate | `Log.Unsorted.KeymapGroup`, `Vcs.FileHistory.ContextMenu`, `Log.FileHistory.KeymapGroup` | as `Annotate` | MISSING | `platform/vcs-log` |
| `Vcs.Log.CheckFirstCommitsIndexData` | (set in code) | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Internal` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.CheckSelectedCommitsIndexData` | (set in code) | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Internal` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.CollapseAll` | (set in code) | `Log.Unsorted.KeymapGroup`, `Vcs.Log.BranchActionsGroup` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.CompactReferencesView` | Compact References View | `Log.Unsorted.KeymapGroup`, `Vcs.Log.PresentationSettings` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.CompareRevisions` | Compare Versions | `Log.Unsorted.KeymapGroup`, `Vcs.Log.ContextMenu`, `Log.KeymapGroup` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.EnableFilterByRegexAction` | Regex | `Log.Unsorted.KeymapGroup`, `Vcs.Log.TextFilterSettings` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ExpandAll` | (set in code) | `Log.Unsorted.KeymapGroup`, `Vcs.Log.BranchActionsGroup` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.FocusTextFilter` | Focus Text Filter | `Log.Unsorted.KeymapGroup`, `Log.KeymapGroup` | `control L` | MISSING | `platform/vcs-log` |
| `Vcs.Log.GoToChild` | Go to Child Commit | `Log.Unsorted.KeymapGroup`, `Vcs.Log.ContextMenu`, `Log.KeymapGroup` | `LEFT` | MISSING | `platform/vcs-log` |
| `Vcs.Log.GoToParent` | Go to Parent Commit | `Log.Unsorted.KeymapGroup`, `Vcs.Log.ContextMenu`, `Log.KeymapGroup` | `RIGHT` | MISSING | `platform/vcs-log` |
| `Vcs.Log.GoToRef` | Go To Hash/Branch/Tag | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Toolbar.RightCorner`, `Log.KeymapGroup` | as `Find` | MISSING | `platform/vcs-log` |
| `Vcs.Log.InvalidateCaches` | (set in code) | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Internal` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.MatchCaseAction` | Match Case | `Log.Unsorted.KeymapGroup`, `Vcs.Log.TextFilterSettings` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.MaximizeCommitMessage` | Maximize Commit Message | `Vcs.Log.ContextMenu` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.MoveDiffPreviewToBottom` | Bottom | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Diff.Preview.Location` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.MoveDiffPreviewToRight` | Right | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Diff.Preview.Location` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.OpenAnotherTab` | (set in code) | `Log.Unsorted.KeymapGroup`, `Vcs.Log.ToolWindow.TabActions`, `Log.KeymapGroup` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.OpenAnotherTabInEditor` | (set in code) | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Toolbar.Internal`, `Vcs.Log.ToolWindow.TabActions.DropDown` (+1) |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.OpenRepositoryVersion` | Show File at Revision | `Log.Unsorted.KeymapGroup`, `Vcs.FileHistory.ContextMenu`, `Log.FileHistory.KeymapGroup` | as `Vcs.OpenRepositoryVersion` | MISSING | `platform/vcs-log` |
| `Vcs.Log.PreferCommitDate` | Commit Timestamp | `Log.Unsorted.KeymapGroup`, `Vcs.Log.PresentationSettings`, `Vcs.FileHistory.PresentationSettings` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.Refresh` | Refresh | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Toolbar.RightCorner`, `Vcs.FileHistory.Toolbar` (+2) | as `Refresh` | MISSING | `platform/vcs-log` |
| `Vcs.Log.ResumeIndexing` | Resume Indexing | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Toolbar.RightCorner`, `Vcs.FileHistory.Toolbar` (+1) |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.SelectInLog` | Show in VCS Log | `Log.Unsorted.KeymapGroup`, `VcsSelectionHistoryDialog.Popup`, `Vcs.FileHistory.ContextMenu` (+1) |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowAllAffected` | Show All Affected Files | `Log.Unsorted.KeymapGroup`, `Vcs.FileHistory.ContextMenu`, `Vcs.FileHistory.Toolbar` (+1) | as `VcsHistory.ShowAllAffected` | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowBigRepositories` | Show Repositories with Disabled Indexing | `Log.Unsorted.KeymapGroup`, `Vcs.Log.Internal` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowChangesFromParents` | Show Changes to Parents | `Log.Unsorted.KeymapGroup`, `Vcs.Log.ChangesBrowser.Popup`, `Log.KeymapGroup` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowDetailsAction` | Show Details | `Log.Unsorted.KeymapGroup`, `Vcs.Log.LayoutConfiguration`, `Log.KeymapGroup` (+1) |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowDiffPreview` | Show Diff Preview | `Log.Unsorted.KeymapGroup`, `Vcs.Log.LayoutConfiguration`, `Log.KeymapGroup` (+1) |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowLongEdges` | Long Edges | `Log.Unsorted.KeymapGroup`, `Vcs.Log.PresentationSettings` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowOnlyAffectedChanges` | Show Only Affected Changes | `Log.Unsorted.KeymapGroup`, `Vcs.Log.ChangesBrowser.Toolbar`, `Log.KeymapGroup` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowRootsColumnAction` | Root Names | `Log.Unsorted.KeymapGroup`, `Vcs.Log.PresentationSettings` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowSettingsAction` | Settings | `Log.Unsorted.KeymapGroup`, `Vcs.Log.ToolWindow.TabActions.DropDown` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowTagNames` | Tag Names | `Log.Unsorted.KeymapGroup`, `Vcs.Log.PresentationSettings` |  | MISSING | `platform/vcs-log` |
| `Vcs.Log.ShowTooltip` | Show Commit Tooltip | `Log.Unsorted.KeymapGroup` | as `QuickJavaDoc` | MISSING | `platform/vcs-log` |
| `Vcs.Show.Log` | Show VCS Log | `Log.Unsorted.KeymapGroup`, `Vcs.Show.Toolwindow.Tab`, `Git.MainMenu` |  | MISSING | `platform/vcs-log` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `vcs.log` | Log |  | project.propVCSSupport.Mappings | project | MISSING |
