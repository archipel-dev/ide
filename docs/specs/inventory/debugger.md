# Debugger

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (133)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ActivateDebugToolWindow` | Debug | `ActivateToolWindowActions` | `alt 5` | MISSING | `platform/xdebugger-impl` |
| `AddBreakpoint` | Add Breakpoint | `XDebugger.Hover.Breakpoint.Context.Menu` |  | MISSING | `platform/xdebugger-impl` |
| `AddConditionalBreakpoint` | Add Conditional Breakpoint… | `XDebugger.Hover.Breakpoint.Context.Menu` |  | MISSING | `platform/xdebugger-impl` |
| `AddLoggingBreakpoint` | Add Logging Breakpoint… | `XDebugger.Hover.Breakpoint.Context.Menu` |  | MISSING | `platform/xdebugger-impl` |
| `AttachProject` | Attach Project… | `FileOpenGroup` |  | MISSING | `platform/platform-impl` |
| `DebugBuildProcess` | Debug Build Process |  |  | MISSING | `java/java-backend` |
| `DebugListen` | Throw Debug Exception | `Internal.Errors` |  | MISSING | `java/debugger` |
| `Debugger.AddInlineWatch` | Add Inline Watch | `XDebugger.Actions`, `EditorPopupMenuDebug` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.AddSteppingFilter` | Add Stepping Filter… | `JavaDebuggerActions`, `XDebugger.Frames.Tree.Popup` |  | MISSING | `java/debugger` |
| `Debugger.AddToWatch` | Add to Watches | `XDebugger.Actions`, `EditorPopupMenuDebug`, `XDebugger.ValueGroup` (+1) |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.AdjustArrayRange` | Adjust Range… | `JavaDebuggerActions`, `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `Debugger.AsyncStacks` | Async Stack Traces | `XDebugger.Frames.Tree.Popup` |  | MISSING | `java/debugger` |
| `Debugger.AutoRenderer` | Auto |  |  | MISSING | `java/debugger` |
| `Debugger.CallTracer` | Trace Calls | `JavaDebuggerActions`, `XDebugger.ToolWindow.TopToolbar` |  | MISSING | `java/debugger` |
| `Debugger.CopyStack` | Copy Stack | `XDebugger.Actions`, `XDebugger.Frames.Tree.Popup` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.CreateRenderer` | Create… | `XDebugger.Watches.Inline.Popup` |  | MISSING | `java/debugger` |
| `Debugger.CustomizeContextView` | Customize Data Views… | `XDebugger.Variables.Tree.Popup`, `XDebugger.Watches.Tree.Popup` |  | MISSING | `java/debugger` |
| `Debugger.CustomizeThreadsView` | Customize Threads View… | `JavaDebuggerActions`, `XDebugger.Frames.Tree.Popup`, `XDebugger.Threads.View.Popup` |  | MISSING | `java/debugger` |
| `Debugger.EditArrayFilter` | Edit… | `JavaDebuggerActions`, `XDebugger.Variables.Tree.Popup`, `XDebugger.Watches.Tree.Popup` | as `XDebugger.SetValue` | MISSING | `java/debugger` |
| `Debugger.EditCustomField` | Edit… | `JavaDebuggerActions`, `XDebugger.Variables.Tree.Popup`, `XDebugger.Watches.Tree.Popup` | as `XDebugger.SetValue` | MISSING | `java/debugger` |
| `Debugger.EvaluateInConsole` | Evaluate in Console | `XDebugger.Actions`, `EditorPopupMenuDebug` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.FilterArray` | Filter… | `JavaDebuggerActions`, `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `Debugger.FocusOnBreakpoint` | Focus On Breakpoint | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.FocusOnFinish` | Focus On Finish | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ForceEarlyReturn` | Force Return | `JavaDebuggerActions`, `XDebugger.Frames.Tree.Popup`, `DebuggingActionsGroup` (+1) |  | MISSING | `java/debugger` |
| `Debugger.FreezeActiveThreadAction` | Suspend Thread | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.FreezeInactiveThreadsAction` | Suspend Other Threads | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.FreezeInactiveThreadsAmongSelected` | Suspend Inactive Threads in a Group | `XDebugger.Actions`, `XDebugger.ParallelStacks.Popup.FreezeThreadGroup` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.FreezeSelectedThreads` | Suspend Threads | `XDebugger.Actions`, `XDebugger.ParallelStacks.Popup.FreezeThreadGroup` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.MarkObject` | Mark Object… | `XDebugger.Actions`, `XDebugger.ValueGroup` | as `ToggleBookmark` | MISSING | `platform/xdebugger-impl` |
| `Debugger.MuteRenderers` | Mute Renderers | `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `Debugger.NewCustomField` | New Class Level Watch… | `JavaDebuggerActions`, `XDebugger.Variables.Tree.Popup`, `XDebugger.Watches.Tree.Popup` |  | MISSING | `java/debugger` |
| `Debugger.PopFrame` | Reset Frame | `XDebugger.Threads.View.Popup`, `XDebugger.ToolWindow.TopToolbar3.Extra`, `XDebugger.Frames.Tree.Popup` (+1) |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ReloadFile` | Compile And Reload File | `JavaDebuggerActions` |  | MISSING | `java/debugger` |
| `Debugger.RemoveAllBreakpoints` | Remove All Breakpoints | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.RemoveAllBreakpointsInFile` | Remove All Breakpoints In The Current File | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.RemoveArrayFilter` | Remove Filter | `JavaDebuggerActions`, `XDebugger.Variables.Tree.Popup`, `XDebugger.Watches.Tree.Popup` | as `XDebugger.RemoveWatch` | MISSING | `java/debugger` |
| `Debugger.RemoveCustomField` | Remove Watch | `JavaDebuggerActions`, `XDebugger.Variables.Tree.Popup`, `XDebugger.Watches.Tree.Popup` | as `XDebugger.RemoveWatch` | MISSING | `java/debugger` |
| `Debugger.RestoreBreakpoint` | Restore Breakpoint | `BreakpointActionsGroup` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ShowCollectionHistory` | Show Collection History | `JavaDebuggerActions`, `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `Debugger.ShowLibraryFrames` | Show/Hide Library Frames | `XDebugger.Actions`, `XDebugger.Frames.Tree.Popup`, `XDebugger.Frames.TopToolbar` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ShowReferring` | Show Referring Objects… | `XDebugger.Actions`, `XDebugger.ValueGroup` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ShowTypes` | Show types | `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `Debugger.StepOutOfBlock` | Step Out of Code Block | `DebuggingActionsGroup`, `XDebugger.ToolWindow.TopToolbar3.Extra` |  | MISSING | `java/debugger` |
| `Debugger.SwitchToTheNextContext` | Switch to the Next Stopped Breakpoint | `JavaDebuggerActions` |  | MISSING | `java/debugger` |
| `Debugger.ThawActiveThreadAction` | Resume Thread | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ThawAllThreadsAction` | Resume All Threads | `XDebugger.Actions`, `XDebugger.ParallelStacks.Popup.FreezeThreadGroup` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ThawSelectedThreads` | Resume Threads | `XDebugger.Actions`, `XDebugger.ParallelStacks.Popup.FreezeThreadGroup` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ThrowException` | Throw Exception | `JavaDebuggerActions`, `XDebugger.Frames.Tree.Popup`, `DebuggingActionsGroup` (+1) |  | MISSING | `java/debugger` |
| `Debugger.Tree.EvaluateInConsole` | Evaluate In Console | `XDebugger.Actions`, `XDebugger.ValueGroup` |  | MISSING | `platform/xdebugger-impl` |
| `Debugger.ViewText` | View Text | `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `Debugger.XThreadsView.FreezeThread` | Freeze | `XDebugger.Threads.View.Popup` |  | MISSING | `java/debugger` |
| `Debugger.XThreadsView.InterruptThread` | Interrupt | `XDebugger.Threads.View.Popup` |  | MISSING | `java/debugger` |
| `Debugger.XThreadsView.ResumeThread` | Resume | `XDebugger.Threads.View.Popup` |  | MISSING | `java/debugger` |
| `DisableStreamDebuggerInlay` | Show Inline Hint | `StreamDebuggerInlayPopup` |  | MISSING | `plugins/stream-debugger` |
| `DumpLibraryUsageStatistics` | Dump Library Usage Statistics |  |  | MISSING | `java/java-impl` |
| `DumpLookupElementWeights` | Dump Lookup Element Weights to Log | `MaintenanceGroup` | `control alt shift W` | MISSING | `platform/lang-impl` |
| `DumpPluginDescriptorsAction` | Dump Plugin Descriptors | `Internal.Dump` |  | MISSING | `platform/platform-impl` |
| `DumpThreads` | Get Thread Dump | `DebuggingActionsGroup`, `XDebugger.ToolWindow.TopToolbar3.Extra` |  | MISSING | `java/debugger` |
| `DumpUastLog` | Dump UAST Tree | `UastInternal` |  | MISSING | `jvm/jvm-analysis-impl` |
| `DumpUastLogByElement` | Dump UAST Tree By Each PsiElement | `UastInternal` |  | MISSING | `jvm/jvm-analysis-impl` |
| `EditBreakpoint` | Edit Breakpoint |  | `control shift F8` | MISSING | `platform/xdebugger-impl` |
| `EvaluateExpression` | Evaluate Expression… | `XDebugger.ToolWindow.TopToolbar`, `XDebugger.ToolWindow.TopToolbar3.Extra`, `XDebugger.ValueGroup` (+3) | `alt F8` | MISSING | `platform/xdebugger-impl` |
| `ExportThreads` | Export Threads… | `XDebugger.Frames.Tree.Popup`, `XDebugger.Threads.View.Popup`, `XDebugger.Actions` |  | MISSING | `java/debugger` |
| `ForceRunToCursor` | Force Run to Cursor | `XDebugger.ToolWindow.TopToolbar3.Extra`, `DebuggingActionsGroup`, `EditorPopupMenuDebug` | `control alt F9` | MISSING | `platform/xdebugger-impl` |
| `ForceStepInto` | Force Step Into | `XDebugger.ToolWindow.TopToolbar3.Extra`, `TouchBarDebug.ForceStepButtons`, `DebuggingActionsGroup` | `alt shift F7` | MISSING | `platform/xdebugger-impl` |
| `ForceStepOver` | Force Step Over | `XDebugger.ToolWindow.TopToolbar3.Extra`, `TouchBarDebug.ForceStepButtons`, `DebuggingActionsGroup` | `alt shift F8` | MISSING | `platform/xdebugger-impl` |
| `Hotswap` | Reload Changed Classes |  |  | MISSING | `java/debugger` |
| `JumpToColorsAndFonts` | Jump to Colors and Fonts |  |  | MISSING | `platform/lang-impl` |
| `JumpToLastWindow` | Jump to Last Tool Window | `ActiveToolwindowGroup` | `F12` | MISSING | `platform/platform-impl` |
| `Memory.CalculateRetainedSize` | Calculate Retained Size… | `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `MemoryView.EnableTrackingWithClosedWindow` | Enable Tracking With Hidden Memory View | `MemoryView.SettingsPopupActionGroup` |  | MISSING | `java/debugger` |
| `MemoryView.JumpToTypeSource` | Jump To Type Source | `MemoryView.ClassesPopupActionGroup` |  | MISSING | `java/debugger` |
| `MemoryView.ShowAllocationStackTrace` | Jump To Allocation Position | `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `MemoryView.ShowInstances` | Show Instances | `MemoryView.ClassesPopupActionGroup` |  | MISSING | `java/debugger` |
| `MemoryView.ShowInstancesFromDebuggerTree` | Show Instances | `XDebugger.ValueGroup` |  | MISSING | `java/debugger` |
| `MemoryView.ShowNewInstances` | Show New Instances | `MemoryView.ClassesPopupActionGroup` |  | MISSING | `java/debugger` |
| `MemoryView.ShowOnlyTracked` | Show Tracked Classes Only | `MemoryView.SettingsPopupActionGroup` |  | MISSING | `java/debugger` |
| `MemoryView.ShowOnlyWithDiff` | Show Non-Zero Diff Only | `MemoryView.SettingsPopupActionGroup` |  | MISSING | `platform/xdebugger-impl` |
| `MemoryView.ShowOnlyWithInstances` | Show With Instances Only | `MemoryView.SettingsPopupActionGroup` |  | MISSING | `java/debugger` |
| `MemoryView.SwitchUpdateMode` | Update Loaded Classes On Debugger Stop | `MemoryView.SettingsPopupActionGroup` |  | MISSING | `platform/xdebugger-impl` |
| `MemoryView.TrackingAction.NewInstancesTracking` | Track New Instances | `MemoryView.ClassesPopupActionGroup` |  | MISSING | `java/debugger` |
| `Pause` | Pause Program | `Pause.Ref` |  | MISSING | `platform/xdebugger-impl` |
| `PauseScanningAndIndexing` | Pause Scanning and Indexing |  |  | MISSING | `platform/lang-impl` |
| `QuickEvaluateExpression` | Quick Evaluate Expression |  | `control alt F8`, `mouse alt button1` | MISSING | `platform/xdebugger-impl` |
| `RepairDebuggerViewAction` | Repair Debugger View | `DebuggingActionsGroup` |  | MISSING | `java/debugger` |
| `Resume` | Resume Program | `Resume.Ref` | `F9` | MISSING | `platform/xdebugger-impl` |
| `RunToCursor` | Run to Cursor | `XDebugger.ToolWindow.TopToolbar`, `XDebugger.ToolWindow.TopToolbar3.Extra`, `TouchBarDebug.ForceStepButtons` (+2) | `alt F9` | MISSING | `platform/xdebugger-impl` |
| `RunToolbarPauseAction` | Pause | `RunToolbarDebuggerAdditionalActions` |  | MISSING | `platform/xdebugger-impl` |
| `RunToolbarResumeAction` | Resume | `RunToolbarDebuggerAdditionalActions` |  | MISSING | `platform/xdebugger-impl` |
| `ShowExecutionPoint` | Show Execution Point | `XDebugger.ToolWindow.TopToolbar`, `XDebugger.ToolWindow.TopToolbar3.Extra`, `DebuggingActionsGroup` | `alt F10` | MISSING | `platform/xdebugger-impl` |
| `SmartStepInto` | Smart Step Into | `XDebugger.ToolWindow.TopToolbar3.Extra`, `TouchBarDebug.ForceStepButtons`, `DebuggingActionsGroup` | `shift F7` | MISSING | `platform/xdebugger-impl` |
| `StepInto` | Step Into | `XDebugger.ToolWindow.TopToolbar`, `XDebugger.ToolWindow.TopToolbar3`, `TouchBarDebug.StepButtons` (+1) | `F7` | MISSING | `platform/xdebugger-impl` |
| `StepOut` | Step Out | `XDebugger.ToolWindow.TopToolbar`, `XDebugger.ToolWindow.TopToolbar3`, `TouchBarDebug.StepButtons` (+1) | `shift F8` | MISSING | `platform/xdebugger-impl` |
| `StepOver` | Step Over | `StepOver.Ref` | `F8` | MISSING | `platform/xdebugger-impl` |
| `StepOverInstruction` | Step Over Instruction | `DebuggingActionsGroup` |  | MISSING | `java/debugger` |
| `StreamTracerAction` | Trace Current Stream Chain | `XDebugger.ToolWindow.TopToolbar`, `XDebugger.ToolWindow.TopToolbar3.Extra` |  | MISSING | `plugins/stream-debugger` |
| `ToggleBreakpointEnabled` | Toggle Breakpoint Enabled | `BreakpointActionsGroup` |  | MISSING | `platform/xdebugger-impl` |
| `ToggleFieldBreakpoint` | Toggle Field Watchpoint | `ToggleBreakpointAction`, `XDebugger.ValueGroup`, `JavaCompileGroup` (+1) |  | MISSING | `java/debugger` |
| `ToggleLineBreakpoint` | Toggle Line Breakpoint | `TouchBarDefault_cmd.Debug`, `BreakpointActionsGroup` | `control F8` | MISSING | `platform/xdebugger-impl` |
| `ToggleMethodBreakpoint` | Toggle Method Breakpoint | `ToggleBreakpointAction`, `JavaCompileGroup` |  | MISSING | `java/debugger` |
| `ToggleTemporaryLineBreakpoint` | Toggle Temporary Line Breakpoint | `BreakpointActionsGroup` | `control shift alt F8` | MISSING | `platform/xdebugger-impl` |
| `ViewBreakpoints` | View Breakpoints… | `XDebugger.ToolWindow.TopToolbar3`, `XDebugger.ToolWindow.LeftToolbar`, `TouchBarDebug_alt` (+1) | `control shift F8` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.Attach.Dialog.ShowOnlyMyProcessesToggleAction` | (set in code) | `XDebugger.Attach.Dialog.Settings` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.AttachToProcess` | (set in code) | `XDebugger.AttachGroup` | `control alt F5` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.CompareValueWithClipboard` | Compare Value with Clipboard | `XDebugger.Actions`, `XDebugger.ValueGroup` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.CopyName` | Copy Name | `XDebugger.Actions`, `XDebugger.ValueGroup` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.CopyValue` | Copy Value | `XDebugger.Actions`, `XDebugger.ValueGroup` | as `$Copy` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.CopyWatch` | Duplicate Watch | `XDebugger.Actions`, `XDebugger.Watches.Tree.Toolbar` | as `EditorDuplicate` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.DisableAllButThisBreakpoint` | Disable other breakpoints | `XDebugger.Actions`, `EditorGutterPopupMenu` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.EditWatch` | Edit… | `XDebugger.Actions`, `XDebugger.Watches.Tree.Popup` | as `XDebugger.SetValue` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.Hotswap.Modified.Files` | Compile and Reload Modified Files | `DebugReloadGroup`, `EditorPopupMenuDebugHotSwap` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.Inline` | Show Variable Values in Editor | `XDebugger.Settings` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.Inspect` | Inspect… | `XDebugger.Actions`, `XDebugger.ValueGroup` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.JumpToSource` | Jump To Source | `XDebugger.Actions`, `XDebugger.ValueGroup` | as `EditSource` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.JumpToTypeSource` | Jump To Type Source | `XDebugger.Actions`, `XDebugger.ValueGroup` | `shift F4` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.MoveWatchDown` | Move Watch Down | `XDebugger.Actions`, `XDebugger.Watches.Tree.Toolbar` | as `MoveLineDown` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.MoveWatchUp` | Move Watch Up | `XDebugger.Actions`, `XDebugger.Watches.Tree.Toolbar` | as `MoveLineUp` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.MuteBreakpoints` | Mute Breakpoints | `XDebugger.Actions`, `XDebugger.ToolWindow.TopToolbar3`, `XDebugger.ToolWindow.LeftToolbar` (+1) |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.NewWatch` | New Watch… | `XDebugger.Actions`, `XDebugger.Watches.Tree.Popup`, `XDebugger.Watches.Tree.Toolbar` | `INSERT` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.PauseWatch` | Pause Watch | `XDebugger.Actions`, `XDebugger.Watches.Tree.Popup` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.PinToTop` | Pin to Top | `XDebugger.ValueGroup` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.PreviewTab` | Open Files in Preview Tab | `XDebugger.Settings` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.RemoveAllButThisBreakpoint` | Remove other breakpoints | `XDebugger.Actions`, `EditorGutterPopupMenu` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.RemoveAllWatches` | Remove All Watches | `XDebugger.Actions`, `XDebugger.Watches.Tree.Popup` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.RemoveWatch` | Remove Watch | `XDebugger.Actions`, `XDebugger.Watches.Tree.Popup`, `XDebugger.Watches.Tree.Toolbar` | as `$Delete` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.SeparateWatches` | Separate Watches | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.SetValue` | Set Value… | `XDebugger.Actions`, `XDebugger.ValueGroup` | `F2` | MISSING | `platform/xdebugger-impl` |
| `XDebugger.Show.Breakpoints.Over.Line.Numbers` | Breakpoints Over Line Numbers | `EditorGutterPopupMenu.Appearance` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.ToggleEvaluateExpressionField` | Show Evaluate Expression Field | `XDebugger.Actions` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.ToggleSortValues` | Sort Variables Alphabetically | `XDebugger.Actions`, `XDebugger.Settings` |  | MISSING | `platform/xdebugger-impl` |
| `XDebugger.UnmuteOnStop` | Unmute Breakpoints on Session Finish | `XDebugger.Settings` |  | MISSING | `platform/xdebugger-impl` |
| `com.intellij.debugger.feedback.KotlinDebuggerFeedbackTestAction` | (set in code) |  |  | MISSING | `java/debugger` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `project.propDebugger` | Debugger | build |  | application | MISSING |
