# Inspections: kotlin (part 1 of 4)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Code (11)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `checkedExceptions` | Unchecked exceptions in Kotlin | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CompanionObjectInExtension` | Companion object in extensions | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForbiddenInSuspectContextMethod` | Forbidden in suspend context method usage | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinApplicableInspectionBaseSimpleCallExpression` | Too broad highlighting range in a Kotlin inspection | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinObjectRegisteredAsExtension` | Kotlin object registered as extension | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinPolySymbolPropertyType` | PolySymbol property type mismatch | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonSerializableTypeInRpcInterface` | Non-serializable type used in RPC interface | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PrivateExtensionClass` | Private modifier used with IntelliJ Platform extension class | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicTestExtensionFunction` | Public extension function on Driver/Finder/IdeaFrameUI in intelij.driver.tests | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RpcInterfaceDoesNotExtendRemoteApi` | @Rpc-annotated interface does not extend fleet.rpc.RemoteApi | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UsePlatformProcessAwaitExit` | Use 'com.intellij.util.io.ProcessKt.awaitExit()' | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Compose (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ComposableCallWithoutComposePlugin` | Composable function call without Compose plugin | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComposeResourcesImageResource` | Vector resource used with imageResource() | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Compose Multiplatform Preview (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `MultiplatformPreviewAnnotationInFunctionWithParameters` | Preview used on a Composable function with parameters | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MultiplatformPreviewMultipleParameterProviders` | Multiple @PreviewParameter are not allowed | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Gradle (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `KotlinOptionsToCompilerOptions` | Use of deprecated 'kotlinOptions' DSL | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin (8)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DifferentKotlinGradleVersion` | Kotlin Gradle and IDE plugins versions are different | Groovy | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DifferentKotlinMavenVersion` | Maven and IDE plugins versions are different | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DifferentMavenStdlibVersion` | Library and maven plugin versions are different | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DifferentStdlibGradleVersion` | Kotlin library and Gradle plugin versions are different | Groovy | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KaptKotlinCompilerPlugin` | Add Kotlin kapt compiler plugin to module build | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinInvalidBundleOrProperty` | Invalid property key | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinMavenPluginPhase` | Kotlin Maven Plugin misconfigured | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinTestJUnit` | kotlin-test-junit could be used | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Code migration (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CanConvertToMultiDollarString` | Multi-dollar interpolation can be used in string literals | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertFromMultiDollarToRegularString` | Removable multi-dollar interpolation prefix | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertLongToDuration` | Long overload to Duration conversion | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CustomComponentDestructuringMigration` | Position-based destructuring should use positional syntax | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InfixCallToOrdinary` | Infix call may be dot call | kotlin | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Coroutine inspections (15)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CoroutineContextWithJob` | 'CoroutineContext' can contain 'Job' element | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DeferredResultUnused` | '@Deferred' result is unused | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForEachJoinOnCollectionOfJob` | 'forEach { it.join() }' call on 'Collection&lt;Job&gt;' instead of single 'joinAll()' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MapAwaitOnCollectionOfDeferred` | 'map { it.await() }' call on 'Collection&lt;Deferred&gt;' instead of single 'awaitAll()' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PreferCurrentCoroutineContextToCoroutineContext` | Potentially ambiguous 'kotlin.coroutine.coroutineContext' usage | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RunBlocking` | RunBlocking in coroutine | kotlin | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `RunBlockingInSuspendFunction` | 'runBlocking' inside suspend function | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableFlowCall` | Function call on 'Flow' type could be simplified | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableFlowCallChain` | Call chain on 'Flow' type can be simplified | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuppressedCancellationException` | Suppressed 'CancellationException' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspendCoroutineLacksCancellationGuarantees` | 'suspendCoroutine' lacks cancellation guarantees | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousImplicitCoroutineScopeReceiverAccess` | Suspicious implicit 'CoroutineScope' receiver access | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousMutableCollectionInStateFlow` | Suspicious mutable collection in 'MutableStateFlow' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedFlow` | Unused flow | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UselessCallOnFlow` | Redundant call on 'Flow' type | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Java interop issues (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `HasPlatformType` | Function or property has platform type | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaCollectionsStaticMethodOnImmutableList` | Call of Java mutator method on immutable Kotlin collection | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaCollectionWithNullableTypeArgument` | Nullable type argument in Java collection that doesn't support nulls | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaDefaultMethodsNotOverriddenByDelegation` | Java default methods are not overridden by delegation | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageDirectoryMismatch` | Package name does not match containing directory | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Logging (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `KotlinLoggerInitializedWithForeignClass` | Logger initialized with foreign class | kotlin | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Migration (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `KotlinDeprecation` | Usage of redundant or deprecated syntax or deprecated symbols | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Naming conventions (12)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassName` | Class naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstPropertyName` | Const property naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EnumEntryName` | Enum entry naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FunctionName` | Function naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InconsistentCommentForJavaParameter` | Inconsistent comment for Java parameter | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LocalVariableName` | Local variable naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectPrivatePropertyName` | Object private property naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectPropertyName` | Object property naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageName` | Package naming convention | kotlin | true | WEAK WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `PrivatePropertyName` | Private property naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PropertyName` | Property naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TestFunctionName` | Test function naming convention | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Numeric issues (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `KotlinBigDecimalEquals` | 'equals()' called on 'BigDecimal' | kotlin | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Kotlin / Other problems (14)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ConvertArgumentToSet` | Argument could be converted to 'Set' to improve performance | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DeprecatedCallableAddReplaceWith` | @Deprecated annotation without 'replaceWith' argument | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EnumValuesSoftDeprecate` | 'Enum.values()' is recommended to be replaced by 'Enum.entries' since 1.9 | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EnumValuesTopLevelFunctionSoftDeprecate` | 'enumValues' is recommended to be replaced by 'enumEntries' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FloatingPointLiteralPrecision` | Floating-point literal exceeds the available precision | kotlin | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KDocMissingDocumentation` | Missing KDoc comments for public declarations | kotlin | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinAmbiguousActuals` | Ambiguous actuals | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinNoActualForExpect` | No actual for expect declaration | kotlin | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MigrateDiagnosticSuppression` | Diagnostic name should be replaced | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicApiImplicitType` | Public API declaration with implicit return type | kotlin | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplacePrintlnWithLogging` | Call to 'print()' or 'println()' | kotlin | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceWithEnumMap` | 'HashMap' can be replaced with 'EnumMap' | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceWithStringBuilderAppendRange` | 'StringBuilder.append(CharArray, offset, len)' call on the JVM | kotlin | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ScriptShebangExecPermission` | Script is not executable. Missing execute permission. | kotlin | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
