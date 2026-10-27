# Find and Replace

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (28)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `EditorSearchSession.NextOccurrenceAction` | (set in code) | `TouchBarEditorSearch` |  | MISSING | `platform/lang-impl` |
| `EditorSearchSession.ToggleMatchCase` | (set in code) | `TouchBarEditorSearch`, `TouchBarEditorSearch_ctrl` |  | MISSING | `platform/lang-impl` |
| `EditorSearchSession.ToggleRegex` | (set in code) | `TouchBarEditorSearch` |  | MISSING | `platform/lang-impl` |
| `EditorSearchSession.ToggleWholeWordsOnlyAction` | (set in code) | `TouchBarEditorSearch`, `TouchBarEditorSearch_ctrl` |  | MISSING | `platform/lang-impl` |
| `Find` | Find… | `FindMenuGroup` | `control F`, `alt F3` | MISSING | `platform/lang-impl` |
| `FindInPath` | Find in Files… | `CommanderPopupMenu`, `ProjectViewPopupMenu`, `NavbarPopupMenu` (+1) | `control shift F` | MISSING | `platform/lang-impl` |
| `FindNext` | Find Next / Move to Next Occurrence | `FindMenuGroup` | `F3`, `control L` | MISSING | `platform/lang-impl` |
| `FindPrevWordAtCaret` | Previous Occurrence of the Word at Caret | `FindMenuGroup` | `control shift F3` | MISSING | `platform/lang-impl` |
| `FindPrevious` | Find Previous / Move to Previous Occurrence | `FindMenuGroup` | `shift F3`, `control shift L` | MISSING | `platform/lang-impl` |
| `FindSelectionInPath` | Find in Files | `EditorPopupMenu1` | as `FindInPath` | platform/04-dumb-mode.md PLAT-DUMB-059 | `platform/lang-impl` |
| `FindUsages` | Find Usages | `Images.EditorPopupMenu`, `CommanderPopupMenu`, `StructureViewPopupMenu` (+6) | `alt F7` | codeinsight/07-navigation.md | `platform/lang-impl` |
| `FindUsagesInFile` | Find Usages in File | `FindUsagesMenuGroup` | `control F7` | codeinsight/07-navigation.md | `platform/lang-impl` |
| `FindWordAtCaret` | Next Occurrence of the Word at Caret | `FindMenuGroup` | `control F3` | MISSING | `platform/lang-impl` |
| `Replace` | Replace… | `FindMenuGroup` | `control R` | java/04-error-catalog.md JAVA-ERR-147 | `platform/lang-impl` |
| `ReplaceInPath` | Replace in Files… | `CommanderPopupMenu`, `ProjectViewPopupMenu`, `NavbarPopupMenu` (+1) | `control shift R` | MISSING | `platform/lang-impl` |
| `ReplaceTempWithQuery` | Replace Temp with Query… | `AdditionalRefactoringsMenu` |  | MISSING | `plugins/rareJavaRefactorings` |
| `SearchContext.Anywhere` | (set in code) | `ShowFilterPopup` |  | MISSING | `platform/lang-impl` |
| `SearchContext.ExceptComments` | (set in code) | `ShowFilterPopup` |  | MISSING | `platform/lang-impl` |
| `SearchContext.ExceptCommentsAndStringLiterals` | (set in code) | `ShowFilterPopup` |  | MISSING | `platform/lang-impl` |
| `SearchContext.ExceptStringLiterals` | (set in code) | `ShowFilterPopup` |  | MISSING | `platform/lang-impl` |
| `SearchContext.InComments` | (set in code) | `ShowFilterPopup` |  | MISSING | `platform/lang-impl` |
| `SearchContext.InStringLiterals` | (set in code) | `ShowFilterPopup` |  | MISSING | `platform/lang-impl` |
| `ShowSearchHistory` | Show Search History |  | `alt down` | MISSING | `platform/lang-impl` |
| `StructuralSearchPlugin.StructuralReplaceAction` | Replace Structurally… | `StructuralSearchActions` |  | MISSING | `platform/structuralsearch` |
| `StructuralSearchPlugin.StructuralSearchAction` | Search Structurally… | `StructuralSearchActions` |  | MISSING | `platform/structuralsearch` |
| `TextSearchAction` | Text… | `GoToTargetEx` | `control shift alt E` | MISSING | `platform/lang-impl` |
| `ToggleFindInSelection` | Search in Selection Only | `EditMenu`, `FindMenuGroup` | `control alt E` | MISSING | `platform/lang-impl` |
| `ToggleScrollToResultsDuringTypingAction` | Scroll to Results During Typing | `FindMenuGroup` |  | MISSING | `platform/lang-impl` |
