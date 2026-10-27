# Cases for 02-rename.md

Notation as in `01-framework.cases.md`. "keys" lists the user actions; dialog and popup contents
are written in display order. Default settings mean: `Editor | General | Refactorings` in-place
mode on, preselect on, rename defaults of REF-REN-040, no code style name prefixes.

### REF-REN-001 case 1: two rename handlers claim the element
settings: default (two handlers registered for the same element, for example a framework handler `Rename Bean` and the default one)
language: java
```before
@Component class Fo<caret>o {}
```
keys: Shift+F6
```after
dialog title: Select Refactoring
label: What would you like to do?
radio buttons (top to bottom): ( ) Rename bean  (o) Rename class
first radio selected; Down arrow moves to the second; Enter confirms; Cancel aborts
```

### REF-REN-002 case 1: element outside the project
settings: default
language: java
```before
class A { void f() { java.lang.Str<caret>ing s; } }
```
keys: Shift+F6
```after
editor error hint:
Cannot perform refactoring.
Selected class is not located inside the project
```

### REF-REN-002 case 2: caret in a comment
settings: default
language: java
```before
class A { // some tex<caret>t
}
```
keys: Shift+F6
```after
editor error hint:
Cannot perform refactoring.
Caret should be positioned at symbol to be renamed
```

### REF-REN-003 case 1: synthetic enum method cannot be renamed
settings: default
language: java
```before
enum E { A; void f() { E.val<caret>ues(); } }
```
keys: Shift+F6
```after
Rename is not available: the action is disabled in the Refactor menu; invoking by shortcut shows the editor hint
Cannot perform refactoring.
Caret should be positioned at symbol to be renamed
```

### REF-REN-004 case 1: usage from a non-project file
settings: default
language: java
```before
--- library-sources/Lib.java   (not part of the project)
class Lib { void f() { app.Helper h<caret>; } }
--- src/app/Helper.java   (project)
package app; public class Helper {}
```
keys: Shift+F6 with the caret on Helper in Lib.java
```after
yes/no dialog:
title: Cannot perform refactoring.
message: Selected element is used from non-project files. These usages won't be renamed. Proceed anyway?
buttons: [Yes] [No]
```

### REF-REN-010 case 1: local variable renames in place, no dialog
settings: default
language: java
```before
class A {
    void f() {
        int co<caret>unt = 1;
        System.out.println(count);
    }
}
```
keys: Shift+F6
```after
template session started in the editor (no dialog); the name `count` is selected; the usage `count` is linked/highlighted
```

### REF-REN-010 case 2: field renames in place with a real refactoring afterwards
settings: default
language: java
```before
--- A.java
class A {
    int co<caret>unt;
}
--- B.java
class B { int g(A a) { return a.count; } }
```
keys: Shift+F6, type "total", Enter
```after
--- A.java
class A {
    int total<caret>;
}
--- B.java
class B { int g(A a) { return a.total; } }
(B.java changed by the refactoring that ran after Enter; one undo step restores both files)
```

### REF-REN-010 case 3: lambda and catch parameters use the in-place variable mode
settings: default
language: java
```before
class A {
    void f(java.util.List<String> l) {
        l.forEach(it<caret>em -> System.out.println(item));
    }
}
```
keys: Shift+F6, type "s", Enter
```after
class A {
    void f(java.util.List<String> l) {
        l.forEach(s<caret> -> System.out.println(s));
    }
}
```

### REF-REN-010 case 4: dialog is used when the caret is in another file than the declaration
settings: default
language: java
```before
--- A.java
class A { void foo() {} }
--- B.java
class B { void g(A a) { a.fo<caret>o(); } }
```
keys: Shift+F6 (caret on the call in B.java, declaration in A.java)
```after
Rename dialog opens (title: Rename), not an in-place session
label: Rename method 'foo() of class A' and its usages to:
```

### REF-REN-011 case 1: local renamed to a field name qualifies the field references
settings: default
language: java
```before
class A {
    int count;
    void f() {
        int x<caret> = 1;
        count = x;
    }
}
```
keys: Shift+F6, type "count", Enter
```after
class A {
    int count;
    void f() {
        int count<caret> = 1;
        this.count = count;
    }
}
```

### REF-REN-012 case 1: member in-place rename of a method updates other files
settings: default
language: java
```before
--- A.java
class A {
    void fo<caret>o() {}
}
--- B.java
class B { void g(A a) { a.foo(); } }
```
keys: Shift+F6, type "bar", Enter
```after
--- A.java
class A {
    void bar<caret>() {}
}
--- B.java
class B { void g(A a) { a.bar(); } }
undo command name: Renaming method foo() to bar
```

### REF-REN-012 case 2: class in-place rename renames the file
settings: default
language: java
```before
--- Foo.java
public class Fo<caret>o {
    Foo() {}
    static Foo make() { return new Foo(); }
}
--- User.java
class User { Foo f = Foo.make(); }
```
keys: Shift+F6, type "Bar", Enter
```after
--- Bar.java
public class Bar<caret> {
    Bar() {}
    static Bar make() { return new Bar(); }
}
--- User.java
class User { Bar f = Bar.make(); }
(the file Foo.java is now named Bar.java; no follow-up dialogs appear here because no inheritor, variable or test has a name containing Foo, see REF-REN-110 for those)
```

### REF-REN-013 case 1: name suggestions for a parameter that implements an interface method
settings: default
language: java
```before
interface Named { void setName(String name); }
class A implements Named {
    public void setName(String <caret>a) {}
}
```
keys: Shift+F6, press Down in the lookup / open the Rename dialog name combo
```after
suggestions, first to last (current name `a` excluded):
name                      (the name of the parameter in the overridden method)
then the properly cased variants of `a` and the type-based names of the code style (not enumerated here)
```

### REF-REN-013 case 2: suggestions for a constant
settings: default
language: java
```before
class A {
    static final int MAX_RETRY_COUNT<caret> = 3;
}
```
keys: Shift+F6, open the suggestion lookup
```after
MAX_RETRY_COUNT is the current name and is not offered; the first suggestions, in order:
RETRY_COUNT
COUNT
(followed by the code style's type-based names)
```

### REF-REN-014 case 1: Rename again switches to the dialog with the typed name
settings: default
language: java
```before
class A {
    void f() {
        int co<caret>unt = 1;
        System.out.println(count);
    }
}
```
keys: Shift+F6, type "n", Shift+F6
```after
Rename dialog (title Rename); label: Rename local variable 'count' and its usages to:
name combo contains, first entry: n   (selected)
the editor text is back to the original (int count = 1;) while the dialog is shown
```

### REF-REN-015 case 1: options popup links comments and strings
settings: default
language: java
```before
class A {
    // call foo before bar
    String s = "foo";
    private void fo<caret>o() {}
    void g() { foo(); }
}
```
keys: Shift+F6, Tab (focus the options inlay), tick "Comments and strings", Escape (close popup, keep session), type "baz", Enter
```after
class A {
    // call baz before bar
    String s = "baz";
    private void baz<caret>() {}
    void g() { baz(); }
}
```

### REF-REN-020 case 1: dialog contents for a method
settings: default
language: java
```before
package p;
class A {
    void ba<caret>r(int x) {}
    void bar(String s) {}
}
```
keys: Shift+F6 twice (or setting "In modal dialogs" and Shift+F6)
```after
title: Rename
label: Rename method 'bar(int) of class p.A' and its usages to:
name combo: bar (selected)
[ ] Search in comments and strings     [ ] Search for text occurrences
[x] Rename overloads
Scope: Project Files (combo)
buttons: [Refactor] [Preview] [Cancel]
```

### REF-REN-020 case 2: dialog contents for a local variable
settings: Editor | General | Refactorings: In modal dialogs
language: java
```before
class A { void f() { int cou<caret>nt = 1; } }
```
keys: Shift+F6
```after
title: Rename
label: Rename local variable 'count' and its usages to:
name combo: count (selected)
[x] Search in comments and strings
(no "Search for text occurrences" checkbox, no Scope section: local scope)
buttons: [Refactor] [Preview] [Cancel]
```

### REF-REN-021 case 1: checkbox defaults are remembered per kind
settings: default
language: java
```before
class A { void fo<caret>o() {} }
```
keys: Shift+F6 twice, tick "Search in comments and strings", name "bar", Refactor; then open Rename on another method
```after
second dialog: [x] Search in comments and strings   [ ] Search for text occurrences
(a field's dialog still has both unticked: kinds are independent)
```

### REF-REN-030 case 1: invalid names
settings: default
language: java
```before
class A { void fo<caret>o() {} }
```
keys: open the Rename dialog, type each name in turn
```after
"a b"    -> error text: 'a b' is not a valid identifier ; [Refactor] [Preview] disabled
"1x"     -> error text: '1x' is not a valid identifier
"class"  -> error text: 'class' is not a valid identifier
""       -> [Refactor] disabled
"ok_1"   -> no error; [Refactor] enabled
```

### REF-REN-030 case 2: restricted type identifier for a class
settings: default (language level 21)
language: java
```before
class Fo<caret>o {}
```
keys: open the Rename dialog, type "var"
```after
error text: 'var' is not a valid identifier ; [Refactor] disabled
```

### REF-REN-040 case 1: initial defaults of the checkboxes
settings: factory settings
language: java
```before
--- A.java
class A { int fie<caret>ld; void meth() {} }
```
keys: open the Rename dialog on class A, then on field, then on method meth, then on a local variable
```after
class A:   comments [ ]  text occurrences [x]
field:     comments [ ]  text occurrences [ ]
method:    comments [ ]  text occurrences [ ]
variable:  comments [x]  (no text occurrences box)
```

### REF-REN-050 case 1: class rename with every reference kind
settings: rename class defaults
language: java
```before
--- p/Foo.java
package p;
/** See {@link Foo#make()}. */
public class Fo<caret>o {
    public Foo() {}
    public static Foo make() { return new Foo(); }
}
--- q/User.java
package q;
import p.Foo;
import java.util.function.Supplier;
class User extends Foo implements Comparable<Foo> {
    Supplier<Foo> s = Foo::new;
    Foo f = Foo.make();
    public int compareTo(Foo o) { return 0; }
}
```
keys: action RenameElement (dialog), name "Bar", uncheck all automatic renamers, Refactor
```after
--- p/Bar.java
package p;
/** See {@link Bar#make()}. */
public class Bar {
    public Bar() {}
    public static Bar make() { return new Bar(); }
}
--- q/User.java
package q;
import p.Bar;
import java.util.function.Supplier;
class User extends Bar implements Comparable<Bar> {
    Supplier<Bar> s = Bar::new;
    Bar f = Bar.make();
    public int compareTo(Bar o) { return 0; }
}
```

### REF-REN-051 case 1: the file follows the class, but not for secondary classes
settings: default
language: java
```before
--- Foo.java
public class Foo {}
class He<caret>lper {}
```
keys: action RenameElement on Helper, name "Aid", Refactor
```after
--- Foo.java        (file name unchanged)
public class Foo {}
class Aid {}
```

### REF-REN-052 case 1: Rename File dialog for a text file
settings: default
language: plain
```before
--- notes.txt   (selected in the Project view)
hello
```
keys: Project view: Refactor > Rename File..., (no editor)
```after
title: Rename
label: Rename file '<absolute path>/notes.txt' and its usages to:
name field: notes.txt   (only "notes" selected)
[x] Search for references
[x] Search in comments and strings     [x] Search for text occurrences
buttons: [Refactor] [Preview] [Cancel]
```

### REF-REN-053 case 1: inner class and the binary name in text files
settings: rename class defaults (text occurrences on)
language: java
```before
--- Outer.java
class Outer {
    static class In<caret>ner {}
}
--- cfg.properties
handler=Outer$Inner
other=Inner
```
keys: action RenameElement, name "Core", Refactor (text occurrences on)
```after
--- Outer.java
class Outer {
    static class Core {}
}
--- cfg.properties
handler=Outer$Core
other=Inner
```

### REF-REN-054 case 1: a class renamed to the name of an imported class
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o {}
--- b/Bar.java
package b;
public class Bar {}
--- c/C.java
package c;
import a.Foo;
import b.Bar;
class C { Foo f; Bar g; }
```
keys: action RenameElement, name "Bar", Refactor
```after
--- a/Bar.java
package a;
public class Bar {}
--- c/C.java
package c;
import a.Bar;
class C { Bar f; b.Bar g; }
```

### REF-REN-060 case 1: method rename with overriders and calls
settings: default
language: java
```before
--- Base.java
class Base {
    void ru<caret>n() {}
}
--- Sub.java
class Sub extends Base {
    @Override void run() {}
    void other() { run(); super.run(); }
}
--- Use.java
class Use { void g(Base b, Sub s) { b.run(); s.run(); Runnable r = b::run; } }
```
keys: action RenameElement on Base.run, name "execute", Refactor
```after
--- Base.java
class Base {
    void execute() {}
}
--- Sub.java
class Sub extends Base {
    @Override void execute() {}
    void other() { execute(); super.execute(); }
}
--- Use.java
class Use { void g(Base b, Sub s) { b.execute(); s.execute(); Runnable r = b::execute; } }
```

### REF-REN-060 case 2: implementers named by the same scheme
settings: default
language: java
```before
interface Shape { double ge<caret>tArea(); }
class Circle implements Shape { public double getArea() { return 1; } }
class Sq implements Shape { public double getArea() { return 2; } }
class Use { double f(Shape s) { return s.getArea(); } }
```
keys: action RenameElement, name "area", Refactor
```after
interface Shape { double area(); }
class Circle implements Shape { public double area() { return 1; } }
class Sq implements Shape { public double area() { return 2; } }
class Use { double f(Shape s) { return s.area(); } }
```

### REF-REN-060 case 3: @Override is removed when a method no longer overrides
settings: Editor | General | Refactorings: In modal dialogs
language: java
```before
class Base { void run() {} }
class Sub extends Base {
    @Override void ru<caret>n() {}
}
```
keys: choose "Current Method" in the Super Method Found dialog, rename to "go", Refactor
```after
class Base { void run() {} }
class Sub extends Base {
    void go() {}
}
```

### REF-REN-061 case 1: Super Method Found dialog
settings: Editor | General | Refactorings: In modal dialogs
language: java
```before
class Base { void run() {} }
class Sub extends Base {
    @Override void ru<caret>n() {}
}
```
keys: Shift+F6 on Sub.run
```after
dialog title: Super Method Found
text (HTML):
Method Sub.run()
overrides method of class Base.

Do you want to rename the base method instead?
buttons: [Base Method] [Current Method] [Cancel]
choosing Base Method opens the Rename dialog for method 'run() of class Base'
```

### REF-REN-061 case 2: in-place popup for the base method choice
settings: default
language: java
```before
interface I { void run(); }
class Sub implements I {
    @Override public void ru<caret>n() {}
}
```
keys: Shift+F6
```after
popup title: Method run() implements method of I
rows: Refactor base method
      Refactor only current method
```

### REF-REN-062 case 1: record component rename renames accessor, field and constructor parameter
settings: default
language: java
```before
record P(int x<caret>, int y) {
    P { if (x < 0) throw new IllegalArgumentException(); }
    int sum() { return x + y(); }
}
class U { int f(P p) { return p.x(); } }
```
keys: Shift+F6, type "left", Enter
```after
record P(int left<caret>, int y) {
    P { if (left < 0) throw new IllegalArgumentException(); }
    int sum() { return left + y(); }
}
class U { int f(P p) { return p.left(); } }
```

### REF-REN-063 case 1: rename overloads
settings: default
language: java
```before
class A {
    void pr<caret>int(int i) {}
    void print(String s) {}
    void use() { print(1); print("x"); }
}
```
keys: action RenameElement, name "show", keep "Rename overloads" ticked, Refactor, in the follow-up dialog keep all rows, OK
```after
follow-up dialog title: Rename Overloads
description: Rename overloads to:
table (checked, Overload name, Rename to): [x] print(String)   show
class A {
    void show(int i) {}
    void show(String s) {}
    void use() { show(1); show("x"); }
}
```

### REF-REN-070 case 1: a field hidden by a parameter gets qualified
settings: default
language: java
```before
class A {
    int x<caret>;
    A(int y) { x = y; }
    int get() { return x; }
}
```
keys: Shift+F6, type "y", Enter
```after
class A {
    int y<caret>;
    A(int y) { this.y = y; }
    int get() { return y; }
}
```

### REF-REN-070 case 2: a static field hidden by a parameter is qualified with the class
settings: default
language: java
```before
class A {
    static int x<caret>;
    static void set(int y) { x = y; }
}
```
keys: Shift+F6, type "y", Enter
```after
class A {
    static int y<caret>;
    static void set(int y) { A.y = y; }
}
```

### REF-REN-071 case 1: accessors renamed with the field
settings: default (Rename accessors on)
language: java
```before
class A {
    private int cou<caret>nt;
    public int getCount() { return count; }
    public void setCount(int count) { this.count = count; }
}
class U { void f(A a) { a.setCount(a.getCount() + 1); } }
```
keys: action RenameElement, name "total", Refactor
```after
follow-up dialog title: Rename Getters/Setters
description: Rename accessors with the following names to:
table: [x] getCount()   getTotal      [x] setCount(int)   setTotal
(OK)
class A {
    private int total;
    public int getTotal() { return total; }
    public void setTotal(int total) { this.total = total; }
}
class U { void f(A a) { a.setTotal(a.getTotal() + 1); } }
```

### REF-REN-072 case 1: parameters renamed in the hierarchy
settings: default
language: java
```before
interface I { void put(String ke<caret>y); }
class A implements I { public void put(String key) { System.out.println(key); } }
```
keys: action RenameElement (dialog) on the interface parameter, name "id", Refactor, follow-up dialog OK
```after
follow-up dialog title: Rename Parameters
description: Rename parameter in hierarchy to:
table: [x] key (A.put)   id
interface I { void put(String id); }
class A implements I { public void put(String id) { System.out.println(id); } }
```

### REF-REN-080 case 1: rename a package through its directory
settings: default
language: java
```before
--- src/app/util/Tool.java
package app.util;
public class Tool {}
--- src/app/Main.java
package app;
import app.util.Tool;
class Main { Tool t; }
```
keys: Project view: select directory src/app/util, Shift+F6, name "helpers", Refactor
```after
--- src/app/helpers/Tool.java
package app.helpers;
public class Tool {}
--- src/app/Main.java
package app;
import app.helpers.Tool;
class Main { Tool t; }
```

### REF-REN-080 case 2: a dotted name is refused
settings: default
language: java
```before
--- src/app/util/Tool.java
package app.util;
public class Tool {}
```
keys: select directory src/app/util, Shift+F6, type "x.y", Refactor
```after
error text: A rename cannot move the package to the parent package of 'app.util'. Give the new short name of the package.
(no change)
```

### REF-REN-081 case 1: package in several source roots
settings: default (module m has roots src/main/java and src/test/java, both with package app)
language: java
```before
--- src/main/java/app/A.java
package app; class A {}
--- src/test/java/app/ATest.java
package app; class ATest {}
```
keys: Project view: select src/main/java/app, Shift+F6
```after
question dialog title: Rename Package Directories
text:
Package 'app'
 is present in multiple directories:

[m] src/main/java/app (selected)
[m] src/test/java/app

Which directories would you like to rename?
buttons: [All Directories] [Selected Directory Only] [Cancel]
```

### REF-REN-100 case 1: method already defined in the class
settings: default
language: java
```before
class A {
    void foo(int a) {}
    void ba<caret>r(int a) {}
}
```
keys: Rename to "foo" via dialog, Refactor
```after
Problems Detected dialog body:
Method foo(int) is already defined in class A
```

### REF-REN-100 case 2: method with the same erasure
settings: default
language: java
```before
import java.util.List;
class A {
    void foo(List<String> a) {}
    void ba<caret>r(List<Integer> a) {}
}
```
keys: Rename to "foo" via dialog, Refactor
```after
Problems Detected dialog body:
Method with the same erasure is already defined in class A
```

### REF-REN-100 case 3: field exists
settings: default
language: java
```before
class A {
    int a;
    int b<caret>;
}
```
keys: Rename to "a" via dialog, Refactor
```after
Problems Detected dialog body:
Field a is already defined in the class A
```

### REF-REN-100 case 4: top-level class and inner class exist
settings: default
language: java
```before
--- p/Foo.java
package p;
public class Fo<caret>o {}
--- p/Bar.java
package p;
public class Bar {}
```
keys: Rename Foo to "Bar" via dialog, Refactor
```after
Problems Detected dialog body:
Class p.Bar already exists
```

### REF-REN-100 case 5: renamed method would override a base method
settings: default
language: java
```before
class Base { void run() {} }
class Sub extends Base {
    void go<caret>() {}
}
```
keys: Rename Sub.go to "run" via dialog, Refactor
```after
Problems Detected dialog body:
Method will override method run() of the super class Base
```

### REF-REN-100 case 6: renamed method would override a final base method
settings: default
language: java
```before
class Base { final void run() {} }
class Sub extends Base {
    void go<caret>() {}
}
```
keys: Rename Sub.go to "run" via dialog, Refactor
```after
Problems Detected dialog body:
Method will override final method run() of super class Base
```

### REF-REN-101 case 1: local collision
settings: default
language: java
```before
class A {
    void f() {
        int a = 1;
        int b<caret> = 2;
        System.out.println(a + b);
    }
}
```
keys: Rename b to "a" via dialog (Shift+F6 twice), Refactor
```after
Problems Detected dialog body:
An existing local variable a has the same name
```

### REF-REN-101 case 2: field renamed into the name of a visible local
settings: default
language: java
```before
class A {
    int x<caret>;
    void f() {
        int y = 1;
        x = y;
    }
}
```
keys: Rename field x to "y" via dialog, Refactor
```after
the rename proceeds without a conflict: the field reference is qualified
class A {
    int y;
    void f() {
        int y = 1;
        this.y = y;
    }
}
```

### REF-REN-102 case 1: subclass member would override the renamed member
settings: default
language: java
```before
class Base { void fo<caret>o() {} }
class Sub extends Base { void bar() {} }
```
keys: Rename Base.foo to "bar" via dialog, Refactor
```after
Problems Detected dialog body:
Method Sub.bar() will override renamed method Base.foo()
```

### REF-REN-103 case 1: an outer member hidden by the renamed member is qualified automatically
settings: default
language: java
```before
class Outer {
    int v;
    class Inner {
        int w<caret>;
        int g() { return v + w; }
    }
}
```
keys: Rename field w to "v" via dialog, Refactor
```after
class Outer {
    int v;
    class Inner {
        int v;
        int g() { return Outer.this.v + v; }
    }
}
```

### REF-REN-104 case 1: skipped usages are reported
settings: default
language: java
```before
class A {
    void f() {
        int a = 1;
        { int b<caret> = 2; System.out.println(a + b); }
    }
}
```
keys: Rename b to "a", Refactor, in the conflicts dialog press Continue
```after
dialog title: Not All Usages Were Renamed
text: Following usages were safely skipped:
(list: the conflicting usage)
```

### REF-REN-110 case 1: related renamers after a class rename
settings: default
language: java
```before
--- Foo.java
public class Foo<caret> {}
--- Uses.java
class Uses {
    Foo foo = new Foo();
    java.util.List<Foo> foos;
    Foo[] fooArray;
    Foo myFoo;
}
class FooImpl extends Foo {}
```
keys: action RenameElement, name "Bar", keep "Rename inheritors", "Rename variables" ticked, Refactor
```after
dialog 1 title: Rename Inheritors
description: Rename inheritors with the following names to:
header: Inheritor name | Rename to
[x] FooImpl    BarImpl
(OK)
dialog 2 title: Rename Variables
description: Rename Variables with the Following Names to:
header: Variable name | Rename to
[x] foo         bar
[x] foos        bars
[x] fooArray    barArray
[x] myFoo       myBar
(OK, then the code:)
class Uses {
    Bar bar = new Bar();
    java.util.List<Bar> bars;
    Bar[] barArray;
    Bar myBar;
}
class BarImpl extends Bar {}
```

### REF-REN-110 case 2: unticking a row keeps that variable
settings: default
language: java
```before
--- A.java
public class Foo<caret> {}
class Uses { Foo foo; Foo myFoo; Foo second; }
```
keys: action RenameElement, name "Bar", Refactor, in the Rename Variables dialog untick the row `foo`, press OK
```after
class Uses { Bar foo; Bar myBar; Bar second; }
```
(only rows whose name contains the old class name are listed: `foo` and `myFoo`; the unticked `foo` keeps its name, `second` is not a candidate; the type references all change)

### REF-REN-111 case 1: simple replacement
settings: default (no name prefixes in the code style)
language: plain
```before
oldClass=Foo   newClass=Bar
variable names: foo | myFoo | fooList | Foo | fooBar
```
keys: proposal(old, new, variableName) for each variable name (the proposal layer: name equal to the old class name -> new class name; name ends with the new class name -> unchanged; else the word-replacement algorithm)
```after
foo      -> bar
myFoo    -> myBar
fooList  -> barList
Foo      -> Bar
fooBar   -> (unchanged, dropped: the name already ends with Bar)
```

### REF-REN-111 case 2: common trailing word
settings: default
language: plain
```before
oldClass=FooService   newClass=BarService
variable names: fooService | myFooService | fooServices (variable of type List<FooService>)
```
keys: proposal(old, new, variableName) for each variable name
```after
fooService     -> barService
myFooService   -> myBarService
fooServices    -> barServices   (collection variable: singular fooService -> barService -> pluralized)
```

### REF-REN-111 case 3: insertion of a word
settings: default
language: plain
```before
oldClass=Foo   newClass=FooImpl
variable names: foo | myFoo | fooList
```
keys: proposal(old, new, variableName) for each variable name
```after
foo      -> fooImpl
myFoo    -> myFooImpl
fooList  -> fooImplList
```

### REF-REN-111 case 4: deletion of a word
settings: default
language: plain
```before
oldClass=AbstractFoo   newClass=Foo
variable names: abstractFoo | abstractFooList | myAbstractFooList
```
keys: proposal(old, new, variableName) for each variable name
```after
abstractFoo        -> (unchanged, dropped: the name already ends with Foo)
abstractFooList    -> fooList
myAbstractFooList  -> myFooList
```

### REF-REN-111 case 5: two replaced words
settings: default
language: plain
```before
oldClass=UserRepository   newClass=AccountStore
variable names: userRepository | myUserRepository | userRepositoryImpl | UserRepository
```
keys: proposal(old, new, variableName) for each variable name
```after
userRepository      -> accountStore
myUserRepository    -> myAccountStore
userRepositoryImpl  -> accountStoreImpl
UserRepository      -> AccountStore
```

### REF-REN-111 case 6: reordered words
settings: default
language: plain
```before
oldClass=FooBar   newClass=BarFoo
variable names: fooBar | myFooBar
```
keys: proposal(old, new, variableName) for each variable name
```after
fooBar    -> barFoo
myFooBar  -> myBarFoo
```

### REF-REN-111 case 7: camel humps with acronyms and digits
settings: default
language: plain
```before
oldClass=HTTPServer   newClass=WebServer
variable names: httpServer | myHTTPServer | server
```
keys: proposal(old, new, variableName) for each variable name
```after
httpServer    -> webServer
myHTTPServer  -> myWebServer
server        -> (not a candidate: the name does not contain HTTPServer)
```

### REF-REN-112 case 1: which variables are candidates
settings: default
language: java
```before
--- A.java
class Foo<caret> {}
class U {
    Foo foo;                       // candidate (type is Foo, name contains foo)
    Foo bar;                       // not a candidate (name does not contain foo)
    java.util.Set<Foo> fooSet;     // candidate (collection of Foo)
    Object fooObject;              // not a candidate (type is not Foo)
    Foo bar2 = null, myFoo;        // myFoo candidate, bar2 not
}
```
keys: action RenameElement, name "Baz", keep "Rename variables", Refactor
```after
Rename Variables dialog rows: foo -> baz, fooSet -> bazSet, myFoo -> myBaz
```

### REF-REN-113 case 1: inheritors across the hierarchy
settings: default
language: java
```before
interface Foo<caret> {}
class FooBase implements Foo {}
class MyFooImpl extends FooBase {}
class Unrelated implements Foo {}
```
keys: action RenameElement, name "Bar", keep "Rename inheritors", Refactor, OK in the dialog
```after
dialog rows: FooBase -> BarBase, MyFooImpl -> MyBarImpl     (Unrelated is not listed: no word Foo)
interface Bar {}
class BarBase implements Bar {}
class MyBarImpl extends BarBase {}
class Unrelated implements Bar {}
```

### REF-REN-121 case 1: Kotlin loop parameter renames in place
settings: default
language: kotlin
```before
fun f(list: List<String>) {
    for (it<caret>em in list) println(item)
}
```
keys: Shift+F6, type "s", Enter
```after
fun f(list: List<String>) {
    for (s<caret> in list) println(s)
}
```

### REF-REN-121 case 2: Kotlin local variable renames in place (member mode)
settings: default
language: kotlin
```before
fun f() {
    val x<caret> = 5
    println(x + x * x)
}
```
keys: Shift+F6, type "y", Enter
```after
fun f() {
    val y<caret> = 5
    println(y + y * y)
}
```

### REF-REN-121 case 3: top-level function from a caret on its name opens a dialog
settings: default
language: kotlin
```before
fun fo<caret>o(a: Int) {}
fun f() { foo(1); foo(217) }
```
keys: Shift+F6
```after
Rename dialog opens; label: Rename function 'foo' and its usages to:  (no in-place session)
```

### REF-REN-122 case 1: renaming the implicit lambda parameter
settings: default
language: kotlin
```before
fun f() {
    val g: (Int) -> Int = { <caret>it + it }
}
```
keys: Shift+F6, type "y", Enter
```after
fun f() {
    val g: (Int) -> Int = { y<caret> -> y + y }
}
```

### REF-REN-122 case 2: backticks are removed when no longer needed
settings: default
language: kotlin
```before
class A(val a: B)
class B(val b: Int)
fun x(`is`: A) {
    val `in`<caret> = `is`.a
    `in`.b
}
```
keys: Shift+F6, type "v", Enter
```after
class A(val a: B)
class B(val b: Int)
fun x(`is`: A) {
    val v<caret> = `is`.a
    v.b
}
```

### REF-REN-122 case 3: backticks are added for a keyword name
settings: default
language: kotlin
```before
fun f() {
    val x<caret> = 1
    println(x)
}
```
keys: Shift+F6, type "fun", Enter
```after
fun f() {
    val `fun`<caret> = 1
    println(`fun`)
}
```

### REF-REN-123 case 1: class rename renames the file and Java callers
settings: default
language: kotlin
```before
--- Foo.kt
class Fo<caret>o
--- Use.java
class Use { Object o = new Foo(); }
```
keys: action RenameElement, name "Bar", Refactor
```after
--- Bar.kt
class Bar
--- Use.java
class Use { Object o = new Bar(); }
```

### REF-REN-123 case 2: file rename renames the facade class
settings: default
language: kotlin
```before
--- util.kt   (selected in the Project view)
package p
fun f() {}
--- Use.java
package p;
class Use { void g() { UtilKt.f(); } }
```
keys: Project view: Rename File..., name "tools.kt", Refactor
```after
--- tools.kt
package p
fun f() {}
--- Use.java
package p;
class Use { void g() { ToolsKt.f(); } }
```

### REF-REN-124 case 1: override choice in Kotlin
settings: default
language: kotlin
```before
open class Base { open fun run() {} }
class Sub : Base() {
    override fun ru<caret>n() {}
}
```
keys: Shift+F6
```after
popup (member rename): title like "Function run overrides function run of Base"
rows: Rename base function
      Rename only current function
```

### REF-REN-125 case 1: Kotlin property rename updates Java accessor calls
settings: default
language: kotlin
```before
--- P.kt
class P(var na<caret>me: String)
--- U.java
class U { void f(P p) { p.setName(p.getName() + "!"); } }
```
keys: action RenameElement, name "title", Refactor
```after
--- P.kt
class P(var title: String)
--- U.java
class U { void f(P p) { p.setTitle(p.getTitle() + "!"); } }
```

### REF-REN-125 case 2: named arguments follow a parameter rename
settings: default
language: kotlin
```before
fun f(a<caret>: Int, b: Int) {}
fun g() { f(a = 1, b = 2) }
```
keys: action RenameElement, name "x", Refactor
```after
fun f(x: Int, b: Int) {}
fun g() { f(x = 1, b = 2) }
```

### REF-REN-126 case 1: Kotlin redeclaration conflict
settings: default
language: kotlin
```before
fun foo(a: Int) {}
fun ba<caret>r(a: Int) {}
```
keys: action RenameElement, name "foo", Refactor
```after
Problems Detected dialog body (shape): <function 'foo(Int)'> is already declared in <file/package>
(the message format is "{0} is already declared in {1}"; description wording of {0} and {1} is the Kotlin plugin's)
```

### REF-REN-127 case 1: renaming an operator function away from its convention
settings: default
language: kotlin
```before
class A { operator fun get<caret>(i: Int): Int = i }
fun f(a: A) = a[1]
```
keys: action RenameElement, name "item", Refactor
```after
class A { operator fun item(i: Int): Int = i }
fun f(a: A) = a.item(1)
```

### REF-REN-127 case 2: renaming a function to an operator name
settings: default
language: kotlin
```before
class A { operator fun fo<caret>o(i: Int): Int = i }
fun f(a: A) = a.foo(1)
```
keys: action RenameElement, name "get", Refactor
```after
class A { operator fun get(i: Int): Int = i }
fun f(a: A) = a[1]
```
