# Inspections: languages-misc

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## RegExp (16)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CustomRegExpInspection` | Custom RegExp inspection |  | true | NON_SWITCHABLE_WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpAnonymousGroup` | Anonymous capturing group or numeric back reference | RegExp | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpDuplicateAlternationBranch` | Duplicate branch in alternation | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpDuplicateCharacterInClass` | Duplicate character in character class | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpEmptyAlternationBranch` | Empty branch in alternation | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpEscapedMetaCharacter` | Escaped meta character | RegExp | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpOctalEscape` | Octal escape | RegExp | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpRedundantClassElement` | Redundant '\\d', '[:digit:]', or '\\D' class elements | RegExp | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpRedundantEscape` | Redundant character escape | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpRedundantNestedCharacterClass` | Redundant nested character class | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpRepeatedSpace` | Consecutive spaces | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpSimplifiable` | Regular expression can be simplified | RegExp | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpSingleCharAlternation` | Single character alternation | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpSuspiciousBackref` | Suspicious back reference | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpUnexpectedAnchor` | Begin or end anchor in unexpected position | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RegExpUnnecessaryNonCapturingGroup` | Unnecessary non-capturing group | RegExp | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
