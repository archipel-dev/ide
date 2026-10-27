# Cases for 05-inline.md

Notation as in `01-framework.cases.md`.

### REF-INL-001 case 1: nothing to inline
settings: default
language: java
```before
class A { // com<caret>ment
}
```
keys: Ctrl+Alt+N
```after
editor error hint:
Cannot perform refactoring.
Caret should be positioned at the name of element to be refactored
```

### REF-INL-010 case 1: popup with two choices
settings: default
language: java
```before
class A {
    int f(int a) {
        int t = a * 2;
        return t + <caret>t;
    }
}
```
keys: Ctrl+Alt+N
```after
popup title: Inline Variable
rows: This reference only
      All 2 references and remove the variable
first row selected; selecting a row highlights its references
```

### REF-INL-010 case 2: all references
settings: default
language: java
```before
class A {
    int f(int a) {
        int t = a * 2;
        return t + <caret>t;
    }
}
```
keys: Ctrl+Alt+N, choose "All 2 references and remove the variable"
```after
class A {
    int f(int a) {
        return (a * 2) + (a * 2);
    }
}
```

### REF-INL-010 case 3: this reference only
settings: default
language: java
```before
class A {
    int f(int a) {
        int t = a * 2;
        return t + <caret>t;
    }
}
```
keys: Ctrl+Alt+N, choose "This reference only"
```after
class A {
    int f(int a) {
        int t = a * 2;
        return t + (a * 2);
    }
}
```

### REF-INL-010 case 4: popup disabled by the setting
settings: Editor | General: Show inline dialog for local variables = off
language: java
```before
class A {
    int f(int a) {
        int t = a * 2;
        return t + <caret>t;
    }
}
```
keys: Ctrl+Alt+N
```after
class A {
    int f(int a) {
        return (a * 2) + (a * 2);
    }
}
```

### REF-INL-010 case 5: no usages
settings: default
language: java
```before
class A { void f() { int un<caret>used = 1; } }
```
keys: Ctrl+Alt+N
```after
editor error hint: Variable unused is never used
```

### REF-INL-011 case 1: invoked on the declaration
settings: default
language: java
```before
class A {
    String f() {
        String s<caret> = "a" + "b";
        return s.trim();
    }
}
```
keys: Ctrl+Alt+N
```after
class A {
    String f() {
        return ("a" + "b").trim();
    }
}
```

### REF-INL-011 case 2: a declaration with several variables keeps the others
settings: default
language: java
```before
class A {
    int f() {
        int a<caret> = 1, b = 2;
        return a + b;
    }
}
```
keys: Ctrl+Alt+N
```after
class A {
    int f() {
        int b = 2;
        return 1 + b;
    }
}
```

### REF-INL-012 case 1: error texts
settings: default
language: java
```before
class A {
    void f() {
        int x<caret>;
        int y = 1;
    }
}
```
keys: Ctrl+Alt+N on x (declaration without initializer)
```after
editor error hint:
Cannot perform refactoring.
Variable x has no initializer
```

### REF-INL-013 case 1: unsafe inline popup
settings: default
language: java
```before
class A {
    int f() {
        int a = 1;
        int t<caret> = a + 1;
        a = 5;
        return t;
    }
}
```
keys: Ctrl+Alt+N
```after
popup title: Unsafe Inline: Variables Used in Initializer Are Updated
rows: Highlight 1 conflicting write
      Ignore writes and continue
(choosing the first highlights "a = 5;" and changes nothing)
```

### REF-INL-014 case 1: variable that only aliases another variable
settings: default
language: java
```before
class A {
    int f(int n) {
        int m = n;
        int t<caret> = m;
        return t + t;
    }
}
```
keys: Ctrl+Alt+N (declaration), choose "Keep 't' name"
```after
class A {
    int f(int n) {
        int t = n;
        return t + t;
    }
}
(the other row, "Keep 'm' name", gives: return m + m; with t removed)
```

### REF-INL-020 case 1: inline method dialog on a call
settings: default
language: java
```before
class A {
    int twice(int x) { return x * 2; }
    int f() { return tw<caret>ice(3) + twice(4); }
}
```
keys: Ctrl+Alt+N
```after
dialog title: Inline Method
label: Inline method 'A.twice(int)' with 2 usages:
(o) Inline all usages, remove the method
( ) Inline all usages, keep the method
( ) Inline this usage only, keep the method
[ ] Search in comments and strings   [ ] Search for text occurrences
buttons: [Refactor] [Preview] [Cancel]
```

### REF-INL-020 case 2: recursive method forces this usage
settings: default
language: java
```before
class A {
    int fact(int n) { return n <= 1 ? 1 : n * fact(n - 1); }
    int g() { return fa<caret>ct(3); }
}
```
keys: Ctrl+Alt+N
```after
dialog radios: ( ) Inline all usages, remove the method [disabled]
               ( ) Inline all usages, keep the method [disabled]
               (o) Inline this usage only, keep the method
```

### REF-INL-021 case 1: abstract method
settings: default
language: java
```before
abstract class A {
    abstract int f();
    int g() { return fo<caret>o(); }
    abstract int foo();
}
```
keys: Ctrl+Alt+N on foo
```after
editor error hint:
Cannot perform refactoring.
Inline Method refactoring cannot inline abstract methods
```

### REF-INL-022 case 1: all usages and remove
settings: default
language: java
```before
class A {
    int twice(int x) { return x * 2; }
    int f(int a) { return tw<caret>ice(a) + twice(4); }
}
```
keys: Ctrl+Alt+N, "Inline all usages, remove the method", Refactor
```after
class A {
    int f(int a) { return a * 2 + 4 * 2; }
}
```

### REF-INL-022 case 2: argument with side effects used twice gets a local
settings: default
language: java
```before
class A {
    int sq(int x) { return x * x; }
    int next() { return 1; }
    int f() { return s<caret>q(next()); }
}
```
keys: Ctrl+Alt+N, "Inline this usage only", Refactor
```after
class A {
    int sq(int x) { return x * x; }
    int next() { return 1; }
    int f() {
        int x = next();
        return x * x;
    }
}
```

### REF-INL-022 case 3: void method as statements
settings: default
language: java
```before
class A {
    void log(String s) { System.out.println("> " + s); }
    void f() { l<caret>og("hi"); }
}
```
keys: Ctrl+Alt+N, "Inline this usage only", Refactor
```after
class A {
    void log(String s) { System.out.println("> " + s); }
    void f() { System.out.println("> " + "hi"); }
}
```

### REF-INL-022 case 4: several returns become single-return form
settings: default
language: java
```before
class A {
    int sign(int a) {
        if (a > 0) return 1;
        return -1;
    }
    int f(int v) { return si<caret>gn(v); }
}
```
keys: Ctrl+Alt+N, "Inline this usage only", Refactor, Continue in the conflicts dialog (message: Inlined method will be transformed to single-return form)
```after
class A {
    int sign(int a) {
        if (a > 0) return 1;
        return -1;
    }
    int f(int v) {
        int result;
        if (v > 0) result = 1;
        else result = -1;
        return result;
    }
}
```

### REF-INL-022 case 5: qualifier becomes this
settings: default
language: java
```before
class A {
    int base = 1;
    int plus(int x) { return x + base; }
}
class U { int f(A a) { return a.pl<caret>us(2); } }
```
keys: Ctrl+Alt+N, "Inline this usage only", Refactor
```after
class A {
    int base = 1;
    int plus(int x) { return x + base; }
}
class U { int f(A a) { return 2 + a.base; } }
```

### REF-INL-023 case 1: inaccessible member
settings: default
language: java
```before
--- p/A.java
package p;
public class A {
    private int secret = 1;
    public int get() { return secret; }
}
--- q/U.java
package q;
class U { int f(p.A a) { return a.ge<caret>t(); } }
```
keys: Ctrl+Alt+N, "Inline this usage only", Refactor
```after
Problems Detected dialog body (shape):
<Field A.secret> that is used in inlined method is not accessible from call site(s) in <class q.U>
```

### REF-INL-024 case 1: remove method after inlining
settings: default
language: java
```before
--- A.java
class A {
    int one() { return 1; }
}
--- B.java
class B { int f(A a) { return a.o<caret>ne() + a.one(); } }
```
keys: Ctrl+Alt+N, "Inline all usages, remove the method", Refactor
```after
--- A.java
class A {
}
--- B.java
class B { int f(A a) { return 1 + 1; } }
undo command name: Inlining Method A.one()
```

### REF-INL-030 case 1: inline constant field
settings: default
language: java
```before
class A {
    static final int MAX<caret> = 10;
    int f() { return MAX * 2; }
    int g() { return MAX + 1; }
}
```
keys: Ctrl+Alt+N
```after
dialog: Inline Field
label: Inline field 'MAX' with 2 usages:
(o) Inline all usages, remove the field
( ) Inline all usages, keep the field
( ) Inline this usage only, keep the field
after Refactor:
class A {
    int f() { return 10 * 2; }
    int g() { return 10 + 1; }
}
```

### REF-INL-030 case 2: non-final field with write usages
settings: default
language: java
```before
class A {
    static int cou<caret>nt = 0;
    void f() { count = 5; }
}
```
keys: Ctrl+Alt+N
```after
editor error hint:
Cannot perform refactoring.
Inline Field refactoring is supported only for final fields
```

### REF-INL-031 case 1: inline parameter
settings: default
language: java
```before
class A {
    void f(int lim<caret>it) { System.out.println(limit); }
    void g() { f(10); f(10); }
}
```
keys: Ctrl+Alt+N
```after
confirmation: Inline parameter 'limit' with initializer '10'?
after OK:
class A {
    void f() { System.out.println(10); }
    void g() { f(); f(); }
}
```

### REF-INL-040 case 1: Kotlin inline function on a call
settings: default
language: kotlin
```before
fun twice(x: Int) = x * 2
fun f(a: Int) = tw<caret>ice(a) + twice(4)
```
keys: Ctrl+Alt+N
```after
dialog title: Inline Function
(o) Inline all references and remove the function
( ) Inline this reference and keep the function
after Refactor with the first option:
fun f(a: Int) = a * 2 + 4 * 2
```

### REF-INL-040 case 2: Kotlin this reference only
settings: default
language: kotlin
```before
fun twice(x: Int) = x * 2
fun f(a: Int) = tw<caret>ice(a) + twice(4)
```
keys: Ctrl+Alt+N, choose "Inline this reference and keep the function", Refactor
```after
fun twice(x: Int) = x * 2
fun f(a: Int) = a * 2 + twice(4)
```

### REF-INL-040 case 3: Kotlin inline local variable
settings: default
language: kotlin
```before
fun f(a: Int): Int {
    val t = a * 2
    return t + <caret>t
}
```
keys: Ctrl+Alt+N, choose "All 2 references and remove the variable"
```after
fun f(a: Int): Int {
    return a * 2 + a * 2
}
```

### REF-INL-041 case 1: property with accessor and backing field
settings: default
language: kotlin
```before
class A {
    var x<caret>: Int = 0
        get() = field + 1
}
```
keys: Ctrl+Alt+N
```after
editor error hint (message line): Cannot inline property with accessor(s) and backing field
```
