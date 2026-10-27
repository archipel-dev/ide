# Plugin development, internal and diagnostic tools

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (278)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ABExperiment.Demo` | A/B Experiment Inspector |  |  | MISSING | `plugins/devkit` |
| `AddAfterLineEndInlayInternal` | Add After Line End Inlay… | `Internal.Editor` |  | MISSING | `plugins/dev` |
| `AddComposeToIJModule` | Enable Compose UI for Module | `ComposeToolsGroup` |  | MISSING | `plugins/devkit` |
| `AddGroupToTestSchemeAction` | Add Group to Events Test Scheme | `FeatureUsage.Internal` |  | MISSING | `platform/statistics` |
| `AddInlayBlockInternal` | Add Block Inlay… | `Internal.Editor` |  | MISSING | `plugins/dev` |
| `AddInlayInternal` | Add Inlay… | `Internal.Editor` |  | MISSING | `plugins/dev` |
| `AddManyTestProcesses` | (set in code) | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `AddRandomNamesTestProcessAction` | (set in code) | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `AddTestProcessAction` | (set in code) | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `AnalyzeUnloadablePlugins` | Analyze Unloadable Plugins | `Internal.DevKit` |  | MISSING | `plugins/devkit` |
| `AnimationPanelTestAction` | Show Animation Panel | `Internal.UI.Animation` |  | MISSING | `plugins/devkit` |
| `AnimatorTestAction` | Test Animator | `Internal.UI.Animation` |  | MISSING | `plugins/devkit` |
| `ApplyFixedWindowSize` | Apply Specific Size | `Internal.PMM` |  | MISSING | `platform/platform-impl` |
| `BrowseSpecialPaths` | Special Files and Folders… | `HelpDiagnosticTools`, `WelcomeScreen.Options` |  | MISSING | `platform/platform-impl` |
| `CallSaul` | Repair IDE |  |  | MISSING | `platform/platform-impl` |
| `CaptureMemorySnapShot` | Capture Memory Snapshot | `DiagnosticGroup`, `HelpMenu` |  | MISSING | `plugins/performanceTesting` |
| `CheckClassLoading` | Check Class Loading | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `CheckVfsSanity` | Run VFS Health Check | `Internal.VFS` |  | MISSING | `platform/platform-impl` |
| `ChooseRuntime` | Choose Boot Java Runtime for the IDE… |  |  | MISSING | `platform/lang-impl` |
| `CleanupEventsTestSchemeAction` | Clean Up Events Test Scheme |  |  | MISSING | `platform/statistics` |
| `CollectFUStatisticsAction` | Collect Statistics from Collector… | `FeatureUsage.Internal` |  | MISSING | `platform/statistics` |
| `CollectTroubleshootingInformation` | Collect Troubleshooting Information | `HelpDiagnosticTools` |  | MISSING | `plugins/dev` |
| `CollectZippedLogs` | Collect Logs and Diagnostic Data | `HelpMenu`, `WelcomeScreen.Configure`, `WelcomeScreen.Options` |  | MISSING | `platform/platform-impl` |
| `ColorBlindnessTest` | Test ColorBlindness | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `CompareKeymaps` | Compare Keymaps… | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `CompletionQualityStats` | Completion Quality Statistics | `Internal` |  | codeinsight/06-completion.md CI-CMP | `plugins/dev` |
| `ComponentPanelTestAction` | Show Standard Panels | `Internal.UI.Ex` |  | MISSING | `plugins/devkit` |
| `ComposePerformanceDemo` | Compose UI Performance Demo | `ComposeToolsGroup` |  | MISSING | `plugins/devkit` |
| `ComposePreviewRefresh` | Refresh | `ComposePreviewActions.Title` |  | MISSING | `plugins/devkit` |
| `ComposePreviewRefreshAuto` | Refresh Automatically | `ComposePreviewActions.Title` |  | MISSING | `plugins/devkit` |
| `ComposeSandbox` | Compose Prototyping Sandbox | `ComposeToolsGroup` |  | MISSING | `plugins/devkit` |
| `ComposeShowcase` | Compose UI Showcase |  |  | MISSING | `plugins/devkit` |
| `ComposeVerboseStackTrace` | Compose Verbose Stack Traces |  |  | MISSING | `plugins/devkit` |
| `ComputeVFStatistics` | (set in code) | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `ConfigureDefaultSize` | Configure Default Size… | `Internal.PMM` |  | MISSING | `platform/platform-impl` |
| `ConfigureEventsSchemeFileAction` | Configure Events Scheme File |  |  | MISSING | `platform/statistics` |
| `ConfigureIntelliJMonster` | Configure IntelliJ Language Server |  |  | MISSING | `plugins/lsp.ui` |
| `ConfigureSoftWraps` | Configure Soft Wraps… | `EditorGutterPopupMenu` |  | MISSING | `platform/lang-impl` |
| `CountVfsFileChildren` | Count VFS File Children | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `CreateAllServicesAndExtensions` | Create All Services and Extensions |  |  | MISSING | `platform/platform-impl` |
| `DataSharingOptions` | Data Sharing Options… | `Internal` |  | MISSING | `platform/platform-impl` |
| `Debug.PsiViewer` | Show PSI Structure… | `XDebugger.ValueGroup` |  | MISSING | `plugins/dev` |
| `Debug.PsiViewer.Toggle.UI` | Use Dialog For PSI Debugger |  |  | MISSING | `plugins/dev` |
| `DetectProjectLeaks` | Detect Memory Leak (Project or Editor) | `HelpDiagnosticTools`, `DetectProjectLeaks.Internal` |  | MISSING | `plugins/dev` |
| `DevKit.AnalyzePersistentSyntaxTreeOverhead` | Analyze Persistent Syntax Tree Overhead… | `Internal.DevKit` |  | MISSING | `plugins/devkit` |
| `DevKit.ApplyTheme` | Preview Theme | `DevKit.ThemeEditorToolbar` |  | MISSING | `plugins/devkit` |
| `DevKit.Bootstrap` | Create IntelliJ IDEA Plugin… | `ToolsMenu` |  | MISSING | `java/java-backend` |
| `DevKit.Debugger.Disable.IDE.State` | Do Not Show IDE State | `XDebugger.Watches.Tree.Popup` | as `$Delete` | MISSING | `plugins/devkit` |
| `DevKit.EditThemeEditorColorScheme` | Edit Editor Color Scheme | `DevKit.ThemeEditorToolbar` |  | MISSING | `plugins/devkit` |
| `DevKit.NewAction` | Action | `DevkitNewActions` |  | MISSING | `plugins/devkit` |
| `DevKit.NewMessageBundle` | Message Bundle | `DevkitObsoleteNewActions` |  | MISSING | `plugins/devkit` |
| `DevKit.NewMessageBundleObject` | Message Bundle | `DevkitNewActions` |  | MISSING | `plugins/devkit` |
| `DevKit.NewService` | Service | `DevkitNewActions` |  | MISSING | `plugins/devkit` |
| `DevKit.NewStartupActivity` | Startup Activity | `DevkitNewActions` |  | MISSING | `plugins/devkit` |
| `DevKit.NewState` | Persistent State | `DevkitNewActions` |  | MISSING | `plugins/devkit` |
| `DevKit.NewTheme` | Theme | `NewGroup` |  | MISSING | `plugins/devkit` |
| `DevKit.OpenThemeReferenceDocs` | Open UI Themes Reference Documentation | `DevKit.ThemeEditorToolbar` |  | MISSING | `plugins/devkit` |
| `DevKit.RollbackTheme` | Stop Theme Preview | `DevKit.ThemeEditorToolbar` |  | MISSING | `plugins/devkit` |
| `DropAnError` | Drop an Error | `Internal.Errors` |  | MISSING | `plugins/dev` |
| `DropAnErrorWithAttachments` | Drop an Error with Attachments | `Internal.Errors` |  | MISSING | `plugins/dev` |
| `DropAnOutOfMemoryError` | Drop an OutOfMemoryError | `Internal.Errors` |  | MISSING | `plugins/dev` |
| `DropPluginError` | Drop an Error in a Random Plugin | `Internal.Errors` |  | MISSING | `plugins/dev` |
| `DumbMode` | Dumb Mode | `Internal.Ex` |  | platform/05-caching-and-modification-tracking.md PLAT-CACHE-074 | `platform/platform-impl` |
| `DumbModeTremble` | Tremble Dumb Mode | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `DumpCodeStyleCache` | Dump Code Style Cache | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpEditorStateInternal` | Dump Editor State | `Internal.Editor` |  | MISSING | `plugins/dev` |
| `DumpExtensions` | Dump Extensions | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpFocusableComponentHierarchyAction` | Dump Focusable Component Hierarchy |  |  | MISSING | `plugins/devkit` |
| `DumpFormattingModel` | Dump Formatting Model | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpIElementTypes` | Dump IElementType Registry | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpInspectionDescriptions` | Dump Inspection Descriptions | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpIntentionsDescriptions` | Dump Intentions Descriptions | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpLoadedInspections` | Dump Initialized Inspections | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpRunConfigurationTypes` | Dump Run Configuration Types | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpScreenConfiguration` | Dump Screen Configuration | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `DumpVfsInfoForExcludedFiles` | Dump VFS Content for Files Under Excluded Roots | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `DumpVfsInfoForFsRoot` | Dump VFS Content For… | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `DumpWatchedRoots` | Show Watched VFS Roots | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `EditCustomProperties` | Edit Custom Properties… | `HelpMenu`, `WelcomeScreen.Configure`, `WelcomeScreen.Options` |  | MISSING | `platform/platform-impl` |
| `EditCustomVmOptions` | Edit Custom VM Options… | `HelpMenu`, `WelcomeScreen.Configure`, `WelcomeScreen.Options` |  | MISSING | `platform/platform-impl` |
| `EditTestSchemeAction` | Edit Events Test Scheme |  |  | MISSING | `platform/statistics` |
| `EditorAddCustomWrap` | Add Custom Wrap | `Internal.Editor` |  | MISSING | `plugins/devkit` |
| `EditorRemoveCustomWrapsAtCaret` | Remove Custom Wraps at Caret | `Internal.Editor` |  | MISSING | `plugins/devkit` |
| `EditorRenderingBenchmarkAction` | Editor Rendering Benchmark | `Internal.Editor` |  | MISSING | `plugins/devkit` |
| `EditorSkeletonFadeInSheet` | Dump Editor Skeleton Fade-in Sheet… | `Internal.UI.Animation` |  | MISSING | `plugins/devkit` |
| `EnableMetalRendering` | Metal Rendering Pipeline | `Internal.UI` |  | MISSING | `plugins/dev` |
| `EnableWayland` | Native Wayland Support | `Internal.UI` |  | MISSING | `plugins/dev` |
| `ExceptionShowCaseAction` | Show Exception | `Internal.Errors` |  | MISSING | `plugins/devkit` |
| `FixLineSeparators` | Fix Line Separators | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `FocusDebugger` | Start Focus Debugger | `Internal.UI.Focus` |  | MISSING | `plugins/devkit` |
| `FocusTracer` | Start Focus Tracing | `Internal.UI.Focus` |  | MISSING | `plugins/devkit` |
| `ForceIndexRebuild` | Reindex Everything |  |  | MISSING | `platform/lang-impl` |
| `ForceIndexRescanning` | Synchronize Indexes |  |  | MISSING | `platform/lang-impl` |
| `FrequentBackgroundReadAction` | Run Frequent Background Read Action | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `FrequentBackgroundWriteAction` | Run Frequent Background Write Action | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `GenerateEventsScheme` | Show FUS Collectors Scheme |  |  | MISSING | `platform/statistics` |
| `GenerateExternalization` | Generate Externalization | `GenerateGroup` |  | MISSING | `plugins/devkit` |
| `GrayFilterConfig` | GrayFilter | `Internal.UI.Ex` |  | MISSING | `plugins/devkit` |
| `GridLayoutTestAction` | Show GridLayout Test | `Internal.UI.KotlinUiDsl` |  | MISSING | `plugins/devkit` |
| `HidpiInfo` | Show HiDPI Info |  |  | MISSING | `platform/platform-impl` |
| `IconScaleTestAction` | Show Icons Scale Test Dialog | `Internal.UI.Icons` |  | MISSING | `plugins/devkit` |
| `IconsLoadTime` | Log Icons Load Time |  |  | MISSING | `plugins/dev` |
| `IdeScriptingConsole` | IDE Scripting Console | `MaintenanceGroup` |  | MISSING | `platform/platform-impl` |
| `ImageDuplicates` | Find Image Duplicates | `Internal` |  | MISSING | `plugins/dev` |
| `ImportSettingDialogAction` | [INTERNAL] 3 Transfer: Show Import Settings Dialog |  |  | MISSING | `plugins/ide-startup` |
| `IoFlooding` | Start IO Flooding | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `JBCefOsrMeasureFps` | JBCef OSR Measure FPS | `Internal.UI` |  | MISSING | `platform/platform-impl` |
| `JBCefTestApp` | JBCef TestApp | `Internal.UI.Demos` |  | MISSING | `platform/platform-impl` |
| `JBTabsDemo` | JBTabs Demo | `Internal.UI.Demos` |  | MISSING | `plugins/devkit` |
| `JEditorPaneDemoAction` | HTML Rendering Playground | `Internal.UI.Demos` |  | MISSING | `plugins/devkit` |
| `JewelActionSystemTest` | Jewel Action System Test |  |  | MISSING | `plugins/devkit` |
| `JewelComponentShowcaseDialog` | Jewel Components Showcase | `ComposeToolsGroup` |  | MISSING | `plugins/devkit` |
| `JewelCustomPopupRenderer` | Use Jewel Custom Popup Renderer |  |  | MISSING | `plugins/devkit` |
| `JewelToolwindow` | Jewel Tool Window | `ComposeToolsGroup` |  | MISSING | `plugins/devkit` |
| `JewelWizardDialog` | Jewel Wizard Dialog | `ComposeToolsGroup` |  | MISSING | `plugins/devkit` |
| `KeymapToCsv` | Export Keymap to CSV… | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `LabelSizeDemoAction` | Test Label Sizes | `Internal.UI.Demos` |  | MISSING | `plugins/devkit` |
| `LaggingMode` | Lagging Mode | `Internal.Performance` |  | MISSING | `plugins/dev` |
| `ListPersistentStateComponents` | List Persistent State Components | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `LoadAllContent` | Load All Files Content | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `LoadAllVFSContent` | Load All VFS Stored Files Content | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `LockReqAnalysis` | Analyze Lock Requirements |  |  | MISSING | `plugins/devkit` |
| `LogDebugConfigure` | Debug Log Settings… | `HelpDiagnosticTools` |  | MISSING | `platform/platform-impl` |
| `LogFocusRequests` | Log Focus Requests | `Internal.UI.Focus` |  | MISSING | `plugins/devkit` |
| `LongBackgroundWriteAction` | Run Long Background Write Action | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `MacMessagesTest` | Test Mac Messages | `Internal.UI.Ex` |  | MISSING | `plugins/devkit` |
| `MakeAllJarsAction` | Prepare All Plugins to Deploy | `PluginDeployActions` |  | MISSING | `plugins/devkit` |
| `MakeJarAction` | Prepare to Deploy | `PluginDeployActions` |  | MISSING | `plugins/devkit` |
| `MarkVfsCorrupted` | Mark VFS as Corrupted | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `ModulesFindUsages` | Find Usages | `ModulesTreePopupMenu` | `alt F7` | codeinsight/07-navigation.md | `plugins/devkit` |
| `MoveFileToAnotherModule` | Move File to Another Module |  |  | MISSING | `plugins/dev` |
| `NewAction` | Action | `DevkitObsoleteNewActions` |  | MISSING | `plugins/devkit` |
| `NotificationTestAction` | Add Test Notification | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `OpenEventsSchemeFileAction` | Open Scheme File |  |  | MISSING | `platform/statistics` |
| `OpenEventsTestSchemeFileAction` | Open Test Scheme File |  |  | MISSING | `platform/statistics` |
| `OpenMouseWheelSmoothScrollSettings` | Smooth Scroll Options |  |  | MISSING | `platform/platform-impl` |
| `OpenStatisticsEventLogAction` | Open Statistics Event Log | `FeatureUsage.Internal` |  | MISSING | `platform/statistics` |
| `PaintEditorLineStripeComponent` | Paint Editor Caret Stripe |  |  | MISSING | `plugins/dev` |
| `PerformGC` | Perform GC |  |  | MISSING | `plugins/dev` |
| `Performance.ActivityMonitor` | Activity Monitor… | `HelpDiagnosticTools` |  | MISSING | `platform/platform-impl` |
| `Performance.DumpThreads` | Dump Threads | `HelpDiagnosticTools` |  | MISSING | `platform/platform-impl` |
| `PlayTestSoundAction` | Play Test Sound | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `PluginManagerFiltersConfigureResetTrustDebugAction` | Plugin Filter for Org: Reset |  |  | MISSING | `plugins/dev` |
| `PluginManagerFiltersConfigureTrustOnlyJetBrainsDebugAction` | Plugin Filter for Org: Trust Only JetBrains |  |  | MISSING | `plugins/dev` |
| `PluginPermissionServiceDemo` | Plugin Permission Service Demo | `Internal.DevKit` |  | MISSING | `plugins/devkit` |
| `PluginUpdateSourcesReinitializeAction` | Rerun Plugin Update Sources Initialization |  |  | MISSING | `plugins/dev` |
| `PluginUpdateSourcesResetAction` | Erase Plugin Update Sources |  |  | MISSING | `plugins/dev` |
| `PooledFreeze` | Freeze Pooled Threads | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `PositionTrackerTest` | Test Position Tracker | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `PrintErrorAutoReporterState` | State of Sending Error Reports | `Internal.Errors.Additional` |  | MISSING | `plugins/performanceTesting` |
| `PrintModulesAndEntitySources` | Print Modules and Entity Sources to Log | `Internal.WorkspaceModel` |  | MISSING | `plugins/dev` |
| `ProgressIconShowcaseAction` | Progress Icon Showcase | `Internal.UI.Icons` |  | MISSING | `plugins/devkit` |
| `PruneEmptyDirectories` | Prune Empty Directories | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `PsiViewer` | View PSI Structure… | `Dev.PsiViewerActions` |  | MISSING | `plugins/dev` |
| `PsiViewerForContext` | View PSI Structure of Current File… | `Dev.PsiViewerActions` |  | MISSING | `plugins/dev` |
| `RecordStateCollectors` | Record State Collectors to Event Log | `FeatureUsage.Internal` |  | MISSING | `platform/statistics` |
| `ReloadProject` | Reload Project | `Internal.WorkspaceModel` |  | MISSING | `plugins/dev` |
| `RemoveMainToolbarActions` | Remove Actions from Main Toolbar |  |  | MISSING | `plugins/dev` |
| `ReportSettingsToFUS` | Record Changed Settings to Event Log |  |  | MISSING | `platform/statistics` |
| `RequestCachesDefragmentation` | Request VFS and Indexes Defragmentation | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `ResetFreezeNotificationState` | Reset Freezes Notification State | `Internal.Errors.Additional` |  | MISSING | `plugins/performanceTesting` |
| `ResetGotItTooltips` | Reset Got It Tooltips | `Internal.Ex` |  | MISSING | `platform/platform-impl` |
| `ResetSettingsNewBadges` | Reset Settings "New" Badge View Counters |  |  | MISSING | `plugins/dev` |
| `ResetWindowsDefenderNotification` | Restart Microsoft Defender Notification | `HelpDiagnosticTools` |  | MISSING | `platform/platform-impl` |
| `RestartIde` | Restart IDE… |  |  | MISSING | `platform/platform-impl` |
| `RestoreDefaultExtensionScripts` | Restore Default Extensions | `ProjectViewPopupMenu` |  | MISSING | `platform/lang-impl` |
| `RestoreFontPreviewTextAction` | Restore Preview Text | `EditorContextBarMenu` |  | MISSING | `platform/platform-impl` |
| `RetypeFile` | Retype Current File | `Internal.Performance` |  | MISSING | `plugins/dev` |
| `RoundedIconTestAction` | Show Rounded Icon | `Internal.UI.Icons` |  | MISSING | `plugins/devkit` |
| `RulerGuide` | Ruler Guide | `Internal.UI.Ex` |  | MISSING | `plugins/devkit` |
| `ScanSourceComments` | Dump All Comments in Project | `Internal.Dump` |  | MISSING | `plugins/dev` |
| `SendEventLogStatistics` | Send Feature Usage Event Log To Prod |  |  | MISSING | `platform/statistics` |
| `SetCaretVisualAttributesInternalAction` | Set Caret Visual Attributes… | `Internal.Editor` |  | MISSING | `plugins/dev` |
| `ShortDumbMode` | Run Short Dumb Action | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `ShowBundleLocalizationDialogAction` | Show Bundles Messages | `Internal.UI` |  | MISSING | `plugins/dev` |
| `ShowCompressibleToolbarAction` | Display Dynamic Compressed Toolbars | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `ShowControlFlow` | Show Control Flow | `Internal` |  | MISSING | `plugins/dev` |
| `ShowCountdownDialog` | Show Countdown Dialog | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `ShowEditorHighlighterTokens` | Show Editor Highlighter Tokens | `Internal.Editor` |  | MISSING | `plugins/devkit` |
| `ShowEdtUtilizationChart` | Show EDT Utilization Chart | `Internal.Performance` |  | MISSING | `plugins/dev` |
| `ShowExperiments` | Experimental Features… | `MaintenanceGroup` |  | MISSING | `platform/platform-impl` |
| `ShowFontsUsedByEditor` | Show Fonts Used by Editor |  |  | MISSING | `platform/platform-impl` |
| `ShowGotItDemo` | Show GotIt Demo | `Internal.UI.Demos` |  | MISSING | `platform/platform-impl` |
| `ShowHelpPageById` | Show Help Page by ID | `Internal.DevKit` |  | MISSING | `plugins/devkit` |
| `ShowHideDebugInfoInUi` | Debugging Info in UI | `Internal.UI` |  | MISSING | `plugins/dev` |
| `ShowIconsNotPatchedForExpUi` | Show Icons Non Patched for ExpUI | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `ShowInlineCompletionStatistics` | Show Inline Completion Statistics |  |  | codeinsight/06-completion.md CI-CMP | `plugins/dev` |
| `ShowMainMenu` | Show Main Menu | `Internal.Ex` |  | MISSING | `platform/platform-impl` |
| `ShowNonRetinaImages` | Show Non-Retina Images | `Internal` |  | MISSING | `plugins/dev` |
| `ShowNotificationIconsDialogAction` | Test Notification Icons | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `ShowPluginLoadingState` | Show Plugin Loading State | `ToolsMenu` |  | MISSING | `plugins/dev` |
| `ShowPoweredProgress` | Show Powered Progress | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `ShowProgressTestDialogAction` | Test Progress Indicators | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `ShowPropertiesComponent` | Properties Component… | `MaintenanceGroup` |  | MISSING | `platform/platform-impl` |
| `ShowRegisteredRemoteApis` | Show Registered Remote APIs | `Internal.Ex` |  | MISSING | `platform/platform-impl` |
| `ShowRegistry` | Registry… | `MaintenanceGroup` |  | MISSING | `platform/platform-impl` |
| `ShowSearchEverywhereItemInfo` | Show Search Everywhere Item Info | `Internal` |  | MISSING | `plugins/dev` |
| `ShowSerializedXml` | Show Serialized XML for Class | `Internal.DevKit` |  | MISSING | `plugins/devkit` |
| `ShowSouthPanelTestDialogAction` | Test DialogWrapper South Panel | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `ShowSplash` | Show Splash | `Internal.UI` |  | MISSING | `plugins/dev` |
| `ShowTrustProjectDialog` | Trust Project… |  |  | MISSING | `platform/platform-impl` |
| `ShowUpdateDialogAction` | Show Update Dialog | `Internal` |  | MISSING | `plugins/dev` |
| `ShowWorkspaceFileState` | Show Workspace File State |  |  | MISSING | `plugins/dev` |
| `ShuffleNamesAction` | Shuffle/Scramble/Anonymize/Permute Names | `Internal.DevKit` |  | MISSING | `plugins/devkit` |
| `SilentHeapDumpSnapshot` | Create and Schedule Heap Dump Analysis (Non-User Invoked) | `Internal.HeapAnalysis` |  | MISSING | `platform/platform-impl` |
| `SimulateFreeze` | Simulate Freeze |  |  | MISSING | `plugins/performanceTesting` |
| `SimulateRWFreeze` | Simulate Read-Write Freeze |  |  | MISSING | `plugins/performanceTesting` |
| `SkipWindowDeactivationEvents` | Skip Window Deactivation Events | `Internal.Ex` |  | MISSING | `plugins/dev` |
| `SwingComposeShowcase` | Swing Compose UI Showcase |  |  | MISSING | `plugins/devkit` |
| `SwitchFileBasedIndexStorageAction` | Switch File-Based Index Storage |  |  | MISSING | `platform/lang-impl` |
| `TestCoroutineProgress` | Test Coroutine Progress | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `TestData.Navigate` | Navigate to Test Data | `TestTreePopupMenu` | as `GotoRelated` | MISSING | `plugins/devkit` |
| `TestGestureAction` | Test Gesture Action | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `TestIconMappings` | Test Icon Mappings | `Internal` |  | MISSING | `plugins/dev` |
| `TestJbProtocolCommand` | Test JB Protocol Command… | `Internal` |  | MISSING | `platform/built-in-server` |
| `TestMessageBoxAction` | Show Test Dialog | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `TestParseEventsSchemeAction` | Test Parsing Events Scheme… |  |  | MISSING | `platform/statistics` |
| `TestProgressWindow` | Test Progress Window | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `TestRubberDuckDebuggerAction` | Test Rubber Duck Debugger |  |  | MISSING | `platform/platform-impl` |
| `TestSearchContributors` | SE Contributors Testing |  |  | MISSING | `platform/lang-impl` |
| `TestTextFieldAction` | Test Text Fields | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `TestTooltipsAction` | Test Tooltips | `Internal.UI.Demos` |  | MISSING | `plugins/devkit` |
| `TextGrayFilterConfig` | TextGrayFilter | `Internal.UI.Ex` |  | MISSING | `plugins/devkit` |
| `ThreadingReqAnalysis` | Analyze Threading Requirements |  |  | MISSING | `plugins/devkit` |
| `ToggleEssentialHighlighting` | Essential Highlighting | `PowerSaveGroup` |  | MISSING | `plugins/dev` |
| `ToggleFileBasedIndexes` | Toggle File-Based Indexes |  |  | MISSING | `platform/lang-impl` |
| `ToggleFocusMode` | Highlight Only Current Declaration | `ViewMenu`, `EditorGutterPopupMenu` |  | MISSING | `plugins/dev` |
| `ToggleHighlightingMarkup` | Toggle Expected Highlighting Markup | `Internal.DevKit` |  | MISSING | `plugins/devkit` |
| `ToggleIncompleteMode` | Enter Incomplete Dependencies Mode | `Internal.Ex` |  | MISSING | `platform/platform-impl` |
| `ToggleInstaller` | Use Installer | `IdeStarterRunModeActions` |  | MISSING | `plugins/devkit` |
| `TogglePowerSave` | Power Save Mode | `PowerSaveGroup` |  | MISSING | `platform/lang-impl` |
| `ToggleSplitMode` | Split Mode | `IdeStarterRunModeActions` |  | MISSING | `plugins/devkit` |
| `ToolbarDemo` | Toolbar Demo | `Internal.UI.Demos` |  | MISSING | `plugins/devkit` |
| `TransferSettingsDemoAction` | Test transfer settings |  |  | MISSING | `plugins/ide-startup` |
| `TreeExpandAll` | Expand Tree | `Internal.UI.Ex` |  | MISSING | `plugins/devkit` |
| `TreeViewModelDemo` | Tree View Model Demo | `Internal.UI.Demos` |  | MISSING | `plugins/devkit` |
| `TriggerLowMemoryNotification` | Trigger Low Memory Notification |  |  | MISSING | `plugins/dev` |
| `TriggerProjectIndexableFilesFilterHealthCheck` | Trigger Project Indexable Files Filter Health Check | `Internal` |  | MISSING | `plugins/dev` |
| `TrustProject` | Trust | `Internal.Trust` |  | MISSING | `plugins/dev` |
| `TypingLatencyReport` | Typing Latency Report | `Internal.Performance` |  | MISSING | `plugins/dev` |
| `UIDefaults` | LaF Defaults | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `UIFreeze` | UI Freeze | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `UISandboxAction` | UI Sandbox | `Internal.UI` |  | MISSING | `plugins/devkit` |
| `UiDebugger` | UI Debugger… | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `UiDslShowcaseAction` | UI DSL Showcase | `UiDslShowcaseInternalGroup` |  | MISSING | `plugins/devkit` |
| `UiInspector` | UI Inspector | `UiInspectorGroup` | `mouse control alt button1` | MISSING | `platform/platform-impl` |
| `UiThemeColorPickerAction` | Enable UI Theme Color Picker | `UiInspectorGroup` |  | MISSING | `platform/platform-impl` |
| `UndoClearAction` | Clear Undo History |  |  | MISSING | `plugins/dev` |
| `UndoDumpAction` | Dump Undo History |  |  | MISSING | `plugins/dev` |
| `UnixInfoTest` | Show Unix Info | `Internal.UI.Ex` |  | MISSING | `plugins/devkit` |
| `UntrustProject` | Do Not Trust | `Internal.Trust` |  | MISSING | `plugins/dev` |
| `UpdateIdeFromSources` | Update IDE from Sources… | `BuildMenu` |  | MISSING | `plugins/devkit` |
| `UpdateIdeFromSourcesSettings` | Update IDE from Sources Settings… |  |  | MISSING | `plugins/devkit` |
| `UsageFiltering.ApiDump` | Show Usages in API Dump Files | `UsageFilteringActionGroup.ApiDump` |  | MISSING | `plugins/devkit` |
| `UserInvokedFullAnalysis` | Create and Immediately Analyze Heap Dump | `Internal.HeapAnalysis` |  | MISSING | `platform/platform-impl` |
| `ViewPluginsAutoUpdateRepository` | View Plugins Auto-Update Repository |  |  | MISSING | `plugins/dev` |
| `VirtualFileInfo` | Virtual File Info | `Internal.VFS` |  | MISSING | `plugins/dev` |
| `WorkspaceEntitiesJsonClipboard` | Dump Workspace Entities to Clipboard in JSON Format | `Internal.WorkspaceModel` |  | MISSING | `plugins/dev` |
| `WorkspaceEntitiesJsonLog` | Dump Workspace Entities to Log in JSON Format | `Internal.WorkspaceModel` |  | MISSING | `plugins/dev` |
| `WorkspaceEntitiesJsonLogFile` | Dump Workspace Entities to Log File in JSON Format | `Internal.WorkspaceModel` |  | MISSING | `plugins/dev` |
| `WorkspaceModelGenerateAllModulesAction` | Generate Workspace Model Implementation for All Modules |  |  | MISSING | `plugins/devkit` |
| `WorkspaceModelGeneration` | Generate Workspace Model Implementation | `ToolsMenu` |  | MISSING | `plugins/devkit` |
| `WrapLayoutTestAction` | WrapLayout Demo | `Internal.UI.Ex` |  | MISSING | `plugins/devkit` |
| `WriteActionUnderProgress` | Test Write Action Under Progress | `Internal.UI.Ex` |  | MISSING | `plugins/dev` |
| `com.intellij.platform.ijent.community.ui.actions.dashboard.EmulateIjentFreezeAction` | Emulate IJent Freeze | `com.intellij.platform.ijent.impl.VerificationActions` |  | MISSING | `platform/ijent` |
| `com.intellij.platform.ijent.community.ui.actions.dashboard.ShowIjentDashboardAction` | IJent Dashboard | `com.intellij.platform.ijent.impl.VerificationActions` |  | MISSING | `platform/ijent` |
| `com.intellij.platform.ijent.impl.IjentDockerVerificationAction` | Test IJent + Docker | `com.intellij.platform.ijent.impl.VerificationActions` |  | MISSING | `platform/ijent` |
| `com.intellij.platform.ijent.impl.IjentLocalVerificationAction` | Test IJent + Local Machine | `com.intellij.platform.ijent.impl.VerificationActions` |  | MISSING | `platform/ijent` |
| `com.intellij.platform.ijent.impl.MultiRoutingFileSystemVerificationAction` | MultiRoutingFileSystem Verification Action | `com.intellij.platform.ijent.impl.VerificationActions` |  | MISSING | `platform/ijent` |
| `deadlock.with.certificate.dialog` | Deadlock IDE with certificate dialog | `tasks.internal` |  | MISSING | `plugins/tasks` |
| `fontEditorPreview.ToggleBoldFont` | Toggle Bold Font | `EditorContextBarMenu` |  | MISSING | `platform/platform-impl` |
| `newIjModule` | IJ Module | `NewGroup1`, `NewProjectOrModuleGroup` |  | MISSING | `plugins/devkit` |
| `performancePlugin.ExecuteScriptAction` | Execute Performance Script… | `DiagnosticGroup` |  | MISSING | `plugins/performanceTesting` |
| `performancePlugin.OpenIndexingDiagnosticsAction` | Open Indexing Diagnostics | `IndexingDiagnosticGroup` |  | MISSING | `plugins/performanceTesting` |
| `performancePlugin.ShowMemoryDialogAction` | Change Memory Settings | `HelpMenu` |  | MISSING | `platform/platform-impl` |
| `show.certificate` | Show certificate dialog | `tasks.internal` |  | MISSING | `plugins/tasks` |
| `smart.update` | Auto Update… | `UpdateEntryPointGroup` |  | MISSING | `platform/smart-update` |
| `toggle.tracing.action` | Toggle Build Tracing |  |  | MISSING | `platform/tracing-ide` |

## Settings pages (3)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `com.intellij.devkit.compose.showcase.SettingsPageOnCompose` | Settings Page on Compose |  | tools | application | MISSING |
| `devkit.debugger` | IDE Debugger |  | project.propDebugger | application | MISSING |
| `ijent.dashboard` | Environment Dashboard | build |  | project | MISSING |

## Tool windows (4)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Statistics Event Log (On Host)` | Statistics Event Log (On Host) | bottom |  | StatisticsEventLogToolWindowFactory | MISSING |
| `Statistics Event Log` | Statistics Events | bottom |  | FrontendStatisticsEventLogToolWindowFactory | MISSING |
| `Modules` | Modules | right |  | ModulesToolWindowFactory | MISSING |
| `EventQueue Watcher` | EventQueue Watcher | bottom |  | EventWatcherToolWindowFactory | MISSING |

## Status bar widgets (4)

| Id | Factory | Order | Spec |
|---|---|---|---|
| `WriteThread` | `WriteThreadIndicatorWidgetFactory` | last | MISSING |
| `EditorAnimationCacheStatistics` | `EditorAnimationCacheStatisticsWidgetFactory` | first | MISSING |
| `SmartModeIndicator` | `SmartModeIndicatorWidgetFactory` |  | MISSING |
| `IndexesAndVfsFlushIndicator` | `IndexesAndVfsFlushIndicatorWidgetFactory` |  | MISSING |
