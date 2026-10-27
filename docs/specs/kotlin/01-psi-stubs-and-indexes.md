# 01. Kotlin PSI, stubs, indexes, file facades

Status: specification, clean-room. Written from the IntelliJ IDEA Community sources (Kotlin plugin, K2
mode) and the Kotlin language specification, in our own words. Requirement prefix `KT-STUB`.
Executable cases: [cases/01-stubs-and-indexes.cases.md](cases/01-stubs-and-indexes.cases.md).

ic: `plugins/kotlin/base/analysis/src/org/jetbrains/kotlin/idea/stubindex/IdeStubIndexService.java`,
`plugins/kotlin/base/analysis/src/org/jetbrains/kotlin/idea/stubindex/IndexUtils.kt`,
`plugins/kotlin/base/indices/src/org/jetbrains/kotlin/idea/stubindex/*.kt`,
`plugins/kotlin/code-insight/impl-base/src/org/jetbrains/kotlin/idea/imports/KotlinIdeDefaultImportProvider.kt`,
`plugins/kotlin/completion/impl-k2/resources/intellij.kotlin.completion.impl.xml` (dumb-mode registry key).
Language semantics (what a declaration *is*) come from the Kotlin language specification,
<https://kotlinlang.org/spec/> (chapters "Declarations", "Packages and imports", "Scopes and identifiers",
"Kotlin/Core", "Kotlin/JVM"); this file specifies only what the IDE derives from a declaration and stores.

Scope: the language-independent stub/index machinery is `platform/03-stubs-and-indexes.md`; dumb mode is
`platform/04-dumb-mode.md`. Here: what Kotlin adds. Resolution that *uses* these indexes is `02-resolve-and-imports.md`;
Java-visible shapes (light classes) are `07-java-interop.md`.

Terminology. *Declaration* = a Kotlin class, interface, object, function, property, type alias, enum entry,
constructor or parameter that declares a property. *Prime symbol* = defined in 1.4. *Receiver key* = defined in 1.5.
*Facade* = the synthetic JVM class that holds a file's top-level functions and properties (1.6).

---

## 1.1 What a Kotlin file is

A Kotlin source file (`.kt`) has, in this order: optional file annotations (`@file:...`), optional `package`
directive, an import list, then top-level declarations. A script (`.kts`) has the same header and then
statements; its declarations live in a synthetic script class (1.8). Every file, also an empty one and one with
syntax errors, has a package directive node and an import list node (possibly empty) so tools can address
"the place where an import goes".

`KT-STUB-001` MUST: the parsed tree exposes for every Kotlin file: file annotation list, package directive (name or
empty = root package), import list with one node per `import` (path, `.*` flag, alias name), and the top-level
declarations in source order. Acceptance: `package a.b` + two imports + one class gives package `a.b`, two import
nodes with the exact path text, one declaration; an empty file has an empty package and an empty import list.

`KT-STUB-002` MUST: the file's package is the directive name, **not** the directory. A mismatch is only reported by the
`PackageDirectoryMismatch` inspection (05). Acceptance: `src/x/A.kt` with `package y` declares `y.A`; lookups by FQ name
`y.A` find it, lookups by `x.A` do not.

## 1.2 Stubs: which declarations are stored

The index layer works on *stubs*: a compact, serialisable subtree of each file kept in the index so declarations can
be found without parsing the file (`platform/03-stubs-and-indexes.md`). Kotlin stubs exist for these nodes and store
at least these fields (field = fact the IDE reads without opening the text; names are ours):

| Node | Stored fields |
|---|---|
| File | package FQ name, file kind (plain, facade, multifile-class facade/part, script, class-file-derived), facade FQ name and, for multifile facades, the part simple names |
| Class / interface / enum / annotation / value class | name, FQ name (null for local), kind flags (interface, enum, annotation, data, value, fun-interface, sealed, inner, local, top-level), modifier list, **super type names** (simple names as written, unresolved; enum adds `Enum`, annotation adds `Annotation`) |
| Object (named, companion, literal) | name, FQ name, companion flag, object-literal flag, super type names |
| Function | name, FQ name when top level, top-level flag, extension flag, has block body, has body, receiver type reference text, type parameters, "has contract" flag |
| Property | name, FQ name when top level, `var`/`val`, extension flag, has initializer, has delegate, has explicit type, receiver type reference |
| Value parameter | name, has `val`/`var` (declares a property), has default value, is mutable |
| Type alias | name, FQ name, the aliased type reference |
| Annotation entry | short name as written, has arguments |
| Modifier list | modifier tokens (visibility, modality, `override`, `inline`, `operator`, `infix`, `suspend`, `const`, `lateinit`, `expect`, `actual`, ...) |
| Type reference / user type | referenced simple name, qualifier chain, nullable flag, function-type arity and receiver and `suspend` flag |
| Enum entry, secondary constructor, primary constructor | name or kind, parameter list |

What is **not** stored: bodies of functions and property initializers/accessors (no stub for local declarations,
lambdas, statements), expressions, string contents, KDoc text. Anything the daemon needs from those parses the file.

`KT-STUB-003` MUST: for every row above the fields are available from the index without parsing the file text and are
identical to what parsing the file yields. Acceptance: for a file with a data class, an extension function and a
companion object, reading fields from stubs equals reading them from the tree (checked for every field of the table).

`KT-STUB-004` MUST: stub fields that depend on *resolution* (the real type of a property without an explicit type, the
real super type of a class) are not stored. Super type names are *syntactic simple names* and are resolved at query
time. Acceptance: `class A : B()` stores super name `B` even when `B` is not imported or does not exist.

## 1.3 The index catalog

Each index maps a key (string) to the declarations that contributed it, per file; queries take a search scope
(resolve scope of the requesting file, `02` 2.8). All indexes below are *stub indexes* (rebuilt when a file's stub
changes) and unavailable in dumb mode except where stated.

| Index (our name) | Key | Contributing declarations | Used by |
|---|---|---|---|
| ClassShortName | class/object simple name (also `DefaultImpls` for every interface) | classes, interfaces, enums, annotations, objects (named, not literals) | name resolution of unqualified types, auto-import, Go to Class |
| FullClassName | class FQ name | non-local classes and objects | resolve of qualified names, imports |
| TypeAliasShortName / FullTypeAliasName | alias simple name / FQ name | type aliases | same as classes |
| TypeAliasByExpansionShortName | simple name(s) of the aliased type (see 1.5 rules) | type aliases | find usages of a class through its aliases |
| SuperClass | each super type simple name written in the header; `Enum` for enums, `Annotation` for annotation classes | classes and objects | Go to Implementation, hierarchy, "Is subclassed by" gutter, sealed-subclass search |
| SubclassObjectName | object simple name | named, non-literal objects that have at least one super type | auto-import and completion of members inherited from a super type but reachable through the object (2.7) |
| FunctionShortName | function simple name | every named function that is top-level or a member (not local functions) | unresolved call import, Go to Symbol, completion |
| PropertyShortName | property simple name | properties and constructor parameters declared `val`/`var` | same |
| TopLevelFunctionFqName / TopLevelPropertyFqName | FQ name | top-level functions/properties | resolve of FQ calls, Optimize Imports |
| TopLevelExpectFunctionFqName / TopLevelExpectPropertyFqName | FQ name | top-level `expect` declarations | multiplatform LATER |
| TopLevelExtensionsByReceiverType | receiver key (1.5) | top-level extension functions and properties | extension auto-import, extension completion |
| ExtensionsInObjectsByReceiverType | receiver key | extension functions/properties declared directly in a named (non-literal) object or companion | importing an object's extension member (`import Obj.ext`) |
| PrimeSymbolName | simple name | prime symbols (1.4) | fast "is this name declared at top level anywhere" test |
| ExactPackages | package FQ name | one per file | package existence, package completion |
| FileFacadeFqName / FileFacadeShortName / FileFacadeClassByPackage | facade FQ name / simple name / package FQ name | files that have a facade | Java-side resolve of `FooKt`, 07 |
| MultiFileClassPart | part class FQ name | multifile facades | 07 |
| Annotations | annotation short name as written **and** every import alias that stands for it in the file | annotation entries | find usages of an annotation class, "annotated with" searches |
| JvmNameAnnotation | the string literal of `@JvmName` | `@JvmName` entries whose value differs from the Kotlin name | Java-side name lookup (07) |
| ScriptFqn | script class FQ name | scripts | script resolve |
| ProbablyInjectedFunctionShortName | function name | functions with a parameter annotated `@Language` | language injection (LATER) |

`KT-STUB-010` MUST: every row of the table is contributed exactly as stated, and queries by key return the declarations
across all files in the scope. Acceptance: see cases 01 (one case per index family).

`KT-STUB-011` MUST: local functions, local classes, anonymous objects and lambdas contribute to **no** index.
Acceptance: a function declared inside another function's body is not returned by FunctionShortName.

`KT-STUB-012` MUST: a name lookup never requires parsing: results come from stubs; only the PSI of the *matching*
declarations may be materialised after the key lookup. Acceptance: indexing of a 10 000-file project then a name
query reads no file text for non-matching files.

`KT-STUB-013` SHOULD: indexes are versioned; a change of any contribution rule of this section invalidates and
rebuilds the affected index. Acceptance: bump rule version, restart: affected index rebuilds, others untouched.

## 1.4 Prime symbols

A *prime symbol* is a declaration that can be reached from other files by a short name without a receiver or an
instance: a **non-private, non-`override`** class, object, function, property or type alias that is
(a) top-level, or (b) a direct member of a top-level `object` declaration, or (c) a direct member of the `companion
object` of a top-level class. The IDE stores only the name; the check is purely syntactic.

`KT-STUB-020` SHOULD: PrimeSymbolName contains the name of every prime symbol and no other. Acceptance: `private fun
x()` top-level: absent; `object O { fun y() }` : `y` present; `class C { fun z() }`: `z` absent; `class C { companion
object { val w = 1 } }`: `w` present; `object O { override fun toString() = "" }`: `toString` absent.

## 1.5 The extension receiver key

For an extension function or property the index key is `receiverTypeName + "\n" + callableName`. The receiver type
name is derived syntactically from the receiver type reference:

1. A user type `a.b.Foo<T>`: the simple name of the last segment, `Foo`.
2. A nullable type `Foo?`: the key of `Foo` (nullability dropped).
3. A type that names one of the declaration's own **type parameters** `T`: the key of its upper bound if it has an
   `extends`-bound written, else `Any`. If the bound is itself a type parameter the rule is applied again (cycles are
   cut).
4. A function type `(A, B) -> R` or `A.(B) -> R`: `Function<n>` where *n* = number of parameters plus 1 if a receiver is
   present; `SuspendFunction<n>` if the type reference carries `suspend`.
5. `dynamic`: `Any`.
6. An intersection `T & Any`: the key of the left side.
7. In addition, if the simple name equals a **import alias** of the declaring file (`import a.Foo as Bar`, use `Bar`),
   the key of the original name is also emitted (so both spellings are found).

A declaration can contribute several keys (rule 7 and rule 3 with several bounds). Names are compared exactly.

`KT-STUB-021` MUST: the keys of rules 1 to 7. Acceptance: `fun String?.foo()` -> key `String\nfoo`;
`fun <T> T.bar()` -> `Any\nbar`; `fun <T : CharSequence> T.baz()` -> `CharSequence\nbaz`; `fun (Int) -> Unit.q()` ->
`Function1\nq`; `suspend`-receiver `suspend () -> Unit` -> `SuspendFunction0\n...`; `fun Foo.Bar.m()` -> `Bar\nm`.

`KT-STUB-022` MUST: a lookup for "extensions applicable to receiver type X named n" asks the index with a key for X
**and for each of X's supertypes** (transitively, plus `Any`), then verifies applicability with real type
information (2.9). Acceptance: `ArrayList<String>` receiver finds extensions declared on `List`, `Collection`,
`Iterable`, `Any` named `n`, not those declared on `Set`.

## 1.6 File facades and top-level declarations

Kotlin top-level functions and properties compile to static members of a *facade class* (JVM). The IDE models the
facade as a class visible to Java and Kotlin tooling, with the following name rule.

| File | Facade FQ name |
|---|---|
| `Foo.kt` in package `p` | `p.FooKt` |
| `my-file.kt` | `p.My_fileKt` (each character that is not a Java identifier part becomes `_`; the first letter is upper-cased) |
| `foo.kt` | `p.FooKt` |
| a file containing `@file:JvmName("Util")` | `p.Util` |
| files with the same `@file:JvmName("Util")` **and** `@file:JvmMultifileClass` in one package | one facade `p.Util`; each file additionally has a *part* class `p.Util__FooKt` hidden from users |
| a file with no top-level functions or properties | no facade (but still declares its classes) |

`KT-STUB-030` MUST: the facade FQ name rule above; the facade exists iff the file has at least one top-level function
or property (classes and type aliases do not create a facade). Acceptance: cases 01 facade table.

`KT-STUB-031` MUST: the facade's members are the file's top-level functions and properties as **static** members in
declaration order; properties appear as accessor methods `getX/setX/isX` plus a private static field when they have a
backing field, `const val` and `@JvmField` properties appear as public static fields. Details of accessor naming: 07.
Acceptance: `val x = 1` and `var y = 2` give `getX()`, `getY()`, `setY(int)`.

`KT-STUB-032` MUST: two files declaring the same top-level function in the same package with the same JVM signature
are a *redeclaration* error (03), independent of the facade. Two files with the same facade name (`a/Foo.kt` and
`b/Foo.kt` in the same package) are a *duplicate JVM class* error. Acceptance: 03 rows `KT-DIAG-0xx` (conflicting
overloads, duplicate class).

## 1.7 Class-file stubs (libraries)

Compiled Kotlin libraries contain a metadata annotation; the IDE builds *the same stub shapes as for source* from the
metadata (so a library class is found by every index above) and shows decompiled Kotlin text for the declaration when
navigating into a `.class` without sources. Plain Java class files produce Java stubs (`java/01-psi-and-stubs.md`).
Source attachment replaces the decompiled text.

`KT-STUB-040` SHOULD: library declarations are indexed with the same keys as source declarations; navigation lands in
attached sources if present else in the decompiled text; `private` and synthetic members of libraries are not
indexed. Acceptance: with `kotlin-stdlib` on the classpath, `listOf` is found by FunctionShortName and by
TopLevelFunctionFqName `kotlin.collections.listOf`; `String.isBlank` by key `CharSequence\nisBlank`.

## 1.8 Scripts

`.kts` files: `*.main.kts`, Gradle `build.gradle.kts` / `settings.gradle.kts`, and plain `.kts`. A script's top-level
declarations belong to a synthetic class whose name derives from the file name; top-level declarations of a script are
**not** importable from outside and **never reported as unused** unless `private`. Script analysis needs a *script
definition* (implicit receivers, default imports, dependencies); without a loaded definition the file is highlighted
with the inspection `KotlinScriptTemplateNotRegistered` (05). Script configuration loading is LATER for Archipel: see
the Gradle spec set for `*.gradle.kts`.

`KT-STUB-050` LATER: script classes and definitions. Acceptance: n/a (tracked by the Gradle/Kotlin-script work).

## 1.9 Dumb mode and incomplete project state

While indexes are being built:

- stub-index lookups are not available; Kotlin features that need them are disabled with the standard "not available
  during indexing" behaviour (`platform/04-dumb-mode.md`);
- **completion keeps working in a reduced form**: names found by a text/PSI scan of the open file and of files
  already parsed (registry `kotlin.auto.completion.dumb.mode.use.psi.completion`, default **on**);
- red errors whose cause could be a not-yet-loaded model are shown as a pale *pending reference* mark with the
  tooltip `Not resolved until the project is fully loaded` instead of an error, until the model is complete.

`KT-STUB-060` MUST: during indexing a reference that fails to resolve is *not* reported with error severity (shown
as pending, tooltip text above); the marks turn into normal errors/resolution after indexing, without user action.
Acceptance: open a project with an unresolved-name file while the index is being built: no red error, the pending mark
has the tooltip; after indexing the real error (or the resolved reference) appears.

`KT-STUB-061` SHOULD: completion in dumb mode offers local names, names visible in the file and keywords, never
shows an error popup. Acceptance: Ctrl+Space in a function body during indexing lists local variables and parameters.

## Open questions

- Q-STUB-1: exact set of modifier tokens kept in the modifier-list stub (all keywords or only those queried). Impact
  low; implement "all", add test.
- Q-STUB-2: ordering guarantees of index results across files (IntelliJ: unspecified). Archipel rule: sort by file path
  then offset for determinism.
