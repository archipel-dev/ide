# Intentions: json-yaml-props-toml

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Intentions (9)

Description = first sentence of the shipped description resource, if any.

| Class | Language | Category | Description | Spec |
|---|---|---|---|---|
| `AddOptionalPropertiesIntention` | JSON | JSON | Inserts all missing properties listed in the corresponding JSON-schema. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `JsonSortPropertiesIntention` | JSON | JSON | Sorts the properties of a JSON object alphabetically. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `CopyPropertyKeyToClipboardIntention` | Properties | Properties | Copies a property key to the clipboard. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `CopyPropertyValueToClipboardIntention` | Properties | Properties | Copies a property value to the clipboard. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `TomlExpandInlineTableIntention` | TOML | TOML | Expands table property with inline table into separate table. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `YAMLAddOptionalPropertiesIntention` | yaml | YAML | Inserts all missing properties listed in the corresponding JSON-schema. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `YAMLExpandAllCollectionsInsideIntentionAction` | yaml | YAML | Expand collection and all its child collections from JSON format to base multiline YAML format | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `YAMLExpandCollectionIntentionAction` | yaml | YAML | Expand collection from JSON format to base multiline YAML format Action also expands all parent collections | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `YAMLInlineCollectionIntentionAction` | yaml | YAML | Inline collection to JSON format | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
