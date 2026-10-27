# Cases for 03 line and text operations

### ED-LINE-001 case 1: duplicate a line, caret follows into the copy
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Ctrl+D
```after
ab
a<caret>b
cd
```

### ED-LINE-001 case 2: duplicate the last line without a line break
settings: default
language: plain
```before
x
ab<caret>
```
keys: Ctrl+D
```after
x
ab
ab<caret>
```

### ED-LINE-001 case 3: duplicate a selection
settings: default
language: plain
```before
a<selection>b<caret></selection>c
```
keys: Ctrl+D
```after
ab<selection>b<caret></selection>c
```

### ED-LINE-001 case 4: duplicate at two carets
settings: default
language: plain
```before
a<caret>
b<caret>
```
keys: Ctrl+D
```after
a
a<caret>
b
b<caret>
```

### ED-LINE-001 case 5: duplicate an empty line
settings: default
language: plain
```before
a
<caret>
b
```
keys: Ctrl+D
```after
a


<caret>
b
```

### ED-LINE-002 case 1: duplicate entire lines of a selection
settings: default
language: plain
```before
<selection>a
b<caret></selection>
c
```
keys: action EditorDuplicateLines
```after
a
b
<selection>a
b
<caret></selection>c
```

### ED-LINE-010 case 1: delete a middle line
settings: default
language: plain
```before
a
<caret>b
c
```
keys: Ctrl+Y
```after
a
<caret>c
```

### ED-LINE-010 case 2: delete the last line also removes the break before it
settings: default
language: plain
```before
a
<caret>b
```
keys: Ctrl+Y
```after
<caret>a
```

### ED-LINE-010 case 3: delete the only line
settings: default
language: plain
```before
<caret>a
```
keys: Ctrl+Y
```after
<caret>
```

### ED-LINE-010 case 4: the column is kept
settings: default
language: plain
```before
abc
d<caret>ef
xy
```
keys: Ctrl+Y
```after
abc
x<caret>y
```

### ED-LINE-010 case 5: delete the lines of a selection
settings: default
language: plain
```before
a
<selection>b
c<caret></selection>
d
```
keys: Ctrl+Y
```after
a
d<caret>
```

### ED-LINE-010 case 6: a selection ending at a line start excludes that line
settings: default
language: plain
```before
a
<selection>b
<caret></selection>c
```
keys: Ctrl+Y
```after
a
<caret>c
```

### ED-LINE-020 case 1: join two lines
settings: default
language: plain
```before
a<caret>
    b
```
keys: Ctrl+Shift+J
```after
a<caret> b
```

### ED-LINE-020 case 2: trailing blanks are collapsed
settings: default
language: plain
```before
a  <caret>
  b
```
keys: Ctrl+Shift+J
```after
a<caret> b
```

### ED-LINE-020 case 3: join on the last line does nothing
settings: default
language: plain
```before
a
b<caret>
```
keys: Ctrl+Shift+J
```after
a
b<caret>
```

### ED-LINE-020 case 4: join a selection of three lines
settings: default
language: plain
```before
<selection>a
b
c<caret></selection>
```
keys: Ctrl+Shift+J
```after
<selection>a b c<caret></selection>
```

### ED-LINE-020 case 5: join in the middle of a line
settings: default
language: plain
```before
x<caret>y
z
```
keys: Ctrl+Shift+J
```after
xy<caret> z
```

### ED-LINE-022 case 1: Java statement joined with its body
settings: default
language: java
```before
class A {
    void f() {
        if (a)<caret>
            b();
    }
}
```
keys: Ctrl+Shift+J
```after
class A {
    void f() {
        if (a)<caret> b();
    }
}
```

### ED-LINE-030 case 1: move a line down
settings: default
language: plain
```before
a<caret>
b
c
```
keys: Alt+Shift+Down
```after
b
a<caret>
c
```

### ED-LINE-030 case 2: move a line up
settings: default
language: plain
```before
a
b<caret>
c
```
keys: Alt+Shift+Up
```after
b<caret>
a
c
```

### ED-LINE-030 case 3: first line cannot move up
settings: default
language: plain
```before
a<caret>
b
```
keys: Alt+Shift+Up
```after
a<caret>
b
```

### ED-LINE-030 case 4: last line cannot move down
settings: default
language: plain
```before
a
b<caret>
```
keys: Alt+Shift+Down
```after
a
b<caret>
```

### ED-LINE-030 case 5: move a selection of lines
settings: default
language: plain
```before
<selection>a
b
<caret></selection>c
d
```
keys: Alt+Shift+Down
```after
c
<selection>a
b
<caret></selection>d
```

### ED-LINE-030 case 6: the caret keeps its column
settings: default
language: plain
```before
abc
d<caret>efgh
ij
```
keys: Alt+Shift+Up
```after
d<caret>efgh
abc
ij
```

### ED-LINE-030 case 7: move line does not re-indent
settings: default
language: java
```before
class A {
    void f() {
        a();
        b();<caret>
    }
}
```
keys: Alt+Shift+Down
```after
class A {
    void f() {
        a();
    }
        b();<caret>
}
```

### ED-LINE-031 case 1: move a statement up over another statement
settings: default
language: java
```before
class A {
    void f() {
        a();
        b();<caret>
    }
}
```
keys: Ctrl+Shift+Up
```after
class A {
    void f() {
        b();<caret>
        a();
    }
}
```

### ED-LINE-031 case 2: move a statement out of a block
settings: default
language: java
```before
class A {
    void f(boolean x) {
        if (x) {
            a();<caret>
        }
    }
}
```
keys: Ctrl+Shift+Down
```after
class A {
    void f(boolean x) {
        if (x) {
        }
        a();<caret>
    }
}
```

### ED-LINE-032 case 1: move an argument right
settings: default
language: java
```before
class A { void g() { f(a<caret>, b); } }
```
keys: Ctrl+Alt+Shift+Right
```after
class A { void g() { f(b, a<caret>); } }
```

### ED-LINE-040 case 1: comment a line, caret goes down (default)
settings: default
language: java
```before
class A {
    void f() {
        int x = 1;<caret>
        int y = 2;
    }
}
```
keys: Ctrl+/
```after
class A {
    void f() {
//        int x = 1;
        int y = 2;<caret>
    }
}
```

### ED-LINE-042 case 1: caret stays when the caret-down setting is off
settings: Move caret down after Comment with Line Comment action=off
language: java
```before
class A {
    void f() {
        int x = 1;<caret>
        int y = 2;
    }
}
```
keys: Ctrl+/
```after
class A {
    void f() {
//        int x = 1;<caret>
        int y = 2;
    }
}
```

### ED-LINE-040 case 2: uncomment a line
settings: Move caret down after Comment with Line Comment action=off
language: java
```before
class A {
    void f() {
//        int x = 1;<caret>
        int y = 2;
    }
}
```
keys: Ctrl+/
```after
class A {
    void f() {
        int x = 1;<caret>
        int y = 2;
    }
}
```

### ED-LINE-041 case 1: indented comment with a space
settings: Line comment at first column=off, Add a space at comment start=on, Move caret down after Comment with Line Comment action=off
language: java
```before
class A {
    void f() {
        int x = 1;<caret>
    }
}
```
keys: Ctrl+/
```after
class A {
    void f() {
        // int x = 1;<caret>
    }
}
```

### ED-LINE-041 case 2: uncomment removes the space too
settings: Line comment at first column=off, Add a space at comment start=on, Move caret down after Comment with Line Comment action=off
language: java
```before
class A {
    void f() {
        // int x = 1;<caret>
    }
}
```
keys: Ctrl+/
```after
class A {
    void f() {
        int x = 1;<caret>
    }
}
```

### ED-LINE-040 case 3: a mixed selection is commented, not uncommented
settings: Line comment at first column=off, Move caret down after Comment with Line Comment action=off
language: java
```before
class A {
    void f() {
        <selection>//a();
        b();
</selection>    }
}
```
keys: Ctrl+/
```after
class A {
    void f() {
<selection>        ////a();
        //b();
</selection>    }
}
```

### ED-LINE-040 case 4: uncomment two selected lines
settings: default
language: java
```before
class A {
    void f() {
<selection>//        a();
//        b();
</selection>    }
}
```
keys: Ctrl+/
```after
class A {
    void f() {
<selection>        a();
        b();
</selection>    }
}
```

### ED-LINE-042 case 2: comment an empty line, caret after the prefix
settings: default
language: java
```before
class A {
    void f() {
<caret>
    }
}
```
keys: Ctrl+/
```after
class A {
    void f() {
//<caret>
    }
}
```

### ED-LINE-044 case 1: block comment without selection
settings: default
language: java
```before
class A { int x<caret>; }
```
keys: Ctrl+Shift+/
```after
class A { int x/*<caret>*/; }
```

### ED-LINE-044 case 2: block comment without selection with spaces
settings: Add a space at block comment start=on
language: java
```before
class A { int x<caret>; }
```
keys: Ctrl+Shift+/
```after
class A { int x/* <caret> */; }
```

### ED-LINE-044 case 3: remove a block comment around the caret
settings: default
language: java
```before
class A { int /*x<caret>*/; }
```
keys: Ctrl+Shift+/
```after
class A { int x<caret>; }
```

### ED-LINE-050 case 1: selection to lower case
settings: default
language: plain
```before
<selection>Foo BAR<caret></selection>
```
keys: Ctrl+Shift+U
```after
<selection>foo bar<caret></selection>
```

### ED-LINE-050 case 2: all lower case goes to upper case
settings: default
language: plain
```before
<selection>foo bar<caret></selection>
```
keys: Ctrl+Shift+U
```after
<selection>FOO BAR<caret></selection>
```

### ED-LINE-050 case 3: the word at the caret end is selected and converted
settings: default
language: plain
```before
foo<caret> bar
```
keys: Ctrl+Shift+U
```after
<selection>FOO<caret></selection> bar
```

### ED-LINE-050 case 4: one direction for all carets
settings: default
language: plain
```before
<selection>ab<caret></selection> <selection>Cd<caret></selection>
```
keys: Ctrl+Shift+U
```after
<selection>ab<caret></selection> <selection>cd<caret></selection>
```

### ED-LINE-050 case 5: the converted text may change length
settings: locale=en
language: plain
```before
<selection>straße<caret></selection>
```
keys: Ctrl+Shift+U
```after
<selection>STRASSE<caret></selection>
```

### ED-LINE-051 case 1: transpose in the middle of a line
settings: default
language: plain
```before
a<caret>bc
```
keys: action EditorTranspose
```after
ba<caret>c
```

### ED-LINE-051 case 2: transpose at the line end swaps the last two
settings: default
language: plain
```before
abc<caret>
```
keys: action EditorTranspose
```after
acb<caret>
```

### ED-LINE-051 case 3: transpose at offset 0 does nothing
settings: default
language: plain
```before
<caret>abc
```
keys: action EditorTranspose
```after
<caret>abc
```

### ED-LINE-052 case 1: sort the whole document
settings: default
language: plain
```before
c
a<caret>
b
```
keys: action EditorSortLines
```after
a<caret>
b
c
```

### ED-LINE-052 case 2: sort the selected lines
settings: default
language: plain
```before
<selection>c
b
a
<caret></selection>x
```
keys: action EditorSortLines
```after
<selection>a
b
c
<caret></selection>x
```

### ED-LINE-052 case 3: sort is case sensitive, upper case first
settings: default
language: plain
```before
b
B
a
```
keys: action EditorSortLines
```after
B
a
b
```

### ED-LINE-052 case 4: reverse lines
settings: default
language: plain
```before
a
b<caret>
c
```
keys: action EditorReverseLines
```after
c
b<caret>
a
```

### ED-LINE-052 case 5: delete duplicate lines
settings: default
language: plain
```before
a
b
a
c<caret>
```
keys: action EditorUniqueLines
```after
a
b
c<caret>
```

### ED-LINE-052 case 6: a single line is not changed
settings: default
language: plain
```before
b<caret>
```
keys: action EditorSortLines
```after
b<caret>
```

### ED-LINE-053 case 1: Auto-Indent Lines
settings: default
language: java
```before
class A {
void f() {
<caret>int x;
}
}
```
keys: Ctrl+Alt+I
```after
class A {
void f() {
        <caret>int x;
}
}
```

### ED-CLIP-001 case 1: copy a selection and paste at the end
settings: default
language: plain
```before
<selection>ab<caret></selection>c
```
keys: Ctrl+C, End, Ctrl+V
```after
abcab<caret>
```

### ED-CLIP-001 case 2: copy does not change the selection
settings: default
language: plain
```before
<selection>ab<caret></selection>c
```
keys: Ctrl+C
```after
<selection>ab<caret></selection>c
```

### ED-CLIP-002 case 1: copy with no selection leaves the caret alone
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Ctrl+C
```after
a<caret>b
cd
```

### ED-CLIP-002 case 2: a line copied without selection is pasted above the caret line
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Ctrl+C, Down, Ctrl+V
```after
ab
ab
c<caret>d
```

### ED-CLIP-003 case 1: cut with no selection removes the line
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Ctrl+X
```after
<caret>cd
```

### ED-CLIP-003 case 2: cut with a selection
settings: default
language: plain
```before
a<selection>b<caret></selection>c
```
keys: Ctrl+X
```after
a<caret>c
```

### ED-CLIP-003 case 3: cut at two carets and paste distributes the pieces
settings: default
language: plain
```before
<selection>a<caret></selection> <selection>b<caret></selection>
```
keys: Ctrl+X, Ctrl+V
```after
a<caret> b<caret>
```

### ED-CLIP-002 case 3: copy with no selection can be switched off
settings: Don't copy/cut the current line when invoking the Copy or Cut action with no selection=on
language: plain
```before
a<caret>b
cd
```
keys: Ctrl+X
```after
a<caret>b
cd
```
