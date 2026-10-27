# Inspections: java (part 6 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Java / Naming conventions (11)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `DollarSignInName` | Use of '$' in identifier | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FieldNamingConvention` | Field naming convention | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaModuleNaming` | Java module name violates convention | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LambdaParameterNamingConvention` | Lambda parameter naming convention | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LocalVariableNamingConvention` | Local variable naming convention | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageNamingConvention` | Package naming convention | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ParameterNameDiffersFromOverriddenParameter` | Parameter name differs from parameter in overridden or overloaded method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ParameterNamingConvention` | Method parameter naming convention | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `QuestionableName` | Questionable name | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StandardVariableNames` | Standard variable names | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UpperCaseFieldNameNotConstant` | Non-constant field with upper-case name | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Numeric issues (28)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `BadOddness` | Suspicious oddness check | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BigDecimalEquals` | 'equals()' called on 'BigDecimal' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BigDecimalMethodWithoutRoundingCalled` | Call to 'BigDecimal' method without a rounding mode argument | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CachedNumberConstructorCall` | Number constructor call with primitive argument | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `CharUsedInArithmeticContext` | 'char' expression used in arithmetic context | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComparisonOfShortAndChar` | Comparison of 'short' and 'char' values | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComparisonToNaN` | Comparison to 'Double.NaN' or 'Float.NaN' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConfusingFloatingPointLiteral` | Confusing floating-point literal | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantMathCall` | Constant call to 'Math' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `DivideByZero` | Division by zero | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FloatingPointEquality` | Floating-point equality comparison | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ImplicitNumericConversion` | Implicit numeric conversion | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InsertLiteralUnderscores` | Numeric literal can use underscore separators | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IntegerDivisionInFloatingPointContext` | Integer division in floating-point context | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LongLiteralsEndingWithLowercaseL` | 'long' literal ending with 'l' instead of 'L' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `LossyConversionCompoundAssignment` | Possibly lossy implicit cast in compound assignment | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NegativeIntConstantInLongContext` | Negative int hexadecimal constant in long context | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonReproducibleMathCall` | Non-reproducible call to 'Math' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NumericOverflow` | Numeric overflow | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OctalAndDecimalIntegersMixed` | Octal and decimal integers in same array | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OctalLiteral` | Octal integer | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverlyComplexArithmeticExpression` | Overly complex arithmetic expression | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PointlessArithmeticExpression` | Pointless arithmetic expression | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RemoveLiteralUnderscores` | Underscores in numeric literal | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousLiteralUnderscore` | Suspicious underscore in number literal | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnaryPlus` | Unary plus | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryUnaryMinus` | Unnecessary unary minus | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnpredictableBigDecimalConstructorCall` | Unpredictable 'BigDecimal' constructor call | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Packaging issues (7)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassOnlyUsedInOnePackage` | Class only used from one other package | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassUnconnectedToPackage` | Class independent of its package | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `DisjointPackage` | Package with disjoint dependency graph | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExceptionPackage` | Exception package | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageInMultipleModules` | Package with classes in multiple modules | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageWithTooFewClasses` | Package with too few classes | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `PackageWithTooManyClasses` | Package with too many classes | JAVA | false | WARNING | global | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Performance (47)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ArraysAsListWithZeroOrOneArgument` | Call to 'Arrays.asList()' with too few arguments | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `AutoBoxing` | Auto-boxing | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AutoUnboxing` | Auto-unboxing | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BooleanConstructor` | Boolean constructor call | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `BoxingBoxedValue` | Boxing of already boxed value | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `BulkFileAttributesRead` | Bulk 'Files.readAttributes()' call can be used | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CallToSimpleGetterInClass` | Call to simple getter from within class | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `CallToSimpleSetterInClass` | Call to simple setter from within class | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassInitializerMayBeStatic` | Instance initializer can be made 'static' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CollectionAddAllCanBeReplacedWithConstructor` | Redundant 'Collection.addAll()' call | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CollectionsMustHaveInitialCapacity` | Collection without initial capacity | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DynamicRegexReplaceableByCompiledPattern` | Dynamic regular expression can be replaced by compiled 'Pattern' | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExplicitArgumentCanBeLambda` | Explicit argument can be lambda | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FieldMayBeStatic` | Field can be made 'static' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IfStatementMissingBreakInLoop` | Early loop exit in 'if' condition | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncorrectLazyConstantUsage` | LazyConstant should be used only in final fields | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstantiatingObjectToGetClassObject` | Instantiating object to get 'Class' object | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IOStreamConstructor` | 'InputStream' and 'OutputStream' can be constructed using 'Files' methods | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KeySetIterationMayUseEntrySet` | Iteration over 'keySet()' can be optimized | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LengthOneStringInIndexOf` | Single character string argument in 'String.indexOf()' call | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `LengthOneStringsInConcatenation` | Single character string concatenation | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ListRemoveInLoop` | 'List.remove()' called in loop | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ManualArrayCopy` | Manual array copy | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ManualArrayToCollectionCopy` | Manual array to collection copy | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `MapReplaceableByEnumMap` | 'Map' can be replaced with 'EnumMap' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodMayBeStatic` | Method can be made 'static' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectAllocationInLoop` | Object allocation in loop | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectEqualsCanBeEquality` | 'equals()' call can be replaced with '==' | JAVA | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectInstantiationInEqualsHashCode` | Object instantiation inside 'equals()' or 'hashCode()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RandomDoubleForRandomInteger` | Using 'Random.nextDouble()' to get random integer | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantStringFormatCall` | Redundant call to 'String.format()' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceAllNonRegex` | Non-regex 'replaceAll()' can be replaced with 'replace()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceInefficientStreamCount` | Inefficient Stream API call chains ending with count() | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SetReplaceableByEnumSet` | 'Set' can be replaced with 'EnumSet' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SlowAbstractSetRemoveAll` | Call to 'set.removeAll(list)' may work slowly | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SlowListContainsAll` | Call to 'list.containsAll(collection)' may have poor performance | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringBufferMustHaveInitialCapacity` | 'StringBuilder' without initial capacity | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringConcatenationInLoops` | String concatenation in loop | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-053 |
| `StringConcatenationInsideStringBufferAppend` | String concatenation as argument to 'StringBuilder.append()' call | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringEqualsEmptyString` | 'String.equals()' can be replaced with 'String.isEmpty()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringReplaceableByStringBuffer` | Non-constant 'String' can be replaced with 'StringBuilder' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TailRecursion` | Tail recursion | JAVA | true | INFORMATION | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ToArrayCallWithZeroLengthArrayArgument` | 'Collection.toArray()' call style | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryTemporaryOnConversionFromString` | Unnecessary temporary object in conversion from 'String' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnnecessaryTemporaryOnConversionToString` | Unnecessary temporary object in conversion to 'String' | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseBulkOperation` | Bulk operation can be used instead of iteration | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WrapperTypeMayBePrimitive` | Wrapper type may be primitive | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Portability (10)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `HardcodedFileSeparators` | Hardcoded file separator | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HardcodedLineSeparators` | Hardcoded line separator | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NativeMethods` | Native method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RuntimeExec` | Call to 'Runtime.exec()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SystemExit` | Call to 'System.exit()' or related methods | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SystemGetenv` | Call to 'System.getenv()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOfAWTPeerClass` | Use of AWT peer class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOfJDBCDriverClass` | Use of concrete JDBC driver class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOfProcessBuilder` | Use of 'java.lang.ProcessBuilder' class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UseOfSunClasses` | Use of 'sun.*' classes | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
