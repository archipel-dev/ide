# Cases for 04 folding

Fold marker notation: `<fold text="P">...</fold>` is a collapsed region with placeholder P,
`<fold text="P" expanded>...</fold>` an expanded one.

### ED-FOLD-030 case 1: two imports are folded on open
settings: default
language: java
```before
import java.util.List;
import java.util.Map;

class A {}
```
keys: open file
```after
import <fold text="...">java.util.List;
import java.util.Map;</fold>

class A {}
```

### ED-FOLD-030 case 2: a single import is not folded
settings: default
language: java
```before
import java.util.List;

class A {}
```
keys: open file
```after
import java.util.List;

class A {}
```

### ED-FOLD-030 case 3: imports setting off
settings: Imports=off
language: java
```before
import java.util.List;
import java.util.Map;

class A {}
```
keys: open file
```after
import <fold text="..." expanded>java.util.List;
import java.util.Map;</fold>

class A {}
```

### ED-FOLD-033 case 1: a simple one-statement method is folded to one line
settings: default
language: java
```before
class A {
    int f() {
        return 1;
    }
}
```
keys: open file
```after
class A {
    int f()<fold text=" { "> {
        </fold>return 1;<fold text=" }">
    }</fold>
}
```

### ED-FOLD-033 case 2: a method with two statements has an expanded body region
settings: default
language: java
```before
class A {
    void f() {
        a();
        b();
    }
}
```
keys: open file
```after
class A {
    void f() <fold text="{...}" expanded>{
        a();
        b();
    }</fold>
}
```

### ED-FOLD-033 case 3: method bodies setting on collapses them
settings: Method bodies=on
language: java
```before
class A {
    void f() {
        a();
        b();
    }
}
```
keys: open file
```after
class A {
    void f() <fold text="{...}">{
        a();
        b();
    }</fold>
}
```

### ED-FOLD-033 case 4: an empty body has no region
settings: default
language: java
```before
class A {
    void f() {}
}
```
keys: open file
```after
class A {
    void f() {}
}
```

### ED-FOLD-033 case 5: one-line methods setting off
settings: one-line methods=off
language: java
```before
class A {
    int f() {
        return 1;
    }
}
```
keys: open file
```after
class A {
    int f() <fold text="{...}" expanded>{
        return 1;
    }</fold>
}
```

### ED-FOLD-032 case 1: a nested class has a region, the outer single class has none
settings: default
language: java
```before
class A {
    class B {
        int x;
    }
}
```
keys: open file
```after
class A {
    class B <fold text="{...}" expanded>{
        int x;
    }</fold>
}
```

### ED-FOLD-035 case 1: documentation comments are expanded by default
settings: default
status: unverified (blanks inside the placeholder)
language: java
```before
class A {
    /**
     * Hello.
     * More.
     */
    void f() {}
}
```
keys: open file
```after
class A {
    <fold text="/** Hello. ... */" expanded>/**
     * Hello.
     * More.
     */</fold>
    void f() {}
}
```

### ED-FOLD-035 case 2: documentation comments setting on
settings: Documentation comments=on
status: unverified (blanks inside the placeholder)
language: java
```before
class A {
    /**
     * Hello.
     * More.
     */
    void f() {}
}
```
keys: open file
```after
class A {
    <fold text="/** Hello. ... */">/**
     * Hello.
     * More.
     */</fold>
    void f() {}
}
```

### ED-FOLD-040 case 1: a Visual Studio style custom region
settings: default
language: java
```before
class A {
    // region Helpers
    void f() {}
    // endregion
}
```
keys: open file
```after
class A {
    <fold text="Helpers" expanded>// region Helpers
    void f() {}
    // endregion</fold>
}
```

### ED-FOLD-040 case 2: custom regions collapsed by default setting
settings: Custom folding regions=on
language: java
```before
class A {
    // region Helpers
    void f() {}
    // endregion
}
```
keys: open file
```after
class A {
    <fold text="Helpers">// region Helpers
    void f() {}
    // endregion</fold>
}
```

### ED-FOLD-040 case 3: an empty description gives the placeholder ...
settings: default
language: java
```before
class A {
    // region
    void f() {}
    // endregion
}
```
keys: open file
```after
class A {
    <fold text="..." expanded>// region
    void f() {}
    // endregion</fold>
}
```

### ED-FOLD-040 case 4: an unmatched region start makes no region
settings: default
language: java
```before
class A {
    // region Helpers
    void f() {}
}
```
keys: open file
```after
class A {
    // region Helpers
    void f() {}
}
```

### ED-FOLD-040 case 5: NetBeans style with a collapsed default state
settings: default
language: java
```before
class A {
    // <editor-fold desc="Fields" defaultstate="collapsed">
    int x;
    // </editor-fold>
}
```
keys: open file
```after
class A {
    <fold text="Fields">// <editor-fold desc="Fields" defaultstate="collapsed">
    int x;
    // </editor-fold></fold>
}
```

### ED-FOLD-011 case 1: Expand on the line where the region starts
settings: default
language: java
```before
class A {
    void f() <caret><fold text="{...}">{
        a();
        b();
    }</fold>
}
```
keys: Ctrl+Numpad+
```after
class A {
    void f() <caret><fold text="{...}" expanded>{
        a();
        b();
    }</fold>
}
```

### ED-FOLD-010 case 1: Collapse on the line where the region starts
settings: default
language: java
```before
class A {
    void f() <caret><fold text="{...}" expanded>{
        a();
        b();
    }</fold>
}
```
keys: Ctrl+Numpad-
```after
class A {
    void f() <caret><fold text="{...}">{
        a();
        b();
    }</fold>
}
```

### ED-FOLD-010 case 2: Collapse inside a body collapses the innermost expanded region
settings: default
status: unverified (caret position after the collapse)
language: java
```before
class A {
    void f(boolean x) <fold text="{...}" expanded>{
        if (x) <fold text="{...}" expanded>{
            a();<caret>
        }</fold>
        b();
    }</fold>
}
```
keys: Ctrl+Numpad-
```after
class A {
    void f(boolean x) <fold text="{...}" expanded>{
        if (x) <caret><fold text="{...}">{
            a();
        }</fold>
        b();
    }</fold>
}
```

### ED-FOLD-010 case 3: a second Collapse collapses the enclosing region
settings: default
status: unverified (caret position after the collapse)
language: java
```before
class A {
    void f(boolean x) <fold text="{...}" expanded>{
        if (x) <caret><fold text="{...}">{
            a();
        }</fold>
        b();
    }</fold>
}
```
keys: Ctrl+Numpad-
```after
class A {
    void f(boolean x) <caret><fold text="{...}">{
        if (x) {
            a();
        }
        b();
    }</fold>
}
```

### ED-FOLD-011 case 2: Expand opens the outermost collapsed region at the caret
settings: default
language: java
```before
class A {
    void f(boolean x) <caret><fold text="{...}">{
        if (x) <fold text="{...}">{
            a();
        }</fold>
        b();
    }</fold>
}
```
keys: Ctrl+Numpad+
```after
class A {
    void f(boolean x) <caret><fold text="{...}" expanded>{
        if (x) <fold text="{...}">{
            a();
        }</fold>
        b();
    }</fold>
}
```

### ED-FOLD-012 case 1: Collapse Recursively collapses the region and the ones inside
settings: default
language: java
```before
class A {
    void f(boolean x) <caret><fold text="{...}" expanded>{
        if (x) <fold text="{...}" expanded>{
            a();
        }</fold>
        b();
    }</fold>
}
```
keys: Ctrl+Alt+Numpad-
```after
class A {
    void f(boolean x) <caret><fold text="{...}">{
        if (x) <fold text="{...}">{
            a();
        }</fold>
        b();
    }</fold>
}
```

### ED-FOLD-013 case 1: Collapse All
settings: default
language: java
```before
class A {
    void f() <fold text="{...}" expanded>{
        a();
        b();
    }</fold>
    void g() <fold text="{...}" expanded>{
        c();
        d();
    }</fold>
}
```
keys: Ctrl+Shift+Numpad-
```after
class A {
    void f() <fold text="{...}">{
        a();
        b();
    }</fold>
    void g() <fold text="{...}">{
        c();
        d();
    }</fold>
}
```

### ED-FOLD-013 case 2: Expand All
settings: default
language: java
```before
class A {
    void f() <fold text="{...}">{
        a();
        b();
    }</fold>
    void g() <fold text="{...}">{
        c();
        d();
    }</fold>
}
```
keys: Ctrl+Shift+Numpad+
```after
class A {
    void f() <fold text="{...}" expanded>{
        a();
        b();
    }</fold>
    void g() <fold text="{...}" expanded>{
        c();
        d();
    }</fold>
}
```

### ED-FOLD-015 case 1: Fold Selection creates a region
settings: default
status: unverified (caret placement)
language: plain
```before
a <selection>bcdef<caret></selection> g
```
keys: Ctrl+.
```after
a <caret><fold text="...">bcdef</fold> g
```

### ED-FOLD-015 case 2: Fold Selection again removes a user-defined region
settings: default
language: plain
```before
a <selection><fold text="...">bcdef</fold><caret></selection> g
```
keys: Ctrl+.
```after
a <selection>bcdef<caret></selection> g
```

### ED-FOLD-050 case 1: Right steps over a collapsed region
settings: default
language: plain
```before
a<caret><fold text="...">bcd</fold>e
```
keys: Right
```after
a<fold text="...">bcd</fold><caret>e
```

### ED-FOLD-050 case 2: Left steps over a collapsed region
settings: default
language: plain
```before
a<fold text="...">bcd</fold><caret>e
```
keys: Left
```after
a<caret><fold text="...">bcd</fold>e
```

### ED-FOLD-050 case 3: Ctrl+Right jumps to the region end
settings: default
language: plain
```before
a<caret><fold text="...">bcd</fold>e
```
keys: Ctrl+Right
```after
a<fold text="...">bcd</fold><caret>e
```

### ED-FOLD-052 case 1: copying a selection over a collapsed region copies the hidden text
settings: default
language: plain
```before
<selection>a<fold text="...">bcd</fold><caret></selection>
z
```
keys: Ctrl+C, Ctrl+End, Ctrl+V
```after
a<fold text="...">bcd</fold>
zabcd<caret>
```
