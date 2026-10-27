# Other languages and text support (TextMate, EditorConfig, regexp, env files)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (13)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ChangeTemplateDataLanguage` | Change Template Data Language | `FilePropertiesGroup`, `EditorPopupMenu`, `EditorTabPopupMenu` |  | MISSING | `platform/lang-impl` |
| `ConvertIndentsToSpaces` | To Spaces | `ConvertIndentsGroup` |  | MISSING | `platform/platform-impl` |
| `ConvertIndentsToTabs` | To Tabs | `ConvertIndentsGroup` |  | MISSING | `platform/platform-impl` |
| `CreateEditorConfigFile` | EditorConfig File | `NewGroup` |  | MISSING | `plugins/editorconfig` |
| `EditMacros` | Edit Macros | `StandardMacroActions` |  | MISSING | `plugins/macros` |
| `GenerateCopyright` | Copyright | `GenerateGroup` |  | MISSING | `plugins/copyright` |
| `Mermaid.ExportDiagram` | Export Mermaid Diagram | `EditorPopupMenu`, `ProjectViewPopupMenu` |  | MISSING | `plugins/mermaid` |
| `NewEditorConfigFile` | (set in code) |  |  | MISSING | `plugins/editorconfig` |
| `PlaySavedMacrosAction` | Play Saved Macros… | `StandardMacroActions` |  | MISSING | `plugins/macros` |
| `PlaybackLastMacro` | Play Back Last Macro | `StandardMacroActions` |  | MISSING | `plugins/macros` |
| `StartStopMacroRecording` | Start/Stop Macro Recording | `StandardMacroActions` |  | MISSING | `plugins/macros` |
| `UpdateCopyright` | Update Copyright… | `ProjectViewPopupMenu`, `CodeMenu`, `NavbarPopupMenu` |  | MISSING | `plugins/copyright` |
| `org.editorconfig.configmanagement.generate.EditorConfigGenerateLanguagePropertiesAction` | Language Properties | `GenerateGroup` |  | MISSING | `plugins/editorconfig` |

## Settings pages (5)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `DotEnv.SettingsForm` | DotEnv | language |  | application | MISSING |
| `Settings.Mermaid` | Mermaid | language |  | application | MISSING |
| `reference.settingsdialog.textmate.bundles` | TextMate Bundles | editor |  | application | MISSING |
| `IntelliLang.Configuration` | Language Injections | editor |  | project | MISSING |
| `copyright` | Copyright | editor |  | project | MISSING |
