# Maven

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (55)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `Maven.AddFileAsMavenProject` | Add as Maven Project | `ProjectViewPopupMenu` |  | MISSING | `plugins/maven` |
| `Maven.AddManagedFiles` | Add Maven Projects | `Maven.NavigatorActionsToolbar` |  | MISSING | `plugins/maven` |
| `Maven.AfterCompile` | Execute After Build | `Maven.TasksGroup` |  | MISSING | `plugins/maven` |
| `Maven.AfterRebuild` | Execute After Rebuild | `Maven.TasksGroup` |  | MISSING | `plugins/maven` |
| `Maven.AlwaysShowArtifactId` | Always Show ArtifactId |  |  | MISSING | `plugins/maven` |
| `Maven.AssignShortcut` | Assign Shortcut… | `Maven.TasksGroup` |  | MISSING | `plugins/maven` |
| `Maven.BeforeCompile` | Execute Before Build | `Maven.TasksGroup` |  | MISSING | `plugins/maven` |
| `Maven.BeforeRebuild` | Execute Before Rebuild | `Maven.TasksGroup` |  | MISSING | `plugins/maven` |
| `Maven.BeforeRun` | Execute Before Run/Debug… | `Maven.TasksGroup` |  | MISSING | `plugins/maven` |
| `Maven.CollapseAll` | Collapse All | `Maven.NavigatorActionsToolbar` | as `CollapseAll` | platform/08-language-extension-points.md PLAT-LANG-063 | `plugins/maven` |
| `Maven.DependencyAnalyzer.GoTo` | (set in code) | `ExternalSystem.DependencyAnalyzer.DependencyListGroup`, `ExternalSystem.DependencyAnalyzer.DependencyTreeGroup`, `ExternalSystem.DependencyAnalyzer.UsagesTreeGroup` |  | MISSING | `plugins/maven` |
| `Maven.DependencyAnalyzer.OpenConfig` | (set in code) | `ExternalSystem.DependencyAnalyzer.DependencyListGroup`, `ExternalSystem.DependencyAnalyzer.DependencyTreeGroup`, `ExternalSystem.DependencyAnalyzer.UsagesTreeGroup` |  | MISSING | `plugins/maven` |
| `Maven.DownloadAllDocs` | Download Documentation | `Maven.DownloadAllGroup` |  | MISSING | `plugins/maven` |
| `Maven.DownloadAllSources` | Download Sources | `Maven.DownloadAllGroup` |  | MISSING | `plugins/maven` |
| `Maven.DownloadAllSourcesAndDocs` | Download Sources and Documentation | `Maven.DownloadAllGroup` |  | MISSING | `plugins/maven` |
| `Maven.DownloadSelectedDocs` | Download Documentation | `Maven.DependencyMenu` |  | MISSING | `plugins/maven` |
| `Maven.DownloadSelectedSources` | Download Sources | `Maven.DependencyMenu` |  | MISSING | `plugins/maven` |
| `Maven.DownloadSelectedSourcesAndDocs` | Download Sources and Documentation | `Maven.DependencyMenu` |  | MISSING | `plugins/maven` |
| `Maven.EditRunConfiguration` | Edit Run Configuration… | `Maven.RunConfigurationMenu` |  | MISSING | `plugins/maven` |
| `Maven.ExecuteGoal` | Execute Maven Goal | `Maven.NavigatorActionsToolbar` |  | MISSING | `plugins/maven` |
| `Maven.ExpandAll` | Expand All | `Maven.NavigatorActionsToolbar` | as `ExpandAll` | MISSING | `plugins/maven` |
| `Maven.GroupProjects` | Group Modules |  |  | MISSING | `plugins/maven` |
| `Maven.IgnoreProjects` | Ignore Projects | `Maven.BaseProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.Killall` | Stop All Maven Connectors | `Maven.Internal` |  | MISSING | `plugins/maven` |
| `Maven.NavigatorDependencyAnalyzer` | (set in code) | `Maven.NavigatorActionsToolbar`, `Maven.NavigatorProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.OpenSettingsXml` | Open settings.xml | `Maven.BaseProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.OpenToolchainsXml` | Open toolchains.xml | `Maven.BaseProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.PrintStatistics` | Print Maven Statistics | `Maven.Internal` |  | MISSING | `plugins/maven` |
| `Maven.ProjectViewDependencyAnalyzer` | (set in code) | `ProjectViewPopupMenu` |  | MISSING | `plugins/maven` |
| `Maven.ReReadAll` | Read All Maven Projects | `Maven.Internal` |  | MISSING | `plugins/maven` |
| `Maven.Reimport` | Reload All Maven Projects | `Maven.SyncAllGroup` |  | MISSING | `plugins/maven` |
| `Maven.ReimportProject` | Sync Project | `Maven.BaseProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.RemoveManagedFiles` | Unlink Maven Projects | `Maven.NavigatorActionsToolbar`, `Maven.BaseProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.RemoveRunConfiguration` | Remove Run Configuration | `Maven.RunConfigurationMenu` |  | MISSING | `plugins/maven` |
| `Maven.ResetProfiles` | Reset Profiles | `Maven.ProfilesMenu` |  | MISSING | `plugins/maven` |
| `Maven.ResetReadStatistic` | ResetMavenStatistics | `Maven.Internal` |  | MISSING | `plugins/maven` |
| `Maven.RunBuild` | Run Maven Build | `Maven.NavigatorActionsToolbar`, `Maven.NavigatorProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.ShowBasicPhasesOnly` | Show Basic Phases Only |  |  | MISSING | `plugins/maven` |
| `Maven.ShowConnectors` | Show Maven Connectors | `Maven.Internal` |  | MISSING | `plugins/maven` |
| `Maven.ShowEffectivePom` | Show Effective POM | `Maven.BaseProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.ShowIgnored` | Show Ignored Projects |  |  | MISSING | `plugins/maven` |
| `Maven.ShowSettings` | Maven Settings | `Maven.ShowSettingsGroup` |  | MISSING | `plugins/maven` |
| `Maven.ShowVersions` | Maven tool window: Show project version next to name |  |  | MISSING | `plugins/maven` |
| `Maven.SyncAllSplitAction` | (set in code) | `Maven.NavigatorActionsToolbar` |  | MISSING | `plugins/maven` |
| `Maven.SyncIncrementally` | Sync All Maven Projects | `Maven.SyncAllGroup` |  | MISSING | `plugins/maven` |
| `Maven.ToggleLookNested` | Toggle Search For Maven Project Recursively |  |  | MISSING | `plugins/maven` |
| `Maven.ToggleOffline` | Toggle Offline Mode | `Maven.NavigatorActionsToolbar` |  | MISSING | `plugins/maven` |
| `Maven.ToggleProfile` | Toggle Profile | `Maven.ProfileMenu` |  | MISSING | `plugins/maven` |
| `Maven.ToggleSkipTests` | Toggle 'Skip Tests' Mode | `Maven.NavigatorActionsToolbar` |  | MISSING | `plugins/maven` |
| `Maven.UpdateFolders` | Generate Sources and Update Folders For All Projects | `Maven.NavigatorActionsToolbar` |  | MISSING | `plugins/maven` |
| `Maven.UpdateFoldersForProject` | Generate Sources and Update Folders | `Maven.BaseProjectMenu` |  | MISSING | `plugins/maven` |
| `Maven.UpdateLocalIndex` | Reindex repository | `Maven.RepositoryMenu` |  | MISSING | `plugins/maven` |
| `Maven.ViewDependencyAnalyzer` | (set in code) | `Maven.DependencyMenu` |  | MISSING | `plugins/maven` |
| `refactoring.extract.dependency` | Extract Managed Dependency | `Maven.RefactoringGroup` | as `ExtractMethod` | MISSING | `plugins/maven` |
| `refactoring.introduce.property` | Property | `Maven.RefactoringGroup` | as `IntroduceVariable` | MISSING | `plugins/maven` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `MavenSettings` | Maven | build.tools |  | project | MISSING |

## Run configuration types (1)

| Class | Type id | Display name | Plugin | Spec |
|---|---|---|---|---|
| `MavenRunConfigurationType` | `MavenRunConfiguration` | Maven | plugins/maven | MISSING |
