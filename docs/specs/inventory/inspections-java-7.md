# Inspections: java (part 7 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Java / Probable bugs (107)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ArrayEquality` | Array comparison using '==', instead of 'Arrays.equals()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ArrayEquals` | 'equals()' called on array | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-015 |
| `ArrayHashCode` | 'hashCode()' called on array | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-016 |
| `ArrayObjectsEquals` | Use of shallow or 'Objects' methods with arrays | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AssertWithSideEffects` | 'assert' statement with side effects | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CapturingCleaner` | Cleaner captures object reference | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CastConflictsWithInstanceof` | Cast conflicts with 'instanceof' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CastToIncompatibleInterface` | Cast to incompatible type | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassGetClass` | Suspicious 'Class.getClass()' call | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ClassNewInstance` | Unsafe call to 'Class.newInstance()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CollectionAddedToSelf` | Collection added to itself | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComparableImplementedButEqualsNotOverridden` | 'Comparable' implemented but 'equals()' not overridden | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComparatorMethodParameterNotUsed` | Suspicious 'Comparator.compare()' implementation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ComparatorResultComparison` | Suspicious usage of compare method | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CompareToUsesNonFinalVariable` | Non-final field referenced in 'compareTo()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConfusingMainMethod` | Confusing 'main()' method | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantAssertCondition` | Constant condition in 'assert' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantValue` | Constant values | JAVA | true | WARNING | local | codeinsight/03-inspections.md CI-INS-105 |
| `Contract` | Contract issues | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CopyConstructorMissesField` | Copy constructor misses field | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CovariantEquals` | Covariant 'equals()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DataFlowIssue` | Nullability and data flow problems | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-011 |
| `EmptyStatementBody` | Statement with empty body | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-032 |
| `EndlessStream` | Non-short-circuit operation consumes infinite stream | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EqualsAndHashcode` | 'equals()' and 'hashCode()' not paired | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EqualsBetweenInconvertibleTypes` | 'equals()' between objects of inconvertible types | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-014 |
| `EqualsOnSuspiciousObject` | 'equals()' called on classes which don't override it | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EqualsUsesNonFinalVariable` | Non-final field referenced in 'equals()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EqualsWhichDoesntCheckParameterClass` | 'equals()' method that does not check the class of its parameter | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EqualsWithItself` | 'equals()' called on itself | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExpressionComparedToItself` | Expression is compared to itself | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HashCodeUsesNonFinalVariable` | Non-final field referenced in 'hashCode()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IgnoreResultOfCall` | Result of method call ignored | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-023 |
| `ImplicitArrayToString` | Call to 'toString()' on array | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InconsistentTextBlockIndent` | Inconsistent whitespace indentation in text block | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncorrectDateTimeFormat` | Incorrect 'DateTimeFormat' pattern | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IncorrectMessageFormat` | Incorrect 'MessageFormat' pattern | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InfiniteRecursion` | Infinite recursion | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-024 |
| `InnerClassReferencedViaSubclass` | Inner class referenced via subclass | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstanceofIncompatibleInterface` | 'instanceof' with incompatible type | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InstantiationOfUtilityClass` | Instantiation of utility class | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `InvalidComparatorMethodReference` | Invalid method reference used for 'Comparator' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IterableUsedAsVararg` | Iterable is used as vararg | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IteratorHasNextCallsIteratorNext` | 'Iterator.hasNext()' which calls 'next()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IteratorNextDoesNotThrowNoSuchElementException` | 'Iterator.next()' which can't throw 'NoSuchElementException' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MagicConstant` | Magic constant | JAVA | true | WARNING | local | java/06-completion.md JAVA-CMP-004 |
| `MalformedFormatString` | Malformed format string | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-026 |
| `MathRandomCastToInt` | 'Math.random()' cast to 'int' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MathRoundingWithIntArgument` | Call math rounding with 'int' argument | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MeaninglessRecordAnnotationInspection` | Meaningless record annotation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MismatchedArrayReadWrite` | Mismatched read and write of array | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MismatchedCollectionQueryUpdate` | Mismatched query and update of collection | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MismatchedStringBuilderQueryUpdate` | Mismatched query and update of 'StringBuilder' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MismatchedStringCase` | Mismatched case in 'String' operation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MisspelledEquals` | 'equal()' instead of 'equals()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NewObjectEquality` | New object is compared using '==' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NewStringBufferWithCharArgument` | StringBuilder constructor call with 'char' argument | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonShortCircuitBoolean` | Non-short-circuit boolean expression | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NullArgumentToVariableArgMethod` | Confusing argument to varargs method | JAVA | true | WARNING | local | java/03-types.md JAVA-TYP-020 |
| `NumberEquality` | Number comparison using '==', instead of 'equals()' | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-018 |
| `ObjectEquality` | Object comparison using '==', instead of 'equals()' | JAVA | true | INFORMATION | local | java/07-inspections.md JAVA-INS-019 |
| `ObjectToString` | Call to default 'toString()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OptionalGetWithoutIsPresent` | Optional.get() is called without isPresent() check | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-020 |
| `OptionalOfNullableMisuse` | Use of Optional.ofNullable with null or non-null argument | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverflowingLoopIndex` | Loop executes zero or billions of times | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OverwrittenKey` | Overwritten Map, Set, or array element | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PrimitiveArrayArgumentToVariableArgMethod` | Confusing primitive array argument to varargs method | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantOperationOnEmptyContainer` | Redundant operation on empty container | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReflectionForUnavailableAnnotation` | Reflective access to a source-only annotation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReplaceAllDot` | Suspicious regex expression argument | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ResultOfObjectAllocationIgnored` | Result of object allocation ignored | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ResultSetIndexZero` | Use of index 0 in JDBC ResultSet | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SafeVarargsHasNoEffect` | @SafeVarargs do not suppress potentially unsafe operations | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ScheduledThreadPoolExecutorWithZeroCoreThreads` | 'ScheduledThreadPoolExecutor' with zero core threads | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SortedCollectionWithNonComparableKeys` | Sorted collection with non-comparable elements | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticCallOnSubclass` | Static method referenced via subclass | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticFieldReferenceOnSubclass` | Static field referenced via subclass | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringConcatenationInFormatCall` | String concatenation as argument to 'format()' call | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringConcatenationInMessageFormatCall` | String concatenation as argument to 'MessageFormat.format()' call | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringConcatenationMissingWhitespace` | Whitespace may be missing in string concatenation | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringEquality` | String comparison using '==', instead of 'equals()' | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-017 |
| `StringEqualsCharSequence` | 'String.equals()' called with 'CharSequence' argument | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StringTokenizerDelimiter` | Duplicated delimiters in 'StringTokenizer' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SubtractionInCompareTo` | Subtraction in 'compareTo()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousArrayCast` | Suspicious array cast | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousArrayMethodCall` | Suspicious 'Arrays' method call | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousDateFormat` | Suspicious date format pattern | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousIndentAfterControlStatement` | Suspicious indentation after control statement without braces | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousIntegerDivAssignment` | Suspicious integer division assignment | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousInvocationHandlerImplementation` | Suspicious 'InvocationHandler' implementation | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousListRemoveInLoop` | Suspicious 'List.remove()' in loop | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousMethodCalls` | Suspicious collection method call | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-028 |
| `SuspiciousNameCombination` | Suspicious variable/parameter name combination | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousReturnByteInputStream` | Suspicious byte value returned from 'InputStream.read()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousSystemArraycopy` | Suspicious 'System.arraycopy()' call | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousTernaryOperatorInVarargsCall` | Suspicious ternary operator in varargs method call | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SuspiciousToArrayCall` | Suspicious 'Collection.toArray()' call | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TextLabelInSwitchStatement` | Text label in 'switch' statement | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThrowableNotThrown` | 'Throwable' not thrown | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-027 |
| `UnreachableCode` | Unreachable code | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnsatisfiedRange` | Return value is outside of declared range | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnsupportedChronoFieldUnitCall` | Call methods with unsupported 'java.time.temporal.ChronoUnit' and 'java.time.temporal.ChronoField' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedAssignment` | Unused assignment | JAVA | true | WARNING | local | codeinsight/03-inspections.md CI-INS-105 |
| `UseOfPropertiesAsHashtable` | Use of 'Properties' object as a 'Hashtable' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `VariableNotUsedInsideIf` | Reference checked for 'null' is not used inside 'if' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WriteOnlyObject` | Write-only object | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WrongPackageStatement` | Wrong package statement | JAVA | true | ERROR | local | codeinsight/03-inspections.md CI-INS-105 |

## Java / Properties files (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `UnresolvedPropertyKey` | Invalid property key | JAVA | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnsupportedCharacter` | Unsupported character | UAST | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Reflective access (4)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Java9ReflectionClassVisibility` | Reflective access across modules issues | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaLangInvokeHandleSignature` | MethodHandle/VarHandle type mismatch | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaReflectionInvocation` | Reflective invocation arguments mismatch | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JavaReflectionMemberAccess` | Reflective access to non-existent or not visible class member | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Resource management (8)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AutoCloseableResource` | AutoCloseable used without 'try'-with-resources | JAVA | true | WARNING | local | java/07-inspections.md JAVA-INS-056 |
| `ChannelResource` | 'Channel' opened but not safely closed | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DriverManagerGetConnection` | Use of 'DriverManager' to get JDBC connection | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HibernateResource` | Hibernate resource opened but not safely closed | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IOResource` | I/O resource opened but not safely closed | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JDBCResource` | JDBC resource opened but not safely closed | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JNDIResource` | JNDI resource opened but not safely closed | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SocketResource` | Socket opened but not safely closed | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
