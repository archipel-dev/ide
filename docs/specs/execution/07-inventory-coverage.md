# 07. Inventory coverage (run, debug, tests)

Status: bookkeeping. Requirement prefix `EXE-INV`. This file maps every action, settings page,
configuration type and tool window listed in `docs/specs/inventory/{run-debug,debugger,test-runner}.md`
(branch `docs/feature-inventory`) to the requirement that specifies it, or to a `LATER` requirement
with a reason. Nothing from those inventories is left unmapped.

## 1. LATER groups

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-INV-001 | LATER | new-UI run widget slots and toolbar variants (`Run Widget`, `Launch Several Configurations Simultaneously`, docked configurations) are a presentation variant of the selector in EXE-CTX-060/061; the classic toolbar is specified first (reason: UI variant). | Not applicable until scheduled. |
| EXE-INV-002 | LATER | Services tool window / run dashboard (grouping, hiding, remote servers `Deploy`/`Undeploy`): not needed for a Java IDE first release; the Run and Debug tool windows are the default (reason: not in scope of the first release). | Not applicable until scheduled. |
| EXE-INV-003 | LATER | Multi-Launch plugin configuration: product feature of a plugin outside the bundled core (reason: plugin). | Not applicable until scheduled. |
| EXE-INV-004 | LATER | Memory view, stream debugger, collection history: Java debugger extras that need the memory agent (reason: needs agent / extras). | Not applicable until scheduled. |
| EXE-INV-005 | LATER | interactive language consoles (`Console.*`), `Run Anything`: belong to the REPL/console and Search Everywhere specs (reason: other area). | Not applicable until scheduled. |
| EXE-INV-006 | LATER | test-suite editing helpers (`Add to JUnit Pattern Suite`, `Exclude from Suite`, TestNG suite actions), `View Report as Test Results`, `Recent Tests`, `Find Unused Test Data` (reason: test extras). | Not applicable until scheduled. |
| EXE-INV-007 | LATER | rarely used debugger/run actions: `Step Out of Code Block`, `Step Over Instruction`, `Trace Calls`, `Dump Threads` break control (`JavaControlBreak`, `JavaSoftExit`), `Debug Build Process`, internal `Throw Debug Exception`, `Repair Debugger View`, attach dialog toggles, multi-thread freeze selection actions, `Attach Project...`, feedback test action (reason: low priority). | Not applicable until scheduled. |
| EXE-INV-008 | LATER | platform/internal actions that only appear in these inventory files by placement: `Dump ... ` internal actions, `Jump to ...`, `Pause Scanning and Indexing`, External Tools, Process Elevation, Language service widget (reason: other area or internal). | Not applicable until scheduled. |

## 2. Mapping

| Id | Text | Specified by | Kind |
|---|---|---|---|
| `ActivateDebugToolWindow` | Debug | EXE-DBG-001 | covered |
| `ActivateRunToolWindow` | Run | EXE-CON-001 | covered |
| `AddBreakpoint` | Add Breakpoint | EXE-DBG-024 | covered |
| `AddConditionalBreakpoint` | Add Conditional Breakpoint… | EXE-DBG-024 | covered |
| `AddLoggingBreakpoint` | Add Logging Breakpoint… | EXE-DBG-024 | covered |
| `AddToISuite` | Add to JUnit Pattern Suite | EXE-INV-006 | LATER |
| `AddToTestNGSuite` | (set in code) | EXE-INV-006 | LATER |
| `AllRunConfigurationsToggle` | Toggle All Run/Debug Configurations | EXE-INV-001 | LATER |
| `application.elevation` | Process Elevation | EXE-INV-008 | LATER |
| `ApplicationConfigurationType` | Application | EXE-CONF-050 | covered |
| `AttachProject` | Attach Project… | EXE-INV-007 | LATER |
| `ChooseDebugConfiguration` | Debug… | EXE-CTX-001 | covered |
| `ChooseRunConfiguration` | Run… | EXE-CTX-062 | covered |
| `com.intellij.debugger.feedback.KotlinDebuggerFeedbackTestAction` | (set in code) | EXE-INV-007 | LATER |
| `CompoundRunConfigurationType` | Compound | EXE-CONF-057 | covered |
| `Console.Execute` | Execute Current Statement in Console | EXE-CON-050 | covered |
| `Console.Execute.Multiline` | Execute Current Statement in Multiline Console | EXE-CON-050 | covered |
| `Console.History.Browse` | Browse Query History | EXE-INV-005 | LATER |
| `Console.History.Next` | Next Entry in Console History | EXE-INV-005 | LATER |
| `Console.History.Previous` | Previous Entry in Console History | EXE-INV-005 | LATER |
| `Console.Open` | Open Console | EXE-INV-005 | LATER |
| `Console.SplitLine` | Split Line | EXE-INV-005 | LATER |
| `ConsoleView.ClearAll` | (set in code) | EXE-CON-016 | covered |
| `ConsoleView.FoldLinesLikeThis` | Fold Lines Like This | EXE-CON-042 | covered |
| `coverage` | Coverage settings page | EXE-COV-002 | covered |
| `Coverage` | Coverage tool window | EXE-COV-005 | covered |
| `DebugBuildProcess` | Debug Build Process | EXE-INV-007 | LATER |
| `Debugger.AddInlineWatch` | Add Inline Watch | EXE-DBG-077 | covered |
| `Debugger.AddSteppingFilter` | Add Stepping Filter… | EXE-DBG-047 | covered |
| `Debugger.AddToWatch` | Add to Watches | EXE-DBG-077 | covered |
| `Debugger.AdjustArrayRange` | Adjust Range… | EXE-DBG-072 | covered |
| `Debugger.AsyncStacks` | Async Stack Traces | EXE-DBG-115 | covered |
| `Debugger.AutoRenderer` | Auto | EXE-DBG-081 | covered |
| `Debugger.CallTracer` | Trace Calls | EXE-INV-007 | LATER |
| `Debugger.CopyStack` | Copy Stack | EXE-DBG-063 | covered |
| `Debugger.CreateRenderer` | Create… | EXE-DBG-081 | covered |
| `Debugger.CustomizeContextView` | Customize Data Views… | EXE-DBG-081 | covered |
| `Debugger.CustomizeThreadsView` | Customize Threads View… | EXE-DBG-062 | covered |
| `Debugger.EditArrayFilter` | Edit… | EXE-DBG-072 | covered |
| `Debugger.EditCustomField` | Edit… | EXE-DBG-081 | covered |
| `Debugger.EvaluateInConsole` | Evaluate in Console | EXE-DBG-077 | covered |
| `Debugger.FilterArray` | Filter… | EXE-DBG-072 | covered |
| `Debugger.FocusOnBreakpoint` | Focus On Breakpoint | EXE-DBG-064 | covered |
| `Debugger.FocusOnFinish` | Focus On Finish | EXE-DBG-064 | covered |
| `Debugger.ForceEarlyReturn` | Force Return | EXE-DBG-050 | covered |
| `Debugger.FreezeActiveThreadAction` | Suspend Thread | EXE-DBG-063 | covered |
| `Debugger.FreezeInactiveThreadsAction` | Suspend Other Threads | EXE-DBG-063 | covered |
| `Debugger.FreezeInactiveThreadsAmongSelected` | Suspend Inactive Threads in a Group | EXE-INV-007 | LATER |
| `Debugger.FreezeSelectedThreads` | Suspend Threads | EXE-INV-007 | LATER |
| `Debugger.MarkObject` | Mark Object… | EXE-DBG-078 | covered |
| `Debugger.MuteRenderers` | Mute Renderers | EXE-DBG-081 | covered |
| `Debugger.NewCustomField` | New Class Level Watch… | EXE-DBG-081 | covered |
| `Debugger.PopFrame` | Reset Frame | EXE-DBG-049 | covered |
| `Debugger.ReloadFile` | Compile And Reload File | EXE-DBG-106 | covered |
| `Debugger.RemoveAllBreakpoints` | Remove All Breakpoints | EXE-DBG-023 | covered |
| `Debugger.RemoveAllBreakpointsInFile` | Remove All Breakpoints In The Current File | EXE-DBG-023 | covered |
| `Debugger.RemoveArrayFilter` | Remove Filter | EXE-DBG-072 | covered |
| `Debugger.RemoveCustomField` | Remove Watch | EXE-DBG-081 | covered |
| `Debugger.RestoreBreakpoint` | Restore Breakpoint | EXE-DBG-022 | covered |
| `Debugger.ShowCollectionHistory` | Show Collection History | EXE-INV-004 | LATER |
| `Debugger.ShowLibraryFrames` | Show/Hide Library Frames | EXE-DBG-061 | covered |
| `Debugger.ShowReferring` | Show Referring Objects… | EXE-DBG-072 | covered |
| `Debugger.ShowTypes` | Show types | EXE-DBG-072 | covered |
| `Debugger.StepOutOfBlock` | Step Out of Code Block | EXE-INV-007 | LATER |
| `Debugger.SwitchToTheNextContext` | Switch to the Next Stopped Breakpoint | EXE-DBG-064 | covered |
| `Debugger.ThawActiveThreadAction` | Resume Thread | EXE-DBG-063 | covered |
| `Debugger.ThawAllThreadsAction` | Resume All Threads | EXE-DBG-063 | covered |
| `Debugger.ThawSelectedThreads` | Resume Threads | EXE-INV-007 | LATER |
| `Debugger.ThrowException` | Throw Exception | EXE-DBG-050 | covered |
| `Debugger.Tree.EvaluateInConsole` | Evaluate In Console | EXE-DBG-072 | covered |
| `Debugger.ViewText` | View Text | EXE-DBG-072 | covered |
| `Debugger.XThreadsView.FreezeThread` | Freeze | EXE-DBG-063 | covered |
| `Debugger.XThreadsView.InterruptThread` | Interrupt | EXE-DBG-063 | covered |
| `Debugger.XThreadsView.ResumeThread` | Resume | EXE-DBG-063 | covered |
| `DebugListen` | Throw Debug Exception | EXE-INV-007 | LATER |
| `DeleteRunConfiguration` | Delete | EXE-CTX-061 | covered |
| `DisableStreamDebuggerInlay` | Show Inline Hint | EXE-INV-004 | LATER |
| `DumpLibraryUsageStatistics` | Dump Library Usage Statistics | EXE-INV-008 | LATER |
| `DumpLookupElementWeights` | Dump Lookup Element Weights to Log | EXE-INV-008 | LATER |
| `DumpPluginDescriptorsAction` | Dump Plugin Descriptors | EXE-INV-008 | LATER |
| `DumpThreads` | Get Thread Dump | EXE-DBG-063 | covered |
| `DumpUastLog` | Dump UAST Tree | EXE-INV-008 | LATER |
| `DumpUastLogByElement` | Dump UAST Tree By Each PsiElement | EXE-INV-008 | LATER |
| `EditBreakpoint` | Edit Breakpoint | EXE-DBG-014 | covered |
| `editRunConfigurations` | Edit Configurations… | EXE-CTX-001 | covered |
| `EvaluateExpression` | Evaluate Expression… | EXE-DBG-090 | covered |
| `excludeFromSuite` | Exclude from Suite | EXE-INV-006 | LATER |
| `excludeFromTestNGSuite` | (set in code) | EXE-INV-006 | LATER |
| `ExecutionTargets` | Targets | EXE-INV-001 | LATER |
| `ExportTestResults` | Export Test Results… | EXE-TEST-056 | covered |
| `ExportThreads` | Export Threads… | EXE-DBG-063 | covered |
| `FindUsedTestData` | Find Unused Test Data | EXE-INV-006 | LATER |
| `ForceRunToCursor` | Force Run to Cursor | EXE-DBG-045 | covered |
| `ForceStepInto` | Force Step Into | EXE-DBG-046 | covered |
| `ForceStepOver` | Force Step Over | EXE-DBG-046 | covered |
| `GenerateCoverageReport` | Generate Coverage Report | EXE-COV-006 | covered |
| `HideCoverage` | Hide Coverage | EXE-COV-006 | covered |
| `Hotswap` | Reload Changed Classes | EXE-DBG-106 | covered |
| `ImportCoverage` | Import External Coverage Report… | EXE-COV-006 | covered |
| `ImportTestsFromFile` | Import Test From File | EXE-TEST-055 | covered |
| `InactiveStopActionPlaceholder` | (set in code) | EXE-INV-001 | LATER |
| `JarApplicationConfigurationType` | JAR Application | EXE-CONF-056 | covered |
| `JavaControlBreak` | (set in code) | EXE-INV-007 | LATER |
| `JavaScratchConfigurationType` | Java Scratch | EXE-CONF-060 | LATER |
| `JavaSoftExit` | (set in code) | EXE-INV-007 | LATER |
| `JumpToColorsAndFonts` | Jump to Colors and Fonts | EXE-INV-008 | LATER |
| `JumpToLastWindow` | Jump to Last Tool Window | EXE-INV-008 | LATER |
| `JUnit.ViewReportAsTestResults` | View Report as Test Results | EXE-INV-006 | LATER |
| `JUnitConfigurationType` | JUnit | EXE-CONF-052 | covered |
| `LanguageServiceStatusBarWidget` | Language service widget | EXE-INV-008 | LATER |
| `ManageTargets` | (set in code) | EXE-CTX-001 | covered |
| `Memory.CalculateRetainedSize` | Calculate Retained Size… | EXE-INV-004 | LATER |
| `MemoryView.EnableTrackingWithClosedWindow` | Enable Tracking With Hidden Memory View | EXE-INV-004 | LATER |
| `MemoryView.JumpToTypeSource` | Jump To Type Source | EXE-INV-004 | LATER |
| `MemoryView.ShowAllocationStackTrace` | Jump To Allocation Position | EXE-INV-004 | LATER |
| `MemoryView.ShowInstances` | Show Instances | EXE-INV-004 | LATER |
| `MemoryView.ShowInstancesFromDebuggerTree` | Show Instances | EXE-INV-004 | LATER |
| `MemoryView.ShowNewInstances` | Show New Instances | EXE-INV-004 | LATER |
| `MemoryView.ShowOnlyTracked` | Show Tracked Classes Only | EXE-INV-004 | LATER |
| `MemoryView.ShowOnlyWithDiff` | Show Non-Zero Diff Only | EXE-INV-004 | LATER |
| `MemoryView.ShowOnlyWithInstances` | Show With Instances Only | EXE-INV-004 | LATER |
| `MemoryView.SwitchUpdateMode` | Update Loaded Classes On Debugger Stop | EXE-INV-004 | LATER |
| `MemoryView.TrackingAction.NewInstancesTracking` | Track New Instances | EXE-INV-004 | LATER |
| `MoreRunToolbarActions` | (set in code) | EXE-INV-001 | LATER |
| `multilaunch.AddExecutableAction` | Add Configuration or Task | EXE-INV-003 | LATER |
| `multilaunch.DeleteExecutableAction` | Delete | EXE-INV-003 | LATER |
| `multilaunch.EditExecutableAction` | Edit… | EXE-INV-003 | LATER |
| `MultiLaunchConfigurationType` | Multi-Launch | EXE-INV-003 | LATER |
| `NewUiRunWidget` | Run Widget | EXE-INV-001 | LATER |
| `Pause` | Pause Program | EXE-DBG-040 | covered |
| `PauseOutput` | (set in code) | EXE-CON-017 | covered |
| `PauseScanningAndIndexing` | Pause Scanning and Indexing | EXE-INV-008 | LATER |
| `PluginConfigurationType` | Plugin (DevKit) | EXE-CONF-060 | LATER |
| `preferences.externalTools` | External Tools | EXE-INV-008 | LATER |
| `preferences.startup.tasks` | Startup Tasks | EXE-CONF-088 | LATER |
| `project.propDebugger` | Debugger settings page | EXE-DBG-100 | covered |
| `QuickEvaluateExpression` | Quick Evaluate Expression | EXE-DBG-093 | covered |
| `RedesignedRunConfigurationSelector` | Select Run/Debug Configuration | EXE-CTX-060 | covered |
| `RemoteConfigurationType` | Remote JVM Debug | EXE-CONF-055 | covered |
| `RemoteServers.ChooseServerDeployment` | Deploy | EXE-INV-002 | LATER |
| `RemoteServers.ChooseServerDeploymentWithDebug` | Debug | EXE-INV-002 | LATER |
| `RemoteServers.ConnectServer` | Connect | EXE-INV-002 | LATER |
| `RemoteServers.DisconnectServer` | Disconnect | EXE-INV-002 | LATER |
| `RemoteServers.EditDeploymentConfig` | Edit Configuration | EXE-INV-002 | LATER |
| `RemoteServers.EditServerConfig` | Edit Configuration | EXE-INV-002 | LATER |
| `RepairDebuggerViewAction` | Repair Debugger View | EXE-INV-007 | LATER |
| `Rerun` | Rerun | EXE-CTX-022 | covered |
| `RerunFailedTests` | Rerun Failed Tests | EXE-TEST-050 | covered |
| `RerunTests` | Rerun Tests | EXE-CTX-028 | covered |
| `Resume` | Resume Program | EXE-DBG-040 | covered |
| `RunAnything` | Run Anything | EXE-INV-005 | LATER |
| `RunConfiguration` | Select Run/Debug Configuration | EXE-CTX-060 | covered |
| `RunConfigurationTemplatesForNewProjects` | (set in code) | EXE-CONF-033 | covered |
| `RunDashboard.AddConfiguration` | Run Configuration… | EXE-INV-002 | LATER |
| `RunDashboard.ClearConsole` | Clear Console | EXE-INV-002 | LATER |
| `RunDashboard.ClearContent` | Clear Content | EXE-INV-002 | LATER |
| `RunDashboard.CopyConfiguration` | Copy Configuration | EXE-INV-002 | LATER |
| `RunDashboard.Debug` | Debug | EXE-INV-002 | LATER |
| `RunDashboard.DoubleClickRun` | Double Click to Run Configuration | EXE-INV-002 | LATER |
| `RunDashboard.EditConfiguration` | Edit Configuration | EXE-INV-002 | LATER |
| `RunDashboard.GroupByStatus` | Status | EXE-INV-002 | LATER |
| `RunDashboard.GroupByType` | Type | EXE-INV-002 | LATER |
| `RunDashboard.GroupConfigurations` | Group Configurations | EXE-INV-002 | LATER |
| `RunDashboard.HideConfiguration` | Hide Configuration | EXE-INV-002 | LATER |
| `RunDashboard.OpenRunningConfigInNewTab` | Open Running Configuration in New Tab | EXE-INV-002 | LATER |
| `RunDashboard.RemoveType` | Remove Configuration Type from Services | EXE-INV-002 | LATER |
| `RunDashboard.RestoreConfiguration` | Restore Configuration | EXE-INV-002 | LATER |
| `RunDashboard.RestoreHiddenConfigurations` | Restore Hidden Configurations | EXE-INV-002 | LATER |
| `RunDashboard.Run` | Run | EXE-INV-002 | LATER |
| `RunDashboard.Stop` | Stop | EXE-INV-002 | LATER |
| `RunDashboard.UngroupConfigurations` | Ungroup Configurations | EXE-INV-002 | LATER |
| `Runner.CloseAllUnpinnedViews` | Close All but Pinned | EXE-CON-005 | covered |
| `Runner.CloseAllViews` | Close All | EXE-CON-005 | covered |
| `Runner.CloseOtherViews` | Close Others | EXE-CON-005 | covered |
| `Runner.CloseView` | Close | EXE-CON-005 | covered |
| `Runner.FocusOnStartup` | Focus on Startup | EXE-CON-005 | covered |
| `Runner.RestoreLayout` | Restore Default Layout | EXE-CON-005 | covered |
| `Runner.ToggleTabLabels` | Show Tab Labels | EXE-CON-005 | covered |
| `Runtime.Targets.Configurable` | Run Targets | EXE-CONF-087 | covered |
| `RunToCursor` | Run to Cursor | EXE-DBG-045 | covered |
| `RunToolbarEditConfigurationAction` | Edit Configuration… | EXE-INV-001 | LATER |
| `RunToolbarMainMultipleStopAction` | Stop | EXE-INV-001 | LATER |
| `RunToolbarMainRunConfigurationsAction` | Add Configuration… | EXE-INV-001 | LATER |
| `RunToolbarMainSlotActive` | Active Process | EXE-INV-001 | LATER |
| `RunToolbarMainSlotInfoAction` | Active Processes | EXE-INV-001 | LATER |
| `RunToolbarMoveToTopAction` | Move Configuration to Top | EXE-INV-001 | LATER |
| `RunToolbarPauseAction` | Pause | EXE-CTX-066 | covered |
| `RunToolbarProcessStartedAction` | Active Process | EXE-INV-001 | LATER |
| `RunToolbarRemoveSlotAction` | Remove | EXE-INV-001 | LATER |
| `RunToolbarRerunAction` | Rerun | EXE-INV-001 | LATER |
| `RunToolbarResumeAction` | Resume | EXE-CTX-066 | covered |
| `RunToolbarRunConfigurationsAction` | Add Configuration… | EXE-INV-001 | LATER |
| `RunToolbarSettingsConfigurableProvider` | Run Widget | EXE-INV-001 | LATER |
| `RunToolbarShowHidePopupAction` | Launch Several Configurations Simultaneously | EXE-INV-001 | LATER |
| `RunToolbarShowToolWindowTab` | Show the process tool window | EXE-INV-001 | LATER |
| `RunToolbarStopAction` | Stop | EXE-INV-001 | LATER |
| `RunToolbarWidgetAction` | Run/Debug Widget | EXE-INV-001 | LATER |
| `SaveTemporaryRunConfiguration` | Save Configuration | EXE-CONF-012 | covered |
| `SegmentedVcsControlAction` | VCS Widget | EXE-INV-001 | LATER |
| `SendEOF` | Send EOF | EXE-CON-051 | covered |
| `Servers.Deploy` | Deploy | EXE-INV-002 | LATER |
| `Servers.DeployWithDebug` | Debug | EXE-INV-002 | LATER |
| `Servers.Undeploy` | Undeploy | EXE-INV-002 | LATER |
| `ServiceView.ConfigureServices` | Configure Services Tool Window | EXE-INV-002 | LATER |
| `ServiceView.GroupByContributor` | Group Services by Type | EXE-INV-002 | LATER |
| `ServiceView.JumpToServices` | Jump to Services | EXE-INV-002 | LATER |
| `ServiceView.OpenEachInNewTab` | Open Each in New Tab | EXE-INV-002 | LATER |
| `ServiceView.OpenInNewTab` | Open in New Tab | EXE-INV-002 | LATER |
| `ServiceView.OpenInToolWindow` | Open in Separate Tool Window | EXE-INV-002 | LATER |
| `ServiceView.SelectActiveService` | Always Select Active Service | EXE-INV-002 | LATER |
| `ServiceView.ShowServices` | Show Services Tree | EXE-INV-002 | LATER |
| `ServiceView.SplitByType` | Open Each Type in New Tab | EXE-INV-002 | LATER |
| `ServiceView.SwitchImplementation` | Turn on split `ServiceView` implementation | EXE-INV-002 | LATER |
| `ShowExecutionPoint` | Show Execution Point | EXE-DBG-048 | covered |
| `ShowLiveRunConfigurations` | Show Running List | EXE-CTX-001 | covered |
| `ShowRecentTests` | Recent Tests | EXE-INV-006 | LATER |
| `SmartStepInto` | Smart Step Into | EXE-DBG-043 | covered |
| `StepInto` | Step Into | EXE-DBG-042 | covered |
| `StepOut` | Step Out | EXE-DBG-044 | covered |
| `StepOver` | Step Over | EXE-DBG-041 | covered |
| `StepOverInstruction` | Step Over Instruction | EXE-INV-007 | LATER |
| `Stop` | Stop | EXE-CTX-024 | covered |
| `StopBackgroundProcesses` | Stop Background Processes… | EXE-CTX-001 | covered |
| `StreamTracerAction` | Trace Current Stream Chain | EXE-INV-004 | LATER |
| `SwitchCoverage` | Show Code Coverage Data | EXE-COV-006 | covered |
| `TestNGConfigurationType` | TestNG | EXE-CONF-054 | covered |
| `ToggleBreakpointEnabled` | Toggle Breakpoint Enabled | EXE-DBG-023 | covered |
| `ToggleFieldBreakpoint` | Toggle Field Watchpoint | EXE-DBG-020 | covered |
| `ToggleLineBreakpoint` | Toggle Line Breakpoint | EXE-DBG-010 | covered |
| `ToggleMethodBreakpoint` | Toggle Method Breakpoint | EXE-DBG-020 | covered |
| `ToggleTemporaryLineBreakpoint` | Toggle Temporary Line Breakpoint | EXE-DBG-012 | covered |
| `UpdateRunningApplication` | Update Running Application | EXE-CTX-029 | covered |
| `ViewBreakpoints` | View Breakpoints… | EXE-DBG-030 | covered |
| `XDebugger.Attach.Dialog.ShowOnlyMyProcessesToggleAction` | (set in code) | EXE-INV-007 | LATER |
| `XDebugger.AttachToProcess` | (set in code) | EXE-DBG-102 | covered |
| `XDebugger.CompareValueWithClipboard` | Compare Value with Clipboard | EXE-DBG-072 | covered |
| `XDebugger.CopyName` | Copy Name | EXE-DBG-072 | covered |
| `XDebugger.CopyValue` | Copy Value | EXE-DBG-072 | covered |
| `XDebugger.CopyWatch` | Duplicate Watch | EXE-DBG-076 | covered |
| `XDebugger.DisableAllButThisBreakpoint` | Disable other breakpoints | EXE-DBG-023 | covered |
| `XDebugger.EditWatch` | Edit… | EXE-DBG-076 | covered |
| `XDebugger.Hotswap.Modified.Files` | Compile and Reload Modified Files | EXE-DBG-106 | covered |
| `XDebugger.Inline` | Show Variable Values in Editor | EXE-DBG-074 | covered |
| `XDebugger.Inspect` | Inspect… | EXE-DBG-072 | covered |
| `XDebugger.JumpToSource` | Jump To Source | EXE-DBG-072 | covered |
| `XDebugger.JumpToTypeSource` | Jump To Type Source | EXE-DBG-072 | covered |
| `XDebugger.MoveWatchDown` | Move Watch Down | EXE-DBG-076 | covered |
| `XDebugger.MoveWatchUp` | Move Watch Up | EXE-DBG-076 | covered |
| `XDebugger.MuteBreakpoints` | Mute Breakpoints | EXE-DBG-021 | covered |
| `XDebugger.NewWatch` | New Watch… | EXE-DBG-076 | covered |
| `XDebugger.PauseWatch` | Pause Watch | EXE-DBG-076 | covered |
| `XDebugger.PinToTop` | Pin to Top | EXE-DBG-078 | covered |
| `XDebugger.PreviewTab` | Open Files in Preview Tab | EXE-DBG-074 | covered |
| `XDebugger.RemoveAllButThisBreakpoint` | Remove other breakpoints | EXE-DBG-023 | covered |
| `XDebugger.RemoveAllWatches` | Remove All Watches | EXE-DBG-076 | covered |
| `XDebugger.RemoveWatch` | Remove Watch | EXE-DBG-076 | covered |
| `XDebugger.SeparateWatches` | Separate Watches | EXE-DBG-078 | covered |
| `XDebugger.SetValue` | Set Value… | EXE-DBG-073 | covered |
| `XDebugger.Show.Breakpoints.Over.Line.Numbers` | Breakpoints Over Line Numbers | EXE-DBG-012 | covered |
| `XDebugger.ToggleEvaluateExpressionField` | Show Evaluate Expression Field | EXE-DBG-094 | covered |
| `XDebugger.ToggleSortValues` | Sort Variables Alphabetically | EXE-DBG-074 | covered |
| `XDebugger.UnmuteOnStop` | Unmute Breakpoints on Session Finish | EXE-DBG-005 | covered |
| `XsltRunConfigType` | XSLT | EXE-CONF-060 | LATER |
