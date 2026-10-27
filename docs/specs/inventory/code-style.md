# Code style, reformat and imports

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (22)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `Arrangement.Alias.Rule.Add` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Alias.Rule.ToolBar`, `Arrangement.Alias.Rule.Context.Menu` | as `NewElement` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Alias.Rule.Edit` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Alias.Rule.Context.Menu` | as `Arrangement.Rule.Edit` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Alias.Rule.Match.Condition.Move.Down` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Alias.Rule.ToolBar` | as `Arrangement.Rule.Match.Condition.Move.Down` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Alias.Rule.Match.Condition.Move.Up` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Alias.Rule.ToolBar` | as `Arrangement.Rule.Match.Condition.Move.Up` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Alias.Rule.Remove` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Alias.Rule.ToolBar`, `Arrangement.Alias.Rule.Context.Menu` | as `$Delete` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Custom.Token.Rule.Edit` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Rule.Match.Control.ToolBar` |  | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Rule.Add` | Add Rule | `ArrangementRulesGroup`, `Arrangement.Rule.Match.Control.Context.Menu`, `Arrangement.Rule.Match.Control.ToolBar` | as `NewElement` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Rule.Edit` | Edit Rule | `ArrangementRulesGroup`, `Arrangement.Rule.Match.Control.Context.Menu` | `F2` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Rule.Group.Condition.Move.Down` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Rule.Group.Control.ToolBar` | as `Arrangement.Rule.Match.Condition.Move.Down` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Rule.Group.Condition.Move.Up` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Rule.Group.Control.ToolBar` | as `Arrangement.Rule.Match.Condition.Move.Up` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Rule.Match.Condition.Move.Down` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Rule.Match.Control.ToolBar` | `alt DOWN` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Rule.Match.Condition.Move.Up` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Rule.Match.Control.ToolBar` | `alt UP` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Rule.Remove` | Remove Rule | `ArrangementRulesGroup`, `Arrangement.Rule.Match.Control.Context.Menu`, `Arrangement.Rule.Match.Control.ToolBar` | as `$Delete` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `Arrangement.Rule.Section.Add` | (set in code) | `ArrangementRulesGroup`, `Arrangement.Rule.Match.Control.Context.Menu`, `Arrangement.Rule.Match.Control.ToolBar` |  | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `AutoIndentLines` | Auto-Indent Lines | `CodeFormatGroup` | `control alt I` | codeinsight/08-optimize-imports-and-reformat.md CI-OPT-032 | `platform/lang-impl` |
| `ChangeCodeStyleScheme` | Code Style Scheme | `ChangeScheme` |  | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `EditorIndentSelection` | Indent Selection | `EditSmartGroup` | `TAB` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/lang-impl` |
| `EditorUnindentSelection` | Unindent Line or Selection | `EditorActions`, `EditSmartGroup` | `shift TAB` | framework: codeinsight/08-optimize-imports-and-reformat.md (CI-OPT) | `platform/platform-impl` |
| `OptimizeImports` | Optimize Imports | `CommanderPopupMenu`, `HierarchyPopupMenu.Common`, `CodeFormatGroup` (+1) | `control alt O` | codeinsight/08-optimize-imports-and-reformat.md CI-OPT-013 | `platform/lang-impl` |
| `RearrangeCode` | Rearrange Code | `CodeFormatGroup` |  | codeinsight/08-optimize-imports-and-reformat.md CI-OPT-032 | `platform/lang-impl` |
| `ReformatCode` | Reformat Code | `CommanderPopupMenu`, `HierarchyPopupMenu.Common`, `CodeFormatGroup` (+2) | `control alt L` | codeinsight/08-optimize-imports-and-reformat.md CI-OPT-032 | `platform/lang-impl` |
| `ShowReformatFileDialog` | Reformat File… | `CodeFormatGroup` | `control shift alt L` | codeinsight/08-optimize-imports-and-reformat.md CI-OPT | `platform/lang-impl` |

## Settings pages (2)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `preferences.sourceCode` | Code Style | editor |  | project | MISSING |
| `editor.preferences.import` | Auto Import |  | preferences.editor | project | codeinsight/03-inspections.md CI-INS-105 |
