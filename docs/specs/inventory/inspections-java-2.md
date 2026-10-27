# Inspections: java (part 2 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## JVM languages (18)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `BlockingMethodInNonBlockingContext` | Possibly blocking call in non-blocking context | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Dependency` | Illegal package dependencies | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptyMethod` | Empty method | UAST | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `IllegalDependencyOnInternalPackage` | Illegal dependency on internal package | JVM | false | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingDeprecatedAnnotationOnScheduledForRemovalApi` | Missing '@Deprecated' annotation on scheduled for removal API | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MustAlreadyBeRemovedApi` | API must already be removed | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonExtendableApiUsage` | Class, interface, or method should not be extended | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverrideOnly` | Method can only be overridden | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableHasSerialVersionUIDField` | Serializable class without 'serialVersionUID' | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Since15` | Usages of API which isn't available at the configured language level | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SourceToSinkFlow` | Non-safe string is passed to safe method | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuppressionAnnotation` | Inspection suppression annotation | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SystemGetProperty` | Call to 'System.getProperty(str)' could be simplified | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadRun` | Call to 'Thread.run()' | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnstableApiUsage` | Unstable API Usage | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnstableTypeUsedInSignature` | Unstable type is used in signature | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UrlHashCode` | Call to 'equals()' or 'hashCode()' on 'URL' object | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UsagesOfObsoleteApi` | Usages of ApiStatus.@Obsolete | UAST | true | TEXT ATTRIBUTES | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / (ungrouped) (10)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ConvertJavadoc` | Asserts your TestNG tests with Javadoc annotations and converts them t… | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertOldAnnotations` | Reports TestNG org.testng.annotations.Configuration annotations. | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DataProviderReturnType` | Reports methods marked with @DataProvider annotation that doesn't retu… | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `dependsOnMethodTestNG` | Reports illegal method names passed to the dependsOnMethods attribute… | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DuplicatedDataProviderNames` | Reports TestNG data providers with equal names if org.testng.TestNGExc… | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExpectedExceptionNeverThrownTestNG` | Reports checked exceptions expected by a TestNG test method that are n… | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `groupsTestNG` | Reports undefined group names passed to the dependsOnGroups or groups… | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JUnitTestNG` | Reports any JUnit-based test class that can be converted into TestNG b… | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MalformedDataProvider` | Reports references to data provider methods that do not exist or are n… | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UndeclaredTests` | Reports test classes that are not registered in testing.xml . | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Abstraction issues (15)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `BooleanParameter` | 'public' method with 'boolean' parameter | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassReferencesSubclass` | Class references one of its subclasses | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DeclareCollectionAsInterface` | Collection declared by class, not interface | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FeatureEnvy` | Feature envy | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstanceofChain` | Chain of 'instanceof' checks | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstanceofThis` | 'instanceof' check for 'this' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InterfaceMethodClashesWithObject` | Interface method clashes with method in 'Object' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MagicNumber` | Magic number | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodOnlyUsedFromInnerClass` | Private method only used from inner class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OptionalUsedAsFieldOrParameterType` | 'Optional' used as field or parameter type | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-021 |
| `OverlyStrongTypeCast` | Overly strong type cast | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicMethodNotExposedInInterface` | 'public' method not exposed in interface | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticMethodOnlyUsedInOneClass` | Static member only used from one other class | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `TypeMayBeWeakened` | Type may be weakened | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOfConcreteClass` | Use of concrete class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Assignment issues (11)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AssignmentToCatchBlockParameter` | Assignment to 'catch' block parameter | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssignmentToForLoopParameter` | Assignment to 'for' loop parameter | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssignmentToLambdaParameter` | Assignment to lambda parameter | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssignmentToMethodParameter` | Assignment to method parameter | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssignmentToNull` | 'null' assignment | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssignmentToStaticFieldFromInstanceMethod` | Assignment to static field from instance context | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssignmentToSuperclassField` | Constructor assigns value to field defined in superclass | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssignmentUsedAsCondition` | Assignment used as condition | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncrementDecrementUsedAsExpression` | Result of '++' or '--' used | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestedAssignment` | Nested assignment | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceAssignmentWithOperatorAssignment` | Assignment can be replaced with operator assignment | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Bitwise operation issues (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `IncompatibleMask` | Incompatible bitwise mask operation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PointlessBitwiseExpression` | Pointless bitwise expression | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ShiftOutOfRange` | Shift operation by inappropriate constant | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Class metrics (9)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AnonymousClassComplexity` | Overly complex anonymous class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AnonymousClassMethodCount` | Anonymous class with too many methods | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassComplexity` | Overly complex class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassCoupling` | Overly coupled class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassInheritanceDepth` | Class too deep in inheritance tree | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassNestingDepth` | Inner class too deeply nested | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstructorCount` | Class with too many constructors | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FieldCount` | Class with too many fields | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodCount` | Class with too many methods | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Class structure (31)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AnonymousInnerClass` | Anonymous class can be replaced with inner class | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassInitializer` | Non-'static' initializer | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassMayBeInterface` | Abstract 'class' may be 'interface' | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassNameDiffersFromFileName` | Class name differs from file name | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassWithOnlyPrivateConstructors` | Class with only 'private' constructors should be declared 'final' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantDeclaredInAbstractClass` | Constant declared in 'abstract' class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantDeclaredInInterface` | Constant declared in interface | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptyClass` | Empty class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FieldCanBeLocal` | Field can be local variable | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-052 |
| `FinalClass` | Class is closed to inheritance | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FinalMethod` | Method can't be overridden | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FinalPrivateMethod` | 'private' method declared 'final' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FinalStaticMethod` | 'static' method declared 'final' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InnerClassOnInterface` | Inner class of interface | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InterfaceMayBeAnnotatedFunctional` | Interface may be annotated as '@FunctionalInterface' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LimitedScopeInnerClass` | Local class | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ListenerMayUseAdapter` | Class may extend adapter instead of implementing listener | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MarkerInterface` | Marker interface | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodReturnAlwaysConstant` | Method returns per-class constant | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `MultipleTopLevelClassesInFile` | Multiple top level classes in single file | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonFinalFieldInEnum` | Non-final field in 'enum' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonFinalUtilityClass` | Utility class is not 'final' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NoopMethodInAbstractClass` | No-op method in 'abstract' class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ParameterCanBeLocal` | Value passed as parameter never read | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-052 |
| `PublicConstructor` | 'public' constructor can be replaced with factory method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Singleton` | Singleton | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticNonFinalField` | 'static', non-'final' field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UtilityClass` | Utility class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UtilityClassCanBeEnum` | Utility class can be 'enum' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UtilityClassWithoutPrivateConstructor` | Utility class without 'private' constructor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UtilityClassWithPublicConstructor` | Utility class with 'public' constructor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Cloning issues (7)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CloneableImplementsClone` | Cloneable class without 'clone()' method | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CloneCallsConstructors` | 'clone()' instantiates objects with constructor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CloneDeclaresCloneNotSupported` | 'clone()' does not declare 'CloneNotSupportedException' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CloneInNonCloneableClass` | 'clone()' method in non-Cloneable class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CloneReturnsClassType` | 'clone()' should have return type equal to the class it contains | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonPublicClone` | 'clone()' method not 'public' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOfClone` | Use of 'clone()' or 'Cloneable' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Code maturity (13)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CommentedOutCode` | Commented out code | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DeprecatedIsStillUsed` | Deprecated member is still used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Deprecation` | Deprecated API usage | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtractMethodRecommender` | Method can be extracted | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MarkedForRemoval` | Usage of API marked for removal | UAST | true | ERROR | local | codeinsight/03-inspections.md CI-INS-105 |
| `ObsoleteCollection` | Use of obsolete collection type | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OptionalAssignedToNull` | Null value for Optional type | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantScheduledForRemovalAnnotation` | Redundant @ScheduledForRemoval annotation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SystemOutErr` | Use of 'System.out' or 'System.err' | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadDumpStack` | Call to 'Thread.dumpStack()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThrowablePrintedToSystemOut` | 'Throwable' printed to 'System.out' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThrowablePrintStackTrace` | Call to 'printStackTrace()' | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-054 |
| `UseOfObsoleteDateTimeApi` | Use of obsolete date-time API | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
