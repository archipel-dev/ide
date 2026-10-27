# Structure view and file structure popup

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (1)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `FileStructurePopup` | File Structure | `GoToCodeGroup` | `control F12` | platform/08-language-extension-points.md PLAT-LANG-084 | `platform/structure-view-impl` |

## Tool windows (1)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Structure` | Structure | left | secondary | StructureViewToolWindowFactory | MISSING |
