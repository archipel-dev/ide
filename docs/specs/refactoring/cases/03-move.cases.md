# Cases for 03-move.md

Notation as in `01-framework.cases.md`. Source roots are the directory roots of each case; blank
lines between `package` statements, imports and classes are not significant unless shown.
Messages containing `<...>` contain an element description whose kind word and name follow the
framework description rules (REF-FRM-024).

### REF-MOV-001 case 1: caret outside any movable element
settings: default
language: java
```before
package p;
<caret>
class A {}
```
keys: F6
```after
editor error hint:
Cannot perform refactoring.
Caret should be positioned at the class, method or field to be refactored
```

### REF-MOV-004 case 1: initial checkbox state
settings: factory settings
language: java
```before
package a;
public class Fo<caret>o {}
```
keys: F6
```after
dialog title: Move
label: Move class a.Foo
(o) To package   ( ) Make inner class of
To package: a
[x] Search in comments and strings
[x] Search for text occurrences
buttons: [Refactor] [Preview] [Cancel]
```

### REF-MOV-010 case 1: move a class to another package
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o {
    public static int N = 1;
}
--- a/Sibling.java
package a;
class Sibling { Foo f; }
--- c/User.java
package c;
import a.Foo;
public class User { int n = Foo.N; Foo f; }
```
keys: F6, To package: "b", Refactor
```after
--- b/Foo.java
package b;
public class Foo {
    public static int N = 1;
}
--- a/Sibling.java
package a;
import b.Foo;
class Sibling { Foo f; }
--- c/User.java
package c;
import b.Foo;
public class User { int n = Foo.N; Foo f; }
undo command name: Moving class Foo to package b
```

### REF-MOV-010 case 2: the moved class gets imports for the old package's classes
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o { Helper h; }
--- a/Helper.java
package a;
public class Helper {}
```
keys: F6, To package: "b", Refactor
```after
--- b/Foo.java
package b;
import a.Helper;
public class Foo { Helper h; }
--- a/Helper.java
package a;
public class Helper {}
```

### REF-MOV-010 case 3: destination package does not exist
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o {}
```
keys: F6, To package: "x.y", Refactor
```after
question dialog: Package x.y does not exist.
Do you want to create it?
buttons: [Yes] [No]
after Yes:
--- x/y/Foo.java
package x.y;
public class Foo {}
```

### REF-MOV-010 case 4: preview text for a package that will be created
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o {}
```
keys: F6, To package: "x.y", Preview
```after
preview tab "Refactoring Preview", targets node text mentions the class; the creation notice:
Package x.y does not exist.
The refactoring would create it later for you.
```

### REF-MOV-011 case 1: a class that shares its file moves into a new file
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Foo {}
class Hel<caret>per {}
```
keys: F6, To package: "b", Refactor
```after
--- a/Foo.java
package a;
public class Foo {}
--- b/Helper.java
package b;
class Helper {}
```

### REF-MOV-014 case 1: move a package under another package
settings: default
language: java
```before
--- src/a/util/Tool.java
package a.util;
public class Tool {}
--- src/a/Main.java
package a;
import a.util.Tool;
class Main { Tool t; }
```
keys: Project view: select directory src/a/util, F6, To package: "b", Refactor
```after
--- src/b/util/Tool.java
package b.util;
public class Tool {}
--- src/a/Main.java
package a;
import b.util.Tool;
class Main { Tool t; }
```

### REF-MOV-014 case 2: a package cannot move into itself
settings: default
language: java
```before
--- src/a/Tool.java
package a;
public class Tool {}
```
keys: select package a, F6, To package: "a.sub", Refactor
```after
error dialog (title Error) text: Cannot move package into itself
(no change)
```

### REF-MOV-017 case 1: make a class an inner class of another
settings: default
language: java
```before
--- p/Foo.java
package p;
public class Fo<caret>o {}
--- p/Holder.java
package p;
public class Holder {}
--- q/U.java
package q;
import p.Foo;
class U { Foo f; }
```
keys: F6, choose "Make inner class of", target class "p.Holder", Refactor
```after
--- p/Holder.java
package p;
public class Holder {
    public static class Foo {}
}
--- q/U.java
package q;
import p.Holder;
class U { Holder.Foo f; }
(p/Foo.java no longer exists; command name: Move class Foo to p.Holder)
```

### REF-MOV-017 case 2: invalid inner-class destinations
settings: default
language: java
```before
--- p/Foo.java
package p;
public class Fo<caret>o {
    static class Inside {}
}
--- p/Other.java
package p;
public class Other {
    class Member {}
}
```
keys: F6, "Make inner class of", type each target in turn
```after
"p.Foo"         -> error text: It is not allowed to move a class into itself
"p.Foo.Inside"  -> error text: It is not allowed to move a class into itself   (the target lies inside the moved class)
"p.Other.Member" -> error text: It is not allowed to move a class into a non-static inner class
"p.Missing"     -> error text: Destination class not found
```

### REF-MOV-018 case 1: invalid package name
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o {}
```
keys: F6, To package: "a..b"
```after
error text: 'a..b' is invalid destination package name
[Refactor] and [Preview] disabled
```

### REF-MOV-020 case 1: package-local class is no longer accessible
settings: default
language: java
```before
--- a/Foo.java
package a;
class Fo<caret>o {}
--- a/User.java
package a;
public class User { Foo f; }
```
keys: F6, To package: "b", Refactor
```after
Problems Detected dialog body:
Package-local class Foo will no longer be accessible from field User.f
```

### REF-MOV-021 case 1: the moved class uses a package-private class of the old package
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o { Helper h; }
--- a/Helper.java
package a;
class Helper {}
```
keys: F6, To package: "b", Refactor
```after
Problems Detected dialog body (shape):
<Field Foo.h> uses package-private <class Helper>
(Helper is package-private, so the import of it from package b would not compile)
```

### REF-MOV-022 case 1: a file with the same name exists in the destination
settings: default
language: java
```before
--- a/Foo.java
package a;
public class Fo<caret>o {}
--- b/Foo.java
package b;
public class Foo {}
```
keys: F6, To package: "b", Refactor
```after
error dialog (title Error):
Directory <path>/b 
already contains a file named 'Foo.java'
(no change)
```

### REF-MOV-030 case 1: instance member selected for Move Static Members
settings: default
language: java
```before
--- A.java
class A {
    int x;
    static void ok() {}
}
```
keys: Structure view: select field x and method ok(), F6
```after
error dialog title: Move Static Members
text:
Field x:int is not static.
Move Static Members refactoring is supported for static members only.
```

### REF-MOV-031 case 1: dialog validation
settings: default
language: java
```before
--- p/A.java
package p;
public class A { static int twice(int x) { return x * 2; } static int other; }
```
keys: caret on twice, F6, then each destination in turn, with the members table as given
```after
destination ""            -> error: No destination class specified
destination "a b"         -> error: 'a b' is not a legal FQ-name
destination "p.A"         -> error: Source and destination classes should be different
no member ticked          -> error: No members selected
destination "p.Missing"   -> on Refactor: question "Class p.Missing does not exist.\nDo you want to create it?" [Yes] [No]
```

### REF-MOV-032 case 1: move a static method, callers are rewritten
settings: default
language: java
```before
--- p/A.java
package p;
public class A {
    public static int tw<caret>ice(int x) { return x * 2; }
}
--- p/B.java
package p;
public class B {}
--- p/U.java
package p;
class U { int f() { return A.twice(2); } }
```
keys: F6, destination class "p.B", visibility As is, Refactor
```after
--- p/A.java
package p;
public class A {
}
--- p/B.java
package p;
public class B {
    public static int twice(int x) { return x * 2; }
}
--- p/U.java
package p;
class U { int f() { return B.twice(2); } }
undo command name: Move method twice(int)
```

### REF-MOV-032 case 2: static imports follow the member
settings: default
language: java
```before
--- p/A.java
package p;
public class A {
    public static int tw<caret>ice(int x) { return x * 2; }
}
--- p/B.java
package p;
public class B {}
--- q/U.java
package q;
import static p.A.twice;
class U { int f() { return twice(2); } }
```
keys: F6, destination class "p.B", Refactor
```after
--- q/U.java
package q;
import static p.B.twice;
class U { int f() { return twice(2); } }
```

### REF-MOV-033 case 1: a non-constant field cannot move to an interface
settings: default
language: java
```before
--- A.java
class A { static int coun<caret>ter = 0; }
interface I {}
```
keys: F6, destination "I", Refactor
```after
Problems Detected dialog body (shape):
Non-constant <field A.counter> will not be compilable when moved to an interface.
```

### REF-MOV-034 case 1: a moved member uses a private member of the old class
settings: default
language: java
```before
--- A.java
class A {
    private static int secret = 1;
    static int use<caret>It() { return secret; }
}
class B {}
```
keys: F6, destination "B", Refactor
```after
Problems Detected dialog body (shape):
<Field A.secret> is private and will not be accessible from <method useIt()> in the target class.
```

### REF-MOV-035 case 1: move an instance method to the class of a parameter
settings: default
language: java
```before
class A {
    int base = 1;
    int su<caret>m(B b) { return b.v + base; }
}
class B { int v; }
class U { int f(A a, B b) { return a.sum(b); } }
```
keys: F6; dialog "Move Instance Method": list `Select an instance expression:` with row `b` selected; name field `Select a name for 'A.this' parameter` = a; Refactor
```after
class A {
    int base = 1;
}
class B {
    int v;
    int sum(A a) { return v + a.base; }
}
class U { int f(A a, B b) { return b.sum(a); } }
```

### REF-MOV-037 case 1: constructor
settings: default
language: java
```before
class A { A<caret>() {} }
```
keys: F6 with the caret in the constructor name
```after
error hint: Cannot perform refactoring.
Move method is not supported for constructors
```

### REF-MOV-037 case 2: no parameter with a reference type
settings: default
language: java
```before
class A { int su<caret>m(int a) { return a; } }
```
keys: F6
```after
error hint: Cannot perform refactoring.
There are no method parameters or containing class fields that have reference type
```

### REF-MOV-040 case 1: static nested class chooser
settings: default
language: java
```before
class Outer {
    static class In<caret>ner {}
}
```
keys: F6
```after
dialog title: Select Refactoring
label: What would you like to do?
(o) Move inner class Inner to upper level
( ) Move inner class Inner to another class
buttons: [OK] [Cancel]
```

### REF-MOV-041 case 1: Move Inner to Upper Level dialog for a non-static class
settings: default
language: java
```before
package p;
public class Outer {
    class In<caret>ner {}
}
```
keys: F6
```after
dialog title: Move Inner to Upper Level
Class name: Inner
Destination package: p
[x] Pass outer class' instance as a parameter    Parameter name: outer
[x] Search in comments and strings
[x] Search for text occurrences
buttons: [Refactor] [Preview] [Cancel]
errors: Class name "" -> "No class name specified"; parameter name "" -> "No parameter name specified"
```

### REF-MOV-042 case 1: static nested class becomes top level
settings: default
language: java
```before
--- p/Outer.java
package p;
public class Outer {
    public static class In<caret>ner { int v; }
    Inner make() { return new Inner(); }
}
```
keys: F6, choose "Move inner class Inner to upper level", Destination package "p", Refactor
```after
--- p/Inner.java
package p;
public class Inner { int v; }
--- p/Outer.java
package p;
public class Outer {
    Inner make() { return new Inner(); }
}
```

### REF-MOV-042 case 2: non-static inner class receives the outer instance
settings: default
language: java
```before
--- p/Outer.java
package p;
public class Outer {
    int count;
    class In<caret>ner {
        int next() { return count + 1; }
    }
    Inner make() { return new Inner(); }
}
```
keys: F6, Destination package "p", keep "Pass outer class' instance as a parameter" with name "outer", Refactor
```after
--- p/Inner.java
package p;
class Inner {
    private final Outer outer;
    Inner(Outer outer) { this.outer = outer; }
    int next() { return outer.count + 1; }
}
--- p/Outer.java
package p;
public class Outer {
    int count;
    Inner make() { return new Inner(this); }
}
```

### REF-MOV-050 case 1: Move Files dialog
settings: default
language: plain
```before
--- Project view selection: docs/a.txt, docs/b.txt
```
keys: F6
```after
dialog title: Move
label: Move specified files
To directory: <field>  (hint: Use Ctrl+Space for path completion)
[x] Search for references
[ ] Open in editor
buttons: [Refactor] [Preview] [Cancel]
```

### REF-MOV-060 case 1: Kotlin class moves to another package
settings: default
language: kotlin
```before
--- src/a/Foo.kt
package a

class Fo<caret>o
--- src/a/Use.kt
package a

fun use(f: Foo) {}
--- src/c/Other.kt
package c

import a.Foo

fun other(f: Foo) {}
```
keys: F6, Move to package "b" with "Update package directive (Kotlin files)" ticked, Refactor
```after
--- src/b/Foo.kt
package b

class Foo
--- src/a/Use.kt
package a

import b.Foo

fun use(f: Foo) {}
--- src/c/Other.kt
package c

import b.Foo

fun other(f: Foo) {}
undo command name: Move Declarations
```

### REF-MOV-060 case 2: a top-level function moves to an existing file
settings: default
language: kotlin
```before
--- p/A.kt
package p

fun hel<caret>per() = 1

fun other() = helper()
--- p/B.kt
package p

fun b() {}
```
keys: F6, Move to file "B.kt" (destination file), Refactor
```after
--- p/A.kt
package p

fun other() = helper()
--- p/B.kt
package p

fun b() {}

fun helper() = 1
```

### REF-MOV-061 case 1: target must be a Kotlin file
settings: default
language: kotlin
```before
--- p/A.kt
package p

fun hel<caret>per() = 1
--- p/B.java
package p;
class B {}
```
keys: F6, Move to file "B.java"
```after
error text: Target must be a Kotlin file
```

### REF-MOV-062 case 1: companion objects cannot be moved
settings: default
language: kotlin
```before
class A {
    companion obj<caret>ect {}
}
```
keys: F6
```after
editor error hint:
Cannot perform refactoring.
Move declaration is not supported for companion objects
```

### REF-MOV-062 case 2: open members cannot be moved
settings: default
language: kotlin
```before
open class A { open fun f<caret>oo() {} }
```
keys: F6
```after
editor error hint:
Cannot perform refactoring.
Move is not supported for 'abstract', 'open' and 'override' members
```
