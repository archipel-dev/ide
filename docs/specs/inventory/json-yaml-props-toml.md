# JSON, YAML, properties and TOML

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (11)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `AddNewPropertyFile` | (set in code) | `NewGroup` |  | MISSING | `plugins/properties` |
| `CombinePropertiesFilesAction` | (set in code) | `ProjectViewPopupMenu` |  | MISSING | `plugins/properties` |
| `ConsoleView.ShowAsJsonAction` | Show as JSON | `ConsoleEditorPopupMenu` |  | MISSING | `json/backend` |
| `CreateResourceBundle` | (set in code) | `NewGroup` |  | MISSING | `plugins/properties` |
| `DissociateResourceBundleAction` | Dissociate Resource Bundle | `ProjectViewPopupMenu` |  | MISSING | `plugins/properties` |
| `EditPropertyValue` | Edit Property Value… |  | `alt F6` | MISSING | `plugins/java-i18n` |
| `JsonCopyPointer` | Copy JSON Pointer | `CutCopyPasteGroup`, `EditorTabPopupMenu`, `EditorPopupMenu` |  | MISSING | `json/backend` |
| `JsonJacksonReformatAction` | Reformat JSON | `ProjectViewPopupMenuModifyGroup`, `CodeFormatGroup` |  | MISSING | `json/backend` |
| `JsonPathEvaluateAction` | Evaluate JSONPath Expression… | `FindMenuGroup`, `EditorPopupMenu` |  | MISSING | `plugins/jsonpath` |
| `JsonPathExportEvaluateResultAction` | Export Result | `EditorContextBarMenu` | as `ExportToTextFile` | MISSING | `plugins/jsonpath` |
| `LoadDefaultYamlProfile` | Load Inspection Profile from YAML |  |  | MISSING | `platform/inspect` |

## Settings pages (2)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `settings.json.schema` | JSON Schema Mappings | preferences.externalResources |  | project | MISSING |
| `settings.json.schema.catalog` | Remote JSON Schemas | preferences.externalResources |  | project | MISSING |

## Status bar widgets (1)

| Id | Factory | Order | Spec |
|---|---|---|---|
| `JSONSchemaSelector` | `JsonSchemaStatusWidgetFactory` | after CodeStyleStatusBarWidget, before ReadOnlyAttribute | MISSING |
