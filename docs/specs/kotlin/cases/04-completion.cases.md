# Cases for 04-completion

Conventions: `keys: Ctrl+Space` opens basic completion at the caret. "list (in order)" gives the first rows of the popup (item text, then tail
text after ` | `, then type text after ` : `). "contains" means the items appear somewhere in the list without implying order. After a
selection key the resulting text is given. Default settings: matching "First letter only", middle matching on (`codeinsight/06`).

### KT-CMP-001 case 1: type position offers classifiers only
settings: default
language: kotlin
```before
class Foo
fun helper() {}
val fooValue = 1
fun t() {
    val x: Fo<caret>
}
```
keys: Ctrl+Space
```after
list contains: Foo
list does not contain: fooValue, helper
```

### KT-CMP-001 case 2: import directive position offers package segments and members
settings: default
language: kotlin
```before
--- src/lib/A.kt
package lib
class Apple
fun apply1() {}
--- src/app/Use.kt
package app
import lib.<caret>
```
keys: Ctrl+Space
```after
list contains: Apple, apply1
```

### KT-CMP-001 case 3: annotation position offers annotation classes only
settings: default
language: kotlin
```before
annotation class Marker
class Marble
@Mar<caret>
fun f() {}
```
keys: Ctrl+Space
```after
list contains: Marker
list does not contain: Marble
```

### KT-CMP-001 case 4: no completion in a comment
settings: default
language: kotlin
```before
class Foo
// Fo<caret>
```
keys: Ctrl+Space
```after
no Kotlin items (only word completion items from the file, if any)
```

### KT-CMP-010 case 5: locals, parameters, members, top-level
settings: default
language: kotlin
```before
val valueTop = 0
class C {
    val valueMember = 1
    fun t(valueParam: Int) {
        val valueLocal = 2
        val<caret>
    }
}
```
keys: Ctrl+Space
```after
list (in order): valueLocal : Int, valueParam : Int, valueMember : Int, valueTop | (<root>) : Int
```

### KT-CMP-010 case 6: members of implicit receivers, own and inherited
settings: default
language: kotlin
```before
open class Base { fun inherited() {}; protected fun prot() {}; private fun priv() {} }
class C : Base() {
    fun own() {}
    fun t() { <caret> }
}
```
keys: type "i", Ctrl+Space; then "p"
```after
after "i": contains inherited()
after "p": contains prot() ; does not contain priv()
```

### KT-CMP-010 case 7: explicit receiver: members and applicable extensions
settings: default
language: kotlin
```before
fun String.shout() = uppercase()
fun Int.shout() = this
fun t(s: String) {
    s.sho<caret>
}
```
keys: Ctrl+Space
```after
list (in order): shout() | for String in <package> : String
list does not contain the Int extension
```

### KT-CMP-010 case 8: safe call on a nullable receiver
settings: default
language: kotlin
```before
fun t(s: String?) {
    s?.leng<caret>
}
```
keys: Ctrl+Space, Enter
```after
fun t(s: String?) {
    s?.length
}
```

### KT-CMP-010 case 9: private members of other classes are not offered
settings: default
language: kotlin
```before
class A { private fun hidden() {}; fun shown() {} }
fun t(a: A) { a.<caret> }
```
keys: Ctrl+Space
```after
list contains: shown()
list does not contain: hidden
```

### KT-CMP-010 case 10: a smart cast widens the available members
settings: default
language: kotlin
```before
fun t(a: Any) {
    if (a is String) {
        a.leng<caret>
    }
}
```
keys: Ctrl+Space, Enter
```after
fun t(a: Any) {
    if (a is String) {
        a.length
    }
}
```

### KT-CMP-010 case 11: enum entries on the enum class, and by expected type
settings: default
language: kotlin
```before
enum class Color { RED, GREEN }
fun t() {
    val c: Color = Color.<caret>
}
```
keys: Ctrl+Space
```after
list (in order): GREEN, RED   (alphabetical for equal relevance; both are Color) [order verify]
```

### KT-CMP-011 case 12: not-imported class is offered and imported on insertion
settings: default
language: kotlin
```before
--- src/lib/Foo.kt
package lib
class Foo
--- src/app/Use.kt
package app

val x = Fo<caret>
```
keys: Ctrl+Space, select the item "Foo | (lib)" , Enter
```after
--- src/app/Use.kt
package app

import lib.Foo

val x = Foo
```

### KT-CMP-011 case 13: not-imported extension is offered and imported on insertion
settings: default
language: kotlin
```before
--- src/ext/E.kt
package ext
fun String.shout() = uppercase()
--- src/app/Use.kt
package app

fun t(s: String) = s.sho<caret>
```
keys: Ctrl+Space, Enter
```after
--- src/app/Use.kt
package app

import ext.shout

fun t(s: String) = s.shout()
```

### KT-CMP-012 case 14: Java synthetic property item
settings: default
language: kotlin
```before
fun t(d: java.util.Date) {
    d.ti<caret>
}
```
keys: Ctrl+Space
```after
list contains: time | (from getTime()/setTime()) : Long
typing "getTi" instead of "ti" also lists: time
```

### KT-CMP-020 case 15: function row
settings: default
language: kotlin
```before
class C {
    fun compute(a: Int, vararg rest: String, flag: Boolean = false): Long = 0
    fun t() { comp<caret> }
}
```
keys: Ctrl+Space
```after
row: compute | (a: Int, vararg rest: String, flag: Boolean = ...) : Long
```

### KT-CMP-020 case 16: top-level function row shows its package
settings: default
language: kotlin
```before
--- src/util/U.kt
package util
fun greet(name: String) {}
--- src/app/Use.kt
package app
fun t() { gre<caret> }
```
keys: Ctrl+Space
```after
row: greet | (name: String) (util) : Unit
```

### KT-CMP-020 case 17: extension row
settings: default
language: kotlin
```before
--- src/ext/E.kt
package ext
fun String.shout(): String = uppercase()
--- src/app/Use.kt
package app
fun t(s: String) { s.sho<caret> }
```
keys: Ctrl+Space
```after
row: shout | () for String in ext : String
```

### KT-CMP-020 case 18: class and constructor rows
settings: default
language: kotlin
```before
--- src/lib/Box.kt
package lib
class Box<T>(val item: T)
--- src/app/Use.kt
package app
fun t() { val b = Bo<caret> }
```
keys: Ctrl+Space
```after
rows: Box | <T> (lib)      and      Box | (item: T) (lib)
```

### KT-CMP-020 case 19: type alias row shows the expansion
settings: default
language: kotlin
```before
typealias Names = List<String>
fun t(n: Nam<caret>) {}
```
keys: Ctrl+Space
```after
row: Names | (<root>) : List<String>
```

### KT-CMP-030 case 20: function without parameters: caret after the parentheses
settings: default
language: kotlin
```before
fun ping() {}
fun t() { pin<caret> }
```
keys: Ctrl+Space, Enter
```after
fun ping() {}
fun t() { ping()<caret> }
```

### KT-CMP-030 case 21: function with parameters: caret inside, parameter info shown
settings: default
language: kotlin
```before
fun add(a: Int, b: Int) {}
fun t() { ad<caret> }
```
keys: Ctrl+Space, Enter
```after
fun add(a: Int, b: Int) {}
fun t() { add(<caret>) }
(parameter info popup is shown)
```

### KT-CMP-030 case 22: existing parentheses are reused
settings: default
language: kotlin
```before
fun add(a: Int, b: Int) {}
fun t() { ad<caret>(1, 2) }
```
keys: Ctrl+Space, Enter
```after
fun add(a: Int, b: Int) {}
fun t() { add<caret>(1, 2) }
```

### KT-CMP-030 case 23: commit with a dot never puts the caret inside parentheses
settings: default
language: kotlin
```before
fun make(a: Int): String = ""
fun t() { mak<caret> }
```
keys: Ctrl+Space, type "."
```after
fun make(a: Int): String = ""
fun t() { make().<caret> }
```

### KT-CMP-030 case 24: type arguments that cannot be inferred
settings: default
language: kotlin
```before
fun <T> create(): List<T> = emptyList()
fun t() { crea<caret> }
```
keys: Ctrl+Space, space (commit with space)
```after
fun <T> create(): List<T> = emptyList()
fun t() { create<<caret>>() }
```

### KT-CMP-030 case 25: variable of function type is called
settings: default
language: kotlin
```before
fun t(callback: (Int) -> Unit) {
    callb<caret>
}
```
keys: Ctrl+Space, Enter
```after
fun t(callback: (Int) -> Unit) {
    callback(<caret>)
}
```

### KT-CMP-030 case 26: callable reference gets no parentheses
settings: default
language: kotlin
```before
fun isEven(n: Int) = n % 2 == 0
fun t() = listOf(1, 2).filter(::isE<caret>)
```
keys: Ctrl+Space, Enter
```after
fun isEven(n: Int) = n % 2 == 0
fun t() = listOf(1, 2).filter(::isEven)
```

### KT-CMP-030 case 27: infix function inserts name and space
settings: default
language: kotlin
```before
infix fun Int.pow(e: Int): Int = 1
fun t() = 2 po<caret>
```
keys: Ctrl+Space, Enter
```after
infix fun Int.pow(e: Int): Int = 1
fun t() = 2 pow <caret>
```

### KT-CMP-031 case 28: trailing-lambda variant is a separate row
settings: default
language: kotlin
```before
fun <T> run2(block: () -> T): T = block()
fun t() { run<caret> }
```
keys: Ctrl+Space
```after
rows include: run2 | (block: () -> T) (<root>) : T      and      run2 | { block: () -> T } (<root>) : T
```

### KT-CMP-031 case 29: choose the trailing-lambda variant
settings: default
language: kotlin
```before
fun <T> run2(block: () -> T): T = block()
fun t() { run<caret> }
```
keys: Ctrl+Space, select the row with " { block: () -> T }", Enter
```after
fun <T> run2(block: () -> T): T = block()
fun t() { run2 { <caret> } }
```

### KT-CMP-040 case 30: class item of a generic class adds no type arguments
settings: default
language: kotlin
```before
class Box<T>
fun t(b: Bo<caret>) {}
```
keys: Ctrl+Space, Enter
```after
class Box<T>
fun t(b: Box<caret>) {}
```
(the class item adds no type arguments; the user writes them)

### KT-CMP-040 case 31: object and enum entry insert only the name
settings: default
language: kotlin
```before
object Registry
fun t() { Reg<caret> }
```
keys: Ctrl+Space, Enter
```after
object Registry
fun t() { Registry<caret> }
```

### KT-CMP-040 case 32: simple template entry in a string
settings: default
language: kotlin
```before
fun t(name: String) = "Hello $na<caret>"
```
keys: Ctrl+Space, Enter
```after
fun t(name: String) = "Hello $name<caret>"
```

### KT-CMP-040 case 33: braces added when an identifier character follows
settings: default
language: kotlin
```before
fun t(name: String) = "Hello $na<caret>ed"
```
keys: Ctrl+Space, Tab
```after
fun t(name: String) = "Hello ${name}<caret>"
```
(Tab replaces the rest of the identifier; with Enter the rest `ed` would be kept and braces added: `"Hello ${name}ed"`)

### KT-CMP-040 case 34: dot inside a string extends the prefix
settings: default
language: kotlin
```before
class P(val name: String)
fun t(p: P) = "Hello ${p.na<caret>}"
```
keys: type "m", then type "."
```after
the lookup stays open, prefix is "m." (no commit) [verify the interplay with a template entry]
```

### KT-CMP-041 case 35: named-argument rows
settings: default
language: kotlin
```before
fun send(to: String, subject: String, urgent: Boolean = false, cc: String? = null) {}
fun t() { send("a", <caret>) }
```
keys: Ctrl+Space
```after
rows include: subject = | String
rows do not include: to =
(rows for urgent = , cc = may be listed after the next-parameter row)
```

### KT-CMP-041 case 36: boolean and null value rows for the next parameter
settings: default
language: kotlin
```before
fun send(to: String, urgent: Boolean) {}
fun t() { send("a", ur<caret>) }
```
keys: Ctrl+Space
```after
rows include: urgent = , urgent = true , urgent = false
```

### KT-CMP-041 case 37: insert a named argument
settings: default
language: kotlin
```before
fun send(to: String, subject: String) {}
fun t() { send(to = "a", sub<caret>) }
```
keys: Ctrl+Space, Enter
```after
fun send(to: String, subject: String) {}
fun t() { send(to = "a", subject = <caret>) }
```

### KT-CMP-041 case 38: no named-argument rows for Java methods
settings: default
language: kotlin
```before
fun t(sb: StringBuilder) { sb.append(<caret>) }
```
keys: Ctrl+Space
```after
no row of the form "<name> =" (Java parameter names are not stable)
```

### KT-CMP-042 case 39: multiple arguments from matching local names
settings: default
language: kotlin
```before
fun draw(width: Int, height: Int, color: String) {}
fun t() {
    val width = 1
    val height = 2
    draw(wi<caret>)
}
```
keys: Ctrl+Space
```after
rows include: width  and  width, height
```

### KT-CMP-042 case 40: bracket operator
settings: default
language: kotlin
```before
fun t(m: Map<String, Int>) = m.ge<caret>
```
keys: Ctrl+Space, select "get" bracket form (the row that ends with "[]"), Enter
```after
fun t(m: Map<String, Int>) = m[<caret>]
```

### KT-CMP-042 case 41: anonymous object row for an interface
settings: default
language: kotlin
```before
fun t() {
    val r: Runnable = obj<caret>
}
```
keys: Ctrl+Space, select "object : Runnable{...}", Enter
```after
fun t() {
    val r: Runnable = object : Runnable {
        override fun run() {
            TODO("Not yet implemented")
        }
    }
}
```

### KT-CMP-050 case 42: member of an object is inserted qualified [verify]
settings: default
language: kotlin
```before
--- src/lib/O.kt
package lib
object Util { fun twice(x: Int) = x * 2 }
--- src/app/Use.kt
package app

fun t() = twi<caret>
```
keys: Ctrl+Space, Enter
```after
--- src/app/Use.kt
package app

import lib.Util

fun t() = Util.twice(<caret>)
```

### KT-CMP-060 case 43: compound keyword at top level
settings: default
language: kotlin
```before
da<caret>
```
keys: Ctrl+Space
```after
rows include: data class
```

### KT-CMP-060 case 44: companion object only once per class
settings: default
language: kotlin
```before
class A {
    companion object {}
    comp<caret>
}
class B {
    comp<caret2>
}
```
keys: Ctrl+Space at each caret
```after
at caret 1: no "companion object" row
at caret 2: row "companion object"
```

### KT-CMP-060 case 45: no keywords after a dot
settings: default
language: kotlin
```before
fun t(s: String) { s.<caret> }
```
keys: Ctrl+Space
```after
no keyword row (such as "if", "return", "is") in the list
```

### KT-CMP-060 case 46: inside annotation arguments only a few keywords
settings: default
language: kotlin
```before
@Suppress(<caret>)
fun f() {}
```
keys: Ctrl+Space
```after
keyword rows, if any, are limited to: if, else, true, false, when
```

### KT-CMP-060 case 47: keyword with a following space
settings: default
language: kotlin
```before
fun t() {
    retu<caret>
}
```
keys: Ctrl+Space, Enter
```after
fun t() {
    return<caret>
}
```
(in a Unit function the item `return` has no space after when the next token... the rule: `return` gets a space after unless the function returns Unit; the observable result here is [verify])

### KT-CMP-060 case 48: this with labels
settings: default
language: kotlin
```before
class Outer {
    inner class Inner {
        fun t() { th<caret> }
    }
}
```
keys: Ctrl+Space
```after
rows include: this, this@Inner, this@Outer
```

### KT-CMP-060 case 49: return with label inside a lambda
settings: default
language: kotlin
```before
fun t() {
    listOf(1).forEach {
        ret<caret>
    }
}
```
keys: Ctrl+Space
```after
rows include: return, return@forEach
```

### KT-CMP-060 case 50: no dynamic on JVM
settings: platform=jvm
language: kotlin
```before
fun t(x: dyn<caret>) {}
```
keys: Ctrl+Space
```after
no "dynamic" row
```

### KT-CMP-070 case 51: expected type ranks matching items first
settings: default
language: kotlin
```before
fun t() {
    val label: String = "a"
    val count: Int = 1
    val s: String = <caret>
}
```
keys: Ctrl+Space
```after
rows in order: label : String, ... count : Int (the String-typed local is before the Int-typed one)
```

### KT-CMP-070 case 52: deprecated items last
settings: default
language: kotlin
```before
@Deprecated("x") fun oldWay() {}
fun newWay() {}
fun t() { way<caret> }
```
keys: Ctrl+Space (typing "way" uses middle matching)
```after
rows in order: newWay, oldWay (oldWay struck through)
```

### KT-CMP-070 case 53: component functions are deprioritised
settings: default
language: kotlin
```before
data class P(val a: Int, val b: Int)
fun t(p: P) { p.<caret> }
```
keys: Ctrl+Space
```after
component1(), component2() appear after a, b, copy(), equals(), hashCode(), toString()
```

### KT-CMP-070 case 54: named-argument rows rank by the position of their parameter
settings: default
language: kotlin
```before
fun f(first: Int, second: Int, third: Int) {}
fun t() { f(1, <caret>) }
```
keys: Ctrl+Space
```after
rows in order: second =, third =, then other items
```

### KT-CMP-070 case 55: kotlinx preferred over other libraries (proximity)
settings: default; kotlinx-coroutines and another library with a class named Job on the classpath
language: kotlin
```before
val j: Jo<caret>
```
keys: Ctrl+Space
```after
rows in order: Job | (kotlinx.coroutines) ..., Job | (other.pkg) ...
```

### KT-CMP-080 case 56: names of overridable members after override
settings: default
language: kotlin
```before
interface I {
    val someVal: Int
    fun foo()
}
class B : I {
    override fun <caret>
}
```
keys: Ctrl+Space
```after
rows: foo() : Unit    (and "someVal" is not offered after "fun")
```

### KT-CMP-080 case 57: choose an override
settings: default
language: kotlin
```before
interface I {
    fun foo(): Int
}
class B : I {
    override fun fo<caret>
}
```
keys: Ctrl+Space, Enter
```after
interface I {
    fun foo(): Int
}
class B : I {
    override fun foo(): Int {
        TODO("Not yet implemented")
    }
}
```

### KT-CMP-080 case 58: override in the primary constructor
settings: default
language: kotlin
```before
interface I { val someVal: Int }
class A(override val <caret>) : I
```
keys: Ctrl+Space
```after
rows: someVal : Int
```

### KT-CMP-080 case 59: parameter name suggestions from classes
settings: default
language: kotlin
```before
class FooBar
fun f(<caret>) {}
```
keys: type "fb", Ctrl+Space
```after
rows include: fooBar: FooBar
```

### KT-CMP-080 case 60: names from existing parameters in the file
settings: default
language: kotlin
```before
fun a(userName: String) {}
fun b(<caret>) {}
```
keys: type "us", Ctrl+Space, Enter
```after
fun a(userName: String) {}
fun b(userName: String<caret>) {}
```

### KT-CMP-090 case 61: when over an enum lists unused entries and else
settings: default
language: kotlin
```before
enum class Color { RED, GREEN, BLUE }
fun t(c: Color) = when (c) {
    Color.RED -> 1
    <caret>
}
```
keys: Ctrl+Space
```after
rows include: Color.GREEN, Color.BLUE, else
rows do not include: Color.RED
```

### KT-CMP-090 case 62: when over a sealed class lists subclasses
settings: default
language: kotlin
```before
sealed class R
class Ok : R()
object Err : R()
fun t(r: R) = when (r) {
    <caret>
}
```
keys: Ctrl+Space
```after
rows include: is Ok, Err, else
```

### KT-CMP-090 case 63: nullable subject offers null
settings: default
language: kotlin
```before
fun t(s: String?) = when (s) {
    <caret>
}
```
keys: Ctrl+Space
```after
rows include: null, else, is String
```

### KT-CMP-100 case 64: comma selects the item
settings: default
language: kotlin
```before
fun t(alpha: Int, beta: Int) = listOf(alp<caret>)
```
keys: Ctrl+Space (lookup open), type ","
```after
fun t(alpha: Int, beta: Int) = listOf(alpha,<caret>)
```

### KT-CMP-100 case 65: dot selects the item
settings: default
language: kotlin
```before
fun t(alpha: String) = alp<caret>
```
keys: Ctrl+Space, type "."
```after
fun t(alpha: String) = alpha.<caret>
```

### KT-CMP-100 case 66: brace selects only function items
settings: default
language: kotlin
```before
fun <T> run2(block: () -> T): T = block()
val run2value = 1
fun t() { run2v<caret> }
```
keys: Ctrl+Space (item "run2value" selected), type "{"
```after
the lookup closes without inserting run2value; text is "run2v{" [item does not accept braces]
```

### KT-CMP-100 case 67: unknown character closes the lookup
settings: default
language: kotlin
```before
fun t(alpha: Int) = alp<caret>
```
keys: Ctrl+Space, type "+"
```after
the lookup closes; text is "alp+" (no insertion)
```

### KT-CMP-100 case 68: colon extends the prefix for callable references
settings: default
language: kotlin
```before
fun isEven(n: Int) = true
fun t() = listOf(1).filter(:<caret>)
```
keys: type ":" (second colon) while the lookup is open
```after
the lookup stays open with prefix "::"
```

### KT-CMP-110 case 69: smart completion filters by expected type
settings: default
language: kotlin
```before
fun t() {
    val name: String = "a"
    val count: Int = 1
    val s: String = <caret>
}
```
keys: Ctrl+Shift+Space
```after
rows include: name ; rows do not include: count
```

### KT-CMP-110 case 70: smart completion offers a chained member
settings: default
language: kotlin
```before
class Holder(val title: String)
fun t(h: Holder) {
    val s: String = <caret>
}
```
keys: Ctrl+Shift+Space
```after
rows include: h.title
```

### KT-CMP-110 case 71: smart completion tail for an argument
settings: default
language: kotlin
```before
fun use(a: Int, b: String) {}
fun t() {
    val n = 1
    use(<caret>)
}
```
keys: Ctrl+Shift+Space, choose "n", Enter
```after
fun use(a: Int, b: String) {}
fun t() {
    val n = 1
    use(n, <caret>)
}
```

### KT-CMP-110 case 72: smart completion offers null and booleans
settings: default
language: kotlin
```before
fun t() {
    val a: String? = <caret>
    val b: Boolean = <caret2>
}
```
keys: Ctrl+Shift+Space at each caret
```after
caret 1 rows include: null ; caret 2 rows include: true, false
```

### KT-CMP-130 case 73: settings dump
settings: default
language: kotlin
```before
(any)
```
keys: action DumpSettings (internal)
```after
kotlin.k2.dot.add.enabled=true
kotlin.auto.completion.prefer.vararg.to.noargs=true
kotlin.auto.completion.insert.constructor.parenthesis=false
kotlin.auto.completion.dumb.mode.use.psi.completion=true
kotlin.k2.chain.completion.enabled=true
kotlin.k2.smart.completion.enabled=true
kotlin.k2.complete.top.level.packages=true
kotlin.java.completion.companion.members=true
kotlin.java.completion.extensions=true
kotlin.disable.auto.completion.inside.expression=false
```
