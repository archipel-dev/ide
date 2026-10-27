# Cases for 04-extract-introduce.md

Notation as in `01-framework.cases.md`. `<selection>...</selection>` marks the selected text.
"Enter" in the keys finishes an in-place session. Newly introduced code is shown formatted with the
default Java/Kotlin code style (4 spaces). Names typed in `keys` make results independent of name
suggestions; cases that assert a default name say so.

### REF-EXT-001 case 1: whole-line expression is used without a chooser
settings: default
language: java
```before
class A {
    int f(int a) {
    <caret>    return a * 2 + 1;
    }
}
```
keys: Ctrl+Alt+V (caret in the indentation before the statement), type "v", Enter
```after
class A {
    int f(int a) {
        int v = a * 2 + 1;
        return v;
    }
}
```

### REF-EXT-001 case 2: nested candidates open the expression chooser
settings: default
language: java
```before
class A {
    int f(int a, int b, int c) {
        int r = a + <caret>b * c;
        return r;
    }
}
```
keys: Ctrl+Alt+V
```after
popup title: Expressions
rows (top to bottom, the first row selected):
b
b * c
a + b * c
(moving the selection highlights the expression range in the editor)
```

### REF-EXT-001 case 3: pick the middle candidate
settings: default
language: java
```before
class A {
    int f(int a, int b, int c) {
        int r = a + <caret>b * c;
        return r;
    }
}
```
keys: Ctrl+Alt+V, Down, Enter (choose "b * c"), type "bc", Enter
```after
class A {
    int f(int a, int b, int c) {
        int bc = b * c;
        int r = a + bc;
        return r;
    }
}
```

### REF-EXT-001 case 4: no expression
settings: default
language: java
```before
class A {
    void f() {
        <caret>
    }
}
```
keys: Ctrl+Alt+V
```after
editor error hint (message line): Selected block should represent an expression
(no code change)
```

### REF-EXT-002 case 1: second row preselected when the first candidate is a local variable
settings: default
language: java
```before
class A {
    int f() {
        int a = 1;
        int b = 2;
        int r = <caret>a + b;
        return r;
    }
}
```
keys: Ctrl+Alt+V
```after
popup title: Expressions
rows: a
      a + b
selected row: a + b   (the first candidate is a reference to the local variable a)
```

### REF-EXT-003 case 1: replace all occurrences
settings: default
language: java
```before
class A {
    String getName() { return ""; }
    void f() {
        g(<selection>getName()</selection>, getName());
    }
    void g(String a, String b) {}
}
```
keys: Ctrl+Alt+V; in the popup "Multiple occurrences found" choose "Replace all 2 occurrences"; Enter (keep the suggested name)
```after
popup title: Multiple occurrences found
rows: Replace this occurrence only
      Replace all 2 occurrences
class A {
    String getName() { return ""; }
    void f() {
        String name = getName();
        g(name, name);
    }
    void g(String a, String b) {}
}
(the suggested name is name; the replaced occurrences are highlighted, status bar: Press Escape to remove the highlighting)
```

### REF-EXT-003 case 2: replace only this occurrence
settings: default
language: java
```before
class A {
    String getName() { return ""; }
    void f() {
        g(<selection>getName()</selection>, getName());
    }
    void g(String a, String b) {}
}
```
keys: Ctrl+Alt+V, choose "Replace this occurrence only", Enter
```after
class A {
    String getName() { return ""; }
    void f() {
        String name = getName();
        g(name, getName());
    }
    void g(String a, String b) {}
}
```

### REF-EXT-003 case 3: occurrences with a write access
settings: default
language: java
```before
class A {
    void f(int[] a) {
        a[0] = <selection>a[0]</selection> + 1;
    }
}
```
keys: Ctrl+Alt+V
```after
popup title: Multiple occurrences found
rows: Replace this occurrence only
      Replace all occurrences but write
      Replace read and write occurrences (will change semantics!)
```

### REF-EXT-004 case 1: void expression
settings: default
language: java
```before
class A {
    void f() {
        <selection>System.out.println("x")</selection>;
    }
}
```
keys: Ctrl+Alt+V
```after
editor error hint (message line): Selected expression has type 'void'
```

### REF-EXT-004 case 2: variable in an interface field initializer
settings: default
language: java
```before
interface I {
    int X = <selection>1 + 2</selection>;
}
```
keys: Ctrl+Alt+V
```after
editor error hint (message line): Cannot extract variable in an interface.
```

### REF-EXT-005 case 1: default in-place introduce variable
settings: default
language: java
```before
class A {
    String getName() { return ""; }
    void f() {
        System.out.println(<selection>getName()</selection>);
    }
}
```
keys: Ctrl+Alt+V, Enter (the suggested name `name` is kept)
```after
class A {
    String getName() { return ""; }
    void f() {
        String name = getName();
        System.out.println(name);
    }
}
```

### REF-EXT-005 case 2: options popup final and var type
settings: default (language level 21)
language: java
```before
class A {
    String getName() { return ""; }
    void f() {
        System.out.println(<selection>getName()</selection>);
    }
}
```
keys: Ctrl+Alt+V, Tab (options popup: [ ] Declare final, [ ] Declare var type), tick both, Esc (close popup), Enter
```after
class A {
    String getName() { return ""; }
    void f() {
        final var name = getName();
        System.out.println(name);
    }
}
(both options are remembered as the new defaults)
```

### REF-EXT-005 case 3: Escape cancels
settings: default
language: java
```before
class A {
    String getName() { return ""; }
    void f() {
        System.out.println(<selection>getName()</selection>);
    }
}
```
keys: Ctrl+Alt+V, type "xyz", Escape
```after
class A {
    String getName() { return ""; }
    void f() {
        System.out.println(getName());
    }
}
```

### REF-EXT-005 case 4: dialog form
settings: Editor | General | Refactorings: In modal dialogs
language: java
```before
class A {
    String getName() { return ""; }
    void f() {
        g(<selection>getName()</selection>, getName());
    }
    void g(String a, String b) {}
}
```
keys: Ctrl+Alt+V
```after
dialog title: Introduce Variable
Name: name (combo with suggestions)
Variable of type: String
[ ] Declare final
[x] Replace all occurrences (2 occurrences)
buttons: [OK] [Cancel]
```

### REF-EXT-006 case 1: braces are added around a single-statement branch
settings: default
language: java
```before
class A {
    int h() { return 1; }
    void g(int x) {}
    void f(boolean c) {
        if (c) g(<selection>h()</selection>);
    }
}
```
keys: Ctrl+Alt+V, type "v", Enter
```after
class A {
    int h() { return 1; }
    void g(int x) {}
    void f(boolean c) {
        if (c) {
            int v = h();
            g(v);
        }
    }
}
```

### REF-EXT-006 case 2: for-loop condition conflict
settings: default
language: java
```before
class A {
    void f(int n) {
        for (int i = 0; i < <selection>n * 2</selection>; i++) { n--; }
    }
}
```
keys: Ctrl+Alt+V
```after
editor error hint (lines, in this order):
Introducing variable may break code logic
Parameter n is modified in loop body
(no code change)
```

### REF-EXT-006 case 3: replace all declares before the common ancestor statement
settings: default
language: java
```before
class A {
    int h() { return 1; }
    void g(int x) {}
    void f(boolean c) {
        if (c) { g(<selection>h()</selection>); } else { g(h()); }
    }
}
```
keys: Ctrl+Alt+V, choose the row "Replace all 2 occurrences", type "v", Enter
```after
class A {
    int h() { return 1; }
    void g(int x) {}
    void f(boolean c) {
        int v = h();
        if (c) { g(v); } else { g(v); }
    }
}
```

### REF-EXT-007 case 1: choosing another type
settings: default
language: java
```before
import java.util.ArrayList;
class A {
    void g(Object o) {}
    void f() {
        g(<selection>new ArrayList<String>()</selection>);
    }
}
```
keys: Ctrl+Alt+V, type "list", Shift+Tab (type field), choose "List<String>" in the lookup, Enter
```after
lookup rows start with: ArrayList<String>  (the expression's type, default) and contain List<String>
import java.util.ArrayList;
import java.util.List;
class A {
    void g(Object o) {}
    void f() {
        List<String> list = new ArrayList<>();
        g(list);
    }
}
```

### REF-EXT-007 case 2: reassign an existing variable
settings: default
language: java
```before
class A {
    String getName() { return ""; }
    void g(String s) {}
    void f() {
        String s = "a";
        g(<selection>getName()</selection>);
    }
}
```
keys: Ctrl+Alt+V (lookup advertisement: Press Ctrl+Alt+V to reassign existing variable), Ctrl+Alt+V again, choose "s" in the popup "Choose variable to reassign"
```after
class A {
    String getName() { return ""; }
    void g(String s) {}
    void f() {
        String s = "a";
        s = getName();
        g(s);
    }
}
```

### REF-EXT-008 case 1: replace reads only
settings: default
language: java
```before
class A {
    void f(int[] a) {
        a[0] = <selection>a[0]</selection> + 1;
    }
}
```
keys: Ctrl+Alt+V, choose "Replace all occurrences but write", type "i", Enter
```after
class A {
    void f(int[] a) {
        int i = a[0];
        a[0] = i + 1;
    }
}
```

### REF-EXT-009 case 1: target code block chooser for an expression lambda
settings: default
language: java
```before
import java.util.function.Supplier;
class A {
    Supplier<String> f(String x) {
        return () -> <selection>x.trim()</selection>;
    }
}
```
keys: Ctrl+Alt+V
```after
popup title: Select Target Code Block
rows: Containing block
      Supplier<String>: () -> x.trim()
(choosing the second row, name "t": the lambda body becomes { String t = x.trim(); return t; } )
```

### REF-EXT-010 case 1: first name suggestion by expression shape
settings: default (no name prefixes)
language: java
```before
class Foo {}
class A {
    Foo foo; java.util.List<String> list;
    String getName() { return ""; }
    boolean isReady() { return true; }
    void f() {
        g(<selection>EXPR</selection>);
    }
    void g(Object o) {}
}
```
keys: for each EXPR in turn: Ctrl+Alt+V and read the first lookup entry (then Esc)
```after
getName()      -> name
isReady()      -> ready
new Foo()      -> foo
list.size()    -> size
this.foo       -> foo
```

### REF-EXT-010 case 2: unique names
settings: default
language: java
```before
class A {
    String getName() { return ""; }
    void f() {
        String name = "n";
        System.out.println(<selection>getName()</selection>);
    }
}
```
keys: Ctrl+Alt+V, Enter
```after
class A {
    String getName() { return ""; }
    void f() {
        String name = "n";
        String name1 = getName();
        System.out.println(name1);
    }
}
```

### REF-EXT-020 case 1: not a constant expression
settings: default
language: java
```before
class A {
    void f(int a) { int b = <selection>a + 1</selection>; }
}
```
keys: Ctrl+Alt+C
```after
editor error hint (message line): Selected expression cannot be a constant initializer
```

### REF-EXT-020 case 2: class chooser for nested classes
settings: default
language: java
```before
class Outer {
    class Inner {
        String s = <selection>"abc"</selection>;
    }
}
```
keys: Ctrl+Alt+C
```after
popup title: Choose class to introduce constant
rows: Inner
      Outer
```

### REF-EXT-021 case 1: introduce constant
settings: default
language: java
```before
class A {
    void connect() {
        open(<selection>"localhost"</selection>);
    }
    void open(String host) {}
}
```
keys: Ctrl+Alt+C, Enter (the suggested name LOCALHOST is kept)
```after
class A {
    public static final String LOCALHOST = "localhost";

    void connect() {
        open(LOCALHOST);
    }
    void open(String host) {}
}
```

### REF-EXT-021 case 2: replace all is off by default
settings: default
language: java
```before
class A {
    void f() {
        open(<selection>"localhost"</selection>);
        open("localhost");
    }
    void open(String host) {}
}
```
keys: Ctrl+Alt+C (popup option: [ ] Replace all occurrences), Enter
```after
class A {
    public static final String LOCALHOST = "localhost";

    void f() {
        open(LOCALHOST);
        open("localhost");
    }
    void open(String host) {}
}
```

### REF-EXT-022 case 1: name clash with an existing field
settings: default
language: java
```before
class A {
    static final String HOST = "h";
    void f() { open(<selection>"localhost"</selection>); }
    void open(String host) {}
}
```
keys: Ctrl+Alt+C, type "HOST", Enter
```after
question (title Introduce Constant):
Field with name HOST
already exists in class 'A'.
Continue?
buttons: [Yes] [No]
```

### REF-EXT-030 case 1: introduce field with initialization in the declaration
settings: default
language: java
```before
class A {
    void f() {
        System.out.println(<selection>new StringBuilder()</selection>);
    }
}
```
keys: Ctrl+Alt+F, type "sb", Enter (Initialize in: Field declaration, the default)
```after
class A {
    private StringBuilder sb = new StringBuilder();

    void f() {
        System.out.println(sb);
    }
}
```

### REF-EXT-030 case 2: interface
settings: default
language: java
```before
interface I {
    default void f() { System.out.println(<selection>"x".length()</selection>); }
}
```
keys: Ctrl+Alt+F
```after
editor error hint (message line): Non-constant fields are not allowed in an interface.
```

### REF-EXT-031 case 1: initialize in the current method
settings: default
language: java
```before
class A {
    String getName() { return ""; }
    void f() {
        System.out.println(<selection>getName()</selection>);
    }
}
```
keys: Ctrl+Alt+F, type "name", Tab, set "Initialize in:" to "Current method", Esc (close popup), Enter
```after
popup contents: Field of type: String | visibility: private | [ ] Declare final | Initialize in: [Field declaration v] | (options: Current method)
class A {
    private String name;

    String getName() { return ""; }
    void f() {
        name = getName();
        System.out.println(name);
    }
}
```

### REF-EXT-032 case 1: only the current method is possible
settings: default
language: java
```before
class A {
    void f(int a) {
        System.out.println(<selection>a * 2</selection>);
    }
}
```
keys: Ctrl+Alt+F, Tab
```after
the "Initialize in:" choice offers only: Current method
(the choices Field declaration and Class constructor(s) are not offered: the expression uses the parameter a)
```

### REF-EXT-040 case 1: introduce parameter
settings: default
language: java
```before
class A {
    void f() { System.out.println(<selection>"hello"</selection>); }
    void g() { f(); }
}
```
keys: Ctrl+Alt+P, type "msg", Enter
```after
class A {
    void f(String msg) { System.out.println(msg); }
    void g() { f("hello"); }
}
```

### REF-EXT-040 case 2: callers in other files
settings: default
language: java
```before
--- A.java
class A {
    void f() { System.out.println(<selection>42</selection>); }
}
--- U.java
class U { void g(A a) { a.f(); } }
```
keys: Ctrl+Alt+P, type "n", Enter
```after
--- A.java
class A {
    void f(int n) { System.out.println(n); }
}
--- U.java
class U { void g(A a) { a.f(42); } }
```

### REF-EXT-040 case 3: error for void expressions
settings: default
language: java
```before
class A {
    void f() { <selection>System.out.println("x")</selection>; }
}
```
keys: Ctrl+Alt+P
```after
editor error hint:
Cannot perform refactoring.
Selected expression has type 'void'
```

### REF-EXT-041 case 1: delegate via overloading method
settings: default
language: java
```before
class A {
    void f() { System.out.println(<selection>"hello"</selection>); }
    void g() { f(); }
}
```
keys: Ctrl+Alt+P, type "msg", Tab, tick "Delegate via overloading method", Esc, Enter
```after
lookup advertisement: Press <shortcut> to delegate via overloading method or <Tab> to show more options
class A {
    void f() { f("hello"); }
    void f(String msg) { System.out.println(msg); }
    void g() { f(); }
}
```

### REF-EXT-042 case 1: existing parameter name
settings: default
language: java
```before
class A {
    void f(String msg) { System.out.println(<selection>"x"</selection> + msg); }
}
```
keys: Ctrl+Alt+P, type "msg", Enter
```after
question (title Introduce Parameter):
<method f(String)> already has a parameter named 'msg'.
Use this name anyway?
buttons: [Yes] [No]
```

### REF-EXT-050 case 1: in-place extract method with the default name
settings: default
language: java
```before
class A {
    int total;
    void f(int a) {
        <selection>total += a;
        System.out.println(total);</selection>
    }
}
```
keys: Ctrl+Alt+M, Enter (the name `extracted` is kept)
```after
class A {
    int total;
    void f(int a) {
        extracted(a);
    }

    private void extracted(int a) {
        total += a;
        System.out.println(total);
    }
}
```

### REF-EXT-050 case 2: typed name and Escape
settings: default
language: java
```before
class A {
    int total;
    void f(int a) {
        <selection>total += a;
        System.out.println(total);</selection>
    }
}
```
keys: Ctrl+Alt+M, type "report", Escape
```after
class A {
    int total;
    void f(int a) {
        total += a;
        System.out.println(total);
    }
}
```

### REF-EXT-050 case 3: expression extraction and the options popup
settings: default
language: java
```before
class A {
    int base;
    int f(int a) {
        return <selection>a * base</selection>;
    }
}
```
keys: Ctrl+Alt+M, type "scaled", Tab, (popup: [ ] Make static and pass fields, link "More options"), Esc, Enter
```after
class A {
    int base;
    int f(int a) {
        return scaled(a);
    }

    private int scaled(int a) {
        return a * base;
    }
}
```

### REF-EXT-050 case 4: static context
settings: default
language: java
```before
class A {
    static void f(int a) {
        <selection>System.out.println(a);</selection>
    }
}
```
keys: Ctrl+Alt+M, type "show", Enter
```after
class A {
    static void f(int a) {
        show(a);
    }

    private static void show(int a) {
        System.out.println(a);
    }
}
```

### REF-EXT-051 case 1: name from the preceding comment
settings: default
language: java
```before
class A {
    void f(int a) {
        // print total
        <selection>System.out.println(a);</selection>
    }
}
```
keys: Ctrl+Alt+M, Enter
```after
class A {
    void f(int a) {
        // print total
        printTotal(a);
    }

    private void printTotal(int a) {
        System.out.println(a);
    }
}
```

### REF-EXT-051 case 2: name from the single output variable
settings: default
language: java
```before
class A {
    int f(int a, int b) {
        <selection>int sum = a + b;</selection>
        return sum * 2;
    }
}
```
keys: Ctrl+Alt+M, Enter
```after
class A {
    int f(int a, int b) {
        int sum = getSum(a, b);
        return sum * 2;
    }

    private int getSum(int a, int b) {
        int sum = a + b;
        return sum;
    }
}
```

### REF-EXT-051 case 3: boolean prefix
settings: default
language: java
```before
class A {
    boolean f(int a) {
        <selection>boolean ok = a > 0;</selection>
        return ok;
    }
}
```
keys: Ctrl+Alt+M, Enter
```after
class A {
    boolean f(int a) {
        boolean ok = isOk(a);
        return ok;
    }

    private boolean isOk(int a) {
        boolean ok = a > 0;
        return ok;
    }
}
```

### REF-EXT-051 case 5: creation prefix
settings: default
language: java
```before
class A {
    Object g() {
        <selection>Object foo = new Object();</selection>
        return foo;
    }
}
```
keys: Ctrl+Alt+M, Enter
```after
class A {
    Object g() {
        Object foo = createFoo();
        return foo;
    }

    private Object createFoo() {
        Object foo = new Object();
        return foo;
    }
}
```

### REF-EXT-051 case 4: name conflicts get numbered
settings: default
language: java
```before
class A {
    void extracted(int a) {}
    void f(int a) {
        <selection>System.out.println(a);</selection>
    }
}
```
keys: Ctrl+Alt+M
```after
the name field of the template starts with: extracted1
```

### REF-EXT-052 case 1: parameters in order of first use
settings: default
language: java
```before
class A {
    void f(int b, int a) {
        <selection>int x = a + b;
        System.out.println(x);</selection>
    }
}
```
keys: Ctrl+Alt+M, type "show", Enter
```after
class A {
    void f(int b, int a) {
        show(a, b);
    }

    private void show(int a, int b) {
        int x = a + b;
        System.out.println(x);
    }
}
```

### REF-EXT-052 case 2: multiple exit points
settings: default
language: java
```before
class A {
    int f(int a) {
        <selection>if (a > 0) return 1;
        if (a < 0) return -1;</selection>
        return 0;
    }
}
```
keys: Ctrl+Alt+M
```after
editor error hint: Unable to extract method. There are multiple exit points.
(the offending statements are highlighted; status bar: Press Escape to remove the highlighting)
```

### REF-EXT-052 case 3: checked exceptions go to the throws clause
settings: default
language: java
```before
class A {
    void f(java.io.File file) throws java.io.IOException {
        <selection>java.io.FileReader r = new java.io.FileReader(file);
        r.close();</selection>
    }
}
```
keys: Ctrl+Alt+M, type "touch", Enter
```after
class A {
    void f(java.io.File file) throws java.io.IOException {
        touch(file);
    }

    private void touch(java.io.File file) throws java.io.IOException {
        java.io.FileReader r = new java.io.FileReader(file);
        r.close();
    }
}
```

### REF-EXT-053 case 1: dialog form
settings: Editor | General | Refactorings: In modal dialogs
language: java
```before
class A {
    int f(int a, int b) { return <selection>a + b</selection>; }
}
```
keys: Ctrl+Alt+M
```after
dialog title: Extract Method
Method name: (field, suggested name)
Visibility: private (public / protected / package-private / private)
[ ] Declare static
Parameters:  [x] int a   [x] int b      (up/down buttons reorder; Alt+Up/Down)
Signature preview: private int <name>(int a, int b)
buttons: [OK] [Cancel] [Preview]
```

### REF-EXT-054 case 1: several output variables
settings: default (language level 21)
language: java
```before
class A {
    void f() {
        <selection>int a = 1;
        int b = 2;</selection>
        System.out.println(a + b);
    }
}
```
keys: Ctrl+Alt+M
```after
information hint:
There are several output variables in the selected code block.
 The method can be extracted if we fold them into a new record.
a template session starts on the name of the newly inserted record; after Enter the extract method flow continues on the rewritten code
```

### REF-EXT-055 case 1: duplicates are replaced
settings: default
language: java
```before
class A {
    void f(int a) {
        <selection>System.out.println(a * 2);</selection>
    }
    void g(int b) {
        System.out.println(b * 2);
    }
}
```
keys: Ctrl+Alt+M, type "show", Enter
```after
class A {
    void f(int a) {
        show(a);
    }

    private void show(int a) {
        System.out.println(a * 2);
    }
    void g(int b) {
        show(b);
    }
}
```

### REF-EXT-080 case 1: Introduce Parameter Object dialog
settings: default
language: java
```before
class A {
    void m<caret>(int x, int y, String label) {}
}
```
keys: action IntroduceParameterObject
```after
dialog title: Introduce Parameter Object
Method to extract parameters from: A.m(int, int, String)
Parameters to Extract (table, all checked): x : int, y : int, label : String
Parameter Class:
(o) Create new class   Name: <field>   Package name: <field>
( ) Use existing class Name: <field>
( ) Create inner class Name: <field>
[ ] Keep method as delegate  [ ] Generate accessors  [ ] Escalate visibility
buttons: [Refactor] [Preview] [Cancel]
```

### REF-EXT-100 case 1: Kotlin introduce variable
settings: default
language: kotlin
```before
fun getName() = "x"
fun f() {
    println(<selection>getName()</selection>)
}
```
keys: Ctrl+Alt+V, Enter (the suggested name `name` is kept)
```after
fun getName() = "x"
fun f() {
    val name = getName()
    println(name)
}
```

### REF-EXT-100 case 2: Declare with var and explicit type
settings: default
language: kotlin
```before
fun getName() = "x"
fun f() {
    println(<selection>getName()</selection>)
}
```
keys: Ctrl+Alt+V, Tab (popup: [ ] Declare with var, [ ] Specify type explicitly), tick both, Esc, Enter
```after
fun getName() = "x"
fun f() {
    var name: String = getName()
    println(name)
}
```

### REF-EXT-100 case 3: occurrences chooser in Kotlin
settings: default
language: kotlin
```before
fun g(a: Int, b: Int) {}
fun f(x: Int) {
    g(<selection>x + 1</selection>, x + 1)
}
```
keys: Ctrl+Alt+V, choose "Replace all 2 occurrences", type "y", Enter
```after
fun g(a: Int, b: Int) {}
fun f(x: Int) {
    val y = x + 1
    g(y, y)
}
```

### REF-EXT-100 case 4: Kotlin errors
settings: default
language: kotlin
```before
fun f() {
    <selection>println("x")</selection>
}
```
keys: Ctrl+Alt+V
```after
editor error hint: Cannot introduce expression of unit type
```

### REF-EXT-101 case 1: Kotlin introduce constant
settings: default
language: kotlin
```before
fun connect() {
    open(<selection>"localhost"</selection>)
}
fun open(host: String) {}
```
keys: Ctrl+Alt+C, Enter (the suggested name LOCALHOST is kept)
```after
shape: a declaration  const val LOCALHOST = "localhost"  appears (with the visibility chosen by the plugin) at file level above fun connect, and the call reads open(LOCALHOST)
```

### REF-EXT-102 case 1: Kotlin introduce parameter
settings: default
language: kotlin
```before
fun f() { println(<selection>"hello"</selection>) }
fun g() { f() }
```
keys: Ctrl+Alt+P, name "msg", OK
```after
fun f(msg: String) { println(msg) }
fun g() { f("hello") }
```

### REF-EXT-102 case 2: introduce default value
settings: default
language: kotlin
```before
fun f() { println(<selection>"hello"</selection>) }
fun g() { f() }
```
keys: Ctrl+Alt+P, name "msg", tick "Introduce default value", OK
```after
fun f(msg: String = "hello") { println(msg) }
fun g() { f() }
```

### REF-EXT-103 case 1: Kotlin extract function
settings: default
language: kotlin
```before
fun f(a: Int) {
    <selection>println(a)
    println(a + 1)</selection>
}
```
keys: Ctrl+Alt+M, type "report", Enter
```after
fun f(a: Int) {
    report(a)
}

private fun report(a: Int) {
    println(a)
    println(a + 1)
}
```

### REF-EXT-103 case 2: member functions stay in the class
settings: default
language: kotlin
```before
class A {
    var total = 0
    fun f(a: Int) {
        <selection>total += a
        println(total)</selection>
    }
}
```
keys: Ctrl+Alt+M, type "report", Enter
```after
class A {
    var total = 0
    fun f(a: Int) {
        report(a)
    }

    private fun report(a: Int) {
        total += a
        println(total)
    }
}
```

### REF-EXT-103 case 3: super call cannot be extracted
settings: default
language: kotlin
```before
open class B { open fun m() {} }
class A : B() {
    override fun m() {
        <selection>super.m()</selection>
    }
}
```
keys: Ctrl+Alt+M
```after
editor error hint (message line): Cannot extract super-call
```
