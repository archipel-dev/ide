# Inspections: java (part 1 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Cast (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CastThatLosesPrecision` | Numeric cast that loses precision | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DoubleLiteralMayBeFloatLiteral` | Cast to 'float' can be 'float' literal | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `IntegerMultiplicationImplicitCastToLong` | Integer multiplication or shift implicitly cast to 'long' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `IntLiteralMayBeLongLiteral` | Cast to 'long' can be 'long' literal | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryExplicitNumericCast` | Unnecessary explicit numeric cast | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Class (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassNamePrefixedWithPackageName` | Class name prefixed with package name | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassNameSameAsAncestorName` | Class name same as ancestor name | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExceptionNameDoesntEndWithException` | Exception class name does not end with 'Exception' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NewClassNamingConvention` | Class naming convention | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonExceptionNameEndsWithException` | Non-exception class name ends with 'Exception' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Code (72)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ActionIsNotPreviewFriendly` | Field blocks intention preview | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ActionPresentationInstantiatedInCtor` | Eager creation of action presentation | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ApplicationServiceAsStaticFinalFieldOrProperty` | Application service assigned to a static final field or immutable property | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CallingMethodShouldBeRequiresBlockingContext` | Calling method should be annotated with @RequiresBlockingContext | UAST | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CanBeDumbAware` | Can be DumbAware | JVM | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CancellationCheckInLoops` | Cancellation check in loops | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComponentNotRegistered` | Component/Action not registered | JVM | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComponentRegistrationProblems` | Component type mismatch | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DebugOnlyTestExtension` | Debug-only JUnit test extension must not be committed | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EventLogDescription` | Missing event log descriptions | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtensionClassShouldBeFinalAndNonPublic` | Extension class should be final and non-public | JVM | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtensionRegisteredAsServiceOrComponent` | Extension registered as service/component | JVM | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FileEqualsUsage` | File.equals() usage | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncorrectCancellationExceptionHandling` | Cancellation exception handled incorrectly | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncorrectParentDisposable` | Incorrect parentDisposable parameter | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncorrectServiceRetrieving` | Incorrect service retrieving | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InspectionUsingGrayColors` | Using new Color(a,a,a) | UAST | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstanceIElementTypeField` | Instance IElementType field | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JComponentDataProvider` | JComponent must use UiDataProvider | JVM | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JetBrainsInternalApiUsage` | Usage of JetBrains internal API | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LeakableMapKey` | Map key may leak | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LightProjectDescriptorEqualsHashCode` | Light project descriptor should implement 'equals()' and 'hashCode()' | JVM | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LightServiceMigrationCode` | A service can be converted to a light one | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LightServiceMustBeFinal` | Light service must be final | JVM | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ListenerImplementationMustNotBeDisposable` | Listener implementation implements 'Disposable' | JVM | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `McpToolMissingDescription` | MCP tool missing description | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `McpToolParameterMissingDescription` | MCP tool parameter missing description | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `McpToolProjectPathParameter` | MCP tool declares projectPath parameter | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MigrateToOptControl` | Migrate to getOptionPane() | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MismatchedLightServiceLevelAndCtor` | Mismatch between light service level and its constructor | JVM | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingAccessibleContext` | Accessible context is missing | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingActionUpdateThread` | ActionUpdateThread is missing | JVM | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingRecentApi` | Usage of IntelliJ API not available in older IDEs | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonDefaultConstructor` | Non-default constructors for service and extension class | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObsoleteDispatchersEdt` | Consider using 'Dispatchers.UI' | JVM | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PathAnnotationInspection` | Path annotation usage problems | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PersistentStatePropertyNotSerialized` | Persistent state property is not serialized | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PotentialDeadlockInServiceInitialization` | Potential deadlock in service initialization | UAST | false | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PresentationAnnotation` | Invalid icon path in @Presentation | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PsiElementConcatenation` | Using PsiElement string representation to generate new expression is incorrect | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `QuickFixGetFamilyNameViolation` | QuickFix's getFamilyName() implementation must not depend on a specific context | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableCtor` | Non-default constructor in serializable class | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableServiceRetrieving` | Simplifiable service retrieving | UAST | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SplitModeApiUsage` | Inappropriate Frontend/Backend API usage in module | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StatefulEp` | Stateful extension | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticInitializationInExtensions` | Static initialization in extension point implementations | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StatisticsCollectorNotRegistered` | Statistics collector not registered | JVM | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TargetGeneralCommandLine` | Implicit process environment in GeneralCommandLine | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TargetProcessBuilder` | ProcessBuilder with an environment-aware argument | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TargetSystemInfo` | Potentially incorrect SystemInfo usage | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadingConcurrency` | Threading and concurrency problems | UAST | false | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TokenSetInParserDefinition` | Non-platform TokenSet declared in ParserDefinition | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UastHintedVisitorAdapterHints` | 'UastHintedVisitorAdapter' hints problems | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UElementAsPsi` | UElement as PsiElement usage | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UndesirableClassUsage` | Undesirable class usage | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnregisteredNamedColor` | Unregistered named color | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnresolvedPluginConfigReference` | Unresolved plugin configuration reference | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnsafeOpenServiceCast` | Unsafe cast of open service | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnsafeReturnStatementVisitor` | Unsafe return statements visitor | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnsafeVfsRecursion` | Unsafe VFS recursion | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnspecifiedActionsPlace` | Unspecified action place | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseDPIAwareBorders` | Use DPI-aware borders | UAST | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseDPIAwareInsets` | Use DPI-aware insets | UAST | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseHtmlChunkToolTip` | Use 'setToolTipText(HtmlChunk)' to avoid HTML injections | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseIntelliJVirtualThreads` | Use 'IntelliJVirtualThreads.ofVirtual' | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseJBColor` | Use Darcula aware JBColor | UAST | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOptimizedEelFunctions` | Use optimized Eel file operations | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UsePluginIdEquals` | Use 'PluginId#equals(Object)' | UAST | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UsePrimitiveTypes` | Use 'PsiType#equals(Object)' with primitive types | UAST | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseVirtualFileEquals` | Use 'VirtualFile#equals(Object)' | UAST | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseVirtualFileToVirtualFileUrlConversion` | Use caching VirtualFile-to-VirtualFileUrl conversion | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseVirtualFileUrlToVirtualFileConversion` | Use caching VirtualFileUrl-to-VirtualFile conversion | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Description file (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `InspectionDescriptionNotFoundInspection` | Inspection description checker | JVM | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IntentionDescriptionNotFoundInspection` | Intention description checker | JVM | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PostfixTemplateDescriptionNotFound` | Postfix template description checker | JVM | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Embedded (11)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AbstractClassWithOnlyOneDirectInheritor` | Abstract class with a single direct inheritor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ArrayLengthInLoopCondition` | Array.length in loop condition | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CheckForOutOfMemoryOnLargeArrayAllocation` | Large array allocation with no OutOfMemoryError check | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConnectionResource` | Connection opened but not safely closed | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InterfaceWithOnlyOneDirectInheritor` | Interface with a single direct inheritor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodCallInLoopCondition` | Method call in loop condition | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MultiplyOrDivideByPowerOfTwo` | Multiplication or division by power of two | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverlyLargePrimitiveArrayInitializer` | Overly large initializer for array of primitive type | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PrivateMemberAccessBetweenOuterAndInnerClass` | Synthetic accessor call | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RecordStoreResource` | 'RecordStore' opened but not safely closed | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SingleCharacterStartsWith` | Single character 'startsWith()' or 'endsWith()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Gradle / Best practices (8)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AvoidApplyPluginMethod` | Legacy 'apply' plugin notation | UAST | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AvoidDependencyNamedArgumentsNotation` | Named arguments dependency notation should be replaced by the GAV string notation | UAST | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AvoidDuplicateDependencies` | Duplicate dependencies | UAST | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AvoidDuplicateRepositories` | Duplicate repositories | UAST | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AvoidRepositoriesInBuildGradle` | Avoid declaring repositories in build scripts | UAST | false | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConfigurationAvoidance` | Configuration avoidance | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantKotlinStdLibDependency` | Redundant Kotlin standard library dependency | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TaskMissingDescription` | Describe custom Tasks | UAST | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Gradle / Probable bugs (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DependencyNotationArgument` | Unrecognized dependency notation | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForeignDelegate` | Possibly misplaced call to Gradle method | UAST | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Gradle / Validity issues (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DeprecatedConfigurations` | Deprecated configurations | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncorrectPluginDslStructure` | Plugin DSL structure | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## JUnit (16)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AssertEqualsCalledOnArray` | 'assertEquals()' called on array | UAST | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssertEqualsMayBeAssertSame` | 'assertEquals()' may be 'assertSame()' | UAST | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExpectedExceptionNeverThrown` | Expected exception never thrown in test method body | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IgnoredJUnitTest` | JUnit test annotated with '@Ignore'/'@Disabled' | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JUnit3StyleTestMethodInJUnit4Class` | Old style JUnit test method in JUnit 4 class | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Junit4Converter` | JUnit 3 test can be JUnit 4 | UAST | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Junit4RunWithInspection` | '@RunWith' annotation already exists in a parent class | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JUnit5AssertionsConverter` | JUnit 5 obsolete assertions | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JUnit5Converter` | JUnit 4 test can be JUnit 5 | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JUnitMalformedDeclaration` | JUnit malformed declaration | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JUnitMixedFramework` | JUnit API usage from multiple versions in a single TestCase | JVM | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MigrateAssertToMatcherAssert` | JUnit assertion can be 'assertThat()' call | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MultipleExceptionsDeclaredOnTestMethod` | Multiple exceptions declared on test method | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ParameterizedParametersStaticCollection` | Parameterized test class without data provider method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuperTearDownInFinally` | JUnit 3 'super.tearDown()' is not called from 'finally' block | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOfObsoleteAssert` | Usage of obsolete 'junit.framework.Assert' method | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
