# Inspections: kotlin (part 4 of 4)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Kotlin / Style issues (110)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AddOperatorModifier` | Function should have 'operator' modifier | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AddVarianceModifier` | Type parameter can have 'in' or 'out' variance | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BooleanLiteralArgument` | Boolean literal argument without parameter name | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CanBeVal` | Local 'var' is never modified and can be declared as 'val' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CascadeIf` | Cascade 'if' can be replaced with 'when' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CollapseCollectionLiteralChainCall` | Type conversion on collection literal can be removed | kotlin | true | INFO | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CollectionConcatenationToBuildCollection` | Collection concatenation can be converted to collection builder | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertCallChainIntoSequence` | Call chain on collection could be converted into 'Sequence' to improve performance | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertCollectionLiteralToFunctionCall` | Collection literal can be replaced with a function call | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertCollectionLiteralToTypeOfExpression` | ''…'' can be used | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertDataClassToValueClass` | Data class can be converted to value class | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertExplicitContextArgumentToImplicit` | Explicit context argument can be implicit | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertImplicitContextArgumentToExplicit` | Implicit context argument can be explicit | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertPairConstructorToToFunction` | Convert Pair constructor to 'to' function | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertRangeCheckToTwoComparisons` | Convert to comparisons | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertReferenceToLambda` | Can be replaced with lambda | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertSealedClassToSealedInterface` | Sealed class can be converted to sealed interface | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertSealedInterfaceToSealedClass` | Sealed interface can be converted to sealed class | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertSecondaryConstructorToPrimary` | Convert to primary constructor | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertToCollectionLiterals` | Collection literal can be used | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertToExplicitBackingFields` | Explicit backing fields can be used | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertToStringTemplate` | String concatenation that can be converted to string template | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertTryFinallyToUseCall` | Convert try / finally to use() call | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertTwoComparisonsToRangeCheck` | Two comparisons should be converted to a range check | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertTypeOfToCollectionLiteral` | Collection literal can be used | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CopyWithoutNamedArguments` | 'copy' method of data class is called without named arguments | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DestructuringDeclaration` | Use destructuring declaration | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DoubleBang` | Not-null assertion can be replaced with 'error()' | kotlin | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `FilterIsInstanceCallWithClassLiteralArgument` | 'filterIsInstance' call with a class literal argument | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `FoldInitializerAndIfToElvis` | If-Null return/break/... foldable to '?:' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IfThenToElvis` | If-Then foldable to '?:' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IfThenToSafeAccess` | If-Then foldable to '?.' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ImplicitThis` | Implicit 'this' | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IntroduceWhenSubject` | 'when' that can be simplified by introducing an argument | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaMapForEach` | Java Map.forEach method call should be replaced with Kotlin's forEach | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JoinDeclarationAndAssignment` | Join declaration and assignment | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LiftReturnOrAssignment` | Return or assignment can be lifted out | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MapGetWithNotNullAssertionOperator` | 'map.get()' with not-null assertion operator (!!) | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MapToForEach` | 'map' call can be replaced with 'forEach' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MayBeConstant` | Might be 'const' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MemberVisibilityCanBePrivate` | Class member can have 'private' visibility | kotlin | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MoveLambdaOutsideParentheses` | Lambda argument inside parentheses | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MoveVariableDeclarationIntoWhen` | Variable declaration could be moved into 'when' | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestedLambdaShadowedImplicitParameter` | Nested lambda has shadowed implicit parameter | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NullableBooleanElvis` | Equality check can be used instead of elvis for nullable boolean check | kotlin | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `NullableHashCode` | Nullable value 'hashCode()' can be simplified | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectLiteralToLambda` | Object literal can be converted to lambda | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ProtectedInFinal` | 'protected' visibility is effectively 'private' in a final class | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantAsSequence` | Redundant 'asSequence' call | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantElseInIf` | Redundant 'else' | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantObjectTypeCheck` | Non-idiomatic 'is' type check for an object | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantRunCatching` | Redundant 'runCatching' call | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveEmptyParenthesesFromAnnotationEntry` | Redundant parentheses in annotation | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveUnnecessaryParentheses` | Redundant parentheses | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceAddAllWithMapTo` | 'addAll(map/filter {})' can be replaced with 'mapTo/filterTo' | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceArrayOfWithLiteral` | 'arrayOf' call can be replaced with array literal [...] | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceAssertBooleanWithAssertEquality` | Assert boolean could be replaced with assert equality | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceAssociateFunction` | 'associate' can be replaced with 'associateBy' or 'associateWith' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceCallWithBinaryOperator` | Can be replaced with binary operator | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceCollectionCountWithSize` | Collection count can be converted to size | kotlin | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceContains` | Replace 'contains' call with 'in' operator | kotlin | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceGetOrSet` | Explicit 'get' or 'set' call | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceGuardClauseWithFunctionCall` | Guard clause can be replaced with Kotlin's function call | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceIfExpressionWithFirstOrNull` | 'if' expression can be replaced with 'firstOrNull()' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceIsEmptyWithIfEmpty` | 'if' condition can be replaced with lambda call | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceJavaStaticMethodWithKotlinAnalog` | Java methods should be replaced with Kotlin analog | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceManualRangeWithIndicesCalls` | Range can be converted to indices or iteration | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceMapGetOrDefault` | 'map.getOrDefault()' can be replaced with indexing and elvis operator | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceMapIndexedWithListGenerator` | Replace 'mapIndexed' with List generator | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceMapKeysCallChainWithKeys` | Replace 'map { it.key }.toSet()' with 'keys' | kotlin | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceNegatedIsEmptyWithIsNotEmpty` | Negated call can be simplified | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceNotNullAssertionWithElvisReturn` | Not-null assertion can be replaced with 'return' | kotlin | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplacePutWithAssignment` | 'map.put()' can be converted to assignment | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceRangeStartEndInclusiveWithFirstLast` | Boxed properties should be replaced with unboxed | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceRangeToWithRangeUntil` | 'rangeTo' or the '..' call should be replaced with '..&lt;' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceRangeToWithUntil` | 'rangeTo' or the '..' call should be replaced with 'until' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceReadLineWithReadln` | 'readLine' can be replaced with 'readln' or 'readlnOrNull' | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceSizeCheckWithIsNotEmpty` | Size check can be replaced with 'isNotEmpty()' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceSizeZeroCheckWithIsEmpty` | Size zero check can be replaced with 'isEmpty()' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceStringFormatWithLiteral` | 'String.format' call can be replaced with string templates | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceSubstringWithDropLast` | 'substring' call should be replaced with 'dropLast' call | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceSubstringWithIndexingOperation` | 'substring' call should be replaced with indexing operator | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceSubstringWithSubstringAfter` | 'substring' call should be replaced with 'substringAfter' | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceSubstringWithSubstringBefore` | 'substring' call should be replaced with 'substringBefore' | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceSubstringWithTake` | 'substring' call should be replaced with 'take' call | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceToStringWithStringTemplate` | Call of 'toString' could be replaced with string template | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceToWithInfixForm` | 'to' call should be replaced with infix form | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceUntilWithRangeUntil` | Replace 'until' with '..&lt;' operator | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceWithCallWithContextCall` | Replace 'with' with 'context' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceWithIgnoreCaseEquals` | Should be replaced with 'equals(..., ignoreCase = true)' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceWithImportAlias` | Fully qualified name can be replaced with existing import alias | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceWithOperatorAssignment` | Assignment can be replaced with operator assignment | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RevertExplicitBackingFields` | Replace explicit backing field with private property | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SafeCastWithReturn` | Safe cast with 'return' should be replaced with 'if' type check | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableCall` | Library function call could be simplified | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableCallChain` | Call chain on collection type can be simplified | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyAssertNotNull` | 'assert' call can be replaced with '!!' or '?:' | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyBooleanWithConstants` | Boolean expression can be simplified | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyNegatedBinaryExpression` | Negated boolean expression can be simplified | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifyNestedEachInScopeFunction` | Scope function with nested forEach can be simplified | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SortModifiers` | Non-canonical modifier order | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousAsDynamic` | Suspicious 'asDynamic' member invocation | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TrailingComma` | Trailing comma recommendations | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnclearPrecedenceOfBinaryExpression` | Multiple operators with different precedence | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnlabeledReturnInsideLambda` | Unlabeled return inside lambda | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnsafeCastWithReturn` | Unsafe cast with 'return' should be replaced with the safe one | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseExpressionBody` | Expression body syntax is preferable here | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UsePropertyAccessSyntax` | Accessor call that can be replaced with property access syntax | kotlin | true | WEAK WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseWithIndex` | Manually incremented index variable can be replaced with use of 'withIndex()' | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `VerboseNullabilityAndEmptiness` | Verbose nullability and emptiness check | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin,React / Probable bugs (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `NonExternalClassifierExtendingStateOrProps` | Non-external classifier extending State or Props | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Lombok / Kotlin (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `LombokKotlinCompilerPlugin` | Add Kotlin Lombok compiler plugin to module build | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Workspace model (4)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `WorkspaceEntityMutableField` | Unsupported 'var' field in an entity | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WorkspaceImplAbsent` | Generate implementation | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WorkspaceImplObsolete` | Obsolete version of entity implementation | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WorkspaceInheritance` | Workspace inheritance | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
