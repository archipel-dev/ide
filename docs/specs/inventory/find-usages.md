# Find Usages and usage views

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (25)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `HighlightUsagesInFile` | Highlight Usages in File | `FindUsagesMenuGroup` | `control shift F7` | codeinsight/07-navigation.md | `platform/lang-impl` |
| `ShowSettingsAndFindUsages` | Find Usages Settings… | `FindUsagesMenuGroup` | `control shift alt F7` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `ShowUsageFeatures` | Show Usage Features |  |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView` |
| `ShowUsages` | Show Usages | `FindUsagesMenuGroup` | `control alt F7` | codeinsight/07-navigation.md | `platform/lang-impl` |
| `SliceBackward` | Analyze Data Flow to Here… | `AnalyzeActions` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `SliceForward` | Analyze Data Flow from Here… | `AnalyzeActions` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/lang-impl` |
| `UsageFiltering.Comments` | Show Usages in Comments | `UsageFilteringActionGroup` |  | codeinsight/07-navigation.md | `platform/platform-api` |
| `UsageFiltering.GeneratedCode` | Show Results In Generated Code | `UsageFilteringActionGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-api` |
| `UsageFiltering.Imports` | Show Import Statements | `UsageFilteringActionGroup` | `control I` | codeinsight/07-navigation.md | `platform/platform-api` |
| `UsageFiltering.ReadAccess` | Show Read Access | `UsageFilteringActionGroup` | `control R` | codeinsight/07-navigation.md | `platform/platform-api` |
| `UsageFiltering.WriteAccess` | Show Write Access | `UsageFilteringActionGroup` | `control W` | codeinsight/07-navigation.md | `platform/platform-api` |
| `UsageGrouping.Directory` | (set in code) | `UsageGroupingActionGroup` | `control alt P` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageGrouping.DirectoryStructure` | (set in code) | `UsageGroupingActionGroup` | `control alt D` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageGrouping.FileStructure` | (set in code) | `UsageGroupingActionGroup` | `control alt F` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageGrouping.FlattenModules` | (set in code) | `UsageGroupingActionGroup` | `control alt O` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageGrouping.Module` | (set in code) | `UsageGroupingActionGroup` | `control alt M` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageGrouping.Package` | (set in code) | `UsageGroupingActionGroup` | as `UsageGrouping.Directory` | framework: codeinsight/07-navigation.md (CI-NAV) | `java/java-backend` |
| `UsageGrouping.Scope` | (set in code) | `UsageGroupingActionGroup` |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageGrouping.ShortFilePath` | (set in code) |  |  | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageGrouping.UsageType` | (set in code) | `UsageGroupingActionGroup` | `control alt T` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageView.Exclude` | Exclude | `TreeNodeExclusion` | as `$Delete` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-api` |
| `UsageView.Include` | Include | `TreeNodeExclusion` | `INSERT` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/platform-api` |
| `UsageView.Remove` | Remove | `UsageView.Popup` | as `$Cut` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageView.Rerun` | Rerun | `UsageView.Popup` | as `Rerun` | framework: codeinsight/07-navigation.md (CI-NAV) | `platform/usageView-impl` |
| `UsageView.ShowRecentFindUsages` | Recent Find Usages | `UsageView.Popup` | as `ShowSearchHistory` | codeinsight/07-navigation.md CI-NAV-036 | `platform/lang-impl` |
