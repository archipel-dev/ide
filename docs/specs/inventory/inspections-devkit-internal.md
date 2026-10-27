# Inspections: devkit-internal

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Code (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `HighlightVisitorInternal` | Good code is red |  | false | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OutdatedIntelliJPlatformGradlePluginVersion` | Outdated IntelliJ Platform Gradle Plugin version |  | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Plugin descriptor (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ToolWindowStripeTitle` | Tool window stripe title in the wrong message bundle |  | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
