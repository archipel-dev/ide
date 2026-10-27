# Inspections: java (part 8 of 10)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Java / Security (16)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ClassLoaderInstantiation` | 'ClassLoader' instantiation | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CloneableClassInSecureContext` | Cloneable class in secure context | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CustomClassloader` | Custom 'ClassLoader' is declared | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CustomSecurityManager` | Custom 'SecurityManager' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DesignForExtension` | Design for extension | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JDBCExecuteWithNonConstantString` | Call to 'Statement.execute()' with non-constant string | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JDBCPrepareStatementWithNonConstantString` | Call to 'Connection.prepare*()' with non-constant string | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LoadLibraryWithNonConstantString` | Call to 'System.loadLibrary()' with non-constant string | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonFinalClone` | Non-final 'clone()' in secure context | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicStaticArrayField` | 'public static' array field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicStaticCollectionField` | 'public static' collection field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RuntimeExecWithNonConstantString` | Call to 'Runtime.exec()' with non-constant string | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableDeserializableClassInSecureContext` | Serializable class in secure context | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SystemProperties` | Access of system properties | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SystemSetSecurityManager` | Call to 'System.setSecurityManager()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnsecureRandomNumberGeneration` | Insecure random number generation | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Serialization issues (23)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `ComparatorNotSerializable` | 'Comparator' class not declared 'Serializable' | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExternalizableWithoutPublicNoArgConstructor` | 'Externalizable' class without 'public' no-arg constructor | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExternalizableWithSerializationMethods` | Externalizable class with 'readObject()' or 'writeObject()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingSerialAnnotation` | '@Serial' annotation can be used | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonSerializableFieldInSerializableClass` | Non-serializable field in a 'Serializable' class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonSerializableObjectBoundToHttpSession` | Non-serializable object bound to 'HttpSession' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonSerializableObjectPassedToObjectStream` | Non-serializable object passed to 'ObjectOutputStream' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonSerializableWithSerializationMethods` | Non-serializable class with 'readObject()' or 'writeObject()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonSerializableWithSerialVersionUIDField` | Non-serializable class with 'serialVersionUID' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReadObjectAndWriteObjectPrivate` | 'readObject()' or 'writeObject()' not declared 'private' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReadObjectInitialization` | Instance field may not be initialized by 'readObject()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ReadResolveAndWriteReplaceProtected` | 'readResolve()' or 'writeReplace()' not declared 'protected' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerialAnnotationUsedOnWrongMember` | '@Serial' annotation used on wrong member | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableHasSerializationMethods` | Serializable class without 'readObject()' and 'writeObject()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableInnerClassHasSerialVersionUIDField` | Serializable non-static inner class without 'serialVersionUID' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableInnerClassWithNonSerializableOuterClass` | Serializable non-'static' inner class with non-Serializable outer class | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableRecordContainsIgnoredMembers` | 'record' contains ignored members | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableStoresNonSerializable` | 'Serializable' object implicitly stores non-'Serializable' object | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerializableWithUnconstructableAncestor` | Serializable class with unconstructable ancestor | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerialPersistentFieldsWithWrongSignature` | 'serialPersistentFields' field not declared 'private static final ObjectStreamField[]' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SerialVersionUIDNotStaticFinal` | 'serialVersionUID' field not declared 'private static final long' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `TransientFieldInNonSerializableClass` | Transient field in non-serializable class | JAVA | false | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |
| `TransientFieldNotInitialized` | Transient field is not initialized on deserialization | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Test frameworks (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AssertWithoutMessage` | Message missing on assertion | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConstantAssertArgument` | Constant assert argument | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MaskedAssertion` | Assertion is suppressed by 'catch' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MisorderedAssertEqualsArguments` | Misordered 'assertEquals()' arguments | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SimplifiableAssertion` | Simplifiable assertion | JAVA | true | WARNING | local+cleanup | framework: codeinsight/03-inspections.md (CI-INS) |

## Java / Threading issues (52)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AccessToNonThreadSafeStaticFieldFromInstance` | Non-thread-safe 'static' field access | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AccessToStaticFieldLockedOnInstance` | Access to 'static' field locked on instance data | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AtomicFieldUpdaterIssues` | Inconsistent 'AtomicFieldUpdater' declaration | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AtomicFieldUpdaterNotStaticFinal` | 'AtomicFieldUpdater' field not declared 'static final' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AwaitNotInLoop` | 'await()' not called in loop | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AwaitWithoutCorrespondingSignal` | 'await()' without corresponding 'signal()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BusyWait` | Busy wait | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CallToNativeMethodWhileLocked` | Call to a 'native' method while locked | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ConditionSignal` | Call to 'signal()' instead of 'signalAll()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `DoubleCheckedLocking` | Double-checked locking | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptySynchronizedStatement` | Empty 'synchronized' statement | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ExtendsThread` | Class directly extends 'Thread' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `FieldAccessedSynchronizedAndUnsynchronized` | Field accessed in both 'synchronized' and unsynchronized contexts | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MethodMayBeSynchronized` | Method with single 'synchronized' block can be replaced with 'synchronized' method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NakedNotify` | 'notify()' or 'notifyAll()' without corresponding state change | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NestedSynchronizedStatement` | Nested 'synchronized' statement | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonAtomicOperationOnVolatileField` | Non-atomic operation on 'volatile' field | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NonSynchronizedMethodOverridesSynchronizedMethod` | Unsynchronized method overrides 'synchronized' method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NotifyCalledOnCondition` | 'notify()' or 'notifyAll()' called on 'java.util.concurrent.locks.Condition' object | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `NotifyWithoutCorrespondingWait` | 'notify()' without corresponding 'wait()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ObjectNotify` | Call to 'notify()' instead of 'notifyAll()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PublicFieldAccessedInSynchronizedContext` | Non-private field accessed in 'synchronized' context | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SafeLock` | Lock acquired but not safely unlocked | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SharedThreadLocalRandom` | 'ThreadLocalRandom' instance might be shared | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SignalWithoutCorrespondingAwait` | 'signal()' without corresponding 'await()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SleepWhileHoldingLock` | Call to 'Thread.sleep()' while synchronized | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `StaticInitializerReferencesSubClass` | Static initializer references subclass | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizationOnGetClass` | Synchronization on 'getClass()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizationOnLocalVariableOrMethodParameter` | Synchronization on local variable or method parameter | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizationOnStaticField` | Synchronization on 'static' field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizedMethod` | 'synchronized' method | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizedOnLiteralObject` | Synchronization on an object initialized with a literal | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizeOnLock` | Synchronization on a 'Lock' object | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizeOnNonFinalField` | Synchronization on a non-final field | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SynchronizeOnThis` | Synchronization on 'this' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SystemRunFinalizersOnExit` | Call to 'System.runFinalizersOnExit()' | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadLocalNotStaticFinal` | 'ThreadLocal' field not declared 'static final' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadLocalSetWithNull` | 'ThreadLocal.set()' with null as an argument | JAVA | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadPriority` | Call to 'Thread.setPriority()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadStartInConstruction` | Call to 'Thread.start()' during object construction | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadStopSuspendResume` | Call to 'Thread.stop()', 'suspend()' or 'resume()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadWithDefaultRunMethod` | Instantiating a 'Thread' with default 'run()' method | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ThreadYield` | Call to 'Thread.yield()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnconditionalWait` | Unconditional 'wait()' call | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `VolatileArrayField` | Volatile array field | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WaitCalledOnCondition` | 'wait()' called on 'java.util.concurrent.locks.Condition' object | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WaitNotifyNotInSynchronizedContext` | 'wait()' or 'notify()' is not in synchronized context | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WaitNotInLoop` | 'wait()' not called in loop | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WaitOrAwaitWithoutTimeout` | 'wait()' or 'await()' without timeout | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WaitWhileHoldingTwoLocks` | 'wait()' while holding two locks | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WaitWithoutCorrespondingNotify` | 'wait()' without corresponding 'notify()' | JAVA | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WhileLoopSpinsOnField` | 'while' loop spins on field | JAVA | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
