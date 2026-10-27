# Navigation (go to, back/forward, recent files, navigation bar)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (85)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `Back` | Back | `LeftToolbarSideGroup`, `GoToMenu`, `MainToolBar` (+1) | `control alt LEFT`, `mouse button4` | codeinsight/07-navigation.md CI-NAV | `platform/platform-impl` |
| `CallHierarchy` | Call Hierarchy | `HierarchyGroup` | `control alt H` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `CallHierarchy.BaseOnThisMethod` | Base on This Method | `CallHierarchyPopupMenu` | as `CallHierarchy` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `DeleteRecentFiles` | Delete from Recent Files | `SwitcherAndRecentFiles` | as `$Delete` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/recentFiles` |
| `EditSource` | Jump to Source | `CommanderPopupMenu`, `ServiceViewItemPopup`, `ExternalSystemView.TaskMenu` (+17) | `F4` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `EditSourceInNewWindow` | Open Tab in New Window | `EditorTabPopupMenu` | `shift F4` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `EditSourceNotInEditor` | Edit Source (Not in Editor) |  |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `Forward` | Forward | `LeftToolbarSideGroup`, `GoToMenu`, `MainToolBar` (+1) | `control alt RIGHT`, `mouse button5` | codeinsight/07-navigation.md | `platform/platform-impl` |
| `GotoChangedFile` | Go to Changed File… | `Diff.KeymapGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-api` |
| `GotoCustomRegion` | Custom Folding… | `NavigateInFileGroup` | `control alt PERIOD` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `GotoDeclaration` | Go to Declaration or Usages | `GoToCodeGroup`, `EditorPopupMenu.GoTo` | `control B`, `mouse ctrl button1` | codeinsight/07-navigation.md | `platform/lang-impl` |
| `GotoDeclarationOnly` | Go to Declaration |  |  | codeinsight/07-navigation.md | `platform/lang-impl` |
| `GotoImplementation` | Go to Implementation(s) | `GoToCodeGroup`, `EditorPopupMenu.GoTo` | `control alt B`, `mouse ctrl alt button1` | codeinsight/07-navigation.md | `platform/lang-impl` |
| `GotoLine` | Go to Line:Column… | `GoToMenu` | `control G` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `GotoNextElementUnderCaretUsage` | Next Highlighted Usage | `FindUsagesMenuGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `GotoNextError` | Next Highlighted Error | `GoToErrorGroup` | `F2` | codeinsight/02-highlight-info.md CI-HLI | `platform/lang-impl` |
| `GotoPrevElementUnderCaretUsage` | Previous Highlighted Usage | `FindUsagesMenuGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `GotoPreviousError` | Previous Highlighted Error | `GoToErrorGroup` | `shift F2` | codeinsight/01-daemon.md CI-DMN-115 | `platform/lang-impl` |
| `GotoSuperMethod` | Go to Super Method | `GoToCodeGroup`, `EditorPopupMenu.GoTo` | `control U` | codeinsight/07-navigation.md | `platform/lang-impl` |
| `GotoTest` | Go to Test | `GoToCodeGroup`, `EditorPopupMenu.GoTo` | `control shift T` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `GotoTypeDeclaration` | Go to Type Declaration | `GoToCodeGroup`, `EditorPopupMenu.GoTo` | `control shift B`, `mouse ctrl shift button1` | codeinsight/07-navigation.md | `platform/lang-impl` |
| `JumpToLastChange` | Last Edit Location | `GoToMenu` | `control shift BACK_SPACE` | codeinsight/07-navigation.md CI-NAV-005 | `platform/platform-impl` |
| `JumpToNextChange` | Next Edit Location | `GoToMenu` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `MethodDown` | Next Method | `NavigateInFileGroup` | `alt DOWN` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `MethodHierarchy` | Method Hierarchy | `HierarchyGroup` | `control shift H` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `MethodHierarchy.BaseOnThisMethod` | Base on This Method | `MethodHierarchyPopupMenu` | as `MethodHierarchy` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `MethodHierarchy.ImplementMethodAction` | Implement Method | `JavaMethodHierarchyPopupMenu` | as `ImplementMethods` | framework: codeinsight/07-navigation.md (CI-NAV) | `java/java-backend` |
| `MethodHierarchy.OverrideMethodAction` | Override Method | `JavaMethodHierarchyPopupMenu` | as `OverrideMethods` | framework: codeinsight/07-navigation.md (CI-NAV) | `java/java-backend` |
| `MethodUp` | Previous Method | `NavigateInFileGroup` | `alt UP` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `NavBar-return` | Perform Action on Selected Item | `NavBarActions` | as `EditorEnter` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBar-selectDown` | Open Corresponding Popup Menu | `NavBarActions` | as `EditorDown` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBar-selectEnd` | Select Last Item | `NavBarActions` | as `EditorLineEnd` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBar-selectHome` | Select First Item | `NavBarActions` | as `EditorLineStart` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBar-selectLeft` | Select Previous Item | `NavBarActions` | as `EditorLeft` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBar-selectRight` | Select Next Item | `NavBarActions` | as `EditorRight` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBar-selectUp` | Open Corresponding Popup Menu | `NavBarActions` | as `EditorUp` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBarLocationBottom` | In Status Bar | `NavbarLocationGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBarLocationHide` | Don't Show | `NavbarLocationGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NavBarLocationTop` | Top | `NavbarLocationGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `NextEditorTab` | Select Next Tab in Multi-Editor File | `TabsActions` | `alt shift RIGHT` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `NextOccurence` | Next Occurrence | `GoToMenu` | `control alt DOWN` | codeinsight/07-navigation.md CI-NAV-021 | `platform/platform-impl` |
| `NextProjectWindow` | Next Project Window | `OpenProjectWindows` | `control alt CLOSE_BRACKET` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `NextSplitter` | Go to Next Splitter | `EditorTabsGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `NextTab` | Select Next Tab | `ActiveToolwindowGroup`, `EditorTabsGroup` | `alt RIGHT` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `OpenFile` | Open… | `FileOpenGroup`, `ProjectWidget.Actions`, `MainToolBar` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `OpenInRightSplit` | Open in Right Split | `ViewMenu`, `SplitRevealGroup`, `UsageView.Popup` | `shift ENTER`, `mouse alt button1 doubleClick` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `PeekDefinition` | Peek Definition |  |  | framework: codeinsight/07-navigation.md (CI-NAV) | `plugins/peek-definition` |
| `PrevSplitter` | Go to Previous Splitter | `EditorTabsGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `PreviousEditorTab` | Select Previous Tab in Multi-Editor File | `TabsActions` | `alt shift LEFT` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `PreviousTab` | Select Previous Tab | `ActiveToolwindowGroup`, `EditorTabsGroup` | `alt LEFT` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `QuickImplementations` | Quick Definition | `QuickActions` | `control shift I` | codeinsight/06-completion.md | `platform/lang-impl` |
| `RecentChangedFiles` | Recently Changed Files | `SwitcherAndRecentFiles`, `ViewRecentActions` |  | codeinsight/07-navigation.md CI-NAV-021 | `platform/recentFiles` |
| `RecentChangedFilesFallback` | Recently Changed Files | `SwitcherAndRecentFilesFallback`, `ViewRecentActions` | as `RecentChangedFiles` | codeinsight/07-navigation.md CI-NAV-021 | `platform/platform-impl` |
| `RecentFiles` | Recent Files | `SwitcherAndRecentFiles`, `ViewRecentActions`, `EditorTabsEntryPoint` | `control E` | platform/04-dumb-mode.md PLAT-DUMB-057 | `platform/recentFiles` |
| `RecentFilesFallback` | Recent Files | `SwitcherAndRecentFilesFallback`, `ViewRecentActions`, `EditorTabsEntryPoint` | as `RecentFiles` | platform/04-dumb-mode.md PLAT-DUMB-057 | `platform/platform-impl` |
| `RecentLocations` | Recent Locations | `ViewRecentActions`, `EditorTabsEntryPoint` | `control shift E` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `SelectIn` | Select In… | `GoToCodeGroup` | `alt F1` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `SelectInProjectView` | Select File in Project View | `Bookmarks.ToolWindow.TitleActions`, `ProjectViewToolbar` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `ShowContent` | Show List of Tabs | `ActiveToolwindowGroup` | `alt DOWN` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `ShowErrorDescription` | Error Description | `CodeEditorViewGroup` | `control F1` | codeinsight/02-highlight-info.md CI-HLI-072 | `platform/lang-impl` |
| `ShowMembersInNavigationBar` | Show Members | `NavbarLocationGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `ShowNavBar` | Jump to Navigation Bar | `EditorPopupMenu.GoTo`, `GoToCodeGroup` | `alt HOME` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `Switcher` | Switcher | `SwitcherAndRecentFiles` | `ctrl TAB`, `ctrl shift TAB` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/recentFiles` |
| `SwitcherBackward` | Iterate Switcher Backward | `SwitcherAndRecentFiles` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/recentFiles` |
| `SwitcherBackwardFallback` | Iterate Switcher Backward | `SwitcherAndRecentFilesFallback` | as `SwitcherBackward` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `SwitcherFallback` | Switcher | `SwitcherAndRecentFilesFallback` | as `Switcher` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `SwitcherForward` | Iterate Switcher Forward | `SwitcherAndRecentFiles` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/recentFiles` |
| `SwitcherForwardFallback` | Iterate Switcher Forward | `SwitcherAndRecentFilesFallback` | as `SwitcherForward` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `SwitcherIterateItems` | Iterate Recent Files | `SwitcherAndRecentFiles`, `ViewRecentActions` | `control E` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/recentFiles` |
| `SwitcherIterateItemsFallback` | Iterate Recent Files | `SwitcherAndRecentFilesFallback`, `ViewRecentActions` | as `SwitcherIterateItems` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `SwitcherNextProblem` | Select Next Problem File | `SwitcherAndRecentFiles` | as `GotoNextError` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/recentFiles` |
| `SwitcherNextProblemFallback` | Select Next Problem File | `SwitcherAndRecentFilesFallback` | as `GotoNextError` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `SwitcherPreviousProblem` | Select Previous Problem File | `SwitcherAndRecentFiles` | as `GotoPreviousError` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/recentFiles` |
| `SwitcherPreviousProblemFallback` | Select Previous Problem File | `SwitcherAndRecentFilesFallback` | as `GotoPreviousError` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `SwitcherRecentEditedChangedToggleCheckBox` | Toggle Changed Only Files | `SwitcherAndRecentFiles`, `ViewRecentActions` | `control E` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/recentFiles` |
| `SwitcherRecentEditedChangedToggleCheckBoxFallback` | Toggle Changed Only Files | `SwitcherAndRecentFilesFallback`, `ViewRecentActions` | as `SwitcherRecentEditedChangedToggleCheckBox` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
| `TypeHierarchy` | Type Hierarchy | `CommanderPopupMenu`, `HierarchyGroup` | `control H` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `TypeHierarchy.Class` | Class Hierarchy | `TypeHierarchyPopupMenu` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `TypeHierarchy.Subtypes` | Subtypes Hierarchy | `TypeHierarchyPopupMenu` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `TypeHierarchy.Supertypes` | Supertypes Hierarchy | `TypeHierarchyPopupMenu` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `TypeHierarchyBase.BaseOnThisType` | Base on This Type | `TypeHierarchyPopupMenu` | as `TypeHierarchy` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `ViewMembersInNavigationBar` | Members in Navigation Bar | `UIToggleActions`, `OldUIViewNavigationBarGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `ViewNavigationBar` | Navigation Bar | `ViewToolbarActionsGroup`, `OldUIViewNavigationBarGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `ViewObsoleteNavBarAction` | Navigation Bar | `UIToggleActions` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/navbar` |
| `ViewSource` | Show Source | `ViewMenu` | `control ENTER` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-impl` |
