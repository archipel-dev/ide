# Cases for 01-framework.md

Notation: `<caret>` is the caret; `--- path` separates files of a multi-file case; dialog and
popup contents are given in display order, one row per line.

### REF-FRM-001 case 1: Refactor menu with the caret on a local variable
settings: default
language: java
```before
class A {
    void f() {
        int count = 1;
        int y = <caret>count + 1;
    }
}
```
keys: open main menu Refactor
```after
Refactor This...                    Ctrl+Alt+Shift+T   enabled
Rename...                           Shift+F6           enabled
Change Signature...                 Ctrl+F6            disabled
---
Extract/Introduce  >                                   enabled
Inline...                           Ctrl+Alt+N         enabled
---
Move...                             F6                 disabled
Copy...                             F5                 disabled
Safe Delete...                      Alt+Delete         enabled
---
Pull Members Up...                                     disabled
Push Members Down...                                   disabled
Type Migration...                   Ctrl+Shift+F6      enabled
Make Static...                                         disabled
Convert To Instance Method...                          disabled
Encapsulate Fields...                                  disabled
Invert Boolean...                                      enabled
```

### REF-FRM-001 case 2: Extract/Introduce submenu in Java
settings: default
language: java
```before
class A {
    void f() { int y = <caret>1 + 2; }
}
```
keys: open main menu Refactor, open submenu Extract/Introduce
```after
Introduce Variable...               Ctrl+Alt+V         enabled
Introduce Constant...               Ctrl+Alt+C         enabled
Introduce Field...                  Ctrl+Alt+F         enabled
Introduce Parameter...              Ctrl+Alt+P         enabled
---
Introduce Functional Parameter...   Ctrl+Shift+Alt+P   disabled
Introduce Functional Variable...                       disabled
Introduce Parameter Object...                          disabled
---
Extract Method...                   Ctrl+Alt+M         enabled
Replace Method With Method Object...                   enabled
---
Extract Delegate...                                    disabled
Extract Interface...                                   disabled
Extract Superclass...                                  disabled
```

### REF-FRM-001 case 3: Extract/Introduce submenu in Kotlin
settings: default
language: kotlin
```before
fun f() { val y = <caret>1 + 2 }
```
keys: open main menu Refactor, open submenu Extract/Introduce
```after
Introduce Variable...               Ctrl+Alt+V         enabled
Introduce Constant...               Ctrl+Alt+C         enabled
Introduce Property...               Ctrl+Alt+F         enabled
Introduce Parameter...              Ctrl+Alt+P         enabled
Introduce Type Alias...             Ctrl+Alt+Shift+A   disabled
---
Extract Function...                 Ctrl+Alt+M         enabled
Extract Function to Scope...        Ctrl+Alt+Shift+M   enabled
```

### REF-FRM-003 case 1: library class member hides refactorings
settings: default
language: java
```before
--- decompiled: java/lang/String.class
public final class String {
    public int length<caret>() { ... }
}
```
keys: open main menu Refactor
```after
Extract Method...      hidden
Introduce Variable...  hidden
Move...                hidden
Rename...              enabled   (invoking it shows the hint: Cannot perform refactoring.\nSelected method is not located inside the project)
```

### REF-FRM-004 case 1: Move action text follows the selection
settings: default
language: java
```before
--- Project view selection
src/p/A.java (class A), src/p/B.java (class B)
```
keys: right-click, open the Refactor submenu
```after
Move Classes...
Copy Classes...
Rename...      (disabled: more than one element)
Safe Delete...
```

### REF-FRM-005 case 1: no handler at the caret
settings: default
language: java
```before
class A { // comment<caret>
}
```
keys: Shift+F6 with the caret inside a comment
```after
editor hint (red balloon, no dialog):
Cannot perform refactoring.
Caret should be positioned at symbol to be renamed
```

### REF-FRM-006 case 1: Refactor This popup on a field name
settings: default
language: java
```before
class A {
    private int cou<caret>nt = 0;
    int get() { return count; }
}
```
keys: Ctrl+Alt+Shift+T
```after
popup title: Refactor This
Rename...                           Shift+F6
Type Migration...                   Ctrl+Shift+F6
---
Extract/Introduce
Introduce Parameter Object...
---
Inline...                           Ctrl+Alt+N
Move...                             F6
Safe Delete...                      Alt+Delete
Encapsulate Fields...
(no disabled rows; items disabled in the Refactor menu are absent)
```

### REF-FRM-007 case 1: starting another refactoring during an in-place rename
settings: default
language: java
```before
class A { void f() { int x<caret> = 1; int y = x; } }
```
keys: Shift+F6, type "z", then Ctrl+Alt+V
```after
dialog (modal, buttons in order):
text: Command Rename has not finished yet
[Continue Started] [Abandon Started]
(Abandon Started: editor text returns to "int x = 1;" and the Introduce Variable flow starts)
```

### REF-FRM-020 case 1: legacy conflicts dialog contents
settings: default
language: java
```before
class A {
    void foo(int a) {}
    void ba<caret>r(int a) {}
}
```
keys: Shift+F6 on bar, switch to dialog (Shift+F6 again), type "foo", Enter (Refactor)
```after
dialog title: Problems Detected
label: The following problems were found:
body:
Method foo(int) is already defined in class A
label: Do you wish to ignore them and continue?
buttons (left to right): [Continue] [View Usages] [Cancel]
default button (Enter): Cancel
```

### REF-FRM-020 case 2: Continue ignores the conflicts
settings: default
language: java
```before
class A {
    void foo(int a) {}
    void ba<caret>r(int a) {}
    void use() { bar(1); }
}
```
keys: action RenameElement, in dialog set name "foo", press Refactor, in conflicts dialog press Continue
```after
class A {
    void foo(int a) {}
    void foo(int a) {}
    void use() { foo(1); }
}
```

### REF-FRM-020 case 3: more than 20 conflicts are collapsed
settings: default
language: java
```before
class A {
    int x<caret>;
    // 25 other classes B1..B25 each contain "int y" and a reference b.x, plus a local named y in a method using x
}
```
keys: rename field x to y (Shift+F6 twice, type "y", Refactor)
```after
conflicts dialog body shows exactly 20 messages separated by blank lines, then the link: Show more...
click the link: all 25 messages are shown (no link)
```

### REF-FRM-021 case 1: View Usages aborts and opens the preview tab
settings: default
language: java
```before
class A {
    void foo(int a) {}
    void ba<caret>r(int a) {}
}
```
keys: Shift+F6 twice, type "foo", Refactor, click View Usages
```after
conflicts dialog closed; code unchanged (bar is still named bar)
Find tool window tab: Refactoring Preview
  node "References in code" containing the element: method foo(int) in class A
buttons of the tab: [Refactor] [Rerun] [Cancel]
```

### REF-FRM-022 case 1: updated conflicts dialog (registry on)
settings: registry refactorings.use.updated.conflicts.detected.dialog=true
language: java
```before
class A {
    void foo(int a) {}
    void ba<caret>r(int a) {}
}
```
keys: Shift+F6 twice, type "foo", Refactor
```after
dialog title: Conflicts Detected
toolbar right label: 1 conflicts
tree: file A.java > "Method foo(int) is already defined in class A"
preview pane: shows the line "void foo(int a) {}" with title "A.java  src/A.java"
buttons: [Refactor Anyway] [Cancel]   left side: [Show Conflicts in View]
focus: the tree
```

### REF-FRM-030 case 1: preview with excluded usage
settings: default
language: java
```before
--- A.java
class A {
    void fo<caret>o() {}
}
--- B.java
class B {
    void g(A a) { a.foo(); a.foo(); }
}
```
keys: action RenameElement (dialog), name "bar", press Preview, in the tab exclude the second usage in B.g, press Refactor
```after
--- A.java
class A {
    void bar() {}
}
--- B.java
class B {
    void g(A a) { a.bar(); a.foo(); }
}
```

### REF-FRM-031 case 1: read-only usage forces preview
settings: default
language: java
```before
--- A.java
class A { void fo<caret>o() {} }
--- ReadOnlyUser.java   (file marked read-only, VCS cannot make it writable)
class R { void g(A a) { a.foo(); } }
```
keys: Shift+F6 twice, type "bar", Refactor
```after
the preview tab "Refactoring Preview" opens instead of a direct change
status bar text: Occurrences found in read-only files
the usage in ReadOnlyUser.java is shown in red
after pressing Refactor in the tab: A.java has bar(); ReadOnlyUser.java still calls foo()
```

### REF-FRM-035 case 1: search in comments and strings and text occurrences
settings: rename defaults for class (comments=off, text=on)
language: java
```before
--- Foo.java
/** See Foo and FooBar. */
class <caret>Foo { String n = "Foo"; }   // Foo here
--- config.properties
type=Foo
other=FooBar
```
keys: Shift+F6 twice, type "Baz", tick "Search in comments and strings" (Alt+C), keep "Search for text occurrences" ticked, Refactor
```after
--- Baz.java   (file renamed too)
/** See Baz and FooBar. */
class Baz { String n = "Foo"; }   // Baz here
--- config.properties
type=Baz
other=FooBar
```

### REF-FRM-040 case 1: one Undo reverts everything
settings: default
language: java
```before
--- A.java
class A<caret> {}
--- B.java
class B { A a; }
--- C.java
class C { A a; }
```
keys: Shift+F6 twice, type "X", Refactor, then action Undo once
```after
--- A.java
class A {}
--- B.java
class B { A a; }
--- C.java
class C { A a; }
(the file is named A.java again)
```

### REF-FRM-045 case 1: redundant import removed after a move
settings: default
language: java
```before
--- a/X.java
package a;
public class <caret>X {}
--- b/Y.java
package b;
import a.X;
public class Y { X x; }
```
keys: action Move, destination package "b", Refactor
```after
--- b/X.java
package b;
public class X {}
--- b/Y.java
package b;
public class Y { X x; }
```

### REF-FRM-051 case 1: in-place rename of a local variable
settings: default
language: java
```before
class A {
    void f() {
        int co<caret>unt = 1;
        System.out.println(count + count);
    }
}
```
keys: Shift+F6, type "n" (the whole name was preselected), Enter
```after
class A {
    void f() {
        int n<caret> = 1;
        System.out.println(n + n);
    }
}
```

### REF-FRM-051 case 2: preselect off puts the caret at the end
settings: Editor | General | Refactorings: Preselect current symbol name = off
language: java
```before
class A {
    void f() {
        int co<caret>unt = 1;
        System.out.println(count);
    }
}
```
keys: Shift+F6, type "2", Enter
```after
class A {
    void f() {
        int count2<caret> = 1;
        System.out.println(count2);
    }
}
```

### REF-FRM-052 case 1: Escape restores the old name
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
keys: Shift+F6, type "zzz", Escape
```after
class A {
    void f() {
        int count = 1;
        System.out.println(count);
    }
}
(undo stack is as before Shift+F6; no refactoring ran)
```

### REF-FRM-052 case 2: invalid identifier keeps the session
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
keys: Shift+F6, type "1x", Enter
```after
class A {
    void f() {
        int 1x = 1;
        System.out.println(1x);
    }
}
(template session still active, field marked as error; no refactoring ran; Escape then restores "count")
```

### REF-FRM-052 case 3: unchanged name just ends the session
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
keys: Shift+F6, Enter
```after
class A {
    void f() {
        int count<caret> = 1;
        System.out.println(count);
    }
}
(no command was added to the undo stack)
```

### REF-FRM-053 case 1: options popup of an in-place member rename
settings: default
language: java
```before
class A {
    private void fo<caret>o() {}
    void g() { foo(); }
}
```
keys: Shift+F6, Tab
```after
inlay tooltip: Choose where to rename occurrences in addition to usages:
– In comments and string literals
– In files that don’t contain source code

Use Alt+Shift+O to navigate to these options     (shortcut text of the Tab/Alt+Shift+O action)
popup title: Also rename in:
[ ] Comments and strings
[ ] Text occurrences        (defaults for methods: comments off, text occurrences off)
More options...
```

### REF-FRM-060 case 1: suggested rename after typing a new name
settings: default
language: java
```before
class A {
    void fo<caret>o() {}
    void g() { foo(); }
}
```
keys: type "x" after "fo" so that the declaration reads "foxo", wait for the daemon, click the gutter icon
```after
gutter tooltip: Rename usages of 'foo' to 'foxo'... Alt+Enter
popup rows: [Rename] [Rename with Preview] [Ignore]
choosing Rename:
class A {
    void foxo() {}
    void g() { foxo(); }
}
```
