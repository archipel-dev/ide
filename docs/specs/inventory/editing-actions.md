# Editing actions (comment, duplicate, move, join, surround, case, smart enter)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (12)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `CodeInspection.OnEditor` | Inspect Code with Editor Settings |  | `alt shift I` | MISSING | `platform/lang-impl` |
| `CommentByBlockComment` | Comment with Block Comment | `CommentGroup` | `control shift SLASH`, `control shift DIVIDE` | MISSING | `platform/lang-impl` |
| `CommentByLineComment` | Comment with Line Comment | `CommentGroup`, `Floating.CodeToolbar.Additional` | `control SLASH`, `control DIVIDE` | MISSING | `platform/lang-impl` |
| `EmacsStyleIndent` | Emacs Tab | `CodeInsightEditorActions` |  | MISSING | `platform/lang-impl` |
| `EmojiPicker.Open` | Emoji \\& Symbols | `EditMenu` | `control alt SEMICOLON` | MISSING | `plugins/emojipicker` |
| `FillParagraph` | Fill Paragraph | `EditorActions`, `EditSmartGroup` |  | MISSING | `platform/lang-impl` |
| `FixDocComment` | Fix Doc Comment |  |  | MISSING | `platform/lang-impl` |
| `NextParameter` | Next Parameter | `LookupActions` | `TAB` | MISSING | `platform/lang-impl` |
| `PrevParameter` | Prev Parameter | `LookupActions` | `shift TAB` | MISSING | `platform/lang-impl` |
| `ShowParameterHintsSettings` | Hints Settings… | `ParameterNameHints` |  | MISSING | `platform/lang-impl` |
| `SurroundWith` | Surround With… | `CodeMenu` | `control alt T` | MISSING | `platform/lang-impl` |
| `Unscramble` | Analyze Stack Trace or Thread Dump… | `AnalyzePlatformMenu` |  | MISSING | `java/unscramble` |
