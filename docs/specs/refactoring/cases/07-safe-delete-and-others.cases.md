# Cases for 07-safe-delete-and-others.md

Notation as in `01-framework.cases.md`.

### REF-SDL-001 case 1: nothing deletable at the caret
settings: default
language: java
```before
class A { // com<caret>ment
}
```
keys: Alt+Delete
```after
editor error hint:
Cannot perform refactoring.
Safe Delete is not supported in current context
```

### REF-SDL-002 case 1: delete confirmation with the safe delete checkbox
settings: default
language: java
```before
--- Project view selection: Foo.java (class Foo)
```
keys: Delete
```after
dialog: Delete class Foo?
[x] Safe delete (with usage search)
[x] Search in comments and strings   [x] Search for text occurrences
```

### REF-SDL-003 case 1: Safe Delete dialog
settings: default
language: java
```before
class A { void fo<caret>o() {} }
```
keys: Alt+Delete
```after
dialog title: Safe Delete
message: Delete method foo()?
[x] Search in comments and strings
[x] Search for text occurrences
```

### REF-SDL-004 case 1: no usages, deleted at once
settings: default
language: java
```before
class A {
    void us<caret>ed() {}
    void other() {}
}
```
keys: Alt+Delete, OK
```after
class A {
    void other() {}
}
undo command name: Deleting method used()
```

### REF-SDL-004 case 2: usage in a comment only
settings: default
language: java
```before
class A {
    // see helper
    void hel<caret>per() {}
}
```
keys: Alt+Delete, OK
```after
class A {
    // see helper
}
(the comment occurrence is not changed and not a conflict)
```

### REF-SDL-004 case 3: deleting a class deletes its file
settings: default
language: java
```before
--- Foo.java
class Fo<caret>o {}
--- Other.java
class Other {}
```
keys: Alt+Delete, OK
```after
--- Other.java
class Other {}
(Foo.java no longer exists)
```

### REF-SDL-005 case 1: unsafe usage
settings: default
language: java
```before
class A {
    void fo<caret>o() {}
    void g() { foo(); }
}
```
keys: Alt+Delete, OK
```after
Problems Detected dialog body (shape):
<method A.foo()> has 1 usage that is not safe to delete.
buttons: [Continue] [View Usages] [Cancel]
View Usages: Find tab "Safe Delete Conflicts", node "Attempting to delete", node "Potentially unsafe to delete references found in code (1)", tab buttons [Rerun Safe Delete] [Delete Anyway]
```

### REF-SDL-006 case 1: imports and extends
settings: default
language: java
```before
--- p/Foo.java
package p;
public class Fo<caret>o {}
--- q/User.java
package q;
import p.Foo;
class User {}
```
keys: Alt+Delete, OK
```after
--- q/User.java
package q;
class User {}
```

### REF-SDL-006 case 2: write-only field usage is deleted
settings: default
language: java
```before
class A {
    int co<caret>unt;
    void f() { count = 5; }
}
```
keys: Alt+Delete, OK
```after
class A {
    void f() { }
}
```

### REF-SDL-006 case 3: local variable keeps side effects
settings: default
language: java
```before
class A {
    int next() { return 1; }
    void f() {
        int un<caret>used = next();
    }
}
```
keys: Alt+Delete, OK
```after
class A {
    int next() { return 1; }
    void f() {
        next();
    }
}
```

### REF-SDL-006 case 4: method implementing an abstract method
settings: default
language: java
```before
interface I { void run(); }
class A implements I {
    public void ru<caret>n() {}
}
```
keys: Alt+Delete
```after
Problems Detected dialog body (shape):
<Method A.run()> implements <method I.run()>.
```

### REF-SDL-007 case 1: unused overriding methods
settings: default
language: java
```before
class Base { void fo<caret>o() {} }
class Sub extends Base { @Override void foo() {} }
```
keys: Alt+Delete, OK
```after
dialog title: Unused Overriding Methods
text: There are unused methods that override the methods that are going to be deleted. Choose the unused overriding methods you want to delete as well:
list (all ticked): Sub.foo()
after OK:
class Base { }
class Sub extends Base { }
```

### REF-SDL-008 case 1: getter and setter of a field
settings: default
language: java
```before
class A {
    private int cou<caret>nt;
    public int getCount() { return count; }
    public void setCount(int c) { count = c; }
}
```
keys: Alt+Delete, OK, answer Yes
```after
question: Getter and setter methods found for the field count. 
Delete them as well?   [Yes] [No]
class A {
}
```

### REF-SDL-009 case 1: unused parameter
settings: default
language: java
```before
class A {
    void f(int a, int b<caret>) { System.out.println(a); }
    void g() { f(1, 2); }
}
```
keys: Alt+Delete, OK
```after
class A {
    void f(int a) { System.out.println(a); }
    void g() { f(1); }
}
```

### REF-SDL-009 case 2: parameter in a method hierarchy
settings: default
language: java
```before
interface I { void put(int k<caret>); }
class A implements I { public void put(int k) {} }
```
keys: Alt+Delete
```after
question: <method I.put(int)> is a part of method hierarchy. Do you want to delete multiple parameters?
buttons: [Yes] [No] [Cancel]
after Yes and OK:
interface I { void put(); }
class A implements I { public void put() {} }
```

### REF-SDL-012 case 1: Kotlin constructor property
settings: default
language: kotlin
```before
class A(val na<caret>me: String)
fun f() { A("x") }
```
keys: Alt+Delete, OK
```after
class A()
fun f() { A() }
```

### REF-SDL-012 case 2: Kotlin unused function
settings: default
language: kotlin
```before
fun us<caret>ed() {}
fun other() {}
```
keys: Alt+Delete, OK
```after
fun other() {}
```

### REF-OTH-001 case 1: pull up dialog
settings: default
language: java
```before
class Base {}
class Sub extends Base {
    int x;
    void m<caret>() {}
}
```
keys: action MembersPullUp
```after
dialog title: Pull Members Up
label: Pull up members of Sub to:   combo: Base
table "Members to be pulled up": [ ] x : int | [x] m() [Make Abstract: [ ]]   (m is ticked: invoked on it)
JavaDoc for abstracts: (o) As is  ( ) Copy  ( ) Move
buttons: [Refactor] [Preview] [Cancel]
```

### REF-OTH-001 case 2: pull up a method
settings: default
language: java
```before
class Base {}
class Sub extends Base {
    void m<caret>() {}
}
```
keys: action MembersPullUp, Refactor
```after
class Base {
    void m() {}
}
class Sub extends Base {
}
```

### REF-OTH-002 case 1: pull up as abstract
settings: default
language: java
```before
abstract class Base {}
class Sub extends Base {
    void m<caret>() { System.out.println(); }
}
```
keys: action MembersPullUp, tick "Make Abstract" for m, Refactor
```after
abstract class Base {
    abstract void m();
}
class Sub extends Base {
    void m() { System.out.println(); }
}
```

### REF-OTH-003 case 1: member uses something not accessible from the superclass
settings: default
language: java
```before
class Base {}
class Sub extends Base {
    private int hidden;
    int m<caret>() { return hidden; }
}
```
keys: action MembersPullUp, Refactor
```after
Problems Detected dialog body (shape):
<Method Sub.m()> uses <field Sub.hidden>, which is not moved to the superclass
```

### REF-OTH-010 case 1: push down
settings: default
language: java
```before
class Base {
    void m<caret>() {}
}
class S1 extends Base {}
class S2 extends Base {}
```
keys: action MemberPushDown, Refactor
```after
dialog: Push Members Down; label: Push members from Base down; table "Members to Be Pushed Down": m()
class Base {
}
class S1 extends Base {
    void m() {}
}
class S2 extends Base {
    void m() {}
}
```

### REF-OTH-010 case 2: no inheritors
settings: default
language: java
```before
class Base {
    void m<caret>() {}
}
```
keys: action MemberPushDown
```after
editor error hint:
Cannot perform refactoring.
Class Base does not have inheritors
```

### REF-OTH-020 case 1: extract interface
settings: default
language: java
```before
class Foo<caret> {
    public void a() {}
    public void b() {}
}
class U { void f(Foo foo) { foo.a(); } }
```
keys: action ExtractInterface, Interface name "IFoo", tick member a(), Refactor, confirm the usage preview if asked
```after
dialog title: Extract Interface
Extract interface from: Foo      Interface name: IFoo      Package for new interface: <current package>
Members to form interface: [x] a()  [ ] b()
[x] Preview usages to be changed
--- IFoo.java
public interface IFoo {
    void a();
}
class Foo implements IFoo {
    public void a() {}
    public void b() {}
}
(U keeps Foo unless "Rename original class" is chosen)
```

### REF-OTH-021 case 1: extract superclass
settings: default
language: java
```before
class Foo<caret> {
    int x;
    void a() {}
}
```
keys: action ExtractSuperclass, Superclass name "Base", tick x, Refactor
```after
--- Base.java
class Base {
    int x;
}
class Foo extends Base {
    void a() {}
}
```

### REF-OTH-030 case 1: encapsulate a field
settings: default
language: java
```before
class A {
    public int co<caret>unt;
}
class U { void f(A a) { a.count = a.count + 1; int c = a.count; } }
```
keys: action EncapsulateFields, keep Getter and Setter ticked, field visibility Private, accessors Public, Refactor
```after
class A {
    private int count;

    public int getCount() {
        return count;
    }

    public void setCount(int count) {
        this.count = count;
    }
}
class U { void f(A a) { a.setCount(a.getCount() + 1); int c = a.getCount(); } }
```

### REF-OTH-030 case 2: interfaces are refused
settings: default
language: java
```before
interface I<caret> { int X = 1; }
```
keys: action EncapsulateFields
```after
editor error hint (message line): Encapsulate fields refactoring cannot be applied to interface
```

### REF-OTH-040 case 1: copy a class to another package
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o {
    Foo self;
}
```
keys: F5, New name "Bar", Destination package "b", Refactor
```after
--- b/Bar.java
package b;
public class Bar {
    Bar self;
}
(a/Foo.java is unchanged)
```

### REF-OTH-040 case 2: clone dialog
settings: default
language: plain
```before
--- Project view selection: notes.txt
```
keys: F5, destination = the same directory
```after
dialog title: Clone
label: Clone file notes.txt
New name: notes.txt
```

### REF-OTH-041 case 1: convert an anonymous class
settings: default
language: java
```before
class A {
    void f(int n) {
        Runnable r = new Run<caret>nable() {
            public void run() { System.out.println(n); }
        };
    }
}
```
keys: action Move on the anonymous class (menu text: Convert Anonymous to Inner...), Class name "Printer", Refactor
```after
dialog: Convert Anonymous to Inner; Class name: <field>; [ ] Make class static; table "Constructor Parameters": [x] n : int
class A {
    void f(int n) {
        Runnable r = new Printer(n);
    }

    private class Printer implements Runnable {
        private final int n;

        private Printer(int n) {
            this.n = n;
        }

        public void run() { System.out.println(n); }
    }
}
```

### REF-OTH-050 case 1: make static
settings: default
language: java
```before
class A {
    int base = 1;
    int plu<caret>s(int x) { return x + base; }
    int g() { return plus(2); }
}
```
keys: action MakeStatic, tick "Add object as a parameter with name:" = "a", Refactor
```after
class A {
    int base = 1;
    static int plus(A a, int x) { return x + a.base; }
    int g() { return plus(this, 2); }
}
```

### REF-OTH-052 case 1: invert boolean method
settings: default
language: java
```before
class A {
    boolean isRea<caret>dy() { return true; }
    void f() { if (isReady()) System.out.println(); }
}
```
keys: action InvertBoolean, new name "isBusy", Refactor
```after
class A {
    boolean isBusy() { return false; }
    void f() { if (!isBusy()) System.out.println(); }
}
```

### REF-OTH-070 case 1: type migration of a field
settings: default
language: java
```before
import java.util.ArrayList;
import java.util.List;
class A {
    List<String> it<caret>ems = new ArrayList<>();
    int n() { return items.size(); }
}
```
keys: Ctrl+Shift+F6, type "ArrayList<String>", Migrate
```after
dialog label: Migrate type List<String> of field items to
dialog scope: Choose scope where change signature may occur
buttons: [Migrate] [Cancel]
import java.util.ArrayList;
import java.util.List;
class A {
    ArrayList<String> items = new ArrayList<>();
    int n() { return items.size(); }
}
```
