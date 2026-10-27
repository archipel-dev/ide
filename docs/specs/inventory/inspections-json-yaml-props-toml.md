# Inspections: json-yaml-props-toml

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Code (4)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `PluginKeysAreProhibitedInPlatformRegistry` | Plugin keys defined in platform regitry.properties | Properties | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnresolvedIntelliJThemeKey` | Unresolved theme key | JSON | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnresolvedIntelliJThemeNamedColor` | Unresolved theme named color | JSON | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedIntelliJThemeNamedColor` | Unused theme named color | JSON | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Gradle / Best practices (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `LatestMinorVersion` | Latest Gradle minor version | Properties | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Gradle / Style (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `UnusedVersionCatalogEntry` | Unused version catalog entry | TOML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## JSON and JSON5 (6)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Json5StandardCompliance` | Compliance with JSON5 standard | JSON5 | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JsonDuplicatePropertyKeys` | Duplicate keys in object literals | JSON | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JsonSchemaCompliance` | Compliance with JSON schema | JSON | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JsonSchemaDeprecation` | Deprecated JSON property | JSON | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JsonSchemaRefReference` | Unresolved '$ref' and '$schema' references | JSON | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JsonStandardCompliance` | Compliance with JSON standard | JSON | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Plugin DevKit (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DevKitPropertiesMessageValidation` | Message format validation in properties files | Properties | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnknownIdInMetaInformation` | Unknown inspection id in meta information | JSON | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Properties files (7)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AlphaUnsortedPropertiesFile` | Properties file or resource bundle is alphabetically unsorted | Properties | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DuplicatePropertyInspection` | Duplicate property | Properties | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncompleteProperty` | Property is incomplete | Properties | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TrailingSpacesInProperty` | Trailing spaces in property | Properties | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedProperty` | Unused property | Properties | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseEllipsisInPropertyInspection` | Three dot characters instead of the ellipsis | Properties | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WrongPropertyKeyValueDelimiter` | Property key/value delimiter doesn't match code style settings | Properties | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## TOML (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `TomlUnresolvedReference` | Unresolved reference | TOML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## YAML (7)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `YAMLDuplicatedKeys` | Duplicated YAML keys | yaml | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `YAMLIncompatibleTypes` | Suspicious type mismatch | yaml | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `YAMLRecursiveAlias` | Recursive alias | yaml | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `YAMLSchemaDeprecation` | Deprecated YAML key | yaml | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `YAMLSchemaValidation` | Validation by JSON Schema | yaml | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `YAMLUnresolvedAlias` | Unresolved alias | yaml | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `YAMLUnusedAnchor` | Unused anchor | yaml | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
