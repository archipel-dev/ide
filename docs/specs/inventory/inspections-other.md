# Inspections: other

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Directive isn't supported (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `UnsupportedDirective` | Directive isn't supported | Mermaid | true | ERROR | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## DotEnv (11)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DotEnvDuplicateKeyInspection` | Duplicate keys | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DotEnvExtraBlankLineInspection` | Extra blank line | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DotEnvIncorrectDelimiterInspection` | Incorrect delimiter | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DotEnvLeadingCharacterInspection` | Invalid leading character | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DotEnvLowercaseKeyInspection` | Key uses lowercase characters | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DotEnvSpaceAroundSeparatorInspection` | Extra spaces surrounding '=' | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DotEnvSpaceInsideNonQuotedInspection` | Space inside non-quoted value | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DotEnvTrailingWhitespaceInspection` | Value has trailing whitespace | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptyNestedVariableInspection` | Reference to variable is empty | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestedVariableOutsideDoubleQuotesInspection` | Nested variables must not appear outside double quotes | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UndefinedNestedVariableInspection` | Reference to nested variable ''…'' is undefined | DotEnv | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## EditorConfig (29)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `EditorConfigCharClassLetterRedundancy` | Duplicate character class letter | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigCharClassRedundancy` | Unnecessary character class | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigDeprecatedDescriptor` | Deprecated property | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigEmptyHeader` | Empty header | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigEmptySection` | Empty section | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigEncoding` | File encoding doesn't match EditorConfig charset |  | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigHeaderUniqueness` | EditorConfig section is not unique | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigKeyCorrectness` | Unknown property | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigListAcceptability` | Unexpected value list | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigMissingRequiredDeclaration` | Required declarations are missing | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigNoMatchingFiles` | No matching files | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigNumerousWildcards` | Too many wildcards | EditorConfig | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigOptionRedundancy` | Redundant property | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigPairAcceptability` | Unexpected key-value pair | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigPartialOverride` | Overlapping sections | EditorConfig | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigPatternEnumerationRedundancy` | Unnecessary braces | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigPatternRedundancy` | Duplicate or redundant pattern | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigReferenceCorrectness` | Invalid reference | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigRootDeclarationCorrectness` | Unexpected top-level declaration | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigRootDeclarationUniqueness` | Extra top-level declaration | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigShadowedOption` | Overridden property | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigShadowingOption` | Overriding property | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigSpaceInHeader` | Space in file pattern | EditorConfig | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigUnexpectedComma` | Unexpected comma | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigUnusedDeclaration` | Unused declaration | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigValueCorrectness` | Invalid property value | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigValueUniqueness` | Non-unique list value | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigVerifyByCore` | Invalid .editorconfig file | EditorConfig | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EditorConfigWildcardRedundancy` | Redundant wildcard | EditorConfig | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## General (10)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Annotator` | Annotator |  | true | ERROR | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptyDirectory` | Empty directory |  | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `InconsistentLineSeparators` | Inconsistent line separators |  | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncorrectFormatting` | Incorrect formatting |  | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InjectedReferences` | Injected references |  | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LongLine` | Line is longer than allowed by code style |  | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ProblematicWhitespace` | Problematic whitespace |  | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReassignedToPlainText` | Reassigned to plain text |  | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantSuppression` | Redundant suppression |  | true | WARNING | global | codeinsight/03-inspections.md CI-INS-105 |
| `TodoComment` | TODO comment |  | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Generic won't be rendered (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `UndeclaredGeneric` | Generic won't be rendered | Mermaid | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Git Graph inspection (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `GitGraph` | Git Graph inspection | Mermaid | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Gradle / Probable bugs (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `BintrayPublishingPlugin` | Bintray publishing plugin may stop working on May 1st, 2021 | Groovy | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JCenterRepository` | Builds will no longer be able to resolve artifacts from JCenter after February 1st, 2022 | Groovy | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MultipleRepositoryUrls` | Multiple repository urls | Groovy | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Internationalization (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `LossyEncoding` | Lossy encoding |  | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonAsciiCharacters` | Non-ASCII characters |  | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## JSONPath (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `JsonPathEvaluateUnknownKey` | Unknown property key used for JSONPath evaluate expression | JSONPath | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JsonPathUnknownFunction` | Unknown JSONPath function | JSONPath | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JsonPathUnknownOperator` | Unknown JSONPath operator | JSONPath | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Proofreading (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `GrazieInspection` | Grammar |  | true | GRAMMAR_ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `GrazieInspectionRunner` | Grammar |  | true | NON_SWITCHABLE_WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `GrazieStyle` | Style |  | true | STYLE_SUGGESTION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LanguageDetectionInspection` | Natural language detection |  | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SpellCheckingInspection` | Spelling |  | true | TYPO | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Structural search (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `SSBasedInspection` | Structural search inspection |  | true | NON_SWITCHABLE_WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Theme inspection (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ThemeInspection` | Theme inspection | Mermaid | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Unused attribute (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `UnusedAttribute` | Unused attribute | Mermaid | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Version control (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `IgnoreFileDuplicateEntry` | Ignore file duplicates | IgnoreLang | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
