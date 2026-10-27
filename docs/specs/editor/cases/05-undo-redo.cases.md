# Cases for 05 undo and redo

### ED-UNDO-002 case 1: typed characters merge
settings: default
language: plain
```before
<caret>
```
keys: type "abc", Ctrl+Z
```after
<caret>
```

### ED-UNDO-002 case 2: redo brings the whole run back
settings: default
language: plain
```before
<caret>
```
keys: type "abc", Ctrl+Z, Ctrl+Shift+Z
```after
abc<caret>
```

### ED-UNDO-002 case 3: the closing bracket typed with the opening one is undone with it
settings: default
language: java
```before
class A { void f() { foo<caret> } }
```
keys: type "(", Ctrl+Z
```after
class A { void f() { foo<caret> } }
```

### ED-UNDO-002 case 4: surrounding a selection while typing is one step
settings: default
language: plain
```before
x <selection>abc<caret></selection> y
```
keys: type "(", Ctrl+Z
```after
x <selection>abc<caret></selection> y
```

### ED-UNDO-003 case 1: an arrow key splits typing
settings: default
language: plain
```before
<caret>
```
keys: type "ab", Left, type "x", Ctrl+Z
```after
a<caret>b
```

### ED-UNDO-004 case 1: Enter is its own step
settings: default
language: plain
```before
<caret>
```
keys: type "ab", Enter, type "cd", Ctrl+Z
```after
ab
<caret>
```

### ED-UNDO-004 case 2: the step before that is the Enter
settings: default
language: plain
```before
<caret>
```
keys: type "ab", Enter, type "cd", Ctrl+Z, Ctrl+Z
```after
ab<caret>
```

### ED-UNDO-004 case 3: Enter with smart indent is one step
settings: default
language: java
```before
class A {
    int x;<caret>
}
```
keys: Enter, Ctrl+Z
```after
class A {
    int x;<caret>
}
```

### ED-UNDO-004 case 4: two Enters merge
settings: default
language: plain
```before
a<caret>
```
keys: Enter, Enter, Ctrl+Z
```after
a<caret>
```

### ED-UNDO-005 case 1: consecutive Backspaces merge
settings: default
language: plain
```before
abcd<caret>
```
keys: Backspace, Backspace, Backspace, Ctrl+Z
```after
abcd<caret>
```

### ED-UNDO-005 case 2: typing and deleting do not merge
settings: default
language: plain
```before
<caret>
```
keys: type "ab", Backspace, Ctrl+Z
```after
ab<caret>
```

### ED-UNDO-005 case 3: Tab presses merge
settings: default
language: plain
```before
<caret>
```
keys: Tab, Tab, Ctrl+Z
```after
<caret>
```

### ED-UNDO-006 case 1: one Undo reverts both carets
settings: default
language: plain
```before
<caret>a
<caret>b
```
keys: type "x", Ctrl+Z
```after
<caret>a
<caret>b
```

### ED-UNDO-008 case 1: paste with reformat is one step
settings: default
language: java
```before
class A {
    void f() {
        <caret>
    }
}
```
keys: paste "a();\n        b();", Ctrl+Z
```after
class A {
    void f() {
        <caret>
    }
}
```

### ED-UNDO-008 case 2: delete line is one step and restores the caret
settings: default
language: plain
```before
a
<caret>b
c
```
keys: Ctrl+Y, Ctrl+Z
```after
a
<caret>b
c
```

### ED-UNDO-008 case 3: duplicate then undo
settings: default
language: plain
```before
a<caret>b
```
keys: Ctrl+D, Ctrl+Z
```after
a<caret>b
```

### ED-UNDO-008 case 4: join lines is one step
settings: default
language: plain
```before
a<caret>
  b
  c
```
keys: Ctrl+Shift+J, Ctrl+Z
```after
a<caret>
  b
  c
```

### ED-UNDO-011 case 1: the first Undo only restores the caret
settings: default
language: plain
```before
<caret>
```
keys: type "abc", Home, Ctrl+Z
```after
abc<caret>
```

### ED-UNDO-011 case 2: the second Undo reverts the text
settings: default
language: plain
```before
<caret>
```
keys: type "abc", Home, Ctrl+Z, Ctrl+Z
```after
<caret>
```

### ED-UNDO-011 case 3: with transparent caret movement one Undo reverts
settings: Use undo transparent caret movement=on
language: plain
```before
<caret>
```
keys: type "abc", Home, Ctrl+Z
```after
<caret>
```

### ED-UNDO-012 case 1: Undo reselects a replaced selection
settings: default
language: plain
```before
a<selection>bc<caret></selection>d
```
keys: type "x", Ctrl+Z
```after
a<selection>bc<caret></selection>d
```

### ED-UNDO-012 case 2: Undo restores a backwards selection
settings: default
language: plain
```before
a<selection><caret>bc</selection>d
```
keys: Backspace, Ctrl+Z
```after
a<selection><caret>bc</selection>d
```

### ED-UNDO-012 case 3: Redo restores the state after
settings: default
language: plain
```before
a<selection>bc<caret></selection>d
```
keys: type "x", Ctrl+Z, Ctrl+Shift+Z
```after
ax<caret>d
```

### ED-UNDO-013 case 1: a new change clears the redo stack
settings: default
language: plain
```before
<caret>
```
keys: type "a", Ctrl+Z, type "b", Ctrl+Shift+Z
```after
b<caret>
```

### ED-UNDO-013 case 2: a caret move does not clear the redo stack, and Redo restores the state first
settings: default
status: unverified (Redo state restore mirrors Undo; not read in the source)
language: plain
```before
<caret>x
```
keys: type "a", Ctrl+Z, Right, Ctrl+Shift+Z
```after
<caret>x
```

### ED-UNDO-013 case 3: the second Redo reapplies the text
settings: default
status: unverified (Redo state restore mirrors Undo; not read in the source)
language: plain
```before
<caret>x
```
keys: type "a", Ctrl+Z, Right, Ctrl+Shift+Z, Ctrl+Shift+Z
```after
a<caret>x
```

### ED-UNDO-016 case 1: Undo works after Save
settings: default
language: plain
```before
<caret>
```
keys: type "abc", Ctrl+S, Ctrl+Z
```after
<caret>
```

### ED-UNDO-001 case 1: each Duplicate is its own step
settings: default
language: plain
```before
a<caret>
```
keys: Ctrl+D, Ctrl+D, Ctrl+Z
```after
a
a<caret>
```
