# Code insight actions (completion, documentation, parameter info, generate, intentions)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (57)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `BraceOrQuoteOut` | Jump Outside Current Bracket/Quote |  | `TAB` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `CallInlineCompletionAction` | Call Inline Completion | `CodeCompletionGroup`, `InlineCompletion` | `shift alt BACK_SLASH` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `ClassNameCompletion` | Second Basic Completion |  | `control alt SPACE` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `CodeCompletion` | Basic | `CodeCompletionGroup` | `control SPACE` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `CodeFloatingToolbar.GotoNextMenu` | Go to Next Toolbar Menu |  | `TAB` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/platform-impl` |
| `CodeFloatingToolbar.GotoPrevMenu` | Go to Previous Toolbar Menu |  | `shift TAB` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/platform-impl` |
| `CodeVision.ShowMore` | (set in code) |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `CodeVisionToggleAction` | Toggle Code Vision for Project |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `DisableDeclarativeInlayAction` | (set in code) | `InlayMenu` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Documentation.Back` | Back | `Documentation.Navigation` | as `Back` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Documentation.EditSource` | Jump to Source | `Documentation.Navigation` | as `EditSource` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Documentation.Forward` | Forward | `Documentation.Navigation` | as `Forward` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Documentation.KeepTab` | Keep This Documentation | `ToolWindowContextMenu` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Documentation.ToggleAutoShow` | Show Automatically During Completion |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Documentation.ToggleAutoUpdate` | Auto-Update from Source |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Documentation.ToggleShowInPopup` | Show Documentation Popup First |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Documentation.ViewExternal` | View External Documentation | `Documentation.PrimaryGroup` | as `ExternalJavaDoc` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `EditorChooseLookupItem` | Choose Lookup Item | `LookupActions` | `ENTER` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `EditorChooseLookupItemCompleteStatement` | Choose Lookup Item and Invoke Complete Statement | `LookupActions` | as `EditorCompleteStatement` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `EditorChooseLookupItemDot` | Choose Lookup Item and Insert Dot | `LookupActions` | `control PERIOD` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `EditorChooseLookupItemReplace` | Choose Lookup Item Replace | `LookupActions` | `TAB` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `EditorLookupDown` | Lookup Down |  | `control DOWN` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `EditorLookupSelectionDown` | Select Next Completion Option |  |  | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `EditorLookupSelectionUp` | Select Previous Completion Option |  |  | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `EditorLookupUp` | Lookup Up |  | `control UP` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `EnableCommandCompletion` | Enable Command Completion | `CodeCompletionGroup` |  | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `ExpressionTypeInfo` | Type Info | `CodeEditorViewGroup` | `control shift P` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `ExternalJavaDoc` | External Documentation | `CodeEditorViewGroup` | `shift F1` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Floating.CodeToolbar.Disable` | Don't Show Toolbar |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/platform-impl` |
| `Floating.CodeToolbar.SurroundWithIf` | if | `Floating.CodeToolbar.Surround` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Floating.CodeToolbar.SurroundWithTryCatch` | try / catch | `Floating.CodeToolbar.Surround` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `Floating.CodeToolbar.SurroundWithTryCatchFinally` | try / catch / finally | `Floating.CodeToolbar.Surround` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `GoToLinkTarget` | Go to Link Target | `ProjectViewPopupMenu` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `HippieBackwardCompletion` | Cyclic Expand Word (Backward) | `CodeCompletionGroup` | `alt shift SLASH` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `HippieCompletion` | Cyclic Expand Word | `CodeCompletionGroup` | `alt SLASH` | codeinsight/06-completion.md | `platform/lang-impl` |
| `InsertInlineCompletionAction` | Insert Inline Proposal | `CodeCompletionGroup`, `InlineCompletion` | `TAB` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `InsertInlineCompletionLineAction` | Insert Inline Proposal's Line | `CodeCompletionGroup`, `InlineCompletion` | as `EditorLineEnd` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `InsertInlineCompletionWordAction` | Insert Inline Proposal's Word | `CodeCompletionGroup`, `InlineCompletion` | as `EditorNextWord` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `NextInlineCompletionSuggestionAction` | Next Inline Proposal | `InlineCompletion` | `alt CLOSE_BRACKET` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `NextTemplateParameter` | Next Live Template Parameter | `TemplateParametersNavigation` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-api` |
| `ParameterInfo` | Parameter Info | `CodeEditorViewGroup` | `control P` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `PopupHector` | Configure Current File Analysis… | `AnalyzeActions` | `ctrl alt shift H` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `PrevInlineCompletionSuggestionAction` | Previous Inline Proposal | `InlineCompletion` | `alt OPEN_BRACKET` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `QuickActionPopup` | Show Quick Actions Popup |  | `control alt ENTER` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/platform-api` |
| `QuickDocCopy` | Copy Quick Doc | `Documentation.PrimaryGroup` | as `$Copy` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `QuickJavaDoc` | Quick Documentation | `QuickActions` | `control Q`, `mouse alt button2` | codeinsight/04-intentions-and-alt-enter.md CI-FIX-026 | `platform/lang-impl` |
| `QuickPreview` | Quick Preview |  | `SPACE` | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `QuickTypeDefinition` | Quick Type Definition | `QuickActions` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `ShowGutterIconsSettings` | Configure Gutter Icons… | `EditorGutterPopupMenu`, `DocCommentGutterIconContextMenu` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `ShowHoverInfo` | Hover Info |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `SmartTypeCompletion` | Type-Matching | `CodeCompletionGroup` | `control shift SPACE` | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `ToggleCompletionHintsAction` | Show Parameter Name Hints on Completion | `ParameterNameHints` |  | codeinsight/06-completion.md CI-CMP | `platform/lang-impl` |
| `ToggleInlayHintsGloballyAction` | (set in code) |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `ToggleInlineHintsAction` | Toggle Parameter Name Hints | `ParameterNameHints` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `TogglePopupHints` | Toggle Popup Hints for Current File |  |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |
| `ViewImportPopups` | Show Import Popups | `EditorToggleActions` |  | codeinsight/05-auto-import.md CI-IMP | `platform/lang-impl` |
| `ViewInplaceComments` | File Details | `ProjectView.RelatedActions`, `ProjectView.ToolWindow.Appearance.Actions` |  | framework: codeinsight/06-completion.md (CI-CMP) | `platform/lang-impl` |

## Settings pages (2)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `preferences.intentionPowerPack` | Intentions | editor |  | application | codeinsight/04-intentions-and-alt-enter.md |
| `inlay.hints` | Inlay Hints |  | editor | project | codeinsight/01-daemon.md CI-DMN-115 |
