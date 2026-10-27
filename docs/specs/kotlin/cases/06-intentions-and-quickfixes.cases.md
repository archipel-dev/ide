# Cases for 06-intentions-and-quickfixes

Row texts marked [verify] in the cases have not been confirmed against a running IDE.

### KT-INT-010 case 1: Convert to block body
settings: default
language: kotlin
```before
fun foo() <caret>= "abc"
```
keys: Alt+Enter, "Convert to block body", Enter
```after
fun foo(): String {
    return "abc"
}
```

### KT-INT-011 case 1: Specify type explicitly
settings: default
language: kotlin
```before
fun main() {
    val <caret>a = listOf(1, 2, 3)
}
```
keys: Alt+Enter, "Specify type explicitly", Enter
```after
fun main() {
    val a: List<Int> = listOf(1, 2, 3)
}
```

### KT-INT-012 case 1: Remove explicit type specification
settings: default
language: kotlin
```before
fun main() {
    val a<caret>: List<Int> = listOf(1, 2, 3)
}
```
keys: Alt+Enter, "Remove explicit type specification", Enter
```after
fun main() {
    val a = listOf(1, 2, 3)
}
```

### KT-INT-013 case 1: Replace 'if' with 'when'
settings: default
language: kotlin
```before
fun f(n: Int): String {
    <caret>if (n == 1) return "one"
    else if (n == 2) return "two"
    else return "?"
}
```
keys: Alt+Enter, "Replace 'if' with 'when'", Enter
```after
fun f(n: Int): String {
    when (n) {
        1 -> return "one"
        2 -> return "two"
        else -> return "?"
    }
}
```

### KT-INT-014 case 1: Replace 'when' with 'if'
settings: default
language: kotlin
```before
fun f(n: Int) {
    <caret>when (n) {
        1 -> println("a")
        else -> println("b")
    }
}
```
keys: Alt+Enter, "Replace 'when' with 'if'", Enter
```after
fun f(n: Int) {
    if (n == 1) println("a")
    else println("b")
}
```

### KT-INT-015 case 1: Add braces to 'if' statement
settings: default
language: kotlin
```before
fun f(flag: Boolean): Boolean {
    <caret>if (flag) return true
    return false
}
```
keys: Alt+Enter, "Add braces to 'if' statement", Enter
```after
fun f(flag: Boolean): Boolean {
    if (flag) {
        return true
    }
    return false
}
```

### KT-INT-016 case 1: Remove braces from 'if' statement
settings: default
language: kotlin
```before
fun f(b: Boolean): Int {
    <caret>if (b) {
        return 1
    }
    return 0
}
```
keys: Alt+Enter, "Remove braces from 'if' statement", Enter
```after
fun f(b: Boolean): Int {
    if (b) return 1
    return 0
}
```

### KT-INT-017 case 1: Convert concatenation to template
settings: default
language: kotlin
```before
fun f(x: String) = <caret>"Hello " + x
```
keys: Alt+Enter, "Convert concatenation to template", Enter
```after
fun f(x: String) = "Hello $x"
```

### KT-INT-018 case 1: Invert 'if' condition
settings: default
language: kotlin
```before
fun f(a: Boolean) {
    <caret>if (!a) {
        println(1)
    } else {
        println(2)
    }
}
```
keys: Alt+Enter, "Invert 'if' condition", Enter
```after
fun f(a: Boolean) {
    if (a) {
        println(2)
    } else {
        println(1)
    }
}
```

### KT-INT-019 case 1: Flip '+'
settings: default
language: kotlin
```before
val n = 1 <caret>+ 2
```
keys: Alt+Enter, "Flip '+'", Enter
```after
val n = 2 + 1
```

### KT-INT-020 case 1: Convert lambda to reference
settings: default
language: kotlin
```before
class Person(val name: String)
val p = listOf("a").map <caret>{ Person(it) }
```
keys: Alt+Enter, "Convert lambda to reference", Enter
```after
class Person(val name: String)
val p = listOf("a").map(::Person)
```

### KT-INT-021 case 1: Replace 'it' with explicit parameter
settings: default
language: kotlin
```before
val r = listOf(1).filter { <caret>it % 2 == 0 }
```
keys: Alt+Enter, "Replace 'it' with explicit parameter", Enter
```after
val r = listOf(1).filter { x -> x % 2 == 0 }
```

### KT-INT-022 case 1: Replace explicit parameter 'x' with 'it'
settings: default
language: kotlin
```before
val r = listOf(1).filter { <caret>x -> x % 2 == 0 }
```
keys: Alt+Enter, "Replace explicit parameter 'x' with 'it'", Enter
```after
val r = listOf(1).filter { it % 2 == 0 }
```

### KT-INT-023 case 1: Add names to call arguments
settings: default
language: kotlin
```before
fun check(expected: Int, actual: Int) {}
fun u() { check(<caret>24, 42) }
```
keys: Alt+Enter, "Add names to call arguments", Enter
```after
fun check(expected: Int, actual: Int) {}
fun u() { check(expected = 24, actual = 42) }
```

### KT-INT-024 case 1: Remove argument name
settings: default
language: kotlin
```before
fun u(l: List<String>) = l.joinToString(<caret>separator = "\n")
```
keys: Alt+Enter, "Remove argument name", Enter
```after
fun u(l: List<String>) = l.joinToString("\n")
```

### KT-INT-025 case 1: Add import for 'invokeLater'
settings: default
language: kotlin
```before
import javax.swing.SwingUtilities
fun foo() {
    <caret>SwingUtilities.invokeLater { }
}
```
keys: Alt+Enter, "Add import for 'invokeLater'", Enter
```after
import javax.swing.SwingUtilities
import javax.swing.SwingUtilities.invokeLater
fun foo() {
    invokeLater { }
}
```

### KT-INT-026 case 1: Import members from 'javax.swing.SwingUtilities'
settings: default
language: kotlin
```before
import javax.swing.SwingUtilities
fun foo() {
    <caret>SwingUtilities.invokeLater { }
}
```
keys: Alt+Enter, "Import members from 'javax.swing.SwingUtilities'", Enter
```after
import javax.swing.SwingUtilities
import javax.swing.SwingUtilities.*
fun foo() {
    invokeLater { }
}
```

### KT-INT-027 case 1: Introduce import alias
settings: default
language: kotlin
```before
import com.test.Movie
class T {
    fun a() = <caret>Movie()
    fun b() = Movie()
}
```
keys: Alt+Enter, "Introduce import alias", Enter
```after
import com.test.Movie as Movie1
class T {
    fun a() = Movie1()
    fun b() = Movie1()
}
```

### KT-INT-028 case 1: Add 'val' or 'var' to parameter 'email'
settings: default
language: kotlin
```before
class User(name: String, <caret>email: String)
```
keys: Alt+Enter, "Add 'val' or 'var' to parameter 'email'", Enter
```after
class User(name: String, val email: String)
```

### KT-INT-029 case 1: Move to constructor
settings: default
language: kotlin
```before
class T {
    private val <caret>text = "Lorem"
}
```
keys: Alt+Enter, "Move to constructor", Enter
```after
class T(private val text: String = "Lorem") {
}
```

### KT-INT-030 case 1: Move to class body
settings: default
language: kotlin
```before
class T(private val <caret>text: String)
```
keys: Alt+Enter, "Move to class body", Enter
```after
class T(text: String) {
    private val text = text
}
```

### KT-INT-031 case 1: Convert property to function
settings: default
language: kotlin
```before
class A(val n: Int) {
    val <caret>foo: Int get() = n + 1
}
fun t() = A(1).foo
```
keys: Alt+Enter, "Convert property to function", Enter
```after
class A(val n: Int) {
    fun foo(): Int = n + 1
}
fun t() = A(1).foo()
```

### KT-INT-032 case 1: Convert function to property
settings: default
language: kotlin
```before
class A(val n: Int) {
    fun <caret>foo(): Int = n + 1
}
fun t() = A(1).foo()
```
keys: Alt+Enter, "Convert function to property", Enter
```after
class A(val n: Int) {
    val foo: Int get() = n + 1
}
fun t() = A(1).foo
```

### KT-INT-033 case 1: Split property declaration
settings: default
language: kotlin
```before
fun f(a: Int, b: Int) {
    val <caret>n = a + b
}
```
keys: Alt+Enter, "Split property declaration", Enter
```after
fun f(a: Int, b: Int) {
    val n: Int
    n = a + b
}
```

### KT-INT-034 case 1: Split 'if' into two
settings: default
language: kotlin
```before
fun f(a: Boolean, b: Boolean) {
    if (a <caret>&& b) println()
}
```
keys: Alt+Enter, "Split 'if' into two", Enter
```after
fun f(a: Boolean, b: Boolean) {
    if (a)
        if (b) println()
}
```

### KT-INT-035 case 1: Merge 'if's
settings: default
language: kotlin
```before
fun f(a: Boolean, b: Boolean) {
    <caret>if (a) {
        if (b) {
            println()
        }
    }
}
```
keys: Alt+Enter, "Merge 'if's", Enter
```after
fun f(a: Boolean, b: Boolean) {
    if (a && b) {
        println()
    }
}
```

### KT-INT-036 case 1: Convert to raw string literal
settings: default
language: kotlin
```before
val s = <caret>"a\nb"
```
keys: Alt+Enter, "Convert to raw string literal", Enter
```after
val s = """a
b"""
```

### KT-INT-037 case 1: Replace overloaded operator with function call
settings: default
language: kotlin
```before
fun f(a: Int, b: Int) = a <caret>* b
```
keys: Alt+Enter, "Replace overloaded operator with function call", Enter
```after
fun f(a: Int, b: Int) = a.times(b)
```

### KT-INT-038 case 1: Replace with infix function call
settings: default
language: kotlin
```before
infix fun Int.foo(x: Int) = x
fun t() = 1.<caret>foo(2)
```
keys: Alt+Enter, "Replace with infix function call", Enter
```after
infix fun Int.foo(x: Int) = x
fun t() = 1 foo 2
```

### KT-INT-039 case 1: Move lambda argument into parentheses
settings: default
language: kotlin
```before
fun bar(n: Int, f: (Int) -> Int) = f(n)
fun t() = bar(2) <caret>{ it * 3 }
```
keys: Alt+Enter, "Move lambda argument into parentheses", Enter
```after
fun bar(n: Int, f: (Int) -> Int) = f(n)
fun t() = bar(2, { it * 3 })
```

### KT-INT-040 case 1: Replace with a for loop
settings: default
language: kotlin
```before
fun t(x: List<String>) {
    x.<caret>forEach { a -> println(a) }
}
```
keys: Alt+Enter, "Replace with a for loop", Enter
```after
fun t(x: List<String>) {
    for (a in x) {
        println(a)
    }
}
```

### KT-INT-041 case 1: Replace with a forEach function call
settings: default
language: kotlin
```before
fun t(list: List<String>) {
    <caret>for (x in list) {
        println(x)
    }
}
```
keys: Alt+Enter, "Replace with a forEach function call", Enter
```after
fun t(list: List<String>) {
    list.forEach { x -> println(x) }
}
```

### KT-INT-042 case 1: Iterate over collection
settings: default
language: kotlin
```before
fun t() {
    <caret>listOf(1, 2, 3)
}
```
keys: Alt+Enter, "Iterate over collection", Enter
```after
fun t() {
    for (i in listOf(1, 2, 3)) {
    }
}
```

### KT-INT-043 case 1: Add remaining branches
settings: default
language: kotlin
```before
enum class E { A, B }
fun t(e: E) {
    <caret>when (e) {
        E.A -> {}
    }
}
```
keys: Alt+Enter, "Add remaining branches", Enter
```after
enum class E { A, B }
fun t(e: E) {
    when (e) {
        E.A -> {}
        E.B -> TODO()
    }
}
```

### KT-INT-044 case 1: Implement abstract member
settings: default
language: kotlin
```before
interface A { <caret>fun foo(): Int }
class B : A
```
keys: Alt+Enter, "Implement abstract member", Enter
```after
interface A { fun foo(): Int }
class B : A {
    override fun foo(): Int {
        TODO("Not yet implemented")
    }
}
```

### KT-INT-045 case 1: Convert to apply
settings: default
language: kotlin
```before
class M { var a = 0; var b = 0 }
fun f() {
    <caret>val m = M()
    m.a = 1
    m.b = 2
}
```
keys: Alt+Enter, "Convert to apply", Enter
```after
class M { var a = 0; var b = 0 }
fun f() {
    val m = M().apply {
        a = 1
        b = 2
    }
}
```

### KT-INT-046 case 1: Convert to with
settings: default
language: kotlin
```before
class M { var a = 0; var b = 0 }
fun f(m: M) {
    <caret>m.a = 1
    m.b = 2
}
```
keys: Alt+Enter, "Convert to with", Enter
```after
class M { var a = 0; var b = 0 }
fun f(m: M) {
    with(m) {
        a = 1
        b = 2
    }
}
```

### KT-INT-047 case 1: Replace safe access expression with 'if' expression
settings: default
language: kotlin
```before
fun f(s: String?) = s<caret>?.length
```
keys: Alt+Enter, "Replace safe access expression with 'if' expression", Enter
```after
fun f(s: String?) = if (s != null) s.length else null
```

### KT-INT-048 case 1: Replace elvis expression with 'if' expression
settings: default
language: kotlin
```before
fun f(s: String?, d: String) = s <caret>?: d
```
keys: Alt+Enter, "Replace elvis expression with 'if' expression", Enter
```after
fun f(s: String?, d: String) = if (s != null) s else d
```

### KT-INT-049 case 1: Add '@JvmStatic' annotation
settings: default
language: kotlin
```before
object O {
    fun <caret>f() {}
}
```
keys: Alt+Enter, "Add '@JvmStatic' annotation", Enter
```after
object O {
    @JvmStatic
    fun f() {}
}
```

### KT-INT-050 case 1: Add '@JvmOverloads' annotation
settings: default
language: kotlin
```before
fun <caret>f(a: Int, b: Int = 1) {}
```
keys: Alt+Enter, "Add '@JvmOverloads' annotation", Enter
```after
@JvmOverloads
fun f(a: Int, b: Int = 1) {}
```

### KT-QF-001 case 2: create function from usage
settings: default
language: kotlin
```before
fun t() {
    val n: Int = <caret>compute("a", 1)
}
```
keys: Alt+Enter, "Create function 'compute'", Enter (template: accept names)
```after
fun t() {
    val n: Int = compute("a", 1)
}

fun compute(s: String, i: Int): Int {
    TODO("Not yet implemented")
}
```

### KT-QF-001 case 3: create class from usage
settings: default
language: kotlin
```before
fun t() {
    val x = <caret>Widget()
}
```
keys: Alt+Enter, "Create class 'Widget'", Enter
```after
fun t() {
    val x = Widget()
}

class Widget
```

### KT-QF-001 case 4: create local variable
settings: default
language: kotlin
```before
fun t() {
    <caret>count = 1
}
```
keys: Alt+Enter, "Create local variable 'count'", Enter
```after
fun t() {
    var count = 1
}
```
[verify var/val]

### KT-QF-001 case 5: create parameter
settings: default
language: kotlin
```before
fun t() {
    println(<caret>name)
}
```
keys: Alt+Enter, "Create parameter 'name'", Enter
```after
fun t(name: Any) {
    println(name)
}
```
[verify inferred type]
