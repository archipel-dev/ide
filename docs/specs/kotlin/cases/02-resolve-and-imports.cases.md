# Cases for 02-resolve-and-imports

Conventions for this file: `[prefix]` after a message means only the quoted prefix is specified; the rest follows the
compiler. "Alt+Enter, <row>" means: open the intention popup with the caret as marked, pick the row whose text is given
(exact), Enter. File names are relative to a Kotlin source root `src/` of one module unless stated otherwise.

### KT-RES-001 case 1: explicit import beats same-package declaration
settings: default
language: kotlin
```before
--- src/a/Foo.kt
package a
class Foo
--- src/b/Foo.kt
package b
class Foo
--- src/a/Use.kt
package a
import b.Foo
val x: F<caret>oo? = null
```
keys: action GotoDeclaration (Ctrl+B)
```after
caret in src/b/Foo.kt at "class Foo"
```

### KT-RES-001 case 2: same-package declaration beats star import
settings: default
language: kotlin
```before
--- src/a/Foo.kt
package a
class Foo
--- src/b/Foo.kt
package b
class Foo
--- src/a/Use.kt
package a
import b.*
val x: F<caret>oo? = null
```
keys: action GotoDeclaration (Ctrl+B)
```after
caret in src/a/Foo.kt at "class Foo"
```

### KT-RES-001 case 3: star import beats default import
settings: default
language: kotlin
```before
--- src/x/List.kt
package x
class List
--- src/a/Use.kt
package a
import x.*
val l: L<caret>ist? = null
```
keys: action GotoDeclaration (Ctrl+B)
```after
caret in src/x/List.kt at "class List"
```

### KT-RES-001 case 4: two star imports with the same function name are ambiguous
settings: default
language: kotlin
```before
--- src/p/F.kt
package p
fun foo() {}
--- src/q/F.kt
package q
fun foo() {}
--- src/a/Use.kt
package a
import p.*
import q.*
fun t() { foo() }
```
keys: wait for highlighting
```after
error on "foo" in "foo()": "Overload resolution ambiguity between candidates:" [prefix]
```

### KT-RES-002 case 5: alias hides the original name
settings: default
language: kotlin
```before
--- src/a/B.kt
package a
class B
--- src/c/Use.kt
package c
import a.B as C
val x: C? = null
val y: B? = null
```
keys: wait for highlighting
```after
no error on "C" in "val x: C?"
error on "B" in "val y: B?": "Unresolved reference 'B'."
```

### KT-RES-003 case 6: star import from an object is an error
settings: default
language: kotlin
```before
--- src/a/O.kt
package a
object Obj { fun f() {} }
--- src/c/Use.kt
package c
import a.Obj.*
```
keys: wait for highlighting
```after
error on "Obj" (the import path): "Cannot import on demand from object 'Obj'." [prefix: "Cannot import on demand from object"]
```

### KT-RES-003 case 7: importing one member of an object is allowed
settings: default
language: kotlin
```before
--- src/a/O.kt
package a
object Obj { fun f() {} }
--- src/c/Use.kt
package c
import a.Obj.f
fun t() { f() }
```
keys: wait for highlighting
```after
no errors
```

### KT-RES-010 case 8: JVM default imports
settings: platform=jvm
language: kotlin
```before
fun t() {
    val t = Thread()
    val sb = StringBuilder()
    val l: ArrayList<String> = ArrayList()
}
```
keys: wait for highlighting
```after
no errors
```

### KT-RES-010 case 9: common source set has no java.lang default import
settings: platform=common
language: kotlin
```before
fun t() { val t = Thread() }
```
keys: wait for highlighting
```after
error on "Thread": "Unresolved reference 'Thread'."
```

### KT-IMP-001 case 10: Import class, single candidate, file without imports
settings: default
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
--- src/app/Use.kt
package app

val x: F<caret>oo? = null
```
keys: Alt+Enter, "Import class 'Foo'", Enter
```after
--- src/app/Use.kt
package app

import lib.Foo

val x: Foo? = null
```

### KT-IMP-001 case 11: Alt+Enter list order, error with import fix
settings: default
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
--- src/app/Use.kt
package app
val x: F<caret>oo? = null
```
keys: Alt+Enter
```after
first popup row: Import class 'Foo'
the popup also contains: Create class 'Foo', Rename reference
```

### KT-IMP-006 case 12: two candidates, same kind, different packages (class)
settings: default
language: kotlin
```before
--- src/util/Helper.kt
package util
class Helper
--- src/internal/Helper.kt
package internal
class Helper
--- src/app/Use.kt
package app
val h = H<caret>elper()
```
keys: Alt+Enter, "Import class 'Helper'", Enter, then choose "class internal.Helper"
```after
popup title "Imports"; rows (identified by FQ name) in order:
internal.Helper
util.Helper
--- src/app/Use.kt
package app

import internal.Helper

val h = Helper()
```

### KT-IMP-008 case 13: chooser Right arrow shows exclude submenu
settings: default
language: kotlin
```before
--- src/util/Helper.kt
package util
class Helper
--- src/internal/Helper.kt
package internal
class Helper
--- src/app/Use.kt
package app
val h = H<caret>elper()
```
keys: Alt+Enter, Enter (chooser opens), Down (select "class util.Helper"), Right
```after
submenu rows (the package `util` has one segment, so no package entry is offered):
Exclude 'util.Helper' from auto-import
```

### KT-IMP-006 case 14: Import extension function
settings: default
language: kotlin
```before
--- src/ext/Ext.kt
package ext
fun List<Int>.total(): Int = sum()
--- src/app/Use.kt
package app
fun t() = listOf(1, 2).to<caret>tal()
```
keys: Alt+Enter, "Import extension function 'List.total'", Enter
```after
--- src/app/Use.kt
package app

import ext.total

fun t() = listOf(1, 2).total()
```

### KT-IMP-002 case 15: extension for a different receiver is not offered
settings: default
language: kotlin
```before
--- src/ext/Ext.kt
package ext
fun Set<Int>.total(): Int = sum()
--- src/app/Use.kt
package app
fun t() = listOf(1, 2).to<caret>tal()
```
keys: Alt+Enter
```after
popup does not contain any row starting with "Import"
```

### KT-IMP-006 case 16: Import infix extension function; non-infix extension is not a candidate
settings: default
language: kotlin
```before
--- src/ext/Ext.kt
package ext
infix fun Int.pow(e: Int): Int = 1
fun Int.pow2(): Int = 1
--- src/app/Use.kt
package app
val r = 2 p<caret>ow 3
```
keys: Alt+Enter, "Import infix extension function 'Int.pow'", Enter
```after
--- src/app/Use.kt
package app

import ext.pow

val r = 2 pow 3
```

### KT-IMP-006 case 17: Import operator
settings: default
language: kotlin
```before
--- src/money/Money.kt
package money
class Money(val v: Int)
operator fun Money.plus(o: Money) = Money(v + o.v)
--- src/app/Use.kt
package app
import money.Money
fun t(a: Money, b: Money) = a <caret>+ b
```
keys: Alt+Enter, "Import operator 'Money.plus'", Enter
```after
--- src/app/Use.kt
package app
import money.Money
import money.plus
fun t(a: Money, b: Money) = a + b
```

### KT-IMP-006 case 18: Import extension property
settings: default
language: kotlin
```before
--- src/ext/Ext.kt
package ext
val String.firstWord: String get() = substringBefore(' ')
--- src/app/Use.kt
package app
fun t(s: String) = s.first<caret>Word
```
keys: Alt+Enter, "Import extension property 'String.firstWord'", Enter
```after
--- src/app/Use.kt
package app

import ext.firstWord

fun t(s: String) = s.firstWord
```

### KT-IMP-006 case 19: Import top-level function
settings: default
language: kotlin
```before
--- src/util/U.kt
package util
fun greet() {}
--- src/app/Use.kt
package app
fun t() { gr<caret>eet() }
```
keys: Alt+Enter, "Import function 'greet'", Enter
```after
--- src/app/Use.kt
package app

import util.greet

fun t() { greet() }
```

### KT-IMP-006 case 20: several candidates of different kinds give the plain text
settings: default
language: kotlin
```before
--- src/a/A.kt
package a
class Thing
--- src/b/B.kt
package b
fun Thing() {}
--- src/app/Use.kt
package app
fun t() { Th<caret>ing() }
```
keys: Alt+Enter
```after
first popup row is exactly: Import
```

### KT-IMP-006 case 21: two functions of one package, "and more" wording
settings: default
language: kotlin
```before
--- src/util/U.kt
package util
fun run1() {}
fun run1(x: Int) {}
fun run1(x: String) {}
--- src/app/Use.kt
package app
fun t() { ru<caret>n1() }
```
keys: Alt+Enter
```after
first popup row is exactly: Import function 'run1'
(all three overloads share one FQ name: one candidate, no chooser)
```

### KT-IMP-002 case 22: annotation position accepts annotation classes only
settings: default
language: kotlin
```before
--- src/a/Tag.kt
package a
annotation class Tag
--- src/b/Tag.kt
package b
class Tag
--- src/app/Use.kt
package app
@Ta<caret>g class X
```
keys: Alt+Enter, "Import class 'Tag'", Enter
```after
--- src/app/Use.kt
package app

import a.Tag

@Tag class X
```

### KT-IMP-002 case 23: type position does not offer functions
settings: default
language: kotlin
```before
--- src/util/U.kt
package util
fun helper() {}
--- src/app/Use.kt
package app
val x: hel<caret>per? = null
```
keys: Alt+Enter
```after
popup does not contain any row starting with "Import"
```

### KT-IMP-002 case 24: private and already-imported declarations are not candidates
settings: default
language: kotlin
```before
--- src/util/U.kt
package util
private fun hidden() {}
class Foo
--- src/app/Use.kt
package app
import util.Foo
fun t() { hid<caret>den(); val f: Foo? = null }
```
keys: Alt+Enter
```after
popup does not contain any row starting with "Import"
```

### KT-IMP-003 case 25: short lowercase names get no import fix
settings: default
language: kotlin
```before
--- src/util/ab.kt
package util
class ab
--- src/app/Use.kt
package app
val x: a<caret>b? = null
```
keys: Alt+Enter
```after
popup does not contain any row starting with "Import"
```

### KT-IMP-004 case 26: excluded package is removed from candidates
settings: auto_import_exclude=util
language: kotlin
```before
--- src/util/Helper.kt
package util
class Helper
--- src/internal/Helper.kt
package internal
class Helper
--- src/app/Use.kt
package app
val h = H<caret>elper()
```
keys: Alt+Enter, "Import class 'Helper'", Enter
```after
--- src/app/Use.kt
package app

import internal.Helper

val h = Helper()
```
(single candidate, no chooser)

### KT-IMP-010 case 27: deprecated candidates are last
settings: default
language: kotlin
```before
--- src/a/Thing.kt
package a
@Deprecated("old") class Thing
--- src/b/Thing.kt
package b
class Thing
--- src/app/Use.kt
package app
val t: Th<caret>ing? = null
```
keys: Alt+Enter, Enter (chooser)
```after
chooser rows in order:
b.Thing
a.Thing
```

### KT-IMP-010 case 28: candidates that fit the call arguments come first
settings: default
language: kotlin
```before
--- src/a/F.kt
package a
fun compute(s: String) {}
--- src/z/F.kt
package z
fun compute(n: Int, s: String) {}
--- src/app/Use.kt
package app
fun t() { comp<caret>ute(1, "x") }
```
keys: Alt+Enter, Enter (chooser)
```after
chooser rows in order (the hint reads "Import z.compute?"):
z.compute
a.compute
```

### KT-IMP-010 case 29: project source ranks above libraries (base weights)
settings: default; kotlinx-coroutines and a project class named Job on the classpath
language: kotlin
```before
--- src/mine/Job.kt
package mine
class Job
--- src/app/Use.kt
package app
val j: Jo<caret>b? = null
```
keys: Alt+Enter, Enter (chooser)
```after
chooser rows in order:
mine.Job
kotlinx.coroutines.Job
```

### KT-IMP-007 case 30: import hint text
settings: default
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
--- src/app/Use.kt
package app
val x: Foo? = null<caret>
```
keys: wait for the daemon, then caret to a neighbouring line without typing
```after
hint above "Foo": "Import lib.Foo?" with the Alt+Enter shortcut
```

### KT-IMP-007 case 31: silent import with one candidate
settings: add_unambiguous_imports_on_the_fly=true
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
--- src/app/Use.kt
package app

val x: Foo<caret>
```
keys: type " = null", Enter (file was edited, caret leaves the reference)
```after
--- src/app/Use.kt
package app

import lib.Foo

val x: Foo = null

```
(undo once removes the import)

### KT-IMP-007 case 32: no silent import for several candidates
settings: add_unambiguous_imports_on_the_fly=true
language: kotlin
```before
--- src/a/Foo.kt
package a
class Foo
--- src/b/Foo.kt
package b
class Foo
--- src/app/Use.kt
package app

val x: Foo<caret>
```
keys: type "?", Enter
```after
no import is added; the hint "Import a.Foo?" is shown
```

### KT-IMP-007 case 33: nested classes are never imported on the fly
settings: add_unambiguous_imports_on_the_fly=true
language: kotlin
```before
--- src/lib/Outer.kt
package lib
class Outer { class Nested }
--- src/app/Use.kt
package app

val x: Nested<caret>
```
keys: type "?", Enter
```after
no import is added; the hint "Import lib.Outer.Nested?" is shown
```

### KT-IMP-011 case 34: nested class import qualifies by the outer class [verify]
settings: default
language: kotlin
```before
--- src/lib/Outer.kt
package lib
class Outer { class Nested }
--- src/app/Use.kt
package app

val x: Nes<caret>ted? = null
```
keys: Alt+Enter, "Import class 'Nested'", Enter
```after
--- src/app/Use.kt
package app

import lib.Outer

val x: Outer.Nested? = null
```

### KT-IMP-011 case 35: nested class import with "Insert imports for nested classes" on [verify]
settings: kotlin.imports.insert_nested_classes=true
language: kotlin
```before
--- src/lib/Outer.kt
package lib
class Outer { class Nested }
--- src/app/Use.kt
package app

val x: Nes<caret>ted? = null
```
keys: Alt+Enter, "Import class 'Nested'", Enter
```after
--- src/app/Use.kt
package app

import lib.Outer.Nested

val x: Nested? = null
```

### KT-IMP-011 case 36: member of an object
settings: default
language: kotlin
```before
--- src/lib/O.kt
package lib
object Util { fun twice(x: Int) = x * 2 }
--- src/app/Use.kt
package app
fun t() = twi<caret>ce(2)
```
keys: Alt+Enter, "Import function 'Util.twice'", Enter
```after
--- src/app/Use.kt
package app

import lib.Util.twice

fun t() = twice(2)
```

### KT-IMP-030 case 37: new import goes to the right place in the layout
settings: default
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
--- src/app/Use.kt
package app

import aaa.A
import java.util.Date
import zzz.Z

val x: F<caret>oo? = null
val y: A? = null
val d: Date? = null
val z: Z? = null
```
keys: Alt+Enter, "Import class 'Foo'", Enter
```after
--- src/app/Use.kt
package app

import aaa.A
import lib.Foo
import java.util.Date
import zzz.Z

val x: Foo? = null
val y: A? = null
val d: Date? = null
val z: Z? = null
```
(`lib.Foo` is inserted after the last import that is not greater than it under the layout comparator, which is `aaa.A`;
the list was not sorted to begin with, so it is not re-sorted)

### KT-IMP-030 case 38: adding an import that already exists changes nothing
settings: default
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
--- src/app/Use.kt
package app

import lib.Foo

val x: Foo? = null
```
keys: action ImportClassFor lib.Foo (programmatic add import)
```after
file unchanged
```

### KT-IMP-020 case 39: unused import: gray text and fixes
settings: default
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
--- src/app/Use.kt
package app

import lib.<caret>Foo

val x = 1
```
keys: wait for highlighting; Alt+Enter
```after
the whole directive "import lib.Foo" is gray (unused symbol style), tooltip "Unused import directive"
popup rows, first three:
Remove unused imports
Optimize imports
Enable optimize imports on the fly
```

### KT-IMP-020 case 40: Remove unused imports does not reorder
settings: default
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
class Bar
class Baz
--- src/app/Use.kt
package app

import lib.Baz
import lib.Foo
import lib.Bar

val x: Bar? = null
val y: Baz? = null
```
keys: Alt+Enter on "import lib.Foo", "Remove unused imports", Enter
```after
--- src/app/Use.kt
package app

import lib.Baz
import lib.Bar

val x: Bar? = null
val y: Baz? = null
```

### KT-IMP-021 case 41: Optimize Imports removes unused and sorts
settings: default
language: kotlin
```before
--- src/lib/L.kt
package lib
class Foo
class Bar
class Unused
--- src/app/Use.kt
package app

import lib.Foo
import lib.Unused
import lib.Bar

val x: Bar? = null
val y: Foo? = null
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
--- src/app/Use.kt
package app

import lib.Bar
import lib.Foo

val x: Bar? = null
val y: Foo? = null
```
(hint "Removed 1 import")

### KT-IMP-021 case 42: java.util is a star-import package by default
settings: default
language: kotlin
```before
package app

import java.util.Date

val d: Date? = null
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
package app

import java.util.*

val d: Date? = null
```

### KT-IMP-021 case 43: five names make a star import
settings: default
language: kotlin
```before
--- src/lib/L.kt
package lib
class A
class B
class C
class D
class E
--- src/app/Use.kt
package app

import lib.A
import lib.B
import lib.C
import lib.D
import lib.E

val a: A? = null
val b: B? = null
val c: C? = null
val d: D? = null
val e: E? = null
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
--- src/app/Use.kt
package app

import lib.*

val a: A? = null
val b: B? = null
val c: C? = null
val d: D? = null
val e: E? = null
```

### KT-IMP-021 case 44: four names stay explicit
settings: default
language: kotlin
```before
--- src/lib/L.kt
package lib
class A
class B
class C
class D
--- src/app/Use.kt
package app

import lib.*

val a: A? = null
val b: B? = null
val c: C? = null
val d: D? = null
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
--- src/app/Use.kt
package app

import lib.A
import lib.B
import lib.C
import lib.D

val a: A? = null
val b: B? = null
val c: C? = null
val d: D? = null
```

### KT-IMP-021 case 45: star import of object members is never produced
settings: default
language: kotlin
```before
--- src/lib/O.kt
package lib
object O { fun a() {}; fun b() {}; fun c() {}; fun d() {}; fun e() {} }
--- src/app/Use.kt
package app

import lib.O.a
import lib.O.b
import lib.O.c
import lib.O.d
import lib.O.e

fun t() { a(); b(); c(); d(); e() }
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
file unchanged (five explicit imports remain; star import from an object is not allowed)
```

### KT-IMP-021 case 46: three Java static members make a member star import
settings: default
language: kotlin
```before
package app

import java.lang.Math.abs
import java.lang.Math.max
import java.lang.Math.min

fun t() = abs(1) + max(1, 2) + min(1, 2)
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
package app

import java.lang.Math.*

fun t() = abs(1) + max(1, 2) + min(1, 2)
```

### KT-IMP-021 case 47: imports implied by default imports are dropped
settings: default
language: kotlin
```before
package app

import kotlin.collections.List
import java.lang.StringBuilder

val l: List<Int> = emptyList()
val s = StringBuilder()
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
package app

val l: List<Int> = emptyList()
val s = StringBuilder()
```

### KT-IMP-021 case 48: alias imports are kept
settings: default
language: kotlin
```before
--- src/lib/L.kt
package lib
class Movie
--- src/app/Use.kt
package app

import lib.Movie as Film

val f: Film? = null
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
file unchanged
```

### KT-IMP-021 case 49: unresolved names keep their imports
settings: default
language: kotlin
```before
package app

import not.there.Foo

val x: Foo? = null
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
file unchanged
```

### KT-IMP-021 case 50: duplicate import removed
settings: default
language: kotlin
```before
--- src/lib/L.kt
package lib
class Foo
--- src/app/Use.kt
package app

import lib.Foo
import lib.Foo

val x: Foo? = null
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
--- src/app/Use.kt
package app

import lib.Foo

val x: Foo? = null
```

### KT-IMP-021 case 51: star import that would change meaning is not created
settings: default
language: kotlin
```before
--- src/lib/L.kt
package lib
class A
class B
class C
class D
class E
--- src/other/A.kt
package other
class A
--- src/app/Use.kt
package app

import other.A
import lib.B
import lib.C
import lib.D
import lib.E

val a: A? = null
val b: B? = null
val c: C? = null
val d: D? = null
val e: E? = null
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
--- src/app/Use.kt
package app

import lib.B
import lib.C
import lib.D
import lib.E
import other.A

val a: A? = null
val b: B? = null
val c: C? = null
val d: D? = null
val e: E? = null
```
(four names from `lib` are used, below the threshold of 5, so they stay explicit)

### KT-IMP-021 case 52: nothing to do
settings: default
language: kotlin
```before
package app

import lib.Foo
```
keys: action OptimizeImports (Ctrl+Alt+O) on a file where "lib.Foo" is used and sorted
```after
hint: "Unused imports not found"
```

### KT-IMP-022 case 53: layout puts java, javax, kotlin after the others, aliases last
settings: default
language: kotlin
```before
package app

import kotlin.math.max
import javax.swing.JButton as Btn
import java.io.File
import com.acme.Zed
import javax.swing.JLabel
import com.acme.Alpha

val a = Alpha()
val z = Zed()
val f: File? = null
val l: JLabel? = null
val b: Btn? = null
val m = max(1, 2)
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
package app

import com.acme.Alpha
import com.acme.Zed
import java.io.File
import javax.swing.JLabel
import kotlin.math.max
import javax.swing.JButton as Btn

val a = Alpha()
val z = Zed()
val f: File? = null
val l: JLabel? = null
val b: Btn? = null
val m = max(1, 2)
```

### KT-IMP-022 case 54: backticks are ignored when sorting
settings: default
language: kotlin
```before
--- src/a/A.kt
package a
fun `b`() {}
fun a() {}
fun c() {}
--- src/app/Use.kt
package app

import a.c
import a.`b`
import a.a

fun t() { a(); `b`(); c() }
```
keys: action OptimizeImports (Ctrl+Alt+O)
```after
--- src/app/Use.kt
package app

import a.a
import a.b
import a.c

fun t() { a(); b(); c() }
```
(redundant backticks are dropped by the writer for non-keyword names; when the name needs them they are kept)

### KT-IMP-023 case 55: optimize on the fly
settings: optimize_imports_on_the_fly=true (project)
language: kotlin
```before
--- src/lib/L.kt
package lib
class Foo
class Unused
--- src/app/Use.kt
package app

import lib.Foo
import lib.Unused

val x: Foo? = null
val y = 1<caret>
```
keys: type "2" (file was edited; caret is outside the import block); wait for the daemon
```after
--- src/app/Use.kt
package app

import lib.Foo

val x: Foo? = null
val y = 12
```

### KT-IMP-023 case 56: no on-the-fly optimize while the caret is inside the imports
settings: optimize_imports_on_the_fly=true (project)
language: kotlin
```before
--- src/lib/L.kt
package lib
class Foo
class Unused
--- src/app/Use.kt
package app

import lib.Foo
import lib.Unu<caret>sed

val x: Foo? = null
```
keys: type "x"; wait for the daemon
```after
imports unchanged (caret in the import block)
```

### KT-IMP-040 case 57: imports are added on paste
settings: insert_imports_on_paste=Always
language: kotlin
```before
--- src/lib/L.kt
package lib
class Foo
--- src/app/Source.kt
package app
import lib.Foo
fun make() = <selection>Foo()</selection>
--- src/other/Target.kt
package other
fun t() = <caret>
```
keys: copy in Source.kt (Ctrl+C), paste in Target.kt (Ctrl+V)
```after
--- src/other/Target.kt
package other

import lib.Foo

fun t() = Foo()
```
notification: "1 import was added" with link "Review Added Imports..."

### KT-IMP-008 case 58: exclude submenu with a multi-segment package
settings: default
language: kotlin
```before
--- src/com/acme/util/Helper.kt
package com.acme.util
class Helper
--- src/internal/Helper.kt
package internal
class Helper
--- src/app/Use.kt
package app
val h = H<caret>elper()
```
keys: Alt+Enter, Enter (chooser opens), select "com.acme.util.Helper" (type "acme" for speed search), Right
```after
submenu rows in order:
Exclude 'com.acme.util.Helper' from auto-import
Exclude 'com.acme.util' from auto-import
Exclude 'com.acme' from auto-import
```
