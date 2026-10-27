# Inspections: java (part 9 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Java / Verbose or redundant code constructs (44)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CastCanBeRemovedNarrowingVariableType` | Too weak variable type leads to unnecessary cast | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CastCanBeReplacedWithVariable` | Cast can be replaced with variable | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConditionCoveredByFurtherCondition` | Condition is covered by further condition | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DuplicateBranchesInSwitch` | Duplicate branches in 'switch' | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DuplicateExpressions` | Multiple occurrences of the same expression | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExcessiveLambdaUsage` | Excessive lambda usage | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExcessiveRangeCheck` | Excessive range check | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExplicitArrayFilling` | Explicit array filling | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LombokGetterMayBeUsed` | Lombok @Getter may be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LombokSetterMayBeUsed` | Lombok @Setter may be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ManualMinMaxCalculation` | Manual min/max calculation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MappingBeforeCount` | Mapping call before count() | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonStrictComparisonCanBeEquality` | Non-strict inequality '&gt;=' or '&lt;=' can be replaced with '==' | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObviousNullCheck` | Null-check method is called with obviously non-null argument | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OnlyOneElementUsed` | Only one element is used | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantArrayCreation` | Redundant array creation | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantCast` | Redundant type cast | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-033 |
| `RedundantClassCall` | Redundant 'isInstance()' or 'cast()' call | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantCollectionOperation` | Redundant 'Collection' operation | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantComparatorComparing` | Comparator method can be simplified | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantCompareCall` | Redundant 'compare()' method call | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-048 |
| `RedundantEmbeddedExpression` | Redundant embedded expression in string template | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantEscapeInRegexReplacement` | Redundant escape in regex replacement string | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantFileCreation` | Redundant 'File' instance creation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantJavaTimeOperations` | Redundant operation on 'java.time' object | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantLengthCheck` | Redundant array length check | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantStreamOptionalCall` | Redundant step in 'Stream' or 'Optional' call chain | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantTypeArguments` | Redundant type arguments | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-036 |
| `RedundantUnmodifiable` | Redundant usage of unmodifiable collection wrappers | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceOnLiteralHasNoEffect` | Replacement operation has no effect | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyCollector` | Simplifiable collector | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyOptionalCallChains` | Optional call chain can be simplified | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyStreamApiCallChains` | Stream API call chain can be simplified | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringBufferReplaceableByString` | 'StringBuilder' can be replaced with 'String' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringOperationCanBeSimplified` | Redundant 'String' operation | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-048 |
| `TrivialStringConcatenation` | Concatenation with empty string | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryBreak` | Unnecessary 'break' statement | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryContinue` | Unnecessary 'continue' statement | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryDefault` | Unnecessary 'default' for enum 'switch' statement | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryLabelOnBreakStatement` | Unnecessary label on 'break' statement | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryLabelOnContinueStatement` | Unnecessary label on 'continue' statement | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryReturn` | Unnecessary 'return' statement | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-043 |
| `UnnecessaryStringEscape` | Unnecessarily escaped character | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnreachableCatch` | Unreachable catch section | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-030 |

## Java / Visibility (18)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AmbiguousFieldAccess` | Access to inherited field looks like access to element from surrounding code | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `AmbiguousMethodCall` | Call to inherited method looks like call to local method | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `AnonymousClassVariableHidesContainingMethodVariable` | Anonymous class variable hides variable in containing method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassEscapesItsScope` | Class is exposed outside of its visibility scope | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FieldHidesSuperclassField` | Subclass field hides superclass field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InnerClassVariableHidesOuterClassVariable` | Inner class field hides outer class field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Java9ModuleExportsPackageToItself` | Module exports/opens package to itself | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Java9UndeclaredServiceUsage` | Usage of service not declared in 'module-info' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaEmptyModuleInfoFile` | Empty 'module-info.java' file | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LambdaParameterHidingMemberVariable` | Lambda parameter hides field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LocalVariableHidingMemberVariable` | Local variable hides field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodOverloadsParentMethod` | Possibly unintended overload of method from superclass | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodOverridesInaccessibleMethodOfSuper` | Method overrides inaccessible method of superclass | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodOverridesStaticMethod` | Method tries to override 'static' method of superclass | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ParameterHidingMemberVariable` | Parameter hides field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PatternVariableHidesField` | Pattern variable hides field | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicConstructorInNonPublicClass` | 'public' constructor in non-public class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TypeParameterHidesVisibleType` | Type parameter hides visible type | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / toString() issues (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassHasNoToStringMethod` | Class does not override 'toString()' method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FieldNotUsedInToString` | Field not used in 'toString()' method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 10 (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `RedundantExplicitVariableType` | Local variable type can be omitted | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `VariableTypeCanBeExplicit` | Variable type can be explicit | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 11 (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ReadWriteStringCanBeUsed` | 'Files.readString()' or 'Files.writeString()' can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 14 (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `EnhancedSwitchBackwardMigration` | Enhanced 'switch' | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EnhancedSwitchMigration` | Statement can be replaced with enhanced 'switch' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 15 (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `TextBlockBackwardMigration` | Text block can be replaced with regular string literal | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TextBlockMigration` | Text block can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 16 (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassCanBeRecord` | Class can be record class | JAVA | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `PatternVariableCanBeUsed` | Pattern variable can be used | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 21 (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DeconstructionCanBeUsed` | Record pattern can be used | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `MathClampMigration` | Clamp function can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SequencedCollectionMethodCanBeUsed` | SequencedCollection method can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringTemplateMigration` | String template can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringTemplateReverseMigration` | String template can be concatenated string | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 23 (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `MarkdownDocumentationCommentsMigration` | Javadoc comment can be Markdown documentation comment | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 25 (4)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ExplicitToImplicitClassMigration` | Explicit class declaration can be converted into a compact source file | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ImplicitToExplicitClassBackwardMigration` | Compact source file can be replaced with a regular class | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MigrateFromJavaLangIo` | Method from 'System.out' can be used instead of 'java.lang.IO' | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MigrateToJavaLangIo` | Method from 'java.lang.IO' class can be used | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 26 (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ComparatorMinMaxCanBeUsed` | 'Comparator.min()' or 'Comparator.max()' can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 5 (10)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `BigDecimalLegacyMethod` | 'BigDecimal' legacy method called | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `CollectionsFieldAccessReplaceableByMethodCall` | Reference to empty collection field can be replaced with method call | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForCanBeForeach` | 'for' loop can be replaced with enhanced for loop | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-038 |
| `IndexOfReplaceableByContains` | 'String.indexOf()' expression can be replaced with 'contains()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodCanBeVariableArityMethod` | Method can have varargs parameter | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RawUseOfParameterizedType` | Raw use of parameterized class | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-055 |
| `StringBufferReplaceableByStringBuilder` | 'StringBuffer' may be 'StringBuilder' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryBoxing` | Unnecessary boxing | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-034 |
| `UnnecessaryUnboxing` | Unnecessary unboxing | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-034 |
| `WhileCanBeForeach` | 'while' loop can be replaced with enhanced 'for' loop | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 7 (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Convert2Diamond` | Explicit type can be replaced with '&lt;&gt;' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EqualsReplaceableByObjectsCall` | 'equals()' expression replaceable by 'Objects.equals()' expression | JAVA | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SafeVarargsDetector` | Possible heap pollution from parameterized vararg type | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TryFinallyCanBeTryWithResources` | 'try finally' can be replaced with 'try' with resources | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TryWithIdenticalCatches` | Identical 'catch' branches in 'try' statement | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 8 (18)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Anonymous2MethodRef` | Anonymous type can be replaced with method reference | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AnonymousHasLambdaAlternative` | Anonymous type has shorter lambda alternative | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CodeBlock2Expr` | Statement lambda can be replaced with expression lambda | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-039 |
| `ComparatorCombinators` | 'Comparator' combinator can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Convert2Lambda` | Anonymous type can be replaced with lambda | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Convert2MethodRef` | Lambda can be replaced with method reference | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-039 |
| `Convert2streamapi` | Loop can be collapsed with Stream API | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FoldExpressionIntoStream` | Expression can be folded into Stream chain | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Guava` | Guava's functional primitives can be replaced with Java | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Java8CollectionRemoveIf` | Loop can be replaced with 'Collection.removeIf()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Java8ListReplaceAll` | Loop can be replaced with 'List.replaceAll()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Java8ListSort` | 'Collections.sort()' can be replaced with 'List.sort()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Java8MapApi` | Simplifiable 'Map' operations | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Java8MapForEach` | 'Map.forEach()' can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LambdaCanBeMethodCall` | Lambda can be replaced with method call | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyForEach` | Simplifiable 'forEach()' call | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticPseudoFunctionalStyleMethod` | Guava pseudo-functional call can be converted to Stream API call | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseHashCodeMethodInspection` | Standard 'hashCode()' method can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java 9 (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Java9CollectionFactory` | Immutable collection creation can be replaced with collection factory call | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaRequiresAutoModule` | Dependencies on automatic modules | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceNullCheck` | Null check can be replaced with method call | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java,Lombok / Redundant definitions (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `RedundantSlf4jDefinition` | @Slf4j | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedLoggerLombok` | Unused logger annotation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java,Lombok / Redundant modifiers (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `RedundantModifiersUtilityClassLombok` | @UtilityClass modifiers | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantModifiersValLombok` | Unnecessary final before 'val' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantModifiersValueLombok` | @Value modifiers | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Java interop issues (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `KotlinInternalInJava` | Usage of Kotlin internal declarations from Java | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinJvmAnnotationInJava` | Kotlin JVM annotation in Java | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinSealedInheritorsInJava` | Inheritance of Kotlin sealed interface/class from Java | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Other problems (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `EnumValuesSoftDeprecateInJava` | 'Enum.values()' is recommended to be replaced by 'Enum.getEntries()' since Kotlin 1.9 | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseNamedGetter` | Use named getter | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
