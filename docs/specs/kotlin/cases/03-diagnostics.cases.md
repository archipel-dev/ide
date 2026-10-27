# Cases for 03-diagnostics-catalog

Conventions: "mark X: message" means a highlight whose range is exactly the text X, with the given description (tooltip is the same
text HTML-escaped). `[prefix]` = only the quoted start is specified (the rest is a rendered declaration or candidate list, see 3.3.7).
"fixes" lists Alt+Enter rows in order (error fixes first; platform rows such as "Edit inspection profile setting" omitted).

### KT-DIAG-010 case 1: unresolved reference on a property name
settings: default
language: kotlin
```before
fun t() {
    val x = unknownName
}
```
keys: wait for highlighting
```after
mark "unknownName" (wrong-reference style, error): Unresolved reference 'unknownName'.
```

### KT-DIAG-010 case 2: unresolved call marks only the callee name
settings: default
language: kotlin
```before
fun t() {
    foo(1, 2)
}
```
keys: wait for highlighting
```after
mark "foo": Unresolved reference 'foo'.
(the arguments carry no mark)
```

### KT-DIAG-010 case 3: unresolved selector after a dot
settings: default
language: kotlin
```before
fun t(s: String) {
    s.nope()
}
```
keys: wait for highlighting
```after
mark "nope": Unresolved reference 'nope'.
```

### KT-DIAG-010 case 4: unresolved type
settings: default
language: kotlin
```before
val x: Missing? = null
```
keys: wait for highlighting
```after
mark "Missing" (not the question mark): Unresolved reference 'Missing'.
```

### KT-DIAG-011 case 5: unresolved import marks the last segment
settings: default
language: kotlin
```before
import not.there.Foo
```
keys: wait for highlighting
```after
mark "Foo": Unresolved reference 'Foo'.
```

### KT-DIAG-014 case 6: invisible reference
settings: default
language: kotlin
```before
class A { private val x = 1 }
fun t(a: A) = a.x
```
keys: wait for highlighting
```after
mark "x" in "a.x": Cannot access 'val x: Int': it is private in 'A'. [prefix: "Cannot access '"]
```

### KT-DIAG-015 case 7: overload ambiguity
settings: default
language: kotlin
```before
fun f(a: Any?, b: String) {}
fun f(a: String, b: Any?) {}
fun t() { f("a", "b") }
```
keys: wait for highlighting
```after
mark "f" in "f("a", "b")": Overload resolution ambiguity between candidates: [prefix, then one candidate per line]
```

### KT-DIAG-016 case 8: none applicable
settings: default
language: kotlin
```before
fun g(a: Int) {}
fun g(a: String) {}
fun t() { g(1.5) }
```
keys: wait for highlighting
```after
mark "g": None of the following candidates is applicable: [prefix, blank line, one candidate per line]
```

### KT-DIAG-017 case 9: conflicting import
settings: default
language: kotlin
```before
--- src/a/Foo.kt
package a
class Foo
--- src/b/Foo.kt
package b
class Foo
--- src/c/Use.kt
package c
import a.Foo
import b.Foo
```
keys: wait for highlighting
```after
mark "Foo" in "import b.Foo": Conflicting import: imported name 'Foo' is ambiguous.
fixes: Remove conflicting import b.Foo [verify text of the placeholder]
```

### KT-DIAG-030 case 10: type mismatch for if-branches
settings: default
language: kotlin
```before
fun t(b: Boolean): Int = if (b) 1 else "no"
```
keys: wait for highlighting
```after
mark "\"no\"": a type-mismatch error whose text starts with "Return type mismatch: expected 'Int', actual" [verify: branch is reported as the return value]
```

### KT-DIAG-031 case 11: argument type mismatch
settings: default
language: kotlin
```before
fun f(a: Int) {}
fun t() { f("x") }
```
keys: wait for highlighting
```after
mark "\"x\"": Argument type mismatch: actual type is 'String', but 'Int' was expected.
fixes include: Change parameter 'a' type of function 'f' to 'String'
```

### KT-DIAG-032 case 12: initializer type mismatch
settings: default
language: kotlin
```before
val x: Int = "a"
```
keys: wait for highlighting
```after
mark "\"a\"": Initializer type mismatch: expected 'Int', actual 'String'.
fixes include: Change type of 'x' to 'String'
```

### KT-DIAG-032 case 13: apply the "change type" fix
settings: default
language: kotlin
```before
val x: Int = <caret>"a"
```
keys: Alt+Enter, "Change type of 'x' to 'String'", Enter
```after
val x: String = "a"
```

### KT-DIAG-033 case 14: return type mismatch
settings: default
language: kotlin
```before
fun f(): Int {
    return "a"
}
```
keys: wait for highlighting
```after
mark "\"a\"": Return type mismatch: expected 'Int', actual 'String'.
```

### KT-DIAG-035 case 15: condition type mismatch
settings: default
language: kotlin
```before
fun t() { if (1) {} }
```
keys: wait for highlighting
```after
mark "1": Condition type mismatch: inferred type is 'Int' but 'Boolean' was expected.
```

### KT-DIAG-037 case 16: null for non-null
settings: default
language: kotlin
```before
val s: String = null
```
keys: wait for highlighting
```after
mark "null": Null cannot be a value of a non-null type 'String'.
fixes include: Change type of 's' to 'String?'
```

### KT-DIAG-038 case 17: unsafe call marks the dot
settings: default
language: kotlin
```before
fun t(s: String?) = s.length
```
keys: wait for highlighting
```after
mark "." between "s" and "length": Only safe (?.) or non-null asserted (!!.) calls are allowed on a nullable receiver of type 'String?'.
```

### KT-DIAG-038 case 18: unsafe call fixes, in order
settings: default
language: kotlin
```before
fun t(s: String?) = s<caret>.length
```
keys: Alt+Enter
```after
first rows, in order:
Replace with safe (?.) call
Wrap with '?.let { ... }' call
Add non-null asserted (s!!) call
```

### KT-DIAG-038 case 19: replace with safe call
settings: default
language: kotlin
```before
fun t(s: String?) = s<caret>.length
```
keys: Alt+Enter, "Replace with safe (?.) call", Enter
```after
fun t(s: String?) = s?.length
```

### KT-DIAG-038 case 20: add non-null assertion
settings: default
language: kotlin
```before
fun t(s: String?) = s<caret>.length
```
keys: Alt+Enter, "Add non-null asserted (s!!) call", Enter
```after
fun t(s: String?) = s!!.length
```

### KT-DIAG-041 case 21: unnecessary safe call
settings: default
language: kotlin
```before
fun t(s: String) = s<caret>?.length
```
keys: wait for highlighting; Alt+Enter, "Replace with dot call", Enter
```after
mark "?." : Unnecessary safe call on a non-null receiver of type 'String'. (warning)
fun t(s: String) = s.length
```

### KT-DIAG-042 case 22: unnecessary non-null assertion
settings: default
language: kotlin
```before
fun t(s: String) = s<caret>!!.length
```
keys: wait for highlighting; Alt+Enter, "Remove unnecessary non-null assertion (!!)", Enter
```after
mark "!!": Unnecessary non-null assertion (!!) on a non-null receiver of type 'String'. (warning)
fun t(s: String) = s.length
```

### KT-DIAG-043 case 23: useless elvis
settings: default
language: kotlin
```before
fun t(s: String) = s <caret>?: "x"
```
keys: wait for highlighting; Alt+Enter, "Remove redundant elvis operator", Enter
```after
mark "?: \"x\"" (grey): Elvis operator (?:) always returns the left operand of non-nullable type 'String'.
fun t(s: String) = s
```

### KT-DIAG-044 case 24: useless cast
settings: default
language: kotlin
```before
fun t(s: String) = s <caret>as String
```
keys: wait for highlighting; Alt+Enter, "Remove redundant cast", Enter
```after
mark "as String" (grey): Cast is redundant.
fun t(s: String) = s
```

### KT-DIAG-046 case 25: unchecked cast is a suppressible warning
settings: default
language: kotlin
```before
fun t(a: Any) = a <caret>as List<String>
```
keys: wait for highlighting; Alt+Enter
```after
mark "as List<String>" (warning): Unchecked cast of 'Any' to 'List<String>'.
the popup contains the group: Suppress 'UNCHECKED_CAST' for fun t, and the item: Compiler warning 'UNCHECKED_CAST' options
```

### KT-DIAG-140 case 26: suppress for the declaration
settings: default
language: kotlin
```before
fun t(a: Any) = a <caret>as List<String>
```
keys: Alt+Enter, "Suppress 'UNCHECKED_CAST' for fun t", Enter
```after
@Suppress("UNCHECKED_CAST")
fun t(a: Any) = a as List<String>
```

### KT-DIAG-140 case 27: suppress for the statement
settings: default
language: kotlin
```before
fun t(a: Any) {
    val l = a <caret>as List<String>
}
```
keys: Alt+Enter, "Suppress 'UNCHECKED_CAST' for statement", Enter
```after
fun t(a: Any) {
    @Suppress("UNCHECKED_CAST")
    val l = a as List<String>
}
```

### KT-DIAG-140 case 28: an existing Suppress silences the warning
settings: default
language: kotlin
```before
@Suppress("UNCHECKED_CAST")
fun t(a: Any) = a as List<String>
```
keys: wait for highlighting
```after
no highlight on "as List<String>"
```

### KT-DIAG-060 case 29: no value passed for a parameter
settings: default
language: kotlin
```before
fun f(a: Int, b: Int) {}
fun t() { f(1) }
```
keys: wait for highlighting
```after
mark "f" in "f(1)": No value passed for parameter 'b'.
```

### KT-DIAG-061 case 30: too many arguments
settings: default
language: kotlin
```before
fun f(a: Int) {}
fun t() { f(1, 2) }
```
keys: wait for highlighting
```after
mark "2": Too many arguments for 'fun f(a: Int): Unit'. [prefix: "Too many arguments for 'fun f("]
fixes include: Remove argument
```

### KT-DIAG-062 case 31: named parameter not found
settings: default
language: kotlin
```before
fun f(a: Int) {}
fun t() { f(b = 1) }
```
keys: wait for highlighting
```after
mark "b": No parameter with name 'b' found.
```

### KT-DIAG-065 case 32: iterator missing
settings: default
language: kotlin
```before
class Box
fun t(b: Box) { for (x in b) {} }
```
keys: wait for highlighting
```after
mark "b" in "in b": For-loop range must have an 'iterator()' method.
```

### KT-DIAG-066 case 33: component function missing
settings: default
language: kotlin
```before
class P(val a: Int, val b: Int)
fun t(p: P) { val (x, y) = p }
```
keys: wait for highlighting
```after
mark "p" (the initializer) for component1 and for component2 (two errors): Destructuring of type 'P' requires operator function 'component1()'. / 'component2()'.
```

### KT-DIAG-067 case 34: no get operator
settings: default
language: kotlin
```before
class Box
fun t(b: Box) = b[0]
```
keys: wait for highlighting
```after
mark "[0]": No 'get' operator method providing array access.
```

### KT-DIAG-080 case 35: missing return marks the closing brace
settings: default
language: kotlin
```before
fun f(x: Int): Int {
    println(x)
}
```
keys: wait for highlighting
```after
mark "}" (the last one): Missing return statement.
```

### KT-DIAG-081 case 36: uninitialized variable
settings: default
language: kotlin
```before
fun t() {
    val x: Int
    println(x)
}
```
keys: wait for highlighting
```after
mark "x" in println(x): Variable 'x' must be initialized.
```

### KT-DIAG-083 case 37: val reassignment and fix
settings: default
language: kotlin
```before
fun t() {
    val x = 1
    <caret>x = 2
}
```
keys: wait for highlighting; Alt+Enter, "Change to 'var'", Enter
```after
mark "x" (left side of the assignment): 'val' cannot be reassigned.
fun t() {
    var x = 1
    x = 2
}
```

### KT-DIAG-084 case 38: non-exhaustive when on an enum
settings: default
language: kotlin
```before
enum class Color { RED, GREEN, BLUE }
fun t(c: Color) = <caret>when (c) {
    Color.RED -> 1
}
```
keys: wait for highlighting
```after
mark "when": 'when' expression must be exhaustive. Add the 'GREEN', 'BLUE' branches or an 'else' branch.
fixes: Add remaining branches, Add remaining branches with * import, Add else branch (the order of the first and the last is [verify])
```

### KT-DIAG-084 case 39: add remaining branches (enum entries qualified)
settings: default
language: kotlin
```before
enum class Color { RED, GREEN, BLUE }
fun t(c: Color) = <caret>when (c) {
    Color.RED -> 1
}
```
keys: Alt+Enter, "Add remaining branches", Enter
```after
enum class Color { RED, GREEN, BLUE }
fun t(c: Color) = when (c) {
    Color.RED -> 1
    Color.GREEN -> TODO()
    Color.BLUE -> TODO()
}
```

### KT-DIAG-084 case 40: when over an Int needs else
settings: default
language: kotlin
```before
fun t(n: Int) = <caret>when (n) {
    1 -> "a"
}
```
keys: wait for highlighting; Alt+Enter, "Add else branch", Enter
```after
mark "when": 'when' expression must be exhaustive. Add an 'else' branch.
fun t(n: Int) = when (n) {
    1 -> "a"
    else -> {}
}
(the content of the new else branch is [verify]; the position is after the last branch)
```

### KT-DIAG-085 case 41: if as expression without else
settings: default
language: kotlin
```before
fun t(b: Boolean): Int = <caret>if (b) 1
```
keys: wait for highlighting; Alt+Enter, "Add else branch", Enter
```after
mark "if": 'if' must have both main and 'else' branches when used as an expression.
fun t(b: Boolean): Int = if (b) 1 else {}
(the content of the new else branch is [verify])
```

### KT-DIAG-086 case 42: conflicting overloads
settings: default
language: kotlin
```before
fun f(a: Int) {}
fun f(a: Int) {}
```
keys: wait for highlighting
```after
mark "fun f(a: Int)" (signature, both declarations): Conflicting overloads: [prefix, then one candidate per line]
```

### KT-DIAG-087 case 43: abstract member not implemented
settings: default
language: kotlin
```before
interface A { fun foo() }
class <caret>B : A
```
keys: wait for highlighting
```after
mark "class B": Class 'B' is not abstract and does not implement abstract member: [prefix; next line: fun foo(): Unit]
fixes include: Implement members, Make 'B' abstract
```

### KT-DIAG-088 case 44: override of nothing
settings: default
language: kotlin
```before
open class A
class B : A() {
    <caret>override fun foo() {}
}
```
keys: wait for highlighting; Alt+Enter, "Remove 'override' modifier", Enter
```after
mark "override": 'foo' overrides nothing.
open class A
class B : A() {
    fun foo() {}
}
```

### KT-DIAG-089 case 45: hides member needs override
settings: default
language: kotlin
```before
open class A { open fun foo() {} }
class B : A() {
    fun <caret>foo() {}
}
```
keys: wait for highlighting; Alt+Enter, "Add 'override' modifier", Enter
```after
mark "foo": 'foo' hides member of supertype 'A' and needs an 'override' modifier.
open class A { open fun foo() {} }
class B : A() {
    override fun foo() {}
}
```

### KT-DIAG-090 case 46: overriding a final member
settings: default
language: kotlin
```before
open class A { fun foo() {} }
class B : A() {
    <caret>override fun foo() {}
}
```
keys: wait for highlighting
```after
mark "override": 'foo' in 'A' is final and cannot be overridden.
fixes include: Make 'foo' in A open
```

### KT-DIAG-091 case 47: supertype with a constructor must be initialised
settings: default
language: kotlin
```before
open class A
class B : <caret>A
```
keys: wait for highlighting; Alt+Enter, "Change to constructor invocation", Enter
```after
mark "A": This type has a constructor, so it must be initialized here.
open class A
class B : A()
```

### KT-DIAG-092 case 48: final supertype
settings: default
language: kotlin
```before
class A
class B : <caret>A()
```
keys: wait for highlighting
```after
mark "A": This type is final, so it cannot be extended.
fixes include: Make 'A' open
```

### KT-DIAG-106 case 49: deprecated usage shows strikethrough
settings: default
language: kotlin
```before
@Deprecated("Use newFun")
fun oldFun() {}
fun t() { oldFun() }
```
keys: wait for highlighting
```after
mark "oldFun" in t() (warning, strikethrough): 'fun oldFun(): Unit' is deprecated. Use newFun. [prefix: "'fun oldFun(): Unit' is deprecated."]
```

### KT-DIAG-106 case 50: deprecation with level ERROR
settings: default
language: kotlin
```before
@Deprecated("gone", level = DeprecationLevel.ERROR)
fun oldFun() {}
fun t() { oldFun() }
```
keys: wait for highlighting
```after
mark "oldFun" in t() (error, strikethrough): 'fun oldFun(): Unit' is deprecated. gone. [prefix: "'fun oldFun(): Unit' is deprecated."]
```

### KT-DIAG-023 case 51: syntax errors show the parser message
settings: default
language: kotlin
```before
fun f( {
}
```
keys: wait for highlighting
```after
error element with description starting "Expecting" (e.g. "Expecting ')'")
```

### KT-DIAG-002 case 52: warnings without a fix still get suppression options
settings: default
language: kotlin
```before
fun t(x: Int) = 1 <caret>/ 0
```
keys: wait for highlighting; Alt+Enter
```after
mark "/" or the expression: Division by zero. (warning)
the popup contains Suppress 'DIVISION_BY_ZERO' for fun t
```

### KT-DIAG-002 case 53: code fragment does not report visibility errors
settings: default
language: kotlin
```before
(code fragment evaluated in the scope of class A, expression:) a.privateField
```
keys: wait for highlighting
```after
no INVISIBLE_REFERENCE error in the fragment
```

### KT-DIAG-084 case 54: add remaining branches with star import
settings: default
language: kotlin
```before
package p

enum class Color { RED, GREEN, BLUE }
fun t(c: Color) = <caret>when (c) {
    Color.RED -> 1
}
```
keys: Alt+Enter, "Add remaining branches with * import", Enter
```after
package p

import p.Color.*

enum class Color { RED, GREEN, BLUE }
fun t(c: Color) = when (c) {
    Color.RED -> 1
    GREEN -> TODO()
    BLUE -> TODO()
}
```

### KT-DIAG-084 case 55: remaining branches of a sealed class use "is"
settings: default
language: kotlin
```before
sealed class R
class Ok : R()
object Err : R()
fun t(r: R) = <caret>when (r) {
    is Ok -> 1
}
```
keys: wait for highlighting; Alt+Enter, "Add remaining branches", Enter
```after
mark "when": 'when' expression must be exhaustive. Add the 'Err' branch or an 'else' branch.
sealed class R
class Ok : R()
object Err : R()
fun t(r: R) = when (r) {
    is Ok -> 1
    Err -> TODO()
}
```
(objects are written without `is`, classes with `is`; generic classes get `<*>` per own type parameter)
