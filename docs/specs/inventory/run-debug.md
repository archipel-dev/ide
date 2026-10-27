# Run and Debug configurations, services, run tool window

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (101)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ActivateRunToolWindow` | Run | `ActivateToolWindowActions` | `alt 4` | MISSING | `platform/execution-impl` |
| `AllRunConfigurationsToggle` | Toggle All Run/Debug Configurations |  |  | MISSING | `platform/execution-impl` |
| `ChooseDebugConfiguration` | Debug… | `RunMenu` | `alt shift F9` | MISSING | `platform/execution-impl` |
| `ChooseRunConfiguration` | Run… | `RunMenu` | `alt shift F10` | MISSING | `platform/execution-impl` |
| `Console.Execute` | Execute Current Statement in Console |  | `ENTER` | MISSING | `platform/platform-api` |
| `Console.Execute.Multiline` | Execute Current Statement in Multiline Console |  | `control ENTER` | MISSING | `platform/platform-api` |
| `Console.History.Browse` | Browse Query History | `Console.HistoryActions` | `control alt E` | MISSING | `platform/platform-api` |
| `Console.History.Next` | Next Entry in Console History | `Console.HistoryActions` |  | MISSING | `platform/platform-api` |
| `Console.History.Previous` | Previous Entry in Console History | `Console.HistoryActions` |  | MISSING | `platform/platform-api` |
| `Console.Open` | Open Console |  | `control shift F10` | MISSING | `platform/platform-api` |
| `Console.SplitLine` | Split Line |  | as `EditorSplitLine` | MISSING | `platform/platform-api` |
| `ConsoleView.ClearAll` | (set in code) | `ConsoleView.PopupMenu` |  | MISSING | `platform/consoleView` |
| `ConsoleView.FoldLinesLikeThis` | Fold Lines Like This | `ConsoleEditorPopupMenu` |  | MISSING | `idea/customization` |
| `DeleteRunConfiguration` | Delete | `RunConfiguration.Group` |  | MISSING | `platform/execution-impl` |
| `ExecutionTargets` | Targets | `ExecutionTargetsToolbarGroup` |  | MISSING | `platform/execution-impl` |
| `FindUsedTestData` | Find Unused Test Data |  |  | MISSING | `java/execution` |
| `ImportTestsFromFile` | Import Test From File | `SmRunTestGroup` |  | MISSING | `platform/smRunner` |
| `InactiveStopActionPlaceholder` | (set in code) | `RunToolbarMainActionGroup` |  | MISSING | `platform/execution-impl` |
| `JavaControlBreak` | (set in code) | `RunTab.TopToolbar`, `RunTab.TopToolbar.Old` |  | MISSING | `java/execution` |
| `JavaSoftExit` | (set in code) | `RunTab.TopToolbar`, `RunTab.TopToolbar.Old` |  | MISSING | `java/execution` |
| `ManageTargets` | (set in code) | `RunMenu` |  | MISSING | `platform/execution-impl` |
| `MoreRunToolbarActions` | (set in code) | `RunToolbarMainActionGroup` |  | MISSING | `platform/execution-impl` |
| `NewUiRunWidget` | Run Widget | `MainToolbarRight` |  | MISSING | `platform/execution-impl` |
| `PauseOutput` | (set in code) | `ConsoleEditorPopupMenu` |  | MISSING | `platform/execution-impl` |
| `RedesignedRunConfigurationSelector` | Select Run/Debug Configuration | `RunToolbarMainActionGroup` |  | MISSING | `platform/execution-impl` |
| `RemoteServers.ChooseServerDeployment` | Deploy | `RemoteServersViewToolbar` |  | MISSING | `platform/remote-servers` |
| `RemoteServers.ChooseServerDeploymentWithDebug` | Debug | `RemoteServersViewToolbar` |  | MISSING | `platform/remote-servers` |
| `RemoteServers.ConnectServer` | Connect | `RemoteServersViewToolbar` |  | MISSING | `platform/remote-servers` |
| `RemoteServers.DisconnectServer` | Disconnect | `RemoteServersViewToolbar` |  | MISSING | `platform/remote-servers` |
| `RemoteServers.EditDeploymentConfig` | Edit Configuration | `RemoteServersViewToolbar` |  | MISSING | `platform/remote-servers` |
| `RemoteServers.EditServerConfig` | Edit Configuration | `RemoteServersViewToolbar` | as `EditSourceInNewWindow` | MISSING | `platform/remote-servers` |
| `Rerun` | Rerun | `RunTab.TopToolbar`, `RunTab.TopToolbar.Old`, `TouchBarDebug` (+1) | `control F5` | MISSING | `platform/execution-impl` |
| `RerunFailedTests` | Rerun Failed Tests |  |  | MISSING | `platform/testRunner` |
| `RerunTests` | Rerun Tests |  | `shift alt R` | MISSING | `platform/execution-impl` |
| `RunAnything` | Run Anything | `RunAnythingInSettingsEntryPoint` | `ctrl control dblClick` | MISSING | `platform/execution-impl` |
| `RunConfiguration` | Select Run/Debug Configuration | `ToolbarRunGroup`, `TouchBarDefaultExecution` |  | MISSING | `platform/execution-impl` |
| `RunConfigurationTemplatesForNewProjects` | (set in code) | `FileOtherSettingsGroup`, `WelcomeScreen.Configure` |  | MISSING | `platform/execution-impl` |
| `RunDashboard.AddConfiguration` | Run Configuration… | `ServiceView.AddService` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.ClearConsole` | Clear Console | `RunDashboardPopup` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.ClearContent` | Clear Content | `RunDashboardPopup` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.CopyConfiguration` | Copy Configuration | `RunDashboardPopup` | as `EditorDuplicate` | MISSING | `platform/execution.dashboard` |
| `RunDashboard.Debug` | Debug | `RunTab.TopToolbar`, `RunTab.TopToolbar.Old`, `XDebugger.ToolWindow.TopToolbar3` (+1) | as `DebugClass` | MISSING | `platform/execution.dashboard` |
| `RunDashboard.DoubleClickRun` | Double Click to Run Configuration | `ServiceView.Gear` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.EditConfiguration` | Edit Configuration | `RunDashboardPopup` | as `EditSourceInNewWindow` | MISSING | `platform/execution.dashboard` |
| `RunDashboard.GroupByStatus` | Status | `RunDashboard.ViewOptions` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.GroupByType` | Type | `RunDashboard.ViewOptions` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.GroupConfigurations` | Group Configurations | `RunDashboardPopup` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.HideConfiguration` | Hide Configuration | `RunDashboardPopup` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.OpenRunningConfigInNewTab` | Open Running Configuration in New Tab | `RunDashboardPopup` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.RemoveType` | Remove Configuration Type from Services | `RunDashboardPopup` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.RestoreConfiguration` | Restore Configuration | `RunDashboardPopup` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.RestoreHiddenConfigurations` | Restore Hidden Configurations | `RunDashboardPopup`, `ServiceView.AddService` |  | MISSING | `platform/execution.dashboard` |
| `RunDashboard.Run` | Run | `RunTab.TopToolbar`, `RunTab.TopToolbar.Old`, `XDebugger.ToolWindow.TopToolbar3` (+1) | as `RunClass` | MISSING | `platform/execution.dashboard` |
| `RunDashboard.Stop` | Stop | `RunDashboardContentToolbar` | as `Stop` | MISSING | `platform/execution.dashboard` |
| `RunDashboard.UngroupConfigurations` | Ungroup Configurations | `RunDashboardPopup` | as `$Delete` | MISSING | `platform/execution.dashboard` |
| `RunToolbarEditConfigurationAction` | Edit Configuration… | `RunToolbarSlotContextMenuGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarMainMultipleStopAction` | Stop | `RunToolbarMainActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarMainRunConfigurationsAction` | Add Configuration… | `RunToolbarMainActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarMainSlotActive` | Active Process | `RunToolbarMainActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarMainSlotInfoAction` | Active Processes | `RunToolbarMainActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarMoveToTopAction` | Move Configuration to Top | `RunToolbarSlotContextMenuGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarProcessStartedAction` | Active Process | `RunToolbarActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarRemoveSlotAction` | Remove | `RunToolbarSlotContextMenuGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarRerunAction` | Rerun | `RunToolbarActionsGroup`, `RunToolbarMainActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarRunConfigurationsAction` | Add Configuration… | `RunToolbarActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarShowHidePopupAction` | Launch Several Configurations Simultaneously | `RunToolbarMainActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarShowToolWindowTab` | Show the process tool window | `RunToolbarSlotContextMenuGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarStopAction` | Stop | `RunToolbarActionsGroup`, `RunToolbarMainActionsGroup` |  | MISSING | `platform/execution-impl` |
| `RunToolbarWidgetAction` | Run/Debug Widget | `RightToolbarSideGroup` |  | MISSING | `platform/execution-impl` |
| `Runner.CloseAllUnpinnedViews` | Close All but Pinned | `Runner.View.Close.Group` |  | MISSING | `platform/execution-impl` |
| `Runner.CloseAllViews` | Close All | `Runner.View.Close.Group` |  | MISSING | `platform/execution-impl` |
| `Runner.CloseOtherViews` | Close Others | `Runner.View.Close.Group` |  | MISSING | `platform/execution-impl` |
| `Runner.CloseView` | Close | `Runner.View.Close.Group`, `Runner.View.Toolbar` |  | MISSING | `platform/execution-impl` |
| `Runner.FocusOnStartup` | Focus on Startup | `Runner.Focus` |  | MISSING | `platform/execution-impl` |
| `Runner.RestoreLayout` | Restore Default Layout | `Runner.Layout` |  | MISSING | `platform/execution-impl` |
| `Runner.ToggleTabLabels` | Show Tab Labels | `RunnerLayoutActions` |  | MISSING | `platform/execution-impl` |
| `SaveTemporaryRunConfiguration` | Save Configuration | `RunConfiguration.Group` |  | MISSING | `platform/execution-impl` |
| `SegmentedVcsControlAction` | VCS Widget | `LeftToolbarSideGroup` |  | MISSING | `platform/execution-impl` |
| `SendEOF` | Send EOF |  | `control D` | MISSING | `platform/execution-impl` |
| `Servers.Deploy` | Deploy | `RemoteServersViewToolbar`, `RunDashboardContentToolbar` |  | MISSING | `platform/remote-servers` |
| `Servers.DeployWithDebug` | Debug | `RemoteServersViewToolbar` |  | MISSING | `platform/remote-servers` |
| `Servers.Undeploy` | Undeploy | `RemoteServersViewToolbar`, `RunDashboardContentToolbar` |  | MISSING | `platform/remote-servers` |
| `ServiceView.ConfigureServices` | Configure Services Tool Window | `ServiceView.Gear` |  | MISSING | `platform/execution.serviceView` |
| `ServiceView.GroupByContributor` | Group Services by Type | `ServiceView.ViewOptions` | `control alt T` | MISSING | `platform/execution.serviceView` |
| `ServiceView.JumpToServices` | Jump to Services | `ServiceViewItemPopup` | as `ShowNavBar` | MISSING | `platform/execution.serviceView` |
| `ServiceView.OpenEachInNewTab` | Open Each in New Tab | `ServiceView.OpenInNewTabGroup`, `ServiceViewItemPopup` |  | MISSING | `platform/execution.serviceView` |
| `ServiceView.OpenInNewTab` | Open in New Tab | `ServiceView.OpenInNewTabGroup`, `ServiceViewItemPopup` |  | MISSING | `platform/execution.serviceView` |
| `ServiceView.OpenInToolWindow` | Open in Separate Tool Window | `ServiceViewItemPopup` |  | MISSING | `platform/execution.serviceView` |
| `ServiceView.SelectActiveService` | Always Select Active Service | `ServiceView.Gear` |  | MISSING | `platform/execution.serviceView` |
| `ServiceView.ShowServices` | Show Services Tree | `ServiceView.Gear` | `control shift T` | MISSING | `platform/execution.serviceView` |
| `ServiceView.SplitByType` | Open Each Type in New Tab | `ServiceView.OpenInNewTabGroup`, `ServiceViewItemPopup` |  | MISSING | `platform/execution.serviceView` |
| `ServiceView.SwitchImplementation` | Turn on split `ServiceView` implementation |  |  | MISSING | `platform/execution.serviceView` |
| `ShowLiveRunConfigurations` | Show Running List | `RunMenu` |  | MISSING | `platform/execution-impl` |
| `ShowRecentTests` | Recent Tests |  | `control shift SEMICOLON` | MISSING | `java/execution` |
| `Stop` | Stop | `ToolbarRunGroup`, `RunMenu`, `RunTab.TopToolbar` (+6) | `control F2` | MISSING | `platform/execution-impl` |
| `StopBackgroundProcesses` | Stop Background Processes… | `RunMenu` | `control shift F2` | MISSING | `platform/execution-impl` |
| `UpdateRunningApplication` | Update Running Application | `DebugReloadGroup` | `control F10` | MISSING | `platform/execution-impl` |
| `editRunConfigurations` | Edit Configurations… | `RunMenu` |  | MISSING | `platform/execution-impl` |
| `multilaunch.AddExecutableAction` | Add Configuration or Task |  |  | MISSING | `plugins/execution-multilaunch` |
| `multilaunch.DeleteExecutableAction` | Delete | `multilaunch.ManageExecutableGroup` |  | MISSING | `plugins/execution-multilaunch` |
| `multilaunch.EditExecutableAction` | Edit… | `multilaunch.ManageExecutableGroup` |  | MISSING | `plugins/execution-multilaunch` |

## Settings pages (5)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `preferences.externalTools` | External Tools | tools |  | application | MISSING |
| `application.elevation` | Process Elevation |  | preferences.general | application | MISSING |
| `Runtime.Targets.Configurable` | Run Targets | build |  | project | MISSING |
| `preferences.startup.tasks` | Startup Tasks | tools |  | project | MISSING |
| `RunToolbarSettingsConfigurableProvider` | Run Widget | build |  | project | MISSING |

## Run configuration types (8)

| Class | Type id | Display name | Plugin | Spec |
|---|---|---|---|---|
| `ApplicationConfigurationType` | `Application` | Application | java/execution | MISSING |
| `JarApplicationConfigurationType` | `JarApplication` | JAR Application | java/execution | MISSING |
| `JavaScratchConfigurationType` | `Java Scratch` | Java Scratch | java/execution | MISSING |
| `RemoteConfigurationType` | `Remote` | Remote JVM Debug | java/execution | MISSING |
| `CompoundRunConfigurationType` | `(see class)` | (see class) | platform/execution-impl | MISSING |
| `PluginConfigurationType` | `#org.jetbrains.idea.devkit.run.PluginConfigurationType` | Plugin | plugins/devkit | MISSING |
| `MultiLaunchConfigurationType` | `com.intellij.execution.configurations.multilaunch` | Multi-Launch | plugins/execution-multilaunch | MISSING |
| `XsltRunConfigType` | `XSLT` | (see class) | plugins/xpath | MISSING |

## Status bar widgets (1)

| Id | Factory | Order | Spec |
|---|---|---|---|
| `LanguageServiceStatusBarWidget` | `LanguageServiceWidgetFactory` | after Position, after AIAssistant, before LineSeparator | MISSING |
