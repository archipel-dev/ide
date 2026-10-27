# Test runner and coverage

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (10)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `AddToISuite` | Add to JUnit Pattern Suite | `junit.add.to.pattern.group` |  | MISSING | `plugins/junit` |
| `AddToTestNGSuite` | (set in code) | `addToTempGroup` |  | MISSING | `plugins/testng` |
| `ExportTestResults` | Export Test Results… |  |  | MISSING | `platform/testRunner` |
| `GenerateCoverageReport` | Generate Coverage Report | `CoverageMenu` |  | MISSING | `plugins/coverage-common` |
| `HideCoverage` | Hide Coverage | `CoverageMenu` |  | MISSING | `plugins/coverage-common` |
| `ImportCoverage` | Import External Coverage Report… |  |  | MISSING | `plugins/coverage-common` |
| `JUnit.ViewReportAsTestResults` | View Report as Test Results | `ProjectViewPopupMenu` |  | MISSING | `plugins/junit` |
| `SwitchCoverage` | Show Code Coverage Data | `CoverageMenu` | `control alt F6` | MISSING | `plugins/coverage-common` |
| `excludeFromSuite` | Exclude from Suite | `junit.exclude.group` |  | MISSING | `plugins/junit` |
| `excludeFromTestNGSuite` | (set in code) | `excludeGroup` |  | MISSING | `plugins/testng` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `coverage` | Coverage | build |  | project | MISSING |

## Tool windows (1)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Coverage` | Coverage | right | secondary | CoverageToolWindowFactory | MISSING |

## Run configuration types (2)

| Class | Type id | Display name | Plugin | Spec |
|---|---|---|---|---|
| `JUnitConfigurationType` | `JUnit` | JUnit | plugins/junit | MISSING |
| `TestNGConfigurationType` | `(see class)` | (see class) | plugins/testng | MISSING |
