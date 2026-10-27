# Cases for 07-java-interop

### KT-JVM-001 case 1: members of a facade, accessors and companion seen from Java
settings: default
language: java
```before
--- src/p/Foo.kt
package p
val x = 1
var y = 2
fun f() {}
class A { companion object { @JvmStatic fun s() {}; fun n() {} } }
object O { fun g() {} }
--- src/p/Use.java
package p;
class Use { void m() { FooKt.getX(); FooKt.setY(1); FooKt.f(); A.s(); A.Companion.n(); O.INSTANCE.g(); } }
```
keys: wait for highlighting
```after
no errors
```

### KT-JVM-001 case 2: non-static companion member called statically
settings: default
language: java
```before
--- src/p/A.kt
package p
class A { companion object { fun n() {} } }
--- src/p/Use.java
package p;
class Use { void m() { A.n(); } }
```
keys: wait for highlighting
```after
error on "n": Cannot resolve method 'n()'  [the member is not a static method of A]
```

### KT-JVM-002 case 3: @JvmName changes the Java name
settings: default
language: java
```before
--- src/p/Foo.kt
@file:JvmName("Util")
package p
@JvmName("greet") fun hello() {}
--- src/p/Use.java
package p;
class Use { void m() { Util.<caret> } }
```
keys: Ctrl+Space
```after
list contains: greet()   ; does not contain: hello(), FooKt
```

### KT-JVM-002 case 4: @JvmOverloads generates overloads
settings: default
language: java
```before
--- src/p/Foo.kt
package p
@JvmOverloads fun f(a: Int, b: Int = 1, c: Int = 2) {}
--- src/p/Use.java
package p;
class Use { void m() { FooKt.<caret> } }
```
keys: Ctrl+Space
```after
list contains: f(int a), f(int a, int b), f(int a, int b, int c)
```

### KT-JVM-010 case 5: Ctrl+B from Java to Kotlin accessor
settings: default
language: java
```before
--- src/p/A.kt
package p
class A { val name = "" }
--- src/p/Use.java
package p;
class Use { String m(A a) { return a.get<caret>Name(); } }
```
keys: action GotoDeclaration
```after
caret in src/p/A.kt at "val name"
```

### KT-JVM-010 case 6: Ctrl+B from Kotlin to a Java getter through a synthetic property
settings: default
language: kotlin
```before
--- src/p/J.java
package p;
public class J { public String getTitle() { return ""; } }
--- src/p/Use.kt
package p
fun t(j: J) = j.ti<caret>tle
```
keys: action GotoDeclaration
```after
caret in src/p/J.java at "getTitle"
```

### KT-JVM-011 case 7: implementation gutter
settings: default
language: kotlin
```before
interface I { fun f() }
class B : I { override fun f() {} }
```
keys: hover the gutter icons
```after
on "fun f()" of I: "Is implemented in" (single) ; on "override fun f()": "Implements function in"
```

### KT-JVM-020 case 8: usages of a Kotlin property include Java getter calls
settings: default
language: kotlin
```before
--- src/p/A.kt
package p
class A { var <caret>name = "" }
--- src/p/Use.java
package p;
class Use { void m(A a) { a.setName(a.getName()); } }
--- src/p/Use.kt
package p
fun t(a: A) { a.name = a.name }
```
keys: action FindUsages
```after
Usages grouped: Use.java: "a.setName(...)" and "a.getName()" ; Use.kt: two property accesses (read and write)
dialog checkboxes include: Readers, Writers
```

### KT-JVM-020 case 9: usages of a top-level function from Java
settings: default
language: kotlin
```before
--- src/p/Foo.kt
package p
fun <caret>f() {}
--- src/p/Use.java
package p;
class Use { void m() { FooKt.f(); } }
```
keys: action FindUsages
```after
one usage: Use.java "FooKt.f()"
```

### KT-JVM-040 case 10: platform types
settings: default
language: kotlin
```before
--- src/p/J.java
package p;
public class J { public static String get() { return null; } }
--- src/p/Use.kt
package p
val a: String = J.get()
val b: String? = J.get()
val c = J.get()
```
keys: wait for highlighting; hover "c"
```after
no errors on a, b; type of c shown as "String!"
```

### KT-JVM-040 case 11: nullability annotations
settings: default
language: kotlin
```before
--- src/p/J.java
package p;
import org.jetbrains.annotations.Nullable;
public class J { @Nullable public static String get() { return null; } }
--- src/p/Use.kt
package p
val a: String = J.get()
```
keys: wait for highlighting
```after
error on "J.get()": Initializer type mismatch: expected 'String', actual 'String?'.
```

### KT-JVM-050 case 12: Kotlin extension in Java completion
settings: default
language: java
```before
--- src/p/E.kt
package p
fun String.shout() = uppercase()
--- src/p/Use.java
package p;
class Use { String m(String s) { return s.sho<caret> } }
```
keys: Ctrl+Space, Enter
```after
return EKt.shout(s)   (with the tail text "[Kotlin Extension]" in the list)
```
