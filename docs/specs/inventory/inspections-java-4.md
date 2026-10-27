# Inspections: java (part 4 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Java / Control flow issues (50)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AssertionCanBeIf` | Assertion can be replaced with 'if' statement | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BooleanExpressionMayBeConditional` | Boolean expression can be replaced with conditional expression | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `BreakStatement` | 'break' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BreakStatementWithLabel` | 'break' statement with label | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConditionalBreakInInfiniteLoop` | Conditional break inside loop | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConditionalCanBePushedInsideExpression` | Conditional can be pushed inside branch expression | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConditionalExpression` | Conditional expression | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConditionalExpressionWithIdenticalBranches` | Conditional expression with identical branches | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConfusingElse` | Redundant 'else' | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantConditionalExpression` | Constant conditional expression | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-042 |
| `ContinueStatement` | 'continue' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ContinueStatementWithLabel` | 'continue' statement with label | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DefaultNotLastCaseInSwitch` | 'default' not last case in 'switch' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `DoubleNegation` | Double negation | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-042 |
| `DuplicateCondition` | Duplicate condition | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EnumSwitchStatementWhichMissesCases` | Enum 'switch' statement that misses case | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExpressionMayBeFactorized` | Expression can be factorized | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FallthruInSwitchStatement` | Fallthrough in 'switch' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForLoopReplaceableByWhile` | 'for' loop may be replaced by 'while' loop | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForLoopWithMissingComponent` | 'for' loop with missing components | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IdempotentLoopBody` | Idempotent loop body | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IfCanBeAssertion` | Statement can be replaced with 'assert' or 'Objects.requireNonNull' | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IfStatementWithIdenticalBranches` | 'if' statement with identical branches or common parts | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IfStatementWithTooManyBranches` | 'if' statement with too many branches | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InfiniteLoopStatement` | Infinite loop statement | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-025 |
| `LabeledStatement` | Labeled statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoopConditionNotUpdatedInsideLoop` | Loop variable not updated inside loop | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoopStatementsThatDontLoop` | Loop statement that does not loop | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoopWithImplicitTerminationCondition` | Loop with implicit termination condition | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NegatedConditional` | Conditional expression with negated condition | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NegatedConditionalExpression` | Negated conditional expression | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `NegatedEqualityExpression` | Negated equality expression | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `NegatedIfElse` | 'if' statement with negated condition | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestedConditionalExpression` | Nested conditional expression | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestedSwitchStatement` | Nested 'switch' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverlyComplexBooleanExpression` | Overly complex boolean expression | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PointlessBooleanExpression` | Pointless boolean expression | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-041 |
| `PointlessIndexOfComparison` | Pointless 'indexOf()' comparison | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PointlessNullCheck` | Unnecessary 'null' check before method call | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableBooleanExpression` | Simplifiable boolean expression | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableConditionalExpression` | Simplifiable conditional expression | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SwitchExpressionCanBePushedDown` | Common subexpression can be extracted from 'switch' | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SwitchStatement` | 'switch' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SwitchStatementDensity` | 'switch' statement with too low of a branch density | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SwitchStatementsWithoutDefault` | 'switch' statement without 'default' branch | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SwitchStatementWithConfusingDeclaration` | Local variable used and declared in different 'switch' branches | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SwitchStatementWithTooFewBranches` | Minimum 'switch' branches | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SwitchStatementWithTooManyBranches` | Maximum 'switch' branches | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TrivialIf` | Redundant 'if' statement | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `WhileCanBeDoWhile` | 'while' can be replaced with 'do while' | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Data flow (8)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `BooleanMethodIsAlwaysInverted` | Boolean method is always inverted | JAVA | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `BooleanVariableAlwaysNegated` | Boolean variable is always inverted | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantValueVariableUse` | Use of variable whose value is known to be constant | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `LawOfDemeter` | Law of Demeter | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NegativelyNamedBooleanVariable` | Negatively named boolean variable | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReuseOfLocalVariable` | Reuse of local variable | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TooBroadScope` | Scope of variable is too broad | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryLocalVariable` | Redundant local variable | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Declaration redundancy (25)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AccessStaticViaInstance` | Access static member via instance reference | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `CanBeFinal` | Declaration can have 'final' modifier | JAVA | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `DefaultAnnotationParam` | Default annotation parameter value | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DuplicateThrows` | Duplicate throws | JAVA | true | WARNING | local+cleanup | java/07-inspections.md JAVA-INS-047 |
| `EmptyInitializer` | Empty class initializer | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FinalMethodInFinalClass` | 'final' method in 'final' class | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `FunctionalExpressionCanBeFolded` | Functional expression can be folded | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Java9RedundantRequiresStatement` | Redundant 'requires' directive in module-info | JAVA | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaModuleDefinition` | Java module definition problems | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ProtectedMemberInFinalClass` | 'protected' member in 'final' class | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantExplicitClose` | Redundant 'close()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantLambdaParameterType` | Redundant lambda parameter types | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantRecordConstructor` | Redundant record constructor | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantThrows` | Redundant 'throws' clause | JAVA | true | WARNING | global | java/07-inspections.md JAVA-INS-047 |
| `SafeVarargsOnNonReifiableType` | @SafeVarargs is not applicable to reifiable types | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SameParameterValue` | Method parameter always has the same value | UAST | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `SameReturnValue` | Method always returns the same value | UAST | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `SillyAssignment` | Variable is assigned to itself | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-029 |
| `TrivialFunctionalExpressionUsage` | Trivial usage of functional expression | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryModuleDependencyInspection` | Unnecessary module dependency |  | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `unused` | Unused declaration | JAVA | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedLabel` | Unused label | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedLibrary` | Unused library |  | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedReturnValue` | Method can be made 'void' | JAVA | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `WeakerAccess` | Declaration access can be weaker | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Dependency issues (7)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassWithTooManyDependencies` | Class with too many dependencies | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassWithTooManyDependents` | Class with too many dependents | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassWithTooManyTransitiveDependencies` | Class with too many transitive dependencies | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassWithTooManyTransitiveDependents` | Class with too many transitive dependents | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `CyclicClassDependency` | Cyclic class dependency | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `CyclicPackageDependency` | Cyclic package dependency | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousPackagePrivateAccess` | Suspicious package-private access | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Encapsulation (8)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AssignmentOrReturnOfFieldWithMutableType` | Assignment or return of field with mutable type | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageVisibleField` | Package-visible field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageVisibleInnerClass` | Package-visible nested class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ProtectedField` | Protected field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ProtectedInnerClass` | Protected nested class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicField` | 'public' field | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicInnerClass` | 'public' nested class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOfAnotherObjectsPrivateField` | Accessing a non-public field of another object | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Error handling (28)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `BadExceptionCaught` | Prohibited 'Exception' caught | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BadExceptionDeclared` | Prohibited exception declared | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BadExceptionThrown` | Prohibited exception thrown | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CatchMayIgnoreException` | Catch block may ignore exception | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-031 |
| `CaughtExceptionImmediatelyRethrown` | Caught exception is immediately rethrown | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CheckedExceptionClass` | Checked exception class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ContinueOrBreakFromFinallyBlock` | 'continue' or 'break' inside 'finally' block | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptyFinallyBlock` | Empty 'finally' block | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptyTryBlock` | Empty 'try' block | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ErrorRethrown` | 'Error' not rethrown | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExceptionFromCatchWhichDoesntWrap` | 'throw' inside 'catch' block which ignores the caught exception | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtendsThrowable` | Class directly extends 'Throwable' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FinallyBlockCannotCompleteNormally` | 'finally' block which can not complete normally | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstanceofCatchParameter` | 'instanceof' on 'catch' parameter | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestedTryStatement` | Nested 'try' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NewExceptionWithoutArguments` | Exception constructor called without arguments | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonFinalFieldOfException` | Non-final field of 'Exception' class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NullThrown` | 'null' thrown | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReturnFromFinallyBlock` | 'return' inside 'finally' block | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadDeathRethrown` | 'ThreadDeath' not rethrown | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThrowableSupplierOnlyThrowException` | Throwable supplier never returns a value | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThrowCaughtLocally` | 'throw' caught by containing 'try' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThrowFromFinallyBlock` | 'throw' inside 'finally' block | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThrowsRuntimeException` | Unchecked exception declared in 'throws' clause | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TooBroadCatch` | Overly broad 'catch' block | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TooBroadThrows` | Overly broad 'throws' clause | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UncheckedExceptionClass` | Unchecked 'Exception' class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryInitCause` | Unnecessary call to 'Throwable.initCause()' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Finalization (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Finalize` | 'finalize()' should not be overridden | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FinalizeNotProtected` | 'finalize()' should be protected, not public | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `NoExplicitFinalizeCalls` | 'finalize()' called explicitly | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
