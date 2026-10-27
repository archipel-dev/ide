# Cases for 06 live templates and postfix

### ED-TMPL-001 case 1: a mirrored variable
settings: default
template: m = $V$ = $V$ + 1;$END$
language: plain
```before
m<caret>
```
keys: Tab, type "n", Tab
```after
n = n + 1;<caret>
```

### ED-TMPL-001 case 2: a literal dollar sign
settings: default
template: d = cost: $$5$END$
language: plain
```before
d<caret>
```
keys: Tab
```after
cost: $5<caret>
```

### ED-TMPL-001 case 3: the end of the text is the final position without $END$
settings: default
template: ab = X
language: plain
```before
ab<caret>
```
keys: Tab
```after
X<caret>
```

### ED-TMPL-011 case 1: no expansion when the abbreviation is part of a word
settings: default
template: ab = X
language: plain
```before
xab<caret>
```
keys: Tab
```after
xab <caret>
```

### ED-TMPL-011 case 2: expansion after a blank
settings: default
template: ab = X
language: plain
```before
x ab<caret>
```
keys: Tab
```after
x X<caret>
```

### ED-TMPL-011 case 3: the longest abbreviation wins
settings: default
template: ab = 1
language: plain
```before
ab<caret>
```
keys: Tab
```after
1<caret>
```

### ED-TMPL-011 case 4: a shorter abbreviation matches at the end of a longer typed text when separated
settings: default
template: b = 2
language: plain
```before
a b<caret>
```
keys: Tab
```after
a 2<caret>
```

### ED-TMPL-012 case 1: an argument template
settings: default
status: unverified (argument consumption rules)
template: x = f($ARG$)$END$
language: plain
```before
a x<caret>
```
keys: Tab
```after
f(a)<caret>
```

### ED-TMPL-030 case 1: first stop is an empty field
settings: default
template: t = f($A$, $B$)$END$
language: plain
```before
t<caret>
```
keys: Tab
```after
f(<caret>, )
```

### ED-TMPL-030 case 2: Tab moves to the next field
settings: default
template: t = f($A$, $B$)$END$
language: plain
```before
t<caret>
```
keys: Tab, type "1", Tab
```after
f(1, <caret>)
```

### ED-TMPL-031 case 1: Tab after the last field finishes at $END$
settings: default
template: t = f($A$, $B$)$END$
language: plain
```before
t<caret>
```
keys: Tab, type "1", Tab, type "2", Tab
```after
f(1, 2)<caret>
```

### ED-TMPL-031 case 2: Enter also moves on
settings: default
template: t = f($A$, $B$)$END$
language: plain
```before
t<caret>
```
keys: Tab, type "1", Enter, type "2", Enter
```after
f(1, 2)<caret>
```

### ED-TMPL-032 case 1: Shift+Tab goes back and selects the field text
settings: default
template: t = f($A$, $B$)$END$
language: plain
```before
t<caret>
```
keys: Tab, type "1", Tab, Shift+Tab, type "9"
```after
f(9<caret>, )
```

### ED-TMPL-032 case 2: Shift+Tab on the first field does nothing
settings: default
template: t = f($A$, $B$)$END$
language: plain
```before
t<caret>
```
keys: Tab, Shift+Tab, type "7"
```after
f(7<caret>, )
```

### ED-TMPL-033 case 1: Escape ends the session
settings: default
template: t = f($A$, $B$)$END$
language: plain
```before
t<caret>
```
keys: Tab, type "1", Escape, type "z"
```after
f(1z<caret>, )
```

### ED-TMPL-002 case 1: a mirror follows the first field
settings: default
template: p = $T$ $T$$END$
language: plain
```before
p<caret>
```
keys: Tab, type "ab"
```after
ab<caret> ab
```

### ED-TMPL-010 case 1: sout in a statement context
settings: default
language: java
```before
class A { void f() { sout<caret> } }
```
keys: Tab
```after
class A { void f() { System.out.println(<caret>); } }
```

### ED-TMPL-010 case 2: psvm at class level
settings: default
language: java
```before
class A {
    psvm<caret>
}
```
keys: Tab
```after
class A {
    public static void main(String[] args) {
        <caret>
    }
}
```

### ED-TMPL-010 case 3: thr
settings: default
language: java
```before
class A { void f() { thr<caret> } }
```
keys: Tab
```after
class A { void f() { throw new <caret> } }
```

### ED-TMPL-014 case 1: sout in a comment is not expanded
settings: default
language: java
```before
class A {
    void f() {
        // sout<caret>
    }
}
```
keys: Tab
```after
class A {
    void f() {
        // sout <caret>
    }
}
```

### ED-TMPL-010 case 4: expand key set to Enter
settings: Expand with=Enter
template: ab = X
language: plain
```before
ab<caret>
```
keys: Tab
```after
ab   <caret>
```

### ED-TMPL-015 case 1: Insert Live Template
settings: default
template: zzq = Q
language: plain
```before
zz<caret>
```
keys: Ctrl+J, Enter
```after
Q<caret>
```

### ED-TMPL-041 case 1: Surround with Live Template
settings: default
status: unverified (caret placement after a selection template)
template: par = ($SELECTION$)
language: plain
```before
x <selection>abc<caret></selection> y
```
keys: Ctrl+Alt+J, Enter
```after
x (abc)<caret> y
```

### ED-TMPL-042 case 1: Surround With if
settings: default
language: java
```before
class A {
    void f() {
        <selection>a();<caret></selection>
    }
}
```
keys: Ctrl+Alt+T, choose "if"
```after
class A {
    void f() {
        if (<caret>) {
            a();
        }
    }
}
```

### ED-POST-002 case 1: if on a boolean
settings: default
language: java
```before
class A {
    void f(boolean b) {
        b.if<caret>
    }
}
```
keys: Tab
```after
class A {
    void f(boolean b) {
        if (b) {
            <caret>
        }
    }
}
```

### ED-POST-002 case 2: null check
settings: default
language: java
```before
class A {
    void f(String s) {
        s.null<caret>
    }
}
```
keys: Tab
```after
class A {
    void f(String s) {
        if (s == null) {
            <caret>
        }
    }
}
```

### ED-POST-002 case 3: not-null check
settings: default
language: java
```before
class A {
    void f(String s) {
        s.nn<caret>
    }
}
```
keys: Tab
```after
class A {
    void f(String s) {
        if (s != null) {
            <caret>
        }
    }
}
```

### ED-POST-002 case 4: negation
settings: default
language: java
```before
class A {
    boolean f(boolean b) {
        return b.not<caret>;
    }
}
```
keys: Tab
```after
class A {
    boolean f(boolean b) {
        return !b<caret>;
    }
}
```

### ED-POST-002 case 5: sout
settings: default
status: unverified (caret placement)
language: java
```before
class A {
    void f(String s) {
        s.sout<caret>
    }
}
```
keys: Tab
```after
class A {
    void f(String s) {
        System.out.println(s);<caret>
    }
}
```

### ED-POST-003 case 1: Undo restores the typed text
settings: default
language: java
```before
class A {
    void f(boolean b) {
        b.if<caret>
    }
}
```
keys: Tab, Ctrl+Z
```after
class A {
    void f(boolean b) {
        b.if<caret>
    }
}
```
