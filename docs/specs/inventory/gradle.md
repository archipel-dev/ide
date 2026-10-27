# Gradle and external systems

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (44)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `AddGradleDslPluginAction` | (set in code) | `GenerateGradleDslGroup` |  | MISSING | `plugins/gradle` |
| `ExternalSystem.AfterCompile` | Execute After Build | `ExternalSystem.Actions`, `ExternalSystemView.TaskActivationGroup` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.AfterRebuild` | Execute After Rebuild | `ExternalSystem.Actions`, `ExternalSystemView.TaskActivationGroup` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.AfterSync` | Execute After Sync | `ExternalSystem.Actions`, `ExternalSystemView.TaskActivationGroup` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.AssignRunConfigurationShortcut` | Assign Shortcut… | `ExternalSystem.Actions`, `ExternalSystemView.RunConfigurationMenu` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.AssignShortcut` | Assign Shortcut… | `ExternalSystem.Actions`, `ExternalSystemView.TaskMenu` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.AttachProject` | (set in code) | `ExternalSystemView.ActionsToolbar.AttachProjectPanel` |  | MISSING | `java/java-backend` |
| `ExternalSystem.BeforeCompile` | Execute Before Build | `ExternalSystem.Actions`, `ExternalSystemView.TaskActivationGroup` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.BeforeRebuild` | Execute Before Rebuild | `ExternalSystem.Actions`, `ExternalSystemView.TaskActivationGroup` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.BeforeRun` | Execute Before Run/Debug… | `ExternalSystem.Actions` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.BeforeSync` | Execute Before Sync | `ExternalSystem.Actions`, `ExternalSystemView.TaskActivationGroup` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.CollapseAll` | Collapse All | `ExternalSystem.Actions`, `ExternalSystemView.ActionsToolbar` | as `CollapseAll` | platform/08-language-extension-points.md PLAT-LANG-063 | `platform/external-system-impl` |
| `ExternalSystem.DetachProject` | (set in code) | `ExternalSystem.Actions`, `ExternalSystemView.BaseProjectMenu`, `ExternalSystemView.ActionsToolbar.AttachProjectPanel` | as `$Delete` | MISSING | `platform/external-system-impl` |
| `ExternalSystem.EditRunConfiguration` | Edit Run Configuration… | `ExternalSystemView.RunConfigurationMenu` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.ExpandAll` | Expand All | `ExternalSystem.Actions`, `ExternalSystemView.ActionsToolbar` | as `ExpandAll` | MISSING | `platform/external-system-impl` |
| `ExternalSystem.GroupModules` | Group Modules | `ExternalSystem.Actions` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.GroupTasks` | Group Tasks | `ExternalSystem.Actions` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.HideProjectRefreshAction` | (set in code) | `ExternalSystem.ProjectRefreshActionGroup` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.IgnoreProject` | (set in code) | `ExternalSystem.Actions`, `ExternalSystemView.BaseProjectMenu`, `ExternalSystemView.ModuleMenu` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.OpenConfig` | (set in code) | `ExternalSystem.Actions`, `ExternalSystemView.BaseProjectMenu`, `ExternalSystemView.ModuleMenu` | as `EditSource` | MISSING | `platform/external-system-impl` |
| `ExternalSystem.OpenTasksActivationManager` | (set in code) | `ExternalSystem.Actions`, `ExternalSystemView.ProjectMenu`, `ExternalSystemView.ModuleMenu` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.ProjectRefreshAction` | (set in code) | `ExternalSystem.ProjectRefreshActionGroup` | `control shift O` | MISSING | `platform/external-system-impl` |
| `ExternalSystem.RefreshAllProjects` | (set in code) | `ExternalSystem.Actions`, `ExternalSystemView.ActionsToolbar.SyncPanel` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.RefreshProject` | (set in code) | `ExternalSystem.Actions`, `ExternalSystemView.BaseProjectMenu`, `ExternalSystemView.ModuleMenu` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.RemoveRunConfiguration` | Remove Run Configuration | `ExternalSystemView.RunConfigurationMenu` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.RunTask` | Run Task | `ExternalSystem.Actions` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.SelectProjectDataToImport` | Select Project Data To Import | `ExternalSystem.Actions`, `ExternalSystemView.ActionsToolbar.SyncPanel` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.ShowCommonSettings` | Auto-Sync Settings… | `ExternalSystem.Actions`, `ExternalSystem.ShowSettingsGroup`, `Maven.ShowSettingsGroup` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.ShowIgnored` | Show Ignored | `ExternalSystem.Actions` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.ShowInheritedTasks` | Display Tasks Inherited from Sub-Projects | `ExternalSystem.Actions` |  | MISSING | `platform/external-system-impl` |
| `ExternalSystem.ShowSettings` | (set in code) | `ExternalSystem.Actions`, `ExternalSystem.ShowSettingsGroup` |  | MISSING | `platform/external-system-impl` |
| `Gradle.AddGradleDslDependencyAction` | (set in code) | `Gradle.GenerateGroup` |  | MISSING | `plugins/gradle-maven` |
| `Gradle.DependencyAnalyzer.GoTo` | (set in code) | `ExternalSystem.DependencyAnalyzer.DependencyListGroup`, `ExternalSystem.DependencyAnalyzer.DependencyTreeGroup`, `ExternalSystem.DependencyAnalyzer.UsagesTreeGroup` |  | MISSING | `plugins/gradle` |
| `Gradle.DependencyAnalyzer.OpenConfig` | (set in code) | `ExternalSystem.DependencyAnalyzer.DependencyListGroup`, `ExternalSystem.DependencyAnalyzer.DependencyTreeGroup`, `ExternalSystem.DependencyAnalyzer.UsagesTreeGroup` |  | MISSING | `plugins/gradle` |
| `Gradle.DownloadSources` | Download Sources | `ExternalSystemView.ActionsToolbar.SyncPanel` |  | MISSING | `plugins/gradle` |
| `Gradle.ExecuteTask` | Execute Gradle Task | `Gradle.View.ActionsToolbar.RunPanel` |  | MISSING | `plugins/gradle` |
| `Gradle.ImportExternalProject` | Link Gradle Project | `ProjectViewPopupMenu` |  | MISSING | `plugins/gradle` |
| `Gradle.OpenProjectCompositeConfiguration` | Composite Build Configuration | `Gradle.View.ActionsToolbar.OtherActionsPanel`, `Gradle.View.ProjectMenu` |  | MISSING | `plugins/gradle` |
| `Gradle.ProjectViewDependencyAnalyzer` | (set in code) | `ProjectViewPopupMenu` |  | MISSING | `plugins/gradle` |
| `Gradle.RefreshDependencies` | (set in code) | `Gradle.View.ProjectMenu` |  | MISSING | `plugins/gradle` |
| `Gradle.ShowDaemons` | Show Gradle Daemons |  |  | MISSING | `plugins/gradle` |
| `Gradle.ToggleOfflineAction` | Toggle Offline Mode | `Gradle.View.ActionsToolbar.RunPanel` |  | MISSING | `plugins/gradle` |
| `Gradle.ToolbarDependencyAnalyzer` | (set in code) | `Gradle.View.ActionsToolbar.OtherActionsPanel` |  | MISSING | `plugins/gradle` |
| `Gradle.ViewDependencyAnalyzer` | (set in code) | `Gradle.View.ProjectMenu`, `Gradle.View.ModuleMenu`, `Gradle.View.DependencyMenu` |  | MISSING | `plugins/gradle` |

## Settings pages (2)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `build.tools` | Build Tools | build |  | project | MISSING |
| `reference.settingsdialog.project.gradle` | Gradle | build.tools |  | project | MISSING |

## Tool windows (1)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Gradle` | Gradle | right |  | GradleToolWindowFactory | MISSING |

## Run configuration types (1)

| Class | Type id | Display name | Plugin | Spec |
|---|---|---|---|---|
| `GradleExternalTaskConfigurationType` | `(see class)` | (see class) | plugins/gradle | MISSING |
