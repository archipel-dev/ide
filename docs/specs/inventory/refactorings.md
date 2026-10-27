# Refactorings

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (53)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `AnonymousToInner` | Convert Anonymous to Inner… |  |  | MISSING | `java/java-backend` |
| `ChangeSignature` | Change Signature… | `RefactoringMenu` | `control F6` | MISSING | `platform/lang-impl` |
| `ChangeTypeSignature` | Type Migration… | `RefactoringMenu1` | `control shift F6` | MISSING | `java/java-backend` |
| `ConvertToInstanceMethod` | Convert To Instance Method… | `RefactoringMenu1` |  | MISSING | `java/java-backend` |
| `ConvertToMacLineSeparators` | (set in code) | `ChangeLineSeparators` |  | MISSING | `platform/platform-impl` |
| `ConvertToUnixLineSeparators` | (set in code) | `ChangeLineSeparators` |  | MISSING | `platform/platform-impl` |
| `ConvertToWindowsLineSeparators` | (set in code) | `ChangeLineSeparators` |  | MISSING | `platform/platform-impl` |
| `CopyElement` | Copy… | `RefactoringMenu` | `F5` | MISSING | `platform/lang-impl` |
| `EncapsulateFields` | Encapsulate Fields… | `RefactoringMenu4` |  | MISSING | `java/java-backend` |
| `ExtractClass` | Extract Delegate… | `IntroduceActionsGroup` |  | MISSING | `platform/lang-impl` |
| `ExtractInclude` | Include File… | `IntroduceActionsGroup` |  | MISSING | `platform/lang-impl` |
| `ExtractInterface` | Extract Interface… | `IntroduceActionsGroup` |  | MISSING | `platform/lang-impl` |
| `ExtractMethod` | Extract Method… | `IntroduceActionsGroup`, `Floating.CodeToolbar.Extract` | `control alt M` | MISSING | `platform/lang-impl` |
| `ExtractModule` | Extract Module… | `IntroduceActionsGroup` |  | MISSING | `platform/lang-impl` |
| `ExtractModuleFromPackage` | Extract Module… | `Java.ImportFrom.Group` |  | MISSING | `java/java-backend` |
| `ExtractSuperclass` | Extract Superclass… | `IntroduceActionsGroup` |  | MISSING | `platform/lang-impl` |
| `Generify` | Convert Raw Types to Generics… | `AdditionalRefactoringsMenu` |  | MISSING | `plugins/rareJavaRefactorings` |
| `I18nize` | Internationalize… | `RefactoringMenu` |  | MISSING | `plugins/java-i18n` |
| `InheritanceToDelegation` | Replace Inheritance with Delegation… | `RefactoringMenu4` |  | MISSING | `java/java-backend` |
| `Inline` | Inline… | `RefactoringMenu` | `control alt N` | codeinsight/06-completion.md CI-CMP-073 | `platform/lang-impl` |
| `IntroduceConstant` | Introduce Constant… | `IntroduceActionsGroup`, `Floating.CodeToolbar.Extract` | `control alt C` | MISSING | `platform/lang-impl` |
| `IntroduceField` | Introduce Field… | `IntroduceActionsGroup` | `control alt F` | MISSING | `platform/lang-impl` |
| `IntroduceFunctionalParameter` | Introduce Functional Parameter… | `IntroduceActionsGroup` | `control shift alt P` | MISSING | `java/java-backend` |
| `IntroduceFunctionalVariable` | Introduce Functional Variable… | `IntroduceActionsGroup` |  | MISSING | `java/java-backend` |
| `IntroduceParameter` | Introduce Parameter… | `IntroduceActionsGroup`, `Floating.CodeToolbar.Extract` | `control alt P` | MISSING | `platform/lang-impl` |
| `IntroduceParameterObject` | Introduce Parameter Object… | `IntroduceActionsGroup` |  | MISSING | `platform/lang-impl` |
| `IntroduceVariable` | Introduce Variable… | `IntroduceActionsGroup`, `Floating.CodeToolbar.Extract` | `control alt V` | MISSING | `platform/lang-impl` |
| `InvertBoolean` | Invert Boolean… | `RefactoringMenu` |  | MISSING | `platform/lang-impl` |
| `MakeStatic` | Make Static… | `RefactoringMenu1` |  | MISSING | `java/java-backend` |
| `MemberPushDown` | Push Members Down… | `RefactoringMenu` |  | MISSING | `platform/lang-impl` |
| `MembersPullUp` | Pull Members Up… | `RefactoringMenu` |  | MISSING | `platform/lang-impl` |
| `Move` | Move… | `RefactoringMenu` | `F6` | MISSING | `platform/lang-impl` |
| `MoveEditorToOppositeTabGroup` | Move to Opposite Group | `EditorTabPopupMenu`, `TabsActions` |  | MISSING | `platform/platform-impl` |
| `MoveElementLeft` | Move Element Left | `CodeMenu` | `control alt shift LEFT` | MISSING | `platform/lang-impl` |
| `MoveElementRight` | Move Element Right | `CodeMenu` | `control alt shift RIGHT` | MISSING | `platform/lang-impl` |
| `MoveLineDown` | Move Line Down | `CodeMenu` | `alt shift DOWN` | MISSING | `platform/lang-impl` |
| `MoveLineUp` | Move Line Up | `CodeMenu` | `alt shift UP` | MISSING | `platform/lang-impl` |
| `MoveMinimap` | Move Minimap | `MinimapActionsGroup` |  | MISSING | `plugins/minimap` |
| `MoveStatementDown` | Move Statement Down | `CodeMenu` | `control shift DOWN` | MISSING | `platform/lang-impl` |
| `MoveStatementUp` | Move Statement Up | `CodeMenu` | `control shift UP` | MISSING | `platform/lang-impl` |
| `MoveTabDown` | Split and Move Down | `EditorTabsGroup`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `MoveTabRight` | Split and Move Right | `EditorTabsGroup`, `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `MoveToolWindowTabFromEditorToToolWindowAction` | Return to Tool Window | `EditorTabPopupMenu` |  | MISSING | `platform/platform-impl` |
| `MoveToolWindowTabToEditorAction` | Open as Editor Tab | `ToolWindowTabContextMenu` | `alt shift ENTER` | MISSING | `platform/platform-impl` |
| `MoveWindowBuiltinDisplayAction` | Move to Built-in Display | `WindowMenu` |  | MISSING | `platform/platform-impl` |
| `Refactorings.QuickListPopupAction` | Refactor This… | `RefactoringMenu` | `control alt shift T` | MISSING | `platform/lang-impl` |
| `RemoveMiddleman` | Remove Middleman… | `AdditionalRefactoringsMenu` |  | MISSING | `plugins/rareJavaRefactorings` |
| `RenameElement` | Rename… | `StructureViewPopupMenu`, `RefactoringMenu`, `ProjectViewPopupMenu` (+2) | `shift F6` | MISSING | `platform/lang-impl` |
| `RenameFile` | Rename File… | `RefactoringMenuRenameFile`, `EditorTabPopupMenuEx` |  | MISSING | `platform/lang-impl` |
| `ReplaceMethodWithMethodObject` | Replace Method With Method Object… | `IntroduceActionsGroup` |  | MISSING | `java/java-backend` |
| `SafeDelete` | Safe Delete… | `RefactoringMenu` | `alt DELETE` | MISSING | `platform/lang-impl` |
| `Unwrap` | Unwrap/Remove… | `CodeMenu` | `control shift DELETE` | java/04-error-catalog.md JAVA-ERR-120 | `platform/lang-impl` |
| `WrapReturnValue` | Wrap Method Return Value… | `AdditionalRefactoringsMenu` |  | MISSING | `plugins/rareJavaRefactorings` |
