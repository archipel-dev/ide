# Cases for 06-change-signature.md

Notation as in `01-framework.cases.md`. In `keys`, "table row" means the row of the Parameters
table; cells are edited by selecting them and typing; `Refactor` is the default button.

### REF-SIG-001 case 1: caret in a parameter list
settings: default
language: java
```before
class A {
    void f(int a, <caret>String b) {}
}
```
keys: Ctrl+F6
```after
dialog title: Change Signature
Name: f     Return type: void     visibility: package local
Parameters: int a | String b
Signature Preview: void f(int a, String b)
```

### REF-SIG-001 case 2: nothing suitable at the caret
settings: default
language: java
```before
class A {
    int x<caret> = 1;
}
```
keys: Ctrl+F6
```after
editor error hint:
Cannot perform refactoring.
Caret should be positioned at the name of method or class to be refactored
```

### REF-SIG-001 case 3: class without type parameters
settings: default
language: java
```before
class A<caret> {}
```
keys: Ctrl+F6
```after
editor error hint:
Cannot perform refactoring.
Class cannot have type parameters
```

### REF-SIG-002 case 1: overriding method asks about the base
settings: Editor | General | Refactorings: In modal dialogs
language: java
```before
class Base { void run(int a) {} }
class Sub extends Base {
    @Override void ru<caret>n(int a) {}
}
```
keys: Ctrl+F6
```after
dialog title: Super Method Found
text: Method Sub.run(int)
overrides method of class Base.

Do you want to refactor the base method instead?
buttons: [Base Method] [Current Method] [Cancel]
```

### REF-SIG-010 case 1: dialog for a constructor
settings: default
language: java
```before
class A {
    A<caret>(int x) {}
}
```
keys: Ctrl+F6
```after
(no Name and no Return type fields: constructors keep the class name)
visibility: package local
Parameters: int x
Method calls: (o) Modify  ( ) Delegate via overloading method
Signature Preview: A(int x)
```

### REF-SIG-011 case 1: validation errors
settings: default
language: java
```before
class A {
    int f<caret>(int a) { return a; }
}
```
keys: Ctrl+F6, then each edit followed by Refactor
```after
clear Return type                    -> error: Return type is not specified
Return type "Foo<"                   -> error: Wrong return type: 'Foo<'
add row with empty Type, Name "b"    -> error: Specify parameter type for parameter 'b'
add row Type "int", Name "1x"        -> error: Wrong name: 1x
add row Type "int...", Name "v" before another row -> error: Vararg parameter should be the last in method signature
Return type "Unknown"                -> confirmation: Return type 'Unknown' cannot be resolved.
                                         Continue?   [Yes] [No]
```

### REF-SIG-011 case 2: exceptions table
settings: default
language: java
```before
class A {
    void f<caret>() {}
}
```
keys: Ctrl+F6, add a row to Exceptions with type "String", Refactor
```after
error: Wrong type 'String' for exception, should extend java.lang.Throwable
```

### REF-SIG-011 case 3: new parameter without a default value while callers exist
settings: default
language: java
```before
class A {
    void f<caret>(int a) {}
    void g() { f(1); }
}
```
keys: Ctrl+F6, add row Type "int", Name "b", leave Default Value empty
```after
inline warning at the dialog bottom: Default value is missing. Method calls will contain blanks instead of the new parameter value.
```

### REF-SIG-012 case 1: default value is inserted at call sites
settings: default
language: java
```before
class A {
    void f<caret>(int a) {}
    void g() { f(1); f(2); }
}
```
keys: Ctrl+F6, add row Type "String", Name "tag", Default Value `"x"`, Refactor
```after
class A {
    void f(int a, String tag) {}
    void g() { f(1, "x"); f(2, "x"); }
}
```

### REF-SIG-012 case 2: Use Any Var picks the matching variable
settings: default
language: java
```before
class A {
    void f<caret>(int a) {}
    void g(String name) { f(1); }
    void h() { f(2); }
}
```
keys: Ctrl+F6, add row Type "String", Name "tag", tick "Use Any Var" in its detail pane, Refactor
```after
class A {
    void f(int a, String tag) {}
    void g(String name) { f(1, name); }
    void h() { f(2, ); }
}
(in h no single String variable exists: the argument slot stays blank)
```

### REF-SIG-012 case 3: the default-value chooser
settings: default
language: java
```before
class A {
    void f<caret>(int a) {}
    void g() { f(1); }
}
```
keys: Ctrl+F6, add row Type "int", Name "b", leave Default Value empty and the inline option untouched, Refactor
```after
dialog title: Default value for parameter "b" needed
choices: ( ) Leave blank
         (o) Feel lucky     (In calls to this method, if exactly one variable of the correct type is found, use it. Otherwise leave blank.)
         ( ) Use selected value
```

### REF-SIG-013 case 1: rename, retype and reorder
settings: default
language: java
```before
class A {
    int f<caret>(int a, String b) { return a + b.length(); }
    int g() { return f(1, "xy"); }
}
```
keys: Ctrl+F6, Name "calc", move row b above row a, rename a to "n", Refactor
```after
class A {
    int calc(String b, int n) { return n + b.length(); }
    int g() { return calc("xy", 1); }
}
```

### REF-SIG-013 case 2: return type and visibility
settings: default
language: java
```before
class A {
    int f<caret>() { return 1; }
    long g() { return f(); }
}
```
keys: Ctrl+F6, Return type "long", visibility "public", Refactor
```after
class A {
    public long f() { return 1; }
    long g() { return f(); }
}
```

### REF-SIG-013 case 3: remove a parameter that is unused
settings: default
language: java
```before
class A {
    void f<caret>(int a, int b) { System.out.println(a); }
    void g() { f(1, 2); }
}
```
keys: Ctrl+F6, select row b, press remove, Refactor
```after
class A {
    void f(int a) { System.out.println(a); }
    void g() { f(1); }
}
```

### REF-SIG-013 case 4: javadoc follows
settings: default
language: java
```before
class A {
    /**
     * @param a first
     * @param b second
     */
    void f<caret>(int a, int b) {}
}
```
keys: Ctrl+F6, rename b to "count", add row Type "int", Name "c", default "0", Refactor
```after
class A {
    /**
     * @param a first
     * @param count second
     * @param c
     */
    void f(int a, int count, int c) {}
}
```

### REF-SIG-014 case 1: overriders and callers in other files
settings: default
language: java
```before
--- Base.java
class Base { void run<caret>(int a) {} }
--- Sub.java
class Sub extends Base { @Override void run(int a) {} }
--- U.java
class U { void g(Base b, Sub s) { b.run(1); s.run(2); } }
```
keys: Ctrl+F6 on Base.run, add row Type "boolean", Name "fast", Default Value "false", Refactor
```after
--- Base.java
class Base { void run(int a, boolean fast) {} }
--- Sub.java
class Sub extends Base { @Override void run(int a, boolean fast) {} }
--- U.java
class U { void g(Base b, Sub s) { b.run(1, false); s.run(2, false); } }
```

### REF-SIG-014 case 2: method reference that no longer matches becomes a lambda
settings: default
language: java
```before
import java.util.function.IntConsumer;
class A {
    void f<caret>(int a) {}
    IntConsumer c = this::f;
}
```
keys: Ctrl+F6, add row Type "int", Name "b", Default Value "0", Refactor, Continue in the conflicts dialog (message: Method is used in one or more method references. Proceeding will convert these method references to lambda expressions.)
```after
import java.util.function.IntConsumer;
class A {
    void f(int a, int b) {}
    IntConsumer c = a -> this.f(a, 0);
}
```

### REF-SIG-014 case 3: varargs position
settings: default
language: java
```before
class A {
    void f<caret>(String... rest) {}
    void g() { f("a", "b"); }
}
```
keys: Ctrl+F6, insert a row Type "int", Name "n", Default Value "1" before rest, Refactor
```after
class A {
    void f(int n, String... rest) {}
    void g() { f(1, "a", "b"); }
}
```

### REF-SIG-014 case 4: lambda implementing a changed interface method
settings: default
language: java
```before
interface Op { int apply<caret>(int a); }
class U { Op op = x -> x + 1; }
```
keys: Ctrl+F6, add row Type "int", Name "b", Default Value "0", Refactor
```after
interface Op { int apply(int a, int b); }
class U { Op op = (x, b) -> x + 1; }
```

### REF-SIG-015 case 1: delegate via overloading method
settings: default
language: java
```before
class A {
    void f<caret>(int a) {}
    void g() { f(1); }
}
```
keys: Ctrl+F6, add row Type "int", Name "b", Default Value "5", select "Delegate via overloading method", Refactor
```after
class A {
    void f(int a) { f(a, 5); }
    void f(int a, int b) {}
    void g() { f(1); }
}
```

### REF-SIG-016 case 1: propagate a new parameter to a caller
settings: default
language: java
```before
class A {
    void f<caret>() {}
    void g() { f(); }
    void h() { g(); }
}
```
keys: Ctrl+F6, add row Type "int", Name "n", Default Value "0", press "Propagate Parameters..." (Alt+G); dialog "Select Methods To Propagate New Parameters" lists tree g -> h; tick g, OK, Refactor
```after
class A {
    void f(int n) {}
    void g(int n) { f(n); }
    void h() { g(0); }
}
```

### REF-SIG-020 case 1: removed parameter still used
settings: default
language: java
```before
class A {
    void f<caret>(int a, int b) { System.out.println(b); }
}
```
keys: Ctrl+F6, remove row b, Refactor
```after
Problems Detected dialog body (shape):
<parameter b> is used in method body
```

### REF-SIG-020 case 2: new parameter collides with a local
settings: default
language: java
```before
class A {
    void f<caret>(int a) { int b = 1; System.out.println(b); }
}
```
keys: Ctrl+F6, add row Type "int", Name "b", Default "0", Refactor
```after
Problems Detected dialog body (shape):
There is already a <local variable b> in the <method f(int)>. It will conflict with the new parameter.
```

### REF-SIG-041 case 1: constructor callers and subclass super calls
settings: default
language: java
```before
class A {
    A<caret>(int x) {}
}
class B extends A { B() { super(1); } }
class U { A a = new A(2); }
```
keys: Ctrl+F6, add row Type "String", Name "s", Default Value `""`, Refactor
```after
class A {
    A(int x, String s) {}
}
class B extends A { B() { super(1, ""); } }
class U { A a = new A(2, ""); }
```

### REF-SIG-042 case 1: record component added
settings: default (language level 21)
language: java
```before
record P(int x<caret>) {}
class U { P p = new P(1); }
```
keys: Ctrl+F6 (caret in the record header), add row Type "int", Name "y", Default Value "0", Refactor
```after
record P(int x, int y) {}
class U { P p = new P(1, 0); }
```

### REF-SIG-050 case 1: Kotlin add a parameter with a default value at call sites
settings: default
language: kotlin
```before
fun f<caret>(a: Int) {}
fun g() { f(1) }
```
keys: Ctrl+F6, add parameter Name "b", Type "String", Default Value `"x"`, Refactor
```after
fun f(a: Int, b: String) {}
fun g() { f(1, "x") }
```

### REF-SIG-050 case 2: Kotlin introduce default value keeps call sites
settings: default
language: kotlin
```before
fun f<caret>(a: Int) {}
fun g() { f(1) }
```
keys: Ctrl+F6, add parameter Name "b", Type "String", Default Value `"x"`, tick "Introduce default value", Refactor
```after
fun f(a: Int, b: String = "x") {}
fun g() { f(1) }
```

### REF-SIG-050 case 3: Kotlin reorder uses named arguments
settings: default
language: kotlin
```before
fun f<caret>(a: Int, b: String, c: Boolean = false) {}
fun g() { f(1, "x") }
```
keys: Ctrl+F6, move b before a, Refactor
```after
fun f(b: String, a: Int, c: Boolean = false) {}
fun g() { f("x", 1) }
```

### REF-SIG-051 case 1: Kotlin dynamic and vararg errors
settings: default
language: kotlin
```before
fun f<caret>(vararg xs: Int) {}
```
keys: Ctrl+F6
```after
editor error hint (message line): Can't refactor the function with variable arguments
```
