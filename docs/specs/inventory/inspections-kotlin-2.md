# Inspections: kotlin (part 2 of 4)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Kotlin / Probable bugs (64)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AddExplicitTargetToParameterAnnotation` | Redundant interpolation prefix | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AmbiguousNonLocalJump` | Ambiguous non-local 'break' or 'continue' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ArrayInDataClass` | Array property in data or value class | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssignedValueIsNeverRead` | Assigned value is never read | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CanSealedSubClassBeObject` | Sealed subclass without state and overridden equals | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConflictingExtensionProperty` | Extension property conflicting with synthetic one | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertNameBasedDestructuringShortFormToFull` | Convert name-based destructuring short form to full | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertNaNEquality` | Convert equality check with 'NaN' to 'isNaN' call | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DataClassPrivateConstructor` | Private data class constructor is exposed via the 'copy' method | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DelegationToVarProperty` | Delegating to 'var' property | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DestructingShortFormNameMismatch` | Declaration name mismatch in destructuring declaration | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DuplicateArgumentsInSetOfAndMapOfFunctions` | Duplicate arguments in 'setOf' and 'mapOf' functions | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptyRange` | Range with start greater than endInclusive is empty | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EqualsOrHashCode` | 'equals()' and 'hashCode()' not paired | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EscapingCapturedVariable` | Escaped captured variable | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Fastjson2MissingKotlinReflect` | The kotlin-reflect library is required for fastjson2-kotlin | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FilterIsInstanceResultIsAlwaysEmpty` | The result of 'filterIsInstance' call is always empty | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForEachParameterNotUsed` | Iterated elements are not used in forEach | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ImplicitNullableNothingType` | Implicit 'Nothing?' type | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncompleteDestructuring` | Incomplete destructuring declaration | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaIoSerializableObjectMustHaveReadResolve` | Serializable object must implement 'readResolve' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KDocUnresolvedReference` | Unresolved link in KDoc | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinArrayHashCode` | 'hashCode()' called on array | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinArrayToString` | 'toString()' called on array | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinConstantConditions` | Constant conditions | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinCovariantEquals` | Covariant 'equals()' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinEqualsBetweenInconvertibleTypes` | 'equals()' between objects of inconvertible types | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinMisorderedAssertEqualsArguments` | Misordered 'assertEquals()' arguments | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinScriptTemplateNotRegistered` | Kotlin script definition is not set up | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinThrowableNotThrown` | Throwable not thrown | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinUnreachableCode` | Unreachable code | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LateinitVarOverridesLateinitVar` | 'lateinit var' property overrides 'lateinit var' property | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LazyWithoutDelegation` | 'lazy' can be used with delegation | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MainFunctionReturnUnit` | Main function should return 'Unit' | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `MainKtsAnnotationUsageInPlainKts` | Using .main.kts annotation in plain .kts script | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonNullableBooleanPropertyInExternalInterface` | External interface contains non-nullable boolean property | kotlin | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonVarPropertyInExternalInterface` | External interface contains val property | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectInheritsException` | Exception should not be an object | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RecursiveEqualsCall` | Recursive equals call | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RecursivePropertyAccessor` | Recursive property accessor | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantDestructuringRename` | Redundant name-based destructuring rename | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantDestructuringUnderscore` | Redundant underscore entry in name-based destructuring | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantLabel` | Redundant label | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SelfAssignment` | Redundant assignment | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SelfReferenceConstructorParameter` | Constructor can never be complete | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SetterBackingFieldAssignment` | Existing backing field without assignment | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringReferentialEquality` | String compared with referential equality | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousCallOnCollectionToAddOrRemovePath` | Suspicious call to add or remove Iterable argument | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousCascadingIf` | Suspicious cascading 'if' expression | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousCollectionReassignment` | Augmented assignment creates a new collection under the hood | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousEqualsCombination` | Suspicious combination of == and === | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousGetterForMutableObject` | Suspicious getter: it returns a new mutable object on each access | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousJavaClassCallableReference` | Suspicious '::javaClass' callable reference | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousVarProperty` | Suspicious 'var' property: its setter does not influence its getter result | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedDataClassCopyResult` | Unused result of data class copy | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedEquals` | Unused equals expression | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedExpression` | Unused expression | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedLambdaExpression` | Unused lambda expression | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedLambdaExpressionBody` | Unused return value of a function with lambda expression body | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedUnaryOperator` | Unused unary operator | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UselessCallOnCollection` | Redundant call on collection type | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UselessCallOnNotNull` | Redundant call on not-null type | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `VariableNeverRead` | Variable is never read | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WrapUnaryOperator` | Ambiguous unary operator use with number constant | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
