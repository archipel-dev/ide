# Inspections: java (part 5 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Java / Imports (8)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `JavaLangImport` | Unnecessary import from the 'java.lang' package | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `MISSORTED_IMPORTS` | Missorted imports | JAVA | true | INFORMATION | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `OnDemandImport` | '*' import | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SamePackageImport` | Unnecessary import from the same package | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `SingleClassImport` | Single class import | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticImport` | Static import | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticImportCanBeUsed` | Static import can be used based on the auto-import table | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UNUSED_IMPORT` | Unused import | JAVA | true | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Inheritance issues (21)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AbstractClassExtendsConcreteClass` | Abstract class extends concrete class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AbstractClassNeverImplemented` | Abstract class which has no concrete subclass | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AbstractClassWithoutAbstractMethods` | Abstract class without 'abstract' methods | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AbstractMethodOverridesAbstractMethod` | Abstract method overrides abstract method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AbstractMethodOverridesConcreteMethod` | Abstract method overrides concrete method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AbstractMethodWithMissingImplementations` | Abstract method with missing implementations | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtendsAnnotation` | Class extends annotation interface | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtendsConcreteCollection` | Class explicitly extends a 'Collection' class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtendsUtilityClass` | Class extends utility class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FrequentlyUsedInheritorInspection` | Class may extend a commonly used base class | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ImplicitSubclassInspection` | Final declaration can't be overridden at runtime | UAST | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InterfaceNeverImplemented` | Interface which has no concrete subclass | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingOverrideAnnotation` | Missing '@Override' annotation | JAVA | true | INFORMATION | local+cleanup | java/07-inspections.md JAVA-INS-049 |
| `NonProtectedConstructorInAbstractClass` | Public constructor in abstract class | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ParameterTypePreventsOverriding` | Parameter type prevents overriding | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ProblematicVarargsMethodOverride` | Non-varargs method overrides varargs method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantImplements` | Redundant interface declaration | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantMethodOverride` | Method is identical to its super method | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RefusedBequest` | Method does not call super method | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticInheritance` | Static inheritance | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TypeParameterExtendsFinalClass` | Type parameter extends 'final' class | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Initialization (11)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AbstractMethodCallInConstructor` | Abstract method called during object construction | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DoubleBraceInitialization` | Double brace initialization | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstanceVariableInitialization` | Instance field may not be initialized | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstanceVariableUninitializedUse` | Instance field used before initialization | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonFinalStaticVariableUsedInClassInitialization` | Non-final static field is used during class initialization | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonThreadSafeLazyInitialization` | Unsafe lazy initialization of 'static' field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverridableMethodCallDuringObjectConstruction` | Overridable method called during object construction | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverriddenMethodCallDuringObjectConstruction` | Overridden method called during object construction | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticVariableInitialization` | Static field may not be initialized | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticVariableUninitializedUse` | Static field used before initialization | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThisEscapedInConstructor` | 'this' reference escaped in object construction | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Internationalization (17)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AbsoluteAlignmentInUserInterface` | Absolute alignment in AWT/Swing code | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CallToSuspiciousStringMethod` | Call to suspicious 'String' method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CharacterComparison` | Character comparison | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConvertToBasicLatin` | Non-Basic Latin character | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DateToString` | Call to 'Date.toString()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DialogTitleCapitalization` | Incorrect string capitalization | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DuplicateStringLiteralInspection` | Duplicate string literal | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HardCodedStringLiteral` | Hardcoded strings | UAST | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ImplicitDefaultCharsetUsage` | Implicit platform default charset | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `MagicCharacter` | Magic character | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NumericToString` | Call to 'Number.toString()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimpleDateFormatWithoutLocale` | 'SimpleDateFormat' without locale | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringConcatenation` | String concatenation | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringTokenizer` | Use of 'StringTokenizer' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringToUpperWithoutLocale` | Call to 'String.toUpperCase()' or 'toLowerCase()' without locale | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TimeToString` | Call to 'Time.toString()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryUnicodeEscape` | Unnecessary Unicode escape sequence | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / JUnit (1)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `MetaAnnotationWithoutRuntimeRetention` | Test annotation without '@Retention(RUNTIME)' annotation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Java language level issues (7)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Annotation` | Annotation | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AnnotationClass` | Annotation interface | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssertStatement` | 'assert' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EnumClass` | Enumerated class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForeachStatement` | Enhanced 'for' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForwardCompatibility` | Forward compatibility | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `VarargParameter` | Varargs method | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Java language level migration aids (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `EnumerationCanBeIteration` | Enumeration can be iteration | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ForEachWithRecordPatternCanBeUsed` | Enhanced 'for' with a record pattern can be used | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `IfCanBeSwitch` | 'if' can be replaced with 'switch' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringRepeatCanBeUsed` | 'repeat()' method can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseCompareMethod` | 'compare()' method can be used to compare numbers | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / JavaBeans issues (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassWithoutConstructor` | Class without constructor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassWithoutNoArgConstructor` | Class without no-arg constructor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FieldHasSetterButNoGetter` | Field has setter but no getter | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PropertyValueSetToItself` | Property value set to itself | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousGetterSetter` | Suspicious getter/setter | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Javadoc (16)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DanglingJavadoc` | Dangling Javadoc comment | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlTagCanBeJavadocTag` | '&lt;code&gt;…&lt;/code&gt;' can be replaced with '{@code …}' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavadocBlankLines` | Blank line should be replaced with &lt;p&gt; to break lines | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavadocDeclaration` | Javadoc declaration problems | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavadocHtmlLint` | HTML problems in Javadoc (DocLint) | JAVA | false | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavadocLinkAsPlainText` | Link specified as plain text | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavadocReference` | Declaration has problems in Javadoc references | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MismatchedJavadocCode` | Mismatch between Javadoc and code | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingDeprecatedAnnotation` | Missing '@Deprecated' annotation | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingJavadoc` | Missing Javadoc | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingPackageInfo` | Missing 'package-info.java' | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageDotHtmlMayBePackageInfo` | 'package.html' may be converted to 'package-info.java' | HTML | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageInfoWithoutPackage` | 'package-info.java' without 'package' statement | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceWithJavadoc` | Comment replaceable with Javadoc | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryInheritDoc` | Unnecessary '{@inheritDoc}' Javadoc comment | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryJavaDocLink` | Unnecessary Javadoc link | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Logging (6)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassWithMultipleLoggers` | Class with multiple loggers | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassWithoutLogger` | Class without logger | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoggerInitializedWithForeignClass` | Logger initialized with foreign class | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonStaticFinalLogger` | Non-constant logger | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicMethodWithoutLogging` | 'public' method without logging | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringConcatenationArgumentToLogCall` | String concatenation as argument to logging call | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Lombok (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DeprecatedLombok` | Deprecated Lombok annotations | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `Lombok` | Lombok annotations | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LombokFlagUsage` | Lombok flag usages | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SpringQualifierCopyableLombok` | @Qualifier not copyable by Lombok | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticMethodImportLombok` | Using static import for Lombok-generated methods | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Memory (8)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AnonymousInnerClassMayBeStatic` | Anonymous class may be a named 'static' inner class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InnerClassMayBeStatic` | Inner class may be 'static' | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-051 |
| `ReturnOfInnerClass` | Return of anonymous, local or inner class instance | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticCollection` | Static collection | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringBufferField` | 'StringBuilder' field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SystemGC` | Call to 'System.gc()' or 'Runtime.gc()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryEmptyArrayUsage` | Unnecessary zero length array usage | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ZeroLengthArrayInitialization` | Zero-length array allocation | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Method metrics (11)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CyclomaticComplexity` | Overly complex method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodCoupling` | Overly coupled method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodWithMultipleLoops` | Method with multiple loops | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MultipleReturnPointsPerMethod` | Method with multiple return points | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestingDepth` | Overly nested method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonCommentSourceStatements` | Overly long method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverlyLongLambda` | Overly long lambda expression | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ParametersPerConstructor` | Constructor with too many parameters | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ParametersPerMethod` | Method with too many parameters | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreeNegationsPerMethod` | Method with more than three negations | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThrownExceptionsPerMethod` | Method with too many exceptions declared | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Modularization issues (6)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassIndependentOfModule` | Class independent of its module | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassOnlyUsedInOneModule` | Class only used from one other module | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassOverriddenAtRuntime` | Class loaded from a different dependency at runtime | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InconsistentLanguageLevel` | Inconsistent language level settings |  | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ModuleWithTooFewClasses` | Module with too few classes | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ModuleWithTooManyClasses` | Module with too many classes | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
