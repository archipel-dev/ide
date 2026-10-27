# Inspections: java (part 3 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Java / Code style issues (80)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ArrayCanBeReplacedWithEnumValues` | Array can be replaced with enum values | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ArrayCreationWithoutNewKeyword` | Array creation without 'new' expression | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssertMessageNotString` | 'assert' message is not a string | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BlockMarkerComments` | Block marker comment | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BoundedWildcard` | Can use bounded wildcard | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CallToStringConcatCanBeReplacedByOperator` | Call to 'String.concat()' can be replaced with '+' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ChainedEquality` | Chained equality comparisons | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ChainedMethodCall` | Chained method calls | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CharsetObjectCanBeUsed` | Standard 'Charset' object can be used | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConditionalCanBeOptional` | Conditional can be replaced with Optional | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConfusingOctalEscape` | Confusing octal escape sequence | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantExpression` | Constant expression can be evaluated | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantOnWrongSideOfComparison` | Constant on wrong side of comparison | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ControlFlowStatementWithoutBraces` | Control flow statement without braces | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `CStyleArrayDeclaration` | C-style array declaration | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `DiamondCanBeReplacedWithExplicitTypeArguments` | Diamond can be replaced with explicit type arguments | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EqualsCalledOnEnumConstant` | 'equals()' called on enum value | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `EscapedSpace` | Non-terminal use of '\\s' escape sequence | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtendsObject` | Class explicitly extends 'Object' | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-045 |
| `FieldMayBeFinal` | Field may be 'final' | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-050 |
| `FillPermitsList` | Same file subclasses are missing from permits clause of a sealed class | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FuseStreamOperations` | Subsequent steps can be fused into Stream API chain | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ImplicitCallToSuper` | Implicit call to 'super()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JoinDeclarationAndAssignmentJava` | Assignment can be joined with declaration | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LambdaBodyCanBeCodeBlock` | Lambda body can be code block | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LambdaCanBeReplacedWithAnonymous` | Lambda can be replaced with anonymous class | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LambdaParameterTypeCanBeSpecified` | Lambda parameter type can be specified | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ListIndexOfReplaceableByContains` | 'List.indexOf()' expression can be replaced with 'contains()' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `LiteralAsArgToStringEquals` | String literal may be 'equals()' qualifier | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `LocalCanBeFinal` | Local variable or parameter can be 'final' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodRefCanBeReplacedWithLambda` | Method reference can be replaced with lambda | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissortedModifiers` | Missorted modifiers | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `MoveFieldAssignmentToInitializer` | Field assignment can be moved to initializer | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MultiCatchCanBeSplit` | Multi-catch can be split into separate catch blocks | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MultipleVariablesInDeclaration` | Multiple variables in one declaration | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestedMethodCall` | Nested method call | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectsEqualsCanBeSimplified` | 'Objects.equals()' can be replaced with 'equals()' | JAVA | true | INFORMATION | local | java/07-inspections.md JAVA-INS-048 |
| `OptionalContainsCollection` | 'Optional' contains array or collection | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OptionalIsPresent` | Non functional style 'Optional.isPresent()' usage | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-022 |
| `OptionalToIf` | 'Optional' can be replaced with sequence of 'if' statements | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PatternVariablesCanBeReplacedWithCast` | Using 'instanceof' with patterns | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReassignedVariable` | Reassigned variable | JAVA | true | TEXT ATTRIBUTES | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RecordCanBeClass` | Record can be converted to class | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantFieldInitialization` | Redundant field initialization | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantLabeledSwitchRuleCodeBlock` | Labeled switch rule has redundant code block | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReturnSeparatedFromComputation` | 'return' separated from the result computation | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReturnThis` | Return of 'this' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableAnnotation` | Simplifiable annotation | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableEqualsExpression` | Unnecessary 'null' check before 'equals()' call | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableIfStatement` | 'if' statement can be replaced with conditional or boolean expression | JAVA | true | INFORMATION | local+cleanup | java/07-inspections.md JAVA-INS-040 |
| `SingleElementAnnotation` | Non-normalized annotation | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SingleStatementInBlock` | Code block contains single statement | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SizeReplaceableByIsEmpty` | 'size() == 0' can be replaced with 'isEmpty()' | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-037 |
| `StreamToLoop` | Stream API call chain can be replaced with loop | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SwitchLabeledRuleCanBeCodeBlock` | Labeled switch rule can have code block | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TrailingWhitespacesInTextBlock` | Trailing whitespace in text block | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TryStatementWithMultipleResources` | 'try' statement with multiple resources can be split | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TypeParameterExtendsObject` | Type parameter explicitly extends 'Object' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnclearBinaryExpression` | Multiple operators with different precedence | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessarilyQualifiedInnerClassAccess` | Unnecessarily qualified inner class access | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessarilyQualifiedStaticallyImportedElement` | Unnecessarily qualified statically imported element | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessarilyQualifiedStaticUsage` | Unnecessarily qualified static access | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryBlockStatement` | Unnecessary code block | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryCallToStringValueOf` | Unnecessary conversion to 'String' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryConstantArrayCreationExpression` | Redundant 'new' expression in constant array creation | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryConstructor` | Redundant no-arg constructor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryFinalOnLocalVariableOrParameter` | Unnecessary 'final' on local variable or parameter | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryFullyQualifiedName` | Unnecessary fully qualified name | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryModifier` | Unnecessary modifier | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-045 |
| `UnnecessaryParentheses` | Unnecessary parentheses | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryQualifierForThis` | Unnecessary qualifier for 'this' or 'super' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessarySemicolon` | Unnecessary semicolon | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-044 |
| `UnnecessarySuperConstructor` | Unnecessary call to 'super()' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessarySuperQualifier` | Unnecessary 'super' qualifier | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryThis` | Unnecessary 'this' qualifier | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryToStringCall` | Unnecessary call to 'toString()' | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-046 |
| `UnqualifiedFieldAccess` | Instance field access not qualified with 'this' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnqualifiedInnerClassAccess` | Unqualified inner class access | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnqualifiedMethodAccess` | Instance method call not qualified with 'this' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnqualifiedStaticUsage` | Unqualified static access | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Compiler issues (4)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `JavacQuirks` | Javac quirks | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PreviewFeature` | Preview Feature warning | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizeOnValueBasedClass` | Value-based warnings | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UNCHECKED_WARNING` | Unchecked warning | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Concurrency annotation issues (6)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `FieldAccessNotGuarded` | Unguarded field access or method call | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstanceGuardedByStatic` | Instance member guarded by static field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonFinalFieldInImmutable` | Non-final field in '@Immutable' class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonFinalGuard` | Non-final '@GuardedBy' field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticGuardedByInstance` | Static member guarded by instance field or this | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnknownGuard` | Unknown '@GuardedBy' field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
