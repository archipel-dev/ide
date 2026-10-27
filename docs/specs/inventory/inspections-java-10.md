# Inspections: java (part 10 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Language injection (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `InjectionNotApplicable` | Injection annotation is not applicable | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LanguageMismatch` | Language mismatch | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnknownLanguage` | Unknown Language ID | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Logging (6)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `LoggingConditionDisagreesWithLogLevelStatement` | Log condition does not match logging call | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoggingGuardedByCondition` | Logging calls guarded by log condition | UAST | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoggingPlaceholderCountMatchesArgumentCount` | Number of placeholders does not match number of arguments in logging call | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoggingSimilarMessage` | Non-distinguishable logging calls | UAST | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoggingStringTemplateAsArgument` | String template as argument to logging call | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LogStatementNotGuardedByLogCondition` | Logging call not guarded by log condition | UAST | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Manifest (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `MissingFinalNewline` | Missing final new line | Manifest | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MisspelledHeader` | Unknown or misspelled header name | Manifest | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Method (9)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `BooleanMethodNameMustStartWithQuestion` | Boolean method name must start with question word | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LambdaUnfriendlyMethodOverload` | Lambda-unfriendly method overload | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodNameSameAsClassName` | Method name same as class name | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodNameSameAsParentName` | Method name same as parent class name | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MisspelledMethodName` | Method names differing only by case | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NewMethodNamingConvention` | Method naming convention | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonBooleanMethodNameMayNotStartWithQuestion` | Non-boolean method name must not start with question word | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverloadedMethodsWithSameNumberOfParameters` | Overloaded methods with same number of parameters | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverloadedVarargsMethod` | Overloaded varargs method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Nullability problems (4)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `NotNullFieldNotInitialized` | Non-null field is not initialized | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NotNullParameterReceivesNull` | Non-null parameter that receives 'null' as an argument | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NullableProblems` | Nullability problems | JAVA | true | WARNING | local | codeinsight/03-inspections.md CI-INS-105 |
| `ReturnNull` | Return of 'null' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Pattern validation (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `PatternNotApplicable` | Pattern annotation is not applicable | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PatternOverriddenByNonAnnotatedMethod` | Non-annotated Method overrides @Pattern Method | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PatternValidation` | Validate annotated patterns | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Properties files (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `InconsistentResourceBundle` | Inconsistent resource bundle | Properties | true | ERROR | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousLocalesLanguages` | Suspicious resource bundle locale languages | Properties | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedMessageFormatParameter` | Missing message format parameter | Properties | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Test frameworks (7)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AssertBetweenInconvertibleTypes` | 'assertEquals()' between objects of inconvertible types | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TestCaseWithConstructor` | TestCase with non-trivial constructors | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TestCaseWithNoTestMethods` | Test class without tests | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TestFailedLine` | Failed line in test | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TestInProductSource` | Test in product source | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TestMethodWithoutAssertion` | Test method without assertions | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TestOnlyProblems` | Test-only usage in production code | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## XML (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DeprecatedClassUsageInspection` | Deprecated API usage in XML | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
