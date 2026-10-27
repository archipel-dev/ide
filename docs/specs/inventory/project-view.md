# Project view and project model UI

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (44)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ImportModule` | Module from Existing Sources… | `JavaNewProjectOrModuleGroup` |  | MISSING | `java/java-backend` |
| `ImportModuleFromImlFile` | Import Module | `Java.ImportFrom.Group` |  | MISSING | `java/java-backend` |
| `MarkAsContentRoot` | Not Excluded | `MarkContentRootGroup` |  | MISSING | `idea/customization` |
| `MarkExcludeRoot` | Excluded | `MarkContentRootGroup` |  | MISSING | `idea/customization` |
| `MarkGeneratedSourceRoot` | (set in code) | `MarkGeneratedSourceRootGroup` |  | MISSING | `java/java-backend` |
| `ModuleSettings` | Module Settings |  |  | MISSING | `java/java-backend` |
| `NewDir` | Directory/Package | `NewGroup` |  | MISSING | `platform/lang-impl` |
| `NewElement` | New… | `ProjectViewToolbar` | `alt INSERT` | MISSING | `idea/customization` |
| `NewElementSamePlace` | New in This Directory… |  | `control alt INSERT` | MISSING | `platform/lang-impl` |
| `NewFile` | File | `NewGroup` |  | MISSING | `platform/lang-impl` |
| `NewModule` | Module… | `JavaNewProjectOrModuleGroup` |  | MISSING | `java/java-backend` |
| `NewModuleInGroup` | Module | `NewGroup1` |  | MISSING | `java/java-backend` |
| `NewModuleInfo` | (set in code) | `NewJavaSpecialFile` |  | MISSING | `java/java-backend` |
| `NewProject` | Project… | `JavaNewProjectGroup`, `ProjectWidget.Actions`, `NonModalWelcomeNewProjectGroup` |  | MISSING | `java/java-backend` |
| `ProjectView.AbbreviatePackageNames` | Abbreviate Qualified Names | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.AutoscrollFromSource` | Always Select Opened File | `ProjectView.ToolWindow.Behavior.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.AutoscrollToSource` | Open Files with Single Click | `ProjectView.ToolWindow.Behavior.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.CompactDirectories` | Compact Directories | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.CustomizeTrees` | Customize Tree Views… | `ProjectView.RelatedActions`, `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/lang-impl` |
| `ProjectView.FileNesting` | File Nesting… | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.FlattenModules` | Flatten Modules | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.FlattenPackages` | Flatten Packages | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.FoldersAlwaysOnTop` | Folders Always on Top | `ProjectView.ToolWindow.Sort.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.HideEmptyMiddlePackages` | Hide Empty Middle Packages | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.ManualOrder` | Manual Order | `ProjectView.ToolWindow.Sort.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.OpenDirectoriesWithSingleClick` | Open Directories with Single Click | `ProjectView.ToolWindow.Behavior.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.OpenInPreviewTab` | Enable Preview Tab | `ProjectView.ToolWindow.Behavior.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.ShowExcludedFiles` | Show Excluded Files | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.ShowLibraryContents` | Show Libraries Contents | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.ShowMembers` | Show Members | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.ShowModules` | Show Modules | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.ShowScratchesAndConsoles` | Show Scratches and Consoles | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.ShowVisibilityIcons` | Show Visibility Icons | `ProjectView.ToolWindow.Appearance.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.SortByName` | Sort by Name | `ProjectView.ToolWindow.Sort.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.SortByTimeAscending` | Sort by Modification Time (Oldest First) | `ProjectView.ToolWindow.Sort.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.SortByTimeDescending` | Sort by Modification Time (Newest First) | `ProjectView.ToolWindow.Sort.Actions` |  | MISSING | `platform/projectView` |
| `ProjectView.SortByType` | Sort by Type | `ProjectView.ToolWindow.Sort.Actions` |  | MISSING | `platform/projectView` |
| `ProjectViewEditSource` | Edit Source | `ProjectViewPopupMenu` | as `EditSource` | codeinsight/07-navigation.md | `platform/platform-impl` |
| `ProjectViewExpandAll` | Expand All |  | as `ExpandAllRegions` | MISSING | `platform/lang-impl` |
| `ReverteOverrideFileTypeAction` | Revert File Type Override | `MarkFileAs` |  | MISSING | `platform/lang-impl` |
| `ShowFilePath` | File Path | `GoToCodeGroup`, `RevealGroup` | `control alt F12` | MISSING | `platform/platform-impl` |
| `ShowProjectStructureSettings` | Project Structure… | `FileMainSettingsGroup`, `SettingsEntryPointGroup` | `control alt shift S` | MISSING | `java/java-backend` |
| `UnmarkGeneratedSourceRoot` | (set in code) | `MarkGeneratedSourceRootGroup` |  | MISSING | `java/java-backend` |
| `UnmarkRoot` | Unmark | `MarkContentRootGroup` |  | MISSING | `idea/customization` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `lombok` | Lombok | language |  | project | MISSING |

## Tool windows (2)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Project` | Project | left |  | ProjectViewToolWindowFactory | MISSING |
| `Project` | Project | left |  | ProjectViewToolWindowFactory | MISSING |
