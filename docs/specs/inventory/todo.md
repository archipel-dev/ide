# TODO tool window

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (4)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `TodoViewEditSource` | Jump to Source |  | as `EditSource` | MISSING | `platform/todo` |
| `TodoViewGroupByFlattenPackage` | (set in code) | `TodoViewGroupByGroup` | `control alt C` | MISSING | `platform/todo` |
| `TodoViewGroupByShowModules` | (set in code) | `TodoViewGroupByGroup` | `control alt M` | MISSING | `platform/todo` |
| `TodoViewGroupByShowPackages` | (set in code) | `TodoViewGroupByGroup` | `control alt P` | MISSING | `platform/todo` |

## Settings pages (1)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `preferences.toDoOptions` | TODO | editor |  | application | MISSING |

## Tool windows (1)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `TODO` | TODO | bottom |  | TodoToolWindowFactory | MISSING |
