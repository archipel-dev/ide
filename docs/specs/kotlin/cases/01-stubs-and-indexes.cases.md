# Cases for 01-psi-stubs-and-indexes

Index cases use `keys: query <Index> <key>`; the `after` block lists the result as `path:name` lines sorted by path then
offset (Archipel rule, Q-STUB-2). `\n` inside a key is a real line feed.

### KT-STUB-010 case 1: class short name index
settings: default
language: kotlin
```before
--- src/a/Foo.kt
package a
class Foo
interface Api
object Single
fun local() { class Inner; val o = object : Api {} }
--- src/b/Foo.kt
package b
class Foo
```
keys: query ClassShortName Foo
```after
src/a/Foo.kt:Foo
src/b/Foo.kt:Foo
```

### KT-STUB-010 case 2: every interface also contributes DefaultImpls
settings: default
language: kotlin
```before
--- src/a/Api.kt
package a
interface Api
```
keys: query ClassShortName DefaultImpls
```after
src/a/Api.kt:Api
```

### KT-STUB-011 case 3: local declarations are not indexed
settings: default
language: kotlin
```before
--- src/a/A.kt
package a
fun outer() {
    fun helper() {}
    class Inner
}
```
keys: query FunctionShortName helper; query ClassShortName Inner
```after
FunctionShortName helper: (empty)
ClassShortName Inner: (empty)
```

### KT-STUB-010 case 4: super class index uses written simple names
settings: default
language: kotlin
```before
--- src/a/A.kt
package a
open class Base
class Child : Base()
enum class Color { RED }
annotation class Marker
object Obj : Base()
val lit = object : Base() {}
```
keys: query SuperClass Base; query SuperClass Enum; query SuperClass Annotation
```after
SuperClass Base: src/a/A.kt:Child, src/a/A.kt:Obj
SuperClass Enum: src/a/A.kt:Color
SuperClass Annotation: src/a/A.kt:Marker
```

### KT-STUB-010 case 5: subclass object index (objects with a super type, not literals)
settings: default
language: kotlin
```before
--- src/a/A.kt
package a
interface Factory { fun make(): String }
object DefaultFactory : Factory { override fun make() = "" }
object Plain
val lit = object : Factory { override fun make() = "" }
```
keys: query SubclassObjectName DefaultFactory; query SubclassObjectName Plain; query SubclassObjectName lit
```after
SubclassObjectName DefaultFactory: src/a/A.kt:DefaultFactory
SubclassObjectName Plain: (empty)
SubclassObjectName lit: (empty)
```

### KT-STUB-010 case 6: property index includes constructor val/var parameters
settings: default
language: kotlin
```before
--- src/a/A.kt
package a
class P(val id: Int, name: String, var tag: String)
val top = 1
```
keys: query PropertyShortName id; query PropertyShortName name; query PropertyShortName tag; query PropertyShortName top
```after
PropertyShortName id: src/a/A.kt:id
PropertyShortName name: (empty)
PropertyShortName tag: src/a/A.kt:tag
PropertyShortName top: src/a/A.kt:top
```

### KT-STUB-020 case 7: prime symbol names
settings: default
language: kotlin
```before
--- src/a/A.kt
package a
private fun hidden() {}
fun visible() {}
object O { fun inObject() {}; override fun toString() = "" }
class C { fun member() {}; companion object { val inCompanion = 1 } }
```
keys: query PrimeSymbolName hidden; query PrimeSymbolName visible; query PrimeSymbolName inObject; query PrimeSymbolName toString; query PrimeSymbolName member; query PrimeSymbolName inCompanion
```after
PrimeSymbolName hidden: (empty)
PrimeSymbolName visible: src/a/A.kt:visible
PrimeSymbolName inObject: src/a/A.kt:inObject
PrimeSymbolName toString: (empty)
PrimeSymbolName member: (empty)
PrimeSymbolName inCompanion: src/a/A.kt:inCompanion
```

### KT-STUB-021 case 8: extension receiver keys
settings: default
language: kotlin
```before
--- src/a/Ext.kt
package a
fun String?.nul() {}
fun <T> T.anyExt() {}
fun <T : CharSequence> T.bounded() {}
fun ((Int) -> Unit).onFn() {}
fun Int.(String) -> Unit.recv() {}
fun Outer.Inner.nested() {}
val List<String>.second get() = this[1]
```
keys: query TopLevelExtensionsByReceiverType "String\nnul"; "Any\nanyExt"; "CharSequence\nbounded"; "Function1\nonFn"; "Function2\nrecv"; "Inner\nnested"; "List\nsecond"
```after
String\nnul: src/a/Ext.kt:nul
Any\nanyExt: src/a/Ext.kt:anyExt
CharSequence\nbounded: src/a/Ext.kt:bounded
Function1\nonFn: src/a/Ext.kt:onFn
Function2\nrecv: src/a/Ext.kt:recv
Inner\nnested: src/a/Ext.kt:nested
List\nsecond: src/a/Ext.kt:second
```

### KT-STUB-021 case 9: import alias contributes the original name too
settings: default
language: kotlin
```before
--- src/a/Ext.kt
package a
import java.util.ArrayList as AL
fun AL<String>.sortedFirst() {}
```
keys: query TopLevelExtensionsByReceiverType "AL\nsortedFirst"; "ArrayList\nsortedFirst"
```after
AL\nsortedFirst: src/a/Ext.kt:sortedFirst
ArrayList\nsortedFirst: src/a/Ext.kt:sortedFirst
```

### KT-STUB-022 case 10: receiver supertypes are queried, siblings are not
settings: default
language: kotlin
```before
--- src/a/Ext.kt
package a
fun Iterable<*>.onIterable() {}
fun Collection<*>.onCollection() {}
fun Set<*>.onSet() {}
fun Any.onAny() {}
--- src/a/Use.kt
package a
fun use(x: ArrayList<String>) {
    x.on<caret>
}
```
keys: action CodeCompletion (Ctrl+Space), show candidates whose name starts with "on"
```after
onAny, onCollection, onIterable
```

### KT-STUB-030 case 11: facade names
settings: default
language: kotlin
```before
--- src/p/Foo.kt
package p
fun a() {}
--- src/p/my-file.kt
package p
fun b() {}
--- src/p/Named.kt
@file:JvmName("Util")
package p
fun c() {}
--- src/p/OnlyClass.kt
package p
class K
--- src/p/Alias.kt
package p
typealias S = String
```
keys: query FileFacadeFqName for every file
```after
src/p/Foo.kt: p.FooKt
src/p/my-file.kt: p.My_fileKt
src/p/Named.kt: p.Util
src/p/OnlyClass.kt: (none)
src/p/Alias.kt: (none)
```

### KT-STUB-030 case 12: multifile class combines files into one facade
settings: default
language: kotlin
```before
--- src/p/A.kt
@file:JvmName("Util")
@file:JvmMultifileClass
package p
fun a() {}
--- src/p/B.kt
@file:JvmName("Util")
@file:JvmMultifileClass
package p
fun b() {}
```
keys: query FileFacadeFqName for every file; query MultiFileClassPart p.Util__AKt
```after
src/p/A.kt: p.Util
src/p/B.kt: p.Util
MultiFileClassPart p.Util__AKt: src/p/A.kt
```

### KT-STUB-031 case 13: facade members seen from Java
settings: default
language: kotlin
```before
--- src/p/Foo.kt
package p
val x = 1
var y = 2
const val C = 3
@JvmField val f = 4
fun fn(a: Int) {}
--- src/p/Use.java
package p;
class Use { void m() { FooKt.<caret> } }
```
keys: action CodeCompletion (Ctrl+Space)
```after
C, f, fn(int a), getX(), getY(), setY(int y)
```

### KT-STUB-060 case 14: pending reference during indexing
settings: dumb_mode=true
language: kotlin
```before
fun f() { unknownName() }
```
keys: wait for highlighting; hover the identifier
```after
mark: pending reference (not error), tooltip: "Not resolved until the project is fully loaded"
```

### KT-STUB-060 case 15: after indexing the real error appears
settings: dumb_mode=false
language: kotlin
```before
fun f() { unknownName() }
```
keys: wait for highlighting; hover the identifier
```after
mark: error on "unknownName", tooltip: "Unresolved reference 'unknownName'."
```
