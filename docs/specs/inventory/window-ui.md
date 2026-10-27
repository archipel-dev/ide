# Windows, tool windows, tabs, splits and view modes

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (137)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ActivateFindToolWindow` | Find | `ActivateToolWindowActions` | `alt 3` | MISSING | `platform/lang-impl` |
| `ActivateHierarchyToolWindow` | Hierarchy | `ActivateToolWindowActions` |  | MISSING | `platform/lang-impl` |
| `AutoShowProcessWindow` | Auto Show | `BackgroundTasks` |  | MISSING | `platform/platform-impl` |
| `ByteCodeDecompiler` | Show Decompiled Class | `idea.java.decompiler.action.group` |  | MISSING | `plugins/java-decompiler` |
| `BytecodeViewer` | Show Bytecode | `ByeCodeViewerGroup` |  | MISSING | `plugins/ByteCodeViewer` |
| `ChangeIdeScale` | Zoom | `ChangeScheme` |  | MISSING | `platform/platform-impl` |
| `ChangeSplitOrientation` | Change Splitter Orientation | `EditorTabsGroup`, `EditorTabsEntryPoint`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `ChangeToolWindowLayout` | Layout | `ChangeScheme` |  | MISSING | `platform/platform-impl` |
| `ChangeView` | View Mode | `ChangeScheme`, `SettingsEntryPointGroup` |  | MISSING | `platform/platform-impl` |
| `CloseActiveTab` | Close Active Tab | `ActiveToolwindowGroup` | `control shift F4` | MISSING | `platform/platform-impl` |
| `CloseAllEditors` | Close All Tabs | `CloseEditorsGroup`, `EditorTabsEntryPoint` |  | MISSING | `platform/platform-impl` |
| `CloseAllEditorsButActive` | Close Other Tabs | `CloseEditorsGroup` |  | MISSING | `platform/platform-impl` |
| `CloseAllNotifications` | Close All | `Notifications` |  | MISSING | `platform/platform-impl` |
| `CloseAllReadonly` | Close All Read-Only Tabs | `CloseEditorsGroup` |  | MISSING | `platform/platform-impl` |
| `CloseAllToTheLeft` | Close Tabs to the Left | `CloseEditorsGroup` |  | MISSING | `platform/platform-impl` |
| `CloseAllToTheRight` | Close Tabs to the Right | `CloseEditorsGroup` |  | MISSING | `platform/platform-impl` |
| `CloseAllUnpinnedEditors` | Close All but Pinned | `CloseEditorsGroup` |  | MISSING | `platform/platform-impl` |
| `CloseContent` | Close Tab | `CloseEditorsGroup` | `control F4` | MISSING | `platform/platform-impl` |
| `CloseEditor` | Close Active Editor |  |  | MISSING | `platform/platform-impl` |
| `CloseFirstNotification` | Close First | `Notifications` |  | MISSING | `platform/platform-impl` |
| `ColoredHeaderAction` | Show Project Gradient | `ToolbarPopupActions.MainToolbarNewUI` |  | MISSING | `platform/platform-impl` |
| `CommanderSwapPanels` | Swap Panels | `PairFileActions` | `control U` | MISSING | `plugins/commander` |
| `CommanderSyncViews` | Synchronize Views | `PairFileActions` | `alt F6` | MISSING | `plugins/commander` |
| `ConfigureEditorTabs` | Configure Editor Tabs… | `EditorTabsGroup`, `EditorTabsEntryPoint`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `DecrementWindowHeight` | Decrement Height |  | as `ResizeToolWindowUp` | MISSING | `platform/platform-impl` |
| `DecrementWindowWidth` | Decrement Width |  | as `ResizeToolWindowLeft` | MISSING | `platform/platform-impl` |
| `DisableMinimap` | Disable | `MinimapActionsGroup` |  | MISSING | `plugins/minimap` |
| `DockPinnedMode` | Dock Pinned | `TWViewModes` |  | MISSING | `platform/platform-impl` |
| `DockToolWindow` | Dock | `ActiveToolwindowGroup` |  | MISSING | `platform/platform-impl` |
| `DockUnpinnedMode` | Dock Unpinned | `TWViewModes` |  | MISSING | `platform/platform-impl` |
| `EditorTab` | Tab | `EditorActions` | `TAB` | MISSING | `platform/platform-impl` |
| `EnableMinimap` | Enable |  |  | MISSING | `plugins/minimap` |
| `FloatMode` | Float | `TWViewModes` |  | MISSING | `platform/platform-impl` |
| `FocusEditor` | Focus Editor | `EditorActions` | `ESCAPE` | MISSING | `platform/platform-api` |
| `FocusMainToolbar` | Focus Main Toolbar |  | `alt PAGE_UP` | MISSING | `platform/platform-impl` |
| `FocusStatusBar` | Focus Status Bar |  | `alt PAGE_DOWN` | MISSING | `platform/platform-impl` |
| `GoToLastTab` | Select the Last Tab | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab1` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab2` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab3` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab4` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab5` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab6` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab7` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab8` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `GoToTab9` | (set in code) | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `HideActiveWindow` | Hide Active Tool Window | `ActiveToolwindowGroup` | `shift ESCAPE` | MISSING | `platform/platform-impl` |
| `HideAllWindows` | Hide All Tool Windows | `ActiveToolwindowGroup` | `control shift F12` | MISSING | `platform/platform-impl` |
| `HideBottomWindows` | Hide Bottom Tool Windows | `ActiveToolwindowGroup` |  | MISSING | `platform/platform-impl` |
| `HideSideWindows` | Hide Side Tool Windows | `ActiveToolwindowGroup` |  | MISSING | `platform/platform-impl` |
| `HideToolWindowStripeButton` | Hide | `ToolWindowStripeButtonPopup` |  | MISSING | `platform/platform-impl` |
| `IncrementWindowHeight` | Increment Height |  | as `ResizeToolWindowDown` | MISSING | `platform/platform-impl` |
| `IncrementWindowWidth` | Increment Width |  | as `ResizeToolWindowRight` | MISSING | `platform/platform-impl` |
| `IncrementalSearch` | Incremental Search |  |  | MISSING | `platform/lang-impl` |
| `KeepTabOpen` | Keep Tab Open | `EditorTabsGroup`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `MainMenuButton.ShowMenu` | Show Collapsed Main Menu |  | `alt BACK_SLASH` | MISSING | `platform/platform-api` |
| `MarkNotificationsAsRead` | (set in code) |  |  | MISSING | `platform/platform-impl` |
| `MaximizeActiveDialog` | Maximize/Restore Active Dialog |  |  | MISSING | `platform/platform-impl` |
| `MaximizeEditorInSplit` | (set in code) | `EditorTabsGroup` |  | MISSING | `platform/platform-impl` |
| `MaximizeToolWindow` | (set in code) | `ActiveToolwindowGroup` | `control shift QUOTE` | MISSING | `platform/platform-impl` |
| `MinimizeCurrentWindow` | Minimize | `WindowMenu` |  | MISSING | `platform/platform-impl` |
| `NextWindow` | Activate Next Window |  |  | MISSING | `platform/lang-impl` |
| `NonModalWindow.PinUnpin` | Pin Window |  |  | MISSING | `platform/platform-api` |
| `OpenEditorInOppositeTabGroup` | Open in Opposite Group | `EditorTabPopupMenu`, `TabsActions` |  | MISSING | `platform/platform-impl` |
| `OpenMinimapSettings` | Minimap Settings | `MinimapActionsGroup` |  | MISSING | `plugins/minimap` |
| `PinActiveEditorTab` | Pin Active Tab | `EditorTabsGroup`, `EditorTabPopupMenu` | as `PinActiveTab` | MISSING | `platform/platform-impl` |
| `PinActiveTab` | Pin Active Tab | `TabsActions` |  | MISSING | `platform/platform-impl` |
| `PinActiveTabToggle` | Pin Active Tab |  |  | MISSING | `platform/platform-impl` |
| `PinToolwindowTab` | Pin Active Tool Window Tab | `ActiveToolwindowGroup` | as `PinActiveTab` | MISSING | `platform/platform-impl` |
| `PreviousOccurence` | Previous Occurrence | `GoToMenu` | `control alt UP` | codeinsight/07-navigation.md CI-NAV-021 | `platform/platform-impl` |
| `PreviousProjectWindow` | Previous Project Window | `OpenProjectWindows` | `control alt OPEN_BRACKET` | MISSING | `platform/lang-impl` |
| `PreviousWindow` | Activate Previous Window |  |  | MISSING | `platform/lang-impl` |
| `ReopenClosedTab` | Reopen Closed Tab | `EditorTabsGroup`, `EditorTabsEntryPoint`, `EditorTabPopupMenu` (+1) |  | MISSING | `platform/platform-impl` |
| `ResetIdeScaleAction` | Reset IDE Scale |  | `shift alt 0` | MISSING | `platform/platform-impl` |
| `ResizeToolWindowDown` | Stretch to Bottom | `ResizeToolWindowGroup` | `control alt shift DOWN` | MISSING | `platform/platform-impl` |
| `ResizeToolWindowLeft` | Stretch to Left | `ResizeToolWindowGroup` | `control alt shift LEFT` | MISSING | `platform/platform-impl` |
| `ResizeToolWindowRight` | Stretch to Right | `ResizeToolWindowGroup` | `control alt shift RIGHT` | MISSING | `platform/platform-impl` |
| `ResizeToolWindowUp` | Stretch to Top | `ResizeToolWindowGroup` | `control alt shift UP` | MISSING | `platform/platform-impl` |
| `RestoreDefaultLayout` | Restore Current Layout | `LayoutsGroup` | `shift F12` | MISSING | `platform/platform-impl` |
| `RestoreFactoryDefaultLayout` | Default | `LayoutsGroup` |  | MISSING | `platform/platform-impl` |
| `ShowProcessWindow` | Show | `BackgroundTasks` |  | MISSING | `platform/platform-impl` |
| `SplitHorizontally` | Split Down | `EditorTabsGroup`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `SplitVertically` | Split Right | `EditorTabsGroup`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `StatusTextModeAction` | Status Text | `ViewStatusBarWidgetsGroup` |  | MISSING | `platform/platform-impl` |
| `StoreDefaultLayout` | Save Changes in Current Layout | `LayoutsGroup` |  | MISSING | `platform/platform-impl` |
| `StoreNewLayout` | Save Current Layout as New… | `LayoutsGroup` |  | MISSING | `platform/platform-impl` |
| `StretchSplitToBottom` | Stretch Editor to Bottom | `EditorTabsGroup` |  | MISSING | `platform/platform-impl` |
| `StretchSplitToLeft` | Stretch Editor to Left | `EditorTabsGroup` |  | MISSING | `platform/platform-impl` |
| `StretchSplitToRight` | Stretch Editor to Right | `EditorTabsGroup` |  | MISSING | `platform/platform-impl` |
| `StretchSplitToTop` | Stretch Editor to Top | `EditorTabsGroup` |  | MISSING | `platform/platform-impl` |
| `SwitchIdeScaleAction` | Zoom IDE | `ViewAppearanceGroup` |  | MISSING | `platform/platform-impl` |
| `TW.CloseAllTabs` | Close All Tabs | `ActiveToolwindowGroup` | as `CloseAllEditors` | MISSING | `platform/platform-impl` |
| `TW.CloseOtherTabs` | Close Other Tabs | `ActiveToolwindowGroup` | as `CloseAllEditorsButActive` | MISSING | `platform/platform-impl` |
| `TW.MoveToNextSplitter` | Go to Next Splitter | `ActiveToolwindowGroup` | as `NextSplitter` | MISSING | `platform/platform-impl` |
| `TW.MoveToPreviousSplitter` | Go to Previous Splitter | `ActiveToolwindowGroup` | as `PrevSplitter` | MISSING | `platform/platform-impl` |
| `TW.SplitAndMoveDown` | Split and Move Down | `ActiveToolwindowGroup` | as `MoveTabDown` | MISSING | `platform/platform-impl` |
| `TW.SplitAndMoveRight` | Split and Move Right | `ActiveToolwindowGroup` | as `MoveTabRight` | MISSING | `platform/platform-impl` |
| `TW.SplitDown` | Split Down | `ActiveToolwindowGroup` | as `SplitHorizontally` | MISSING | `platform/platform-impl` |
| `TW.SplitRight` | Split Right | `ActiveToolwindowGroup` | as `SplitVertically` | MISSING | `platform/platform-impl` |
| `TW.Unsplit` | Unsplit | `ActiveToolwindowGroup` | as `Unsplit` | MISSING | `platform/platform-impl` |
| `TabList` | Show Hidden Tabs | `EditorTabsGroup` |  | MISSING | `platform/platform-impl` |
| `TextEditorWithPreview.Layout.EditorAndPreview` | (set in code) | `TextEditorWithPreview.LayoutGroup` |  | MISSING | `platform/platform-impl` |
| `TextEditorWithPreview.Layout.EditorOnly` | (set in code) | `TextEditorWithPreview.LayoutGroup` |  | MISSING | `platform/platform-impl` |
| `TextEditorWithPreview.Layout.PreviewOnly` | (set in code) | `TextEditorWithPreview.LayoutGroup` |  | MISSING | `platform/platform-impl` |
| `TextEditorWithPreview.SplitHorizontally` | Horizontal Layout | `TextEditorWithPreview.SplitGroup` |  | MISSING | `platform/platform-impl` |
| `TextEditorWithPreview.SplitVertically` | Vertical Layout | `TextEditorWithPreview.SplitGroup` |  | MISSING | `platform/platform-impl` |
| `ToggleCompactMode` | Compact Mode | `ToggleFullScreenGroup` |  | MISSING | `platform/platform-impl` |
| `ToggleContentUiTypeMode` | Group Tabs | `ActiveToolwindowGroup` |  | MISSING | `platform/platform-impl` |
| `ToggleDistractionFreeMode` | Toggle Distraction Free Mode | `ToggleFullScreenGroup` |  | MISSING | `platform/platform-impl` |
| `ToggleDockMode` | Docked Mode | `TWViewModesLegacy` |  | MISSING | `platform/platform-impl` |
| `ToggleFloatingMode` | Floating Mode | `TWViewModesLegacy` |  | MISSING | `platform/platform-impl` |
| `ToggleFullScreen` | Toggle Full Screen Mode | `ToggleFullScreenGroup` |  | MISSING | `platform/platform-impl` |
| `ToggleMinimap` | Show Minimap | `EditorTabsEntryPoint` |  | MISSING | `plugins/minimap` |
| `ToggleMinimapScaleMode` | Fit to Document | `MinimapActionsGroup` |  | MISSING | `plugins/minimap` |
| `TogglePinnedMode` | Pinned Mode | `TWViewModesLegacy` |  | MISSING | `platform/platform-impl` |
| `TogglePresentationAssistantAction` | Presentation Assistant | `UIToggleActions` |  | MISSING | `platform/platform-impl` |
| `TogglePresentationMode` | Toggle Presentation Mode | `ToggleFullScreenGroup` |  | MISSING | `platform/platform-impl` |
| `ToggleSideMode` | Split Mode | `TWViewModesLegacy` |  | MISSING | `platform/platform-impl` |
| `ToggleWindowedMode` | Windowed Mode | `TWViewModesLegacy` |  | MISSING | `platform/platform-impl` |
| `ToggleZenMode` | Toggle Zen Mode | `ToggleFullScreenGroup` |  | MISSING | `platform/platform-impl` |
| `ToolWindowShowNamesAction` | Show Tool Window Names | `ToolWindowStripeButtonPopup` |  | MISSING | `platform/platform-impl` |
| `TopStripeEnableAction` | Use Single Tool Window Bar |  |  | MISSING | `platform/platform-impl` |
| `UndockMode` | Undock | `TWViewModes` |  | MISSING | `platform/platform-impl` |
| `Unsplit` | Unsplit | `EditorTabsGroup`, `EditorTabsEntryPoint`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `UnsplitAll` | Unsplit All | `EditorTabsGroup`, `EditorTabsEntryPoint`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `ViewMainMenu` | Main Menu | `UIToggleActions` |  | MISSING | `platform/platform-impl` |
| `ViewNewToolbarAction` | Toolbar | `UIToggleActions` |  | MISSING | `platform/platform-impl` |
| `ViewObsoleteToolbarAction` | Toolbar Classic | `UIToggleActions` |  | MISSING | `platform/platform-impl` |
| `ViewStatusBar` | Status Bar | `UIToggleActions` |  | MISSING | `platform/platform-impl` |
| `ViewToolBar` | Toolbar | `ViewToolbarActionsGroup`, `MainToolbarPopupActions` |  | MISSING | `platform/platform-impl` |
| `ViewToolButtons` | Tool Window Bars | `UIToggleActions` |  | MISSING | `platform/platform-impl` |
| `WindowMode` | Window | `TWViewModes` |  | MISSING | `platform/platform-impl` |
| `ZoomCurrentWindow` | Zoom | `WindowMenu` |  | MISSING | `platform/platform-impl` |
| `ZoomInIdeAction` | Zoom In IDE |  | `shift alt EQUALS` | MISSING | `platform/platform-impl` |
| `ZoomOutIdeAction` | Zoom Out IDE |  | `shift alt MINUS` | MISSING | `platform/platform-impl` |
| `main.toolbar.Filename` | File Name Widget (when editor tabs are hidden or “Always show full path” is enabled) | `MainToolbarCenter` |  | MISSING | `platform/platform-impl` |
| `main.toolbar.Project` | Project Widget | `MainToolbarLeft` |  | MISSING | `platform/platform-impl` |

## Tool windows (1)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Notifications` | Notifications | right | secondary | NotificationsToolWindowFactory | MISSING |
