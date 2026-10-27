# Inspections: kotlin (part 3 of 4)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Kotlin / Redundant constructs (67)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CanBeParameter` | Constructor parameter is never used as a property | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CanBePrimaryConstructorProperty` | Property is explicitly assigned to constructor parameter | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CanUnescapeDollarLiteral` | Redundant escaped dollar characters in string literals | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComplexRedundantLet` | Redundant argument-based 'let' call | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantConditionIf` | Condition of 'if' expression is constant | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ControlFlowWithEmptyBody` | Control flow with empty body | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExplicitThis` | Redundant explicit 'this' | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FunctionWithLambdaExpressionBody` | Function with '= { ... }' and inferred return type | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IfExpressionWithIdenticalBranches` | 'if' expression with identical branches | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinDoubleNegation` | Redundant double negation | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinRedundantOverride` | Redundant overriding method | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinUnusedImport` | Unused import directive | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NullChecksToSafeCall` | Null-checks can be replaced with safe-calls | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantCompanionReference` | Redundant 'Companion' reference | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantConstructorKeyword` | Redundant 'constructor' keyword | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantContextSensitiveResolutionQualifier` | Redundant qualifier removable via context-sensitive resolution | kotlin | false | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantElvisReturnNull` | Redundant '?: return null' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantEmptyInitializerBlock` | Redundant empty initializer block | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantEnumConstructorInvocation` | Redundant enum constructor invocation | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantExplicitType` | Obvious explicit type | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantGetter` | Redundant property getter | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantIf` | Redundant 'if' statement | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantInnerClassModifier` | Redundant 'inner' modifier | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantInterpolationPrefix` | Redundant interpolation prefix | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantLabeledReturnOnLastExpressionInLambda` | Redundant labeled return on the last expression in a lambda | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantLambdaArrow` | Redundant lambda arrow | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantLambdaOrAnonymousFunction` | Redundant creation of lambda or anonymous function | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantModalityModifier` | Redundant modality modifier | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantNullableReturnType` | Redundant nullable return type | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantRequireNotNullCall` | Redundant 'requireNotNull' or 'checkNotNull' call | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantReturnKeyword` | Redundant 'return' keyword | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantReturnLabel` | Redundant 'return' label | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantSamConstructor` | Redundant SAM constructor | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantSemicolon` | Redundant semicolon | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantSetter` | Redundant property setter | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantSuspendModifier` | Redundant 'suspend' modifier | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantUnitExpression` | Redundant 'Unit' | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantUnitReturnType` | Redundant 'Unit' return type | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantUpperBound` | Redundant upper bound 'Any?' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantValueArgument` | Redundant value argument | kotlin | false | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantVisibilityModifier` | Redundant visibility modifier | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantWith` | Redundant 'with' call | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveCurlyBracesFromTemplate` | Redundant curly braces in string template | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveEmptyClassBody` | Replace empty class body | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveEmptyParenthesesFromLambdaCall` | Unnecessary parentheses in function call with lambda | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveEmptyPrimaryConstructor` | Redundant empty primary constructor | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveEmptySecondaryConstructorBody` | Redundant constructor body | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveExplicitSuperQualifier` | Unnecessary supertype qualification | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveExplicitTypeArguments` | Unnecessary type argument | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveRedundantBackticks` | Redundant backticks | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveRedundantQualifierName` | Redundant qualifier name | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveRedundantSpreadOperator` | Redundant spread operator | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveSetterParameterType` | Redundant setter parameter type | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveSingleExpressionStringTemplate` | Redundant string template | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveToStringInStringTemplate` | Redundant call to 'toString()' in string template | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ScopeFunctionConversion` | Call can be replaced with another scope function | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimpleRedundantLet` | Redundant receiver-based 'let' call | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyWhenWithBooleanConstantCondition` | Simplifiable 'when' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousCallableReferenceInLambda` | Suspicious callable reference used as lambda result | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryOptInAnnotation` | Unnecessary '@OptIn' annotation | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryVariable` | Unnecessary local variable | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedContextParameterCall` | Unused context call arguments | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedReceiverParameter` | Unused receiver parameter | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedSymbol` | Unused symbol | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedVariable` | Unused variable | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `VariableInitializerIsRedundant` | Variable initializer is redundant | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WhenWithOnlyElse` | 'when' has only 'else' branch and can be simplified | kotlin | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
