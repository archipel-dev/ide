# Code folding

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (30)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `CollapseAll` | Collapse All | `Bookmarks.ToolWindow.TitleActions`, `RunDashboardContentToolbar`, `ProjectViewToolbar` (+6) | `control SUBTRACT`, `control MINUS` | platform/08-language-extension-points.md PLAT-LANG-063 | `platform/platform-api` |
| `CollapseAllRegions` | Collapse All | `FoldingGroup` | `control shift SUBTRACT`, `control shift MINUS` | platform/08-language-extension-points.md PLAT-LANG-063 | `platform/lang-impl` |
| `CollapseBlock` | Fold Code Block | `FoldingGroup` | `control shift PERIOD` | MISSING | `platform/lang-impl` |
| `CollapseDocComments` | Collapse Doc Comments | `LanguageSpecificFoldingGroup` |  | MISSING | `platform/lang-impl` |
| `CollapseExpandableComponent` | Collapse Component |  | `shift ENTER`, `control SUBTRACT`, `control MINUS` | MISSING | `platform/platform-impl` |
| `CollapseRegion` | Collapse | `FoldingGroup` | `control SUBTRACT`, `control MINUS` | MISSING | `platform/lang-impl` |
| `CollapseRegionRecursively` | Collapse Recursively | `FoldingGroup` | `control alt SUBTRACT`, `control alt MINUS` | MISSING | `platform/lang-impl` |
| `CollapseSelection` | Fold Selection / Remove Region | `FoldingGroup` | `control PERIOD` | MISSING | `platform/lang-impl` |
| `CollapseTreeNode` | Collapse Tree Node |  | `SUBTRACT` | MISSING | `platform/platform-impl` |
| `ExpandAll` | Expand All | `Bookmarks.ToolWindow.TitleActions`, `RunDashboardContentToolbar`, `ProjectViewToolbar` (+6) | `control ADD`, `control EQUALS` | MISSING | `platform/platform-api` |
| `ExpandAllRegions` | Expand All | `FoldingGroup` | `control shift ADD`, `control shift EQUALS` | MISSING | `platform/lang-impl` |
| `ExpandAllToLevel1` | 1 | `ExpandAllToLevel` | `control shift MULTIPLY, 1`, `control shift MULTIPLY, NUMPAD1` | MISSING | `platform/lang-impl` |
| `ExpandAllToLevel2` | 2 | `ExpandAllToLevel` | `control shift MULTIPLY, 2`, `control shift MULTIPLY, NUMPAD2` | MISSING | `platform/lang-impl` |
| `ExpandAllToLevel3` | 3 | `ExpandAllToLevel` | `control shift MULTIPLY, 3`, `control shift MULTIPLY, NUMPAD3` | MISSING | `platform/lang-impl` |
| `ExpandAllToLevel4` | 4 | `ExpandAllToLevel` | `control shift MULTIPLY, 4`, `control shift MULTIPLY, NUMPAD4` | MISSING | `platform/lang-impl` |
| `ExpandAllToLevel5` | 5 | `ExpandAllToLevel` | `control shift MULTIPLY, 5`, `control shift MULTIPLY, NUMPAD5` | MISSING | `platform/lang-impl` |
| `ExpandCollapseToggleAction` | Toggle Folding | `FoldingGroup` |  | MISSING | `platform/lang-impl` |
| `ExpandDocComments` | Expand Doc Comments | `LanguageSpecificFoldingGroup` |  | MISSING | `platform/lang-impl` |
| `ExpandExpandableComponent` | Expand Component |  | `shift ENTER`, `control ADD`, `control EQUALS` | MISSING | `platform/platform-impl` |
| `ExpandLiveTemplateByTab` | Expand Live Template by Tab | `LookupActions` | `TAB` | MISSING | `platform/lang-impl` |
| `ExpandLiveTemplateCustom` | Expand Live Template / Emmet Abbreviation | `LookupActions` |  | MISSING | `platform/lang-impl` |
| `ExpandRecursively` | Expand Selected | `ProjectViewToolbar` | as `ExpandRegion` | MISSING | `platform/lang-impl` |
| `ExpandRegion` | Expand | `FoldingGroup` | `control ADD`, `control EQUALS` | MISSING | `platform/lang-impl` |
| `ExpandRegionRecursively` | Expand Recursively | `FoldingGroup` | `control alt ADD`, `control alt EQUALS` | MISSING | `platform/lang-impl` |
| `ExpandToLevel1` | 1 | `ExpandToLevel` | `control MULTIPLY, 1`, `control MULTIPLY, NUMPAD1` | MISSING | `platform/lang-impl` |
| `ExpandToLevel2` | 2 | `ExpandToLevel` | `control MULTIPLY, 2`, `control MULTIPLY, NUMPAD2` | MISSING | `platform/lang-impl` |
| `ExpandToLevel3` | 3 | `ExpandToLevel` | `control MULTIPLY, 3`, `control MULTIPLY, NUMPAD3` | MISSING | `platform/lang-impl` |
| `ExpandToLevel4` | 4 | `ExpandToLevel` | `control MULTIPLY, 4`, `control MULTIPLY, NUMPAD4` | MISSING | `platform/lang-impl` |
| `ExpandToLevel5` | 5 | `ExpandToLevel` | `control MULTIPLY, 5`, `control MULTIPLY, NUMPAD5` | MISSING | `platform/lang-impl` |
| `ExpandTreeNode` | Expand Tree Node |  | `ADD` | MISSING | `platform/platform-impl` |
