# Cases for 05-inspections

Conventions: after Alt+Enter the row with the given text is chosen (exact text; rows may carry the suffix `...` only where the text says so).
"mark" gives the highlighted range and message with the level of the profile default (`warning`, `weak warning`, `information` = no mark).
Each case is also expected to be silent when the premise is made unresolvable (KT-INS-002) and to be undone in one step.

### KT-INS-010 case 1: UnusedSymbol
settings: default
language: kotlin
```before
class A {
    private val <caret>unusedField = 1
    fun f() {}
}
```
keys: wait for highlighting; Alt+Enter, "Safe delete '<name>'", Enter
```after
mark [name identifier; grey (unused-symbol look)] (warning, on by default): `<Kind> '<name>' is never used` (Kind: Class, Interface, Object, Function, Property, Parameter, Constructor, Type alias, Enum entry, Type parameter)
class A {
    fun f() {}
}
```

### KT-INS-011 case 1: UnusedVariable
settings: default
language: kotlin
```before
fun f() {
    val <caret>unused = 42
    println("x")
}
```
keys: wait for highlighting; Alt+Enter, "Remove variable '<name>' (simple initializer)", Enter
```after
mark [variable name] (warning, on by default): `Unused variable`
fun f() {
    println("x")
}
```

### KT-INS-012 case 1: VariableNeverRead
settings: default
language: kotlin
```before
fun f() {
    var <caret>local = 42
    local = 0
}
```
keys: wait for highlighting
```after
mark [variable name] (warning): `Variable 'local' is never read`
(no automatic fix)
```

### KT-INS-013 case 1: AssignedValueIsNeverRead
settings: default
language: kotlin
```before
fun foo(): Int = 42
fun f() {
    var local = 0
    println(local)
    <caret>local = foo()
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant assignment", Enter
```after
mark [the assignment] (warning, on by default): `Assigned value is never read`
fun foo(): Int = 42
fun f() {
    var local = 0
    println(local)
    foo()
}
```

### KT-INS-014 case 1: VariableInitializerIsRedundant
settings: default
language: kotlin
```before
fun f() {
    var x = <caret>0
    x = 1
    println(x)
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant initializer", Enter
```after
mark [the initializer] (warning, on by default): `Variable initializer is redundant`
fun f() {
    var x: Int
    x = 1
    println(x)
}
```

### KT-INS-015 case 1: CanBeVal
settings: default
language: kotlin
```before
fun f() {
    <caret>var names = listOf("a", "b")
    println(names)
}
```
keys: wait for highlighting; Alt+Enter, "Change to 'val'", Enter
```after
mark [the `var` keyword] (warning, on by default): `Variable is never modified, so it can be declared using 'val'`
fun f() {
    val names = listOf("a", "b")
    println(names)
}
```

### KT-INS-016 case 1: RedundantExplicitType
settings: default
language: kotlin
```before
fun f() {
    val i<caret>: Int = 42
}
```
keys: wait for highlighting; Alt+Enter, "Remove explicit type specification", Enter
```after
mark [the type reference (and colon)] (warning, on by default): `Explicitly given type is redundant here`
fun f() {
    val i = 42
}
```

### KT-INS-017 case 1: RedundantVisibilityModifier
settings: default
language: kotlin
```before
<caret>public fun f() {}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant visibility modifier", Enter
```after
mark [the modifier] (warning, on by default): `Redundant visibility modifier`
fun f() {}
```

### KT-INS-018 case 1: RedundantModalityModifier
settings: default
language: kotlin
```before
<caret>final class Foo
```
keys: wait for highlighting; Alt+Enter, "Remove redundant modality modifier", Enter
```after
mark [the modifier] (warning, on by default): `Redundant modality modifier`
class Foo
```

### KT-INS-019 case 1: RedundantUnitReturnType
settings: default
language: kotlin
```before
fun f()<caret>: Unit {
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant 'Unit' return type", Enter
```after
mark [the return type] (warning, on by default): `Redundant 'Unit' return type`
fun f() {
}
```

### KT-INS-020 case 1: RedundantUnitExpression
settings: default
language: kotlin
```before
fun f() {
    println()
    <caret>Unit
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant 'Unit'", Enter
```after
mark [the `Unit` expression] (warning, on by default): `Redundant 'Unit'`
fun f() {
    println()
}
```

### KT-INS-021 case 1: RedundantSemicolon
settings: default
language: kotlin
```before
val x = 1<caret>;
```
keys: wait for highlighting; Alt+Enter, "Remove redundant semicolon", Enter
```after
mark [the semicolon] (warning, on by default): `Redundant semicolon`
val x = 1
```

### KT-INS-022 case 1: RedundantLambdaArrow
settings: default
language: kotlin
```before
fun foo(f: () -> Unit) = f()
fun bar() {
    foo { <caret>-> println("Hi!") }
}
```
keys: wait for highlighting; Alt+Enter, "Remove arrow", Enter
```after
mark [the arrow] (warning, on by default): `Redundant lambda arrow`
fun foo(f: () -> Unit) = f()
fun bar() {
    foo { println("Hi!") }
}
```

### KT-INS-023 case 1: RedundantSamConstructor
settings: default
language: kotlin
```before
fun foo(other: Runnable) {}
fun main() {
    foo(<caret>Runnable { println("Hi!") })
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant SAM constructor", Enter
```after
mark [the SAM constructor name] (warning, on by default): `Redundant SAM constructor`
fun foo(other: Runnable) {}
fun main() {
    foo({ println("Hi!") })
}
```

### KT-INS-024 case 1: RemoveRedundantQualifierName
settings: default
language: kotlin
```before
package my.simple.name
class Foo
fun main() {
    val a = my.simple.name.<caret>Foo()
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant qualifier name", Enter
```after
mark [the redundant qualifier] (warning, on by default): `Redundant qualifier name`
package my.simple.name
class Foo
fun main() {
    val a = Foo()
}
```

### KT-INS-025 case 1: RedundantCompanionReference
settings: default
language: kotlin
```before
class A {
    companion object { fun create() = A() }
}
fun test() {
    val s = A.<caret>Companion.create()
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant Companion reference", Enter
```after
mark [the `Companion` segment] (warning, on by default): `Redundant Companion reference`
class A {
    companion object { fun create() = A() }
}
fun test() {
    val s = A.create()
}
```

### KT-INS-026 case 1: RemoveCurlyBracesFromTemplate
settings: default
language: kotlin
```before
fun f() {
    val x = 4
    val y = "<caret>${x}"
}
```
keys: wait for highlighting; Alt+Enter, "Remove curly braces", Enter
```after
mark [the entry `${x}`] (weak warning, on by default): `Redundant curly braces in string template`
fun f() {
    val x = 4
    val y = "$x"
}
```

### KT-INS-027 case 1: ConstantConditionIf
settings: default
language: kotlin
```before
fun example() {
    if (<caret>true) {
        throw IllegalStateException("Unexpected state")
    }
}
```
keys: wait for highlighting; Alt+Enter, "Simplify expression", Enter
```after
mark [the condition] (weak warning, on by default): `Condition of 'if' expression is constant`
fun example() {
    throw IllegalStateException("Unexpected state")
}
```

### KT-INS-028 case 1: KotlinConstantConditions
settings: default
language: kotlin
```before
fun process(x: Int?) {
    if (x == null) return
    if (<caret>x != null) {
        println(x)
    }
}
```
keys: wait for highlighting; Alt+Enter, "Simplify expression", Enter
```after
mark [the condition or value] (warning, on by default): `Condition 'x != null' is always 'true'` / `Condition is always 'true'` / `Value is always null` / `'when' branch is never reachable` / `Cast will always fail` ...
fun process(x: Int?) {
    if (x == null) return
    println(x)
}
```

### KT-INS-029 case 1: KotlinUnreachableCode
settings: default
language: kotlin
```before
fun value(): Int {
    return 42
    <caret>print("Hello, world !")
}
```
keys: wait for highlighting; Alt+Enter, "Remove unreachable code", Enter
```after
mark [each unreachable statement] (warning, on by default): `Unreachable code`
fun value(): Int {
    return 42
}
```

### KT-INS-030 case 1: UseExpressionBody
settings: default
language: kotlin
```before
fun sign(x: Int): Int {
    <caret>return when {
        x < 0 -> -1
        x > 0 -> 1
        else -> 0
    }
}
```
keys: wait for highlighting; Alt+Enter, "Convert to expression body", Enter
```after
mark [the `{` of the body or the `return`] (information, on by default): `Use expression body instead of one-line 'return'` / `... of 'return when'` / `... of block body`
fun sign(x: Int): Int = when {
    x < 0 -> -1
    x > 0 -> 1
    else -> 0
}
```

### KT-INS-031 case 1: UsePropertyAccessSyntax
settings: default
language: kotlin
```before
fun test(f: java.io.File) = f.<caret>getName()
```
keys: wait for highlighting; Alt+Enter, "Use property access syntax", Enter
```after
mark [the call's name] (weak warning, on by default): `Use of getter method instead of property access syntax` / `Use of setter method instead of property access syntax`
fun test(f: java.io.File) = f.name
```

### KT-INS-032 case 1: ConvertToStringTemplate
settings: default
language: kotlin
```before
fun f(capital: String, country: String) {
    print(<caret>capital + " is a capital of " + country)
}
```
keys: wait for highlighting; Alt+Enter, "Convert concatenation to template", Enter
```after
mark [the whole concatenation] (weak warning, on by default): `'String' concatenation can be converted to a template`
fun f(capital: String, country: String) {
    print("$capital is a capital of $country")
}
```

### KT-INS-033 case 1: CascadeIf
settings: default
language: kotlin
```before
fun translate(n: Int): String {
    return <caret>if (n == 1) {
        "one"
    } else if (n == 2) {
        "two"
    } else {
        "???"
    }
}
```
keys: wait for highlighting; Alt+Enter, "Replace 'if' with 'when'", Enter
```after
mark [the first `if` keyword] (weak warning, on by default): `Cascade 'if' should be replaced with 'when'`
fun translate(n: Int): String {
    return when (n) {
        1 -> {
            "one"
        }
        2 -> {
            "two"
        }
        else -> {
            "???"
        }
    }
}
```

### KT-INS-034 case 1: IfThenToElvis
settings: default
language: kotlin
```before
fun maybeFoo(): String? = "foo"
fun f() {
    val foo = maybeFoo()
    val bar = <caret>if (foo == null) "hello" else foo
}
```
keys: wait for highlighting; Alt+Enter, "Replace 'if' expression with elvis expression", Enter
```after
mark [the `if` keyword] (weak warning, on by default): `If-Then foldable to '?:'`
fun maybeFoo(): String? = "foo"
fun f() {
    val foo = maybeFoo()
    val bar = foo ?: "hello"
}
```

### KT-INS-035 case 1: IfThenToSafeAccess
settings: default
language: kotlin
```before
fun bar(x: String) = ""
fun foo(a: String?) {
    <caret>if (a != null) bar(a) else null
}
```
keys: wait for highlighting; Alt+Enter, "Replace 'if' expression with safe access expression", Enter
```after
mark [the `if` keyword] (weak warning, on by default): `If-Then foldable to '?.'`
fun bar(x: String) = ""
fun foo(a: String?) {
    a?.let { bar(it) }
}
```

### KT-INS-036 case 1: FoldInitializerAndIfToElvis
settings: default
language: kotlin
```before
fun test(foo: Int?, bar: Int): Int {
    val i = foo
    <caret>if (i == null) {
        return bar
    }
    return i
}
```
keys: wait for highlighting; Alt+Enter, "Replace 'if' with elvis operator", Enter
```after
mark [the `if` keyword] (weak warning, on by default): `If-Null return/break/... foldable to '?:'`
fun test(foo: Int?, bar: Int): Int {
    val i = foo ?: return bar
    return i
}
```

### KT-INS-037 case 1: LiftReturnOrAssignment
settings: default
language: kotlin
```before
fun foo(arg: Int): String {
    <caret>when (arg) {
        0 -> return "Zero"
        1 -> return "One"
        else -> return "Multiple"
    }
}
```
keys: wait for highlighting; Alt+Enter, "Lift return out of 'when'", Enter
```after
mark [the `when`/`if`/`try` keyword] (weak warning, on by default): `'return' can be lifted out of 'when'` / `Assignment can be lifted out of 'if'`
fun foo(arg: Int): String {
    return when (arg) {
        0 -> "Zero"
        1 -> "One"
        else -> "Multiple"
    }
}
```

### KT-INS-038 case 1: JoinDeclarationAndAssignment
settings: default
language: kotlin
```before
fun f() {
    val <caret>x: String
    x = System.getProperty("a")
}
```
keys: wait for highlighting; Alt+Enter, "Join with initializer", Enter
```after
mark [the declaration name] (weak warning, on by default): `Join declaration and assignment`
fun f() {
    val x = System.getProperty("a")
}
```

### KT-INS-039 case 1: ReplaceSizeZeroCheckWithIsEmpty
settings: default
language: kotlin
```before
fun foo(list: List<Int>) = <caret>list.size == 0
```
keys: wait for highlighting; Alt+Enter, "Replace size zero check with 'isEmpty'", Enter
```after
mark [the comparison] (weak warning, on by default): `Size zero check can be replaced with 'isEmpty()'`
fun foo(list: List<Int>) = list.isEmpty()
```

### KT-INS-040 case 1: ReplaceSizeCheckWithIsNotEmpty
settings: default
language: kotlin
```before
fun foo(list: List<Int>) = <caret>list.size > 0
```
keys: wait for highlighting; Alt+Enter, "Replace size check with 'isNotEmpty'", Enter
```after
mark [the comparison] (weak warning, on by default): `Size check can be replaced with 'isNotEmpty()'`
fun foo(list: List<Int>) = list.isNotEmpty()
```

### KT-INS-041 case 1: MoveLambdaOutsideParentheses
settings: default
language: kotlin
```before
fun square(a: Int, b: (Int) -> Int) = b(a * a)
fun foo() = square(2, <caret>{ it })
```
keys: wait for highlighting; Alt+Enter, "Move lambda argument out of parentheses", Enter
```after
mark [the lambda] (weak warning, on by default): `Lambda argument inside parentheses`
fun square(a: Int, b: (Int) -> Int) = b(a * a)
fun foo() = square(2) { it }
```

### KT-INS-042 case 1: ReplaceGetOrSet
settings: default
language: kotlin
```before
class Test { operator fun get(i: Int): Int = 0 }
fun test() { Test().<caret>get(0) }
```
keys: wait for highlighting; Alt+Enter, "Replace get or set call with indexing operator", Enter
```after
mark [the call] (weak warning, on by default): `Explicit 'get' or 'set' call`
class Test { operator fun get(i: Int): Int = 0 }
fun test() { Test()[0] }
```

### KT-INS-043 case 1: ReplacePutWithAssignment
settings: default
language: kotlin
```before
fun foo(map: MutableMap<Int, String>) {
    map.<caret>put(42, "foo")
}
```
keys: wait for highlighting; Alt+Enter, "Convert put to assignment", Enter
```after
mark [the call] (weak warning, on by default): `map.put() should be converted to assignment`
fun foo(map: MutableMap<Int, String>) {
    map[42] = "foo"
}
```

### KT-INS-044 case 1: ClassName
settings: default
language: kotlin
```before
class <caret>foo
```
keys: wait for highlighting
```after
mark [the class name] (weak warning): `Class name 'foo' should start with an uppercase letter`
(no automatic fix)
```

### KT-INS-045 case 1: FunctionName
settings: default
language: kotlin
```before
fun <caret>Foo() {}
```
keys: wait for highlighting
```after
mark [the function name] (weak warning): `Function name 'Foo' should start with a lowercase letter`
(no automatic fix)
```

### KT-INS-046 case 1: PropertyName
settings: default
language: kotlin
```before
class A { val <caret>Foo = 1 }
```
keys: wait for highlighting
```after
mark [the property name] (weak warning): `Property name 'Foo' should start with a lowercase letter`
(no automatic fix)
```

### KT-INS-047 case 1: LocalVariableName
settings: default
language: kotlin
```before
fun f() { val <caret>Foo = 1 }
```
keys: wait for highlighting
```after
mark [the name] (weak warning): `Local variable name 'Foo' should start with a lowercase letter`
(no automatic fix)
```

### KT-INS-048 case 1: PackageDirectoryMismatch
settings: default
language: kotlin
```before
--- src/a/b/Foo.kt
package a.c<caret>
class Foo
```
keys: wait for highlighting; Alt+Enter, "Move file to package-matching directory", Enter
```after
mark [the package directive] (weak warning, on by default): `Package directive does not match the file location`
(either) src/a/c/Foo.kt with unchanged text, or src/a/b/Foo.kt with `package a.b`
```

### KT-INS-049 case 1: UnusedReceiverParameter
settings: default
language: kotlin
```before
fun <caret>String.foo(): Int = 1
```
keys: wait for highlighting; Alt+Enter, "Remove redundant receiver parameter", Enter
```after
mark [the receiver type] (warning, on by default): `Receiver parameter is never used`
fun foo(): Int = 1
```

### KT-INS-050 case 1: SelfAssignment
settings: default
language: kotlin
```before
fun test() {
    var bar = 1
    <caret>bar = bar
}
```
keys: wait for highlighting; Alt+Enter, "Remove self assignment", Enter
```after
mark [the assignment] (warning, on by default): `Variable 'bar' is assigned to itself`
fun test() {
    var bar = 1
}
```

### KT-INS-051 case 1: UselessCallOnNotNull
settings: default
language: kotlin
```before
fun test(s: String) {
    val x = s.<caret>orEmpty()
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant call", Enter
```after
mark [the call] (warning, on by default): `Redundant call on not-null type`
fun test(s: String) {
    val x = s
}
```

### KT-INS-052 case 1: RedundantReturnKeyword
settings: default
language: kotlin
```before
fun foo(flag: Boolean): String {
    return if (flag) {
        "foo"
    } else {
        <caret>return "bar"
    }
}
```
keys: wait for highlighting; Alt+Enter, "Remove 'return' keyword", Enter
```after
mark [the `return` keyword] (warning, on by default): `Redundant 'return' keyword`
fun foo(flag: Boolean): String {
    return if (flag) {
        "foo"
    } else {
        "bar"
    }
}
```

### KT-INS-053 case 1: RemoveRedundantBackticks
settings: default
language: kotlin
```before
val <caret>`a` = 1
```
keys: wait for highlighting; Alt+Enter, "Remove redundant backticks", Enter
```after
mark [the backticked name] (warning, on by default): `Redundant backticks`
val a = 1
```

### KT-INS-054 case 1: ReplaceToWithInfixForm
settings: default
language: kotlin
```before
fun foo(a: Int, b: Int) = a.<caret>to(b)
```
keys: wait for highlighting; Alt+Enter, "Replace 'to' with infix form", Enter
```after
mark [the call] (weak warning, on by default): `'to' call should be replaced with infix form`
fun foo(a: Int, b: Int) = a to b
```

### KT-INS-055 case 1: SimplifiableCallChain
settings: default
language: kotlin
```before
fun main() {
    listOf(1, 2, 3).<caret>filter { it > 1 }.count()
}
```
keys: wait for highlighting; Alt+Enter, "Merge call chain to 'count'", Enter
```after
mark [from the first to the last call of the chain] (weak warning, on by default): `Call chain on collection type can be simplified`
fun main() {
    listOf(1, 2, 3).count { it > 1 }
}
```

### KT-INS-056 case 1: ReplaceRangeToWithUntil
settings: default
language: kotlin
```before
fun foo(a: Int) {
    for (i in 0<caret>..a - 1) {
    }
}
```
keys: wait for highlighting; Alt+Enter, "Replace with 'until'", Enter
```after
mark [the operator] (weak warning, on by default): `'rangeTo' or the '..' call should be replaced with 'until'`
fun foo(a: Int) {
    for (i in 0 until a) {
    }
}
```

### KT-INS-057 case 1: DestructuringDeclaration
settings: default
language: kotlin
```before
data class My(val first: String, val second: Int)
fun foo(m: My) {
    val <caret>my = m
    println(my.first)
    println(my.second)
}
```
keys: wait for highlighting; Alt+Enter, "Use destructuring declaration", Enter
```after
mark [the declaration or lambda parameter] (weak warning, on by default): `Use destructuring declaration`
data class My(val first: String, val second: Int)
fun foo(m: My) {
    val (first, second) = m
    println(first)
    println(second)
}
```

### KT-INS-058 case 1: ConvertTwoComparisonsToRangeCheck
settings: default
language: kotlin
```before
fun checkMonth(month: Int): Boolean {
    return <caret>month >= 1 && month <= 12
}
```
keys: wait for highlighting; Alt+Enter, "Convert comparisons to 'in 1..12'", Enter
```after
mark [the `&&` expression] (weak warning, on by default): `Two comparisons should be converted to a range check`
fun checkMonth(month: Int): Boolean {
    return month in 1..12
}
```

### KT-INS-059 case 1: ReplaceWithOperatorAssignment
settings: default
language: kotlin
```before
fun foo() {
    var n = 1
    <caret>n = n + 4
}
```
keys: wait for highlighting; Alt+Enter, "Replace with operator-assignment", Enter
```after
mark [the assignment] (weak warning, on by default): `Replaceable with operator-assignment`
fun foo() {
    var n = 1
    n += 4
}
```

### KT-INS-060 case 1: UnnecessaryVariable
settings: default
language: kotlin
```before
fun sum(a: Int, b: Int): Int {
    val <caret>c = a + b
    return c
}
```
keys: wait for highlighting; Alt+Enter, "Inline variable", Enter
```after
mark [the variable name] (weak warning, on by default): `Variable used only in following return and can be inlined` / `Variable is same as 'x' and can be inlined`
fun sum(a: Int, b: Int): Int {
    return a + b
}
```

### KT-INS-061 case 1: NullableBooleanElvis
settings: default
language: kotlin
```before
fun check(a: Boolean? = null) {
    if (a <caret>?: false) throw IllegalStateException()
}
```
keys: wait for highlighting; Alt+Enter, "Replace with equality check", Enter
```after
mark [the elvis expression] (information, on by default): `Equality check should be used instead of elvis for nullable boolean check`
fun check(a: Boolean? = null) {
    if (a == true) throw IllegalStateException()
}
```

### KT-INS-062 case 1: RemoveToStringInStringTemplate
settings: default
language: kotlin
```before
fun foo(a: Int, b: Int) = a + b
fun test() = "Foo: ${foo(0, 4).<caret>toString()}"
```
keys: wait for highlighting; Alt+Enter, "Remove 'toString()' call", Enter
```after
mark [the call] (weak warning, on by default): `Redundant 'toString()' call in string template`
fun foo(a: Int, b: Int) = a + b
fun test() = "Foo: ${foo(0, 4)}"
```

### KT-INS-063 case 1: UselessCallOnCollection
settings: default
language: kotlin
```before
fun test(list: List<String>) {
    val x = list.<caret>filterNotNull()
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant call", Enter
```after
mark [the call] (warning, on by default): `Redundant call on collection type`
fun test(list: List<String>) {
    val x = list
}
```

### KT-INS-064 case 1: MayBeConstant
settings: default
language: kotlin
```before
object A {
    val <caret>foo = 1
}
```
keys: wait for highlighting; Alt+Enter, "Add 'const' modifier", Enter
```after
mark [the property name] (weak warning, on by default): `Might be 'const'`
object A {
    const val foo = 1
}
```

### KT-INS-065 case 1: ProtectedInFinal
settings: default
language: kotlin
```before
class FinalClass {
    <caret>protected fun foo() {}
}
```
keys: wait for highlighting; Alt+Enter, "Make private", Enter
```after
mark [the modifier] (weak warning, on by default): `'protected' visibility is effectively 'private' in a final class`
class FinalClass {
    private fun foo() {}
}
```

### KT-INS-066 case 1: AddOperatorModifier
settings: default
language: kotlin
```before
class Complex(val real: Double) {
    fun <caret>plus(other: Complex) = Complex(real + other.real)
}
```
keys: wait for highlighting; Alt+Enter, "Add 'operator' modifier", Enter
```after
mark [the function name] (information, on by default): `Function should have 'operator' modifier`
class Complex(val real: Double) {
    operator fun plus(other: Complex) = Complex(real + other.real)
}
```

### KT-INS-067 case 1: EqualsOrHashCode
settings: default
language: kotlin
```before
class C1 {
    override fun <caret>equals(other: Any?) = true
}
```
keys: wait for highlighting; Alt+Enter, "Generate 'hashCode()'", Enter
```after
mark [the name of the declared method] (warning, on by default): `'equals()' and 'hashCode()' not paired`
class C1 {
    override fun equals(other: Any?) = true
    override fun hashCode(): Int {
        return javaClass.hashCode()
    }
}
```

### KT-INS-068 case 1: StringReferentialEquality
settings: default
language: kotlin
```before
fun foo(s: String, t: String) = t <caret>=== s
```
keys: wait for highlighting; Alt+Enter, "Replace '===' with '=='", Enter
```after
mark [the operator] (warning, on by default): `String compared with referential equality`
fun foo(s: String, t: String) = t == s
```

### KT-INS-069 case 1: KotlinDoubleNegation
settings: default
language: kotlin
```before
val truth = <caret>!!true
```
keys: wait for highlighting; Alt+Enter, "Remove redundant negations", Enter
```after
mark [the negation] (weak warning, on by default): `Redundant double negation`
val truth = true
```

### KT-INS-070 case 1: RedundantIf
settings: default
language: kotlin
```before
fun test(foo: Boolean): Boolean {
    <caret>if (foo) {
        return true
    } else {
        return false
    }
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant 'if' statement", Enter
```after
mark [the `if` keyword] (weak warning, on by default): `Redundant 'if' statement`
fun test(foo: Boolean): Boolean {
    return foo
}
```

### KT-INS-071 case 1: EmptyRange
settings: default
language: kotlin
```before
val range = 2<caret>..1
```
keys: wait for highlighting; Alt+Enter, "Replace with 'downTo'", Enter
```after
mark [the range] (warning, on by default): `This range is empty. Did you mean to use 'downTo'?`
val range = 2 downTo 1
```

### KT-INS-072 case 1: SimplifyBooleanWithConstants
settings: default
language: kotlin
```before
fun use(arg: Boolean) {
    if (<caret>false == arg) {
    }
}
```
keys: wait for highlighting; Alt+Enter, "Simplify boolean expression", Enter
```after
mark [the sub-expression] (weak warning, on by default): `Boolean expression can be simplified`
fun use(arg: Boolean) {
    if (!arg) {
    }
}
```

### KT-INS-073 case 1: RedundantRequireNotNullCall
settings: default
language: kotlin
```before
fun foo(i: Int) {
    <caret>requireNotNull(i)
    println(i)
}
```
keys: wait for highlighting; Alt+Enter, "Remove 'requireNotNull' call", Enter
```after
mark [the call] (weak warning, on by default): `Redundant 'requireNotNull' or 'checkNotNull' call`
fun foo(i: Int) {
    println(i)
}
```

### KT-INS-074 case 1: UnusedExpression
settings: default
language: kotlin
```before
fun example(): Int {
    val x = 10
    <caret>x
}
```
keys: wait for highlighting; Alt+Enter, "Add 'return' to last expression", Enter
```after
mark [the expression] (warning, on by default): `Expression is unused`
fun example(): Int {
    val x = 10
    return x
}
```

### KT-INS-075 case 1: UnusedEquals
settings: default
language: kotlin
```before
fun t(a: Int) {
    <caret>a == 1
}
```
keys: wait for highlighting
```after
mark [the `==` expression] (warning): `Unused equals expression`
(no automatic fix)
```

### KT-INS-076 case 1: UnclearPrecedenceOfBinaryExpression
settings: default
language: kotlin
```before
fun foo(i: Int?) {
    val y = i <caret>?: 0 + 1
}
```
keys: wait for highlighting; Alt+Enter, "Add parentheses to clarify precedence", Enter
```after
mark [the operator] (warning, on by default): `Multiple operators with different precedence`
fun foo(i: Int?) {
    val y = i ?: (0 + 1)
}
```

### KT-INS-077 case 1: SortModifiers
settings: default
language: kotlin
```before
<caret>infix private fun Int.wrongOrder(expr: Int) {}
```
keys: wait for highlighting; Alt+Enter, "Sort modifiers", Enter
```after
mark [the modifier list] (weak warning, on by default): `Non-canonical modifiers order`
private infix fun Int.wrongOrder(expr: Int) {}
```

### KT-INS-078 case 1: ReplaceNegatedIsEmptyWithIsNotEmpty
settings: default
language: kotlin
```before
fun main() {
    val list = listOf(1, 2, 3)
    if (<caret>!list.isEmpty()) {
    }
}
```
keys: wait for highlighting; Alt+Enter, "Replace negated 'isEmpty' with 'isNotEmpty'", Enter
```after
mark [the negation] (weak warning, on by default): `Negated call can be simplified`
fun main() {
    val list = listOf(1, 2, 3)
    if (list.isNotEmpty()) {
    }
}
```

### KT-INS-079 case 1: ReplaceWithIgnoreCaseEquals
settings: default
language: kotlin
```before
fun main(a: String, b: String) = a.<caret>lowercase() == b.lowercase()
```
keys: wait for highlighting; Alt+Enter, "Replace with 'equals(..., ignoreCase = true)'", Enter
```after
mark [the comparison] (weak warning, on by default): `Should be replaced with 'equals(..., ignoreCase = true)'`
fun main(a: String, b: String) = a.equals(b, ignoreCase = true)
```

### KT-INS-080 case 1: MapGetWithNotNullAssertionOperator
settings: default
language: kotlin
```before
fun test(map: Map<Int, String>): String = map.get(0)<caret>!!
```
keys: wait for highlighting; Alt+Enter, "Replace with 'getValue' call", Enter
```after
mark [the `!!`] (information, on by default): `'map.get()' with not-null assertion operator '(!!)'`
fun test(map: Map<Int, String>): String = map.getValue(0)
```

### KT-INS-081 case 1: BooleanLiteralArgument
settings: default
language: kotlin
```before
fun check(checkName: Boolean, checkAddress: Boolean) {}
fun usage() {
    check(<caret>true, false)
}
```
keys: wait for highlighting; Alt+Enter, "Add names to call arguments", Enter
```after
mark [the literal] (weak warning, on by default): `Boolean literal argument without a parameter name`
fun check(checkName: Boolean, checkAddress: Boolean) {}
fun usage() {
    check(checkName = true, checkAddress = false)
}
```

### KT-INS-082 case 1: CanBeParameter
settings: default
language: kotlin
```before
class Task(<caret>val name: String) {
    init { print("Task created: $name") }
}
```
keys: wait for highlighting; Alt+Enter, "Remove 'val'", Enter
```after
mark [the `val`/`var` keyword] (warning, on by default): `Constructor parameter is never used as a property`
class Task(name: String) {
    init { print("Task created: $name") }
}
```

### KT-INS-083 case 1: CanBePrimaryConstructorProperty
settings: default
language: kotlin
```before
class User(name: String) {
    val <caret>name = name
}
```
keys: wait for highlighting; Alt+Enter, "Move to constructor", Enter
```after
mark [the property] (warning, on by default): `Property is explicitly assigned to parameter 'name', so it can be declared directly in the constructor`
class User(val name: String) {
}
```

### KT-INS-084 case 1: ConvertSecondaryConstructorToPrimary
settings: default
language: kotlin
```before
class User {
    val name: String
    <caret>constructor(name: String) {
        this.name = name
    }
}
```
keys: wait for highlighting; Alt+Enter, "Convert to primary constructor", Enter
```after
mark [the `constructor` keyword] (information, on by default): `Convert to primary constructor`
class User(val name: String) {
}
```

### KT-INS-085 case 1: ArrayInDataClass
settings: default
language: kotlin
```before
data class Text(val <caret>lines: Array<String>)
```
keys: wait for highlighting; Alt+Enter, "Generate equals() and hashCode()", Enter
```after
mark [the parameter] (warning, on by default): `Property with 'Array' type in a 'data class': it is recommended to override 'equals()' and 'hashCode()'`
data class Text(val lines: Array<String>) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as Text

        return lines.contentEquals(other.lines)
    }

    override fun hashCode(): Int {
        return lines.contentHashCode()
    }
}
```

### KT-INS-086 case 1: AddVarianceModifier
settings: default
language: kotlin
```before
class Box<<caret>T>(val obj: T)
```
keys: wait for highlighting; Alt+Enter, "Add 'out' variance", Enter
```after
mark [the type parameter] (information, on by default): `Type parameter can have 'out' variance` / `'in' variance`
class Box<out T>(val obj: T)
```

### KT-INS-087 case 1: SuspiciousCollectionReassignment
settings: default
language: kotlin
```before
fun test() {
    var list = <caret>listOf(0)
    list += 42
}
```
keys: wait for highlighting; Alt+Enter, "Change type to mutable", Enter
```after
mark [the operator] (weak warning, on by default): `'+=' on a read-only List creates a new List under the hood`
fun test() {
    val list = mutableListOf(0)
    list += 42
}
```

### KT-INS-088 case 1: RemoveRedundantSpreadOperator
settings: default
language: kotlin
```before
fun foo(vararg s: String) { }
fun bar() {
    foo(<caret>*arrayOf("abc"))
}
```
keys: wait for highlighting; Alt+Enter, "Remove redundant spread operator", Enter
```after
mark [the `*`] (warning, on by default): `Redundant spread operator`
fun foo(vararg s: String) { }
fun bar() {
    foo("abc")
}
```

### KT-INS-089 case 1: ReplaceMapGetOrDefault
settings: default
language: kotlin
```before
fun test(map: Map<Int, String>) = map.<caret>getOrDefault(1, "foo")
```
keys: wait for highlighting; Alt+Enter, "Replace with indexing and elvis operator", Enter
```after
mark [the call] (information, on by default): `'map.getOrDefault()' can be replaced with indexing and elvis operator`
fun test(map: Map<Int, String>) = map[1] ?: "foo"
```

### KT-INS-090 case 1: ConvertTryFinallyToUseCall
settings: default
language: kotlin
```before
fun example(r: java.io.Reader) {
    <caret>try {
        r.read()
    } finally {
        r.close()
    }
}
```
keys: wait for highlighting; Alt+Enter, "Convert try", Enter
```after
mark [the `try`] (weak warning, on by default): `Convert try / finally to use() call`
fun example(r: java.io.Reader) {
    r.use {
        r.read()
    }
}
```

### KT-INS-091 case 1: ReplaceWithEnumMap
settings: default
language: kotlin
```before
enum class E { A }
val m = <caret>HashMap<E, Int>()
```
keys: wait for highlighting; Alt+Enter, "Replace with 'EnumMap'", Enter
```after
mark [the constructor call] (warning, on by default): `'HashMap' can be replaced with 'EnumMap'`
enum class E { A }
val m = EnumMap<E, Int>(E::class.java)
```

### KT-INS-092 case 1: TrailingComma
settings: default
language: kotlin
```before
fun f(
    a: Int,
    b: Int<caret>
) {}
```
keys: wait for highlighting; Alt+Enter, "Add trailing comma", Enter
```after
mark [the last element] (weak warning, on by default): `Missing trailing comma` / `Redundant trailing comma` (per the code-style setting `Use trailing comma`)
fun f(
    a: Int,
    b: Int,
) {}
```

### KT-INS-093 case 1: MemberVisibilityCanBePrivate
settings: default
language: kotlin
```before
class Service(val <caret>url: String) {
    fun connect() = url.length
}
```
keys: wait for highlighting; Alt+Enter, "Make private", Enter
```after
mark [the name] (weak warning, off by default): `Property 'url' could be private` / `Function 'f' could be private`
class Service(private val url: String) {
    fun connect() = url.length
}
```

### KT-INS-094 case 1: ReplaceSubstringWithTake
settings: default
language: kotlin
```before
fun foo(s: String) = s.<caret>substring(0, 10)
```
keys: wait for highlighting; Alt+Enter, "Replace 'substring' call with 'take' call", Enter
```after
mark [the call] (information, on by default): `'substring' call should be replaced with 'take' call`
fun foo(s: String) = s.take(10)
```

### KT-INS-095 case 1: ConvertReferenceToLambda
settings: default
language: kotlin
```before
fun Int.isEven() = this % 2 == 0
fun example(numbers: List<Int>) = numbers.filter(<caret>Int::isEven)
```
keys: wait for highlighting; Alt+Enter, "Convert reference to lambda", Enter
```after
mark [the reference] (information, on by default): `Can be replaced with lambda`
fun Int.isEven() = this % 2 == 0
fun example(numbers: List<Int>) = numbers.filter { it.isEven() }
```

### KT-INS-096 case 1: ObjectLiteralToLambda
settings: default
language: kotlin
```before
fun foo(pool: java.util.concurrent.ExecutorService) {
    pool.submit(<caret>object : Runnable {
        override fun run() { println("hello") }
    })
}
```
keys: wait for highlighting; Alt+Enter, "Convert to lambda", Enter
```after
mark [the object literal] (weak warning, on by default): `Object literal can be converted to lambda`
fun foo(pool: java.util.concurrent.ExecutorService) {
    pool.submit { println("hello") }
}
```

### KT-INS-002 case 2: silent on unresolved premises
settings: default
language: kotlin
```before
fun f() {
    var <caret>x = unknownCall()
    println(x)
}
```
keys: wait for highlighting
```after
only the compiler error "Unresolved reference 'unknownCall'." ; no CanBeVal mark on "var"
```

### KT-INS-002 case 3: no CanBeVal when the variable is modified
settings: default
language: kotlin
```before
fun f() {
    var <caret>x = 1
    x = 2
    println(x)
}
```
keys: wait for highlighting
```after
no mark on "var"
```

### KT-INS-003 case 4: cleanup applies a local+cleanup fix over the whole file
settings: default
language: kotlin
```before
public fun a() {};
final class B { public val x = 1 }
```
keys: action CodeCleanup (Code | Code Cleanup) on the file
```after
fun a() {}
class B { val x = 1 }
```

### KT-INS-003 case 5: Fix all problems in file
settings: default
language: kotlin
```before
fun a(): Unit {}
fun b(): Unit<caret> {}
```
keys: Alt+Enter, Right on "Remove redundant 'Unit' return type", "Fix all 'Redundant 'Unit' return type' problems in file", Enter
```after
fun a() {}
fun b() {}
```

### KT-INS-006 case 6: unused private function marked grey with Safe delete
settings: default
language: kotlin
```before
class A {
    private fun <caret>helper() {}
    fun used() {}
}
```
keys: wait for highlighting; Alt+Enter, "Safe delete 'helper'", Enter
```after
mark "helper" (grey): Function 'helper' is never used
class A {
    fun used() {}
}
```

### KT-INS-006 case 7: used private function is not marked
settings: default
language: kotlin
```before
class A {
    private fun helper() {}
    fun used() { helper() }
}
```
keys: wait for highlighting
```after
no mark
```

### KT-INS-006 case 8: public member used from another file is not marked
settings: default
language: kotlin
```before
--- src/a/A.kt
package a
class A { fun api() {} }
--- src/a/Use.kt
package a
fun t(a: A) { a.api() }
```
keys: wait for highlighting
```after
no mark on "api"
```

### KT-INS-006 case 9: unused parameter of a private function
settings: default
language: kotlin
```before
private fun f(<caret>unusedParam: Int) {}
fun g() { f(1) }
```
keys: wait for highlighting; Alt+Enter, "Safe delete parameter 'unusedParam'", Enter
```after
mark "unusedParam" (grey): Parameter 'unusedParam' is never used
private fun f() {}
fun g() { f() }
```

### KT-INS-006 case 10: override parameters and operator functions are never reported
settings: default
language: kotlin
```before
interface I { fun f(x: Int) }
class C : I {
    override fun f(x: Int) {}
    operator fun plus(other: C) = this
}
```
keys: wait for highlighting
```after
no mark on "x" (override) and none on "plus"
```

### KT-INS-006 case 11: main is an entry point
settings: default
language: kotlin
```before
fun main(args: Array<String>) {}
```
keys: wait for highlighting
```after
no mark on "main" and none on "args" (see UnusedMainParameter in Appendix A: separate inspection)
```

