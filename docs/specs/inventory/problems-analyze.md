# Problems view, Analyze and inspection UI

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (35)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `AnalyzeHeapDumpSnapshot` | Analyze Heap Dump | `Internal.HeapAnalysis` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/platform-impl` |
| `ChangeInspectionProfile` | Error Highlighting | `ChangeScheme` |  | codeinsight/01-daemon.md CI-DMN-062 | `platform/lang-impl` |
| `CodeCleanup` | Code Cleanup… | `InspectCodeGroup` |  | codeinsight/03-inspections.md CI-INS-095 | `platform/lang-impl` |
| `ConfigureInspectionsAction` | Configure Inspections… |  |  | codeinsight/03-inspections.md CI-INS | `platform/lang-impl` |
| `DisableInspection` | (set in code) | `InspectionToolWindow.TreePopup` |  | codeinsight/03-inspections.md CI-INS | `platform/lang-impl` |
| `EditAnalysisIgnoreDefaults` | Edit Default Analysis Exclusions… |  |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/lang-impl` |
| `EditInspectionSettings` | Settings… | `InspectionToolWindow.TreePopup` |  | codeinsight/03-inspections.md CI-INS | `platform/lang-impl` |
| `InspectCode` | Inspect Code… | `CommanderPopupMenu`, `InspectCodeGroup`, `InspectCodeActionInPopupMenus` |  | codeinsight/03-inspections.md CI-INS-095 | `platform/lang-impl` |
| `ProblemsView.AutoscrollToSource` | Open Files with Single Click | `ProblemsView.ToolWindow.SecondaryActions` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.CopyProblemDescription` | Copy Problem Description | `ProblemsView.ToolWindow.TreePopup` | as `$Copy` | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.AutoscrollToSource` | Open Files with Single Click | `ProblemsView.Frontend.SecondaryActions` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.CopyProblemDescription` | Copy Problem Description | `ProblemsView.Frontend.TreePopup` | as `$Copy` | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.EditSource` | Jump to Source | `ProblemsView.Frontend.TreePopup` | as `EditSource` | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.GroupByToolId` | Group by Inspection | `ProblemsView.Frontend.Options` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.OpenInPreviewTab` | Enable Preview Tab | `ProblemsView.Frontend.SecondaryActions` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.QuickFixes` | Show Quick-Fixes | `ProblemsView.Frontend.TreePopup`, `ProblemsView.Frontend.Toolbar` | as `ShowIntentionActions` | codeinsight/02-highlight-info.md CI-HLI-072 | `platform/problemsView` |
| `ProblemsView.Frontend.ShowPreview` | Open Editor Preview | `ProblemsView.Frontend.SecondaryActions`, `ProblemsView.Frontend.Toolbar` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.SortByName` | Name | `ProblemsView.Frontend.Options` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.SortBySeverity` | Severity | `ProblemsView.Frontend.Options` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.SortFoldersFirst` | Folders Always on Top | `ProblemsView.Frontend.Options` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.Frontend.SwitchImplementation` | Turn on Split `Problems View` Implementation |  |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.GroupByToolId` | Group by Inspection | `ProblemsView.Options` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.OpenInPreviewTab` | Enable Preview Tab | `ProblemsView.ToolWindow.SecondaryActions` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.QuickFixes` | Show Quick-Fixes | `ProblemsView.ToolWindow.Toolbar`, `ProblemsView.ToolWindow.TreePopup` | as `ShowIntentionActions` | codeinsight/02-highlight-info.md CI-HLI-072 | `platform/problemsView` |
| `ProblemsView.ShowPreview` | Open Editor Preview | `ProblemsView.ToolWindow.SecondaryActions`, `ProblemsView.ToolWindow.Toolbar` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.SortByName` | Name | `ProblemsView.Options` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.SortBySeverity` | Severity | `ProblemsView.Options` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `ProblemsView.SortFoldersFirst` | Folders Always on Top | `ProblemsView.Options` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/problemsView` |
| `RunInspection` | (set in code) | `AnalyzeActions` | `control shift alt I` | codeinsight/03-inspections.md CI-INS | `platform/lang-impl` |
| `RunInspectionOn` | (set in code) | `InspectionToolWindow.TreePopup` |  | codeinsight/03-inspections.md CI-INS | `platform/lang-impl` |
| `ScopeView.EditScopes` | Edit Scopes… | `ProjectView.RelatedActions`, `ProjectView.ToolWindow.SecondaryActions` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/lang-impl` |
| `SeverityEditorDialog` | (set in code) |  |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/lang-impl` |
| `ShowPackageDeps` | Analyze Dependencies… | `ShowPackageDepsGroup` |  | framework: codeinsight/03-inspections.md (CI-INS) | `java/java-backend` |
| `SilentCodeCleanup` | Silent Code Cleanup | `AnalyzeActions` |  | codeinsight/08-optimize-imports-and-reformat.md CI-OPT | `platform/lang-impl` |
| `ViewOfflineInspection` | View Offline Inspection Results… | `AnalyzeActions` |  | framework: codeinsight/03-inspections.md (CI-INS) | `platform/lang-impl` |

## Settings pages (2)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `Errors` | Inspections | editor |  | project | codeinsight/01-daemon.md CI-DMN-006 |
| `project.scopes` | Scopes | appearance |  | project | MISSING |

## Tool windows (1)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Problems View` | Problems | bottom |  | ProblemsView | MISSING |
