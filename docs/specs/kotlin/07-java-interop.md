# 07. Java and Kotlin interoperability in the IDE

Status: specification, clean-room, our own words. Requirement prefix `KT-JVM`. Executable cases:
[cases/07-java-interop.cases.md](cases/07-java-interop.cases.md).

ic: `plugins/kotlin/base/resources/resources-en/messages/KotlinBundle.properties` (find-usages strings, gutter texts),
`plugins/kotlin/base/analysis/src/org/jetbrains/kotlin/idea/stubindex/*` (facade, JvmName and multifile indexes, see 01),
`plugins/kotlin/completion/impl-k2/.../java/{KotlinCompanionMemberContributor,KotlinExtensionContributor}.kt`,
`plugins/kotlin/code-insight/line-markers/**`. The light-class machinery lives in the Kotlin compiler's Analysis API; the **observable
shape** is fixed by the Kotlin/Java interop documentation, <https://kotlinlang.org/docs/java-to-kotlin-interop.html> and
<https://kotlinlang.org/docs/java-interop.html>, and by the JVM section of the language specification.

## 7.1 Light classes: what Java sees

For every Kotlin class the IDE presents a *light* Java class to Java code (resolve, completion, find usages, inspections), with the members below.

| Kotlin | Java view |
|---|---|
| `class A` in package `p` | `p.A`, public final (open: non-final), with a constructor per primary/secondary constructor; `@JvmOverloads` adds overloads dropping trailing default parameters |
| `val x`, `var y` (member) | private field `x`/`y` when a backing field exists, methods `getX()`, `getY()`, `setY(T)`; Boolean `isFoo` -> `isFoo()`/`setFoo(T)` |
| `@JvmField val x` / `const val X` | public field (static for `const`, object and companion members) |
| member function `f` | method `f`, parameter names kept |
| `@JvmName("g") fun f()` | method `g`; for accessors `@get:JvmName`/`@set:JvmName` |
| top-level function/property in `Foo.kt` | static member of facade `FooKt` (`01` 1.6) |
| `object O` | class with `public static final O INSTANCE`; members are instance methods unless `@JvmStatic`/`@JvmField`/`const` |
| `companion object` | static nested class `Companion` and field `Companion` in the outer class; `@JvmStatic` members are also static methods of the outer class; `@JvmField`/`const`/`lateinit` properties are static fields of the outer class |
| `@JvmStatic` in object | static method (and, for properties, static accessors) |
| `data class` | adds `componentN`, `copy`, `equals`, `hashCode`, `toString` |
| `enum class` | Java enum with `values()`, `valueOf`, (and `getEntries()`) |
| `interface` with default bodies | interface; with `-Xjvm-default` default methods else `DefaultImpls` class (`01` index `DefaultImpls`) |
| `suspend fun f(a: A): R` | `f(A, Continuation<? super R>)` returning `Object` |
| `fun f() throws via @Throws(X::class)` | `throws X` |
| `inline class`/value class | mangled method names (`f-<hash>`), the class itself erased to its underlying type where it appears in signatures |
| `internal` member | public with a mangled name `f$module` |
| `private` top-level in file | private static in the facade |
| `vararg a: T` | `T...`; `Array<T>` -> `T[]`; `Unit` return -> `void` |
| Kotlin nullability | `@NotNull`/`@Nullable` annotations on members |
| property type `List<Int>` | `List<Integer>`; wildcards per declaration-site variance (`List<? extends X>` for parameters) |

`KT-JVM-001` MUST: the light shape above for resolve, completion and inspections in Java files. Acceptance: Java code calling `FooKt.f()`, `a.getX()`, `O.INSTANCE.g()`, `A.Companion.h()`, `A.h()` (with `@JvmStatic`) resolves; calling a non-static companion member as `A.h()` is the Java error `Non-static method 'h()' cannot be referenced from a static context`.

`KT-JVM-002` MUST: `@JvmName`, `@JvmStatic`, `@JvmField`, `@JvmOverloads`, `@Throws`, `@JvmMultifileClass`, `@file:JvmName` change the Java view exactly as in the table; renaming the Kotlin declaration updates the annotation-driven names only when the user confirms (see 7.4). Acceptance: cases KT-JVM-002.

## 7.2 Navigation across languages

- **Go to Declaration** (Ctrl+B) on a Java reference to a light member lands on the Kotlin declaration (for an accessor `getX()` on the property `x`; for `FooKt` on the file's facade -> the file; for `INSTANCE` on the object; for `Companion` on the companion).
- **Go to Declaration** from Kotlin on a Java member lands in the Java source (or decompiled/attached source); on a synthetic property `length` of a Java getter, to the getter.
- **Go to Implementation** (Ctrl+Alt+B) and the gutter icons work across languages: a Kotlin interface member shows `Has implementations`/`Is implemented in`; overriding members show `Overrides function in` / `Implements function in`
  (properties: `Overrides property in`, `Implements property in`); classes show `Is subclassed by` / `Is implemented by`; chooser titles `Choose Implementation of <name>` and `Choose Super Implementation of <name>`.
- **Go to Super** (Ctrl+U): for Kotlin members the actions `Go to Super Function`/`Super Property`/`Super Class or Interface` with chooser titles `Choose Super Function`, `Choose Super Property`, `Choose Super Class or Interface`.
- **Type hierarchy / call hierarchy** include members of both languages.

`KT-JVM-010` MUST: Ctrl+B in both directions as above. Acceptance: cases KT-JVM-010.
`KT-JVM-011` SHOULD: gutter texts and choosers above. Acceptance: cases KT-JVM-011.

## 7.3 Find Usages across languages

Find Usages (Alt+F7) on a Kotlin declaration also finds Java usages of its light members, and vice versa:

| Declaration | Usages found in Java |
|---|---|
| property `x` | calls of `getX()`/`setX()`, uses of the field if `@JvmField`/`const`; the dialog has `Readers` and `Writers` checkboxes to separate getter/setter usages |
| function with `@JvmName("g")` | calls of `g` |
| companion/`@JvmStatic` member | `A.h()` and `A.Companion.h()` |
| top-level function | `FooKt.f()` |
| class | `new A()`, subclassing, type references |
| `data class` property | `componentN()` calls (setting `Fast data class component search`) |

The Kotlin Find Usages dialog for a function shows: `Usages of functions`, `Include overloaded functions and extensions`, `Implementing functions`, `Overriding functions`; for a property: `Usages of properties`, `Readers`, `Writers`, `Implementing properties`, `Overriding properties`; for a class: `Derived classes`, `Derived interfaces`, plus expected/actual checkboxes `Expected classes/functions/properties`. Usage-type groups in the Usages tool window: `Named argument`, `Type alias`, `Callable reference`, `Constructor delegation reference`, `Type constraint`, `Parameter type`, `Class/object property type`, `Function return types`, `Supertype`, `Target type of 'is' operation`, `Nested class/object`, `Companion object`, `Function call`, `Implicit 'get'`, `Implicit 'set'`, `Implicit 'invoke'`, `Implicit iteration`, `Property delegation`, `Extension receiver type`, `Super type qualifier`, `Receiver`, `Delegate`, `Package directive`, `Package member access`, `Import alias`, `Label`, `Lambda`.
Element words in titles: `class`, `companion object`, `constructor`, `facade class`, `function`, `getter`, `setter`, `interface`, `label`, `lambda`, `object`, `parameter`, `property`, `property accessor`, `type alias`, `type parameter`, `variable`.

`KT-JVM-020` MUST: usages of light members in Java found from Kotlin declarations and Kotlin usages found from Java elements, with the grouping above. Acceptance: cases KT-JVM-020.

## 7.4 Rename and refactoring across languages

Rename of a Kotlin property renames its getter/setter usages in Java (`getName` -> `getNewName`); rename of a Java getter `getX()` offers to rename the Kotlin property usage `x`. When a declaration has `@JvmName`, rename of the Kotlin name keeps the annotation. Renaming a class renames its file when the file has the same name (checkbox `Rename file`). Renaming `Foo.kt` offers to rename the facade `FooKt` references in Java. (Details: refactoring specs.) `KT-JVM-030` SHOULD.

## 7.5 Java inspections for Kotlin code

`HasPlatformType`, `JavaCollectionsStaticMethodOnImmutableList`, `JavaCollectionWithNullableTypeArgument`, `JavaDefaultMethodsNotOverriddenByDelegation`, `PackageDirectoryMismatch` (Appendix A of `05`) and the unresolved nullability of Java platform types: a Java type `String` is shown as `String!` and accepted for `String` and `String?`; a Java member with `@Nullable`/`@NotNull` (JetBrains, JSR-305, AndroidX, JSpecify) has the corresponding Kotlin nullability; a diagnostic `Type mismatch` with the Java-specific forms of `03` applies.

`KT-JVM-040` MUST: platform types and nullability annotations. Acceptance: cases KT-JVM-040.

## 7.6 Kotlin in Java completion

`KT-JVM-050` SHOULD: Kotlin companion members and extension functions in Java completion (`04` 4.13): `Foo.<caret>` lists companion members; `str.<caret>` lists Kotlin extensions on `String` marked `[Kotlin Extension]`; choosing inserts `FooKt.ext(str)` and the import of `FooKt`.

## Open questions
- Q-JVM-1: exact set of Java-visible mangled names for value classes and internal members (follow the JVM backend of the targeted compiler).
