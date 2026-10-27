# Cases for 01 caret, selection and movement

### ED-CARET-001 case 1: Right over a surrogate pair
settings: default
language: plain
```before
<caret>😀a
```
keys: Right
```after
😀<caret>a
```

### ED-CARET-001 case 2: Left at document start
settings: default
language: plain
```before
<caret>abc
```
keys: Left
```after
<caret>abc
```

### ED-CARET-001 case 3: Right at document end
settings: default
language: plain
```before
abc<caret>
```
keys: Right
```after
abc<caret>
```

### ED-CARET-001 case 4: Right at line end goes to next line start
settings: default
language: plain
```before
ab<caret>
cd
```
keys: Right
```after
ab
<caret>cd
```

### ED-CARET-002 case 1: Left collapses a selection to its start
settings: default
language: plain
```before
x <selection>abc<caret></selection> y
```
keys: Left
```after
x <caret>abc y
```

### ED-CARET-002 case 2: Right collapses a selection to its end
settings: default
language: plain
```before
x <selection><caret>abc</selection> y
```
keys: Right
```after
x abc<caret> y
```

### ED-CARET-003 case 1: remembered column through a short line
settings: default
language: plain
```before
0123456789
01
01234567<caret>89
```
keys: Up, Down
```after
0123456789
01
01234567<caret>89
```

### ED-CARET-003 case 2: Down from a selection starts at the selection end
settings: default
language: plain
```before
<selection>ab<caret></selection>c
def
ghi
```
keys: Down
```after
abc
de<caret>f
ghi
```

### ED-CARET-004 case 1: Up on the first line does not move
settings: default
language: plain
```before
ab<caret>c
def
```
keys: Up
```after
ab<caret>c
def
```

### ED-CARET-004 case 2: Down on the last line does not move
settings: default
language: plain
```before
abc
de<caret>f
```
keys: Down
```after
abc
de<caret>f
```

### ED-CARET-006 case 1: Shift+Left past the anchor flips the selection
settings: default
language: plain
```before
ab<caret>cdef
```
keys: Shift+Right, Shift+Right, Shift+Left, Shift+Left, Shift+Left
```after
a<selection><caret>b</selection>cdef
```

### ED-CARET-010 case 1: Ctrl+Right stops at the word end (IDE default)
settings: default
language: plain
```before
<caret>foo bar
```
keys: Ctrl+Right
```after
foo<caret> bar
```

### ED-CARET-010 case 2: second Ctrl+Right reaches the end of the second word
settings: default
language: plain
```before
<caret>foo bar
```
keys: Ctrl+Right, Ctrl+Right
```after
foo bar<caret>
```

### ED-CARET-010 case 3: Ctrl+Left stops at the word start
settings: default
language: plain
```before
foo ba<caret>r
```
keys: Ctrl+Left
```after
foo <caret>bar
```

### ED-CARET-010 case 4: Windows caret stop policy stops at the next word start
settings: caret-stop=windows
language: plain
```before
<caret>foo bar
```
keys: Ctrl+Right
```after
foo <caret>bar
```

### ED-CARET-010 case 5: punctuation is its own word
settings: default
language: plain
```before
<caret>foo.bar
```
keys: Ctrl+Right
```after
foo<caret>.bar
```

### ED-CARET-011 case 1: Ctrl+Right at a line end stops at the next line start
settings: default
language: plain
```before
foo<caret>
bar
```
keys: Ctrl+Right
```after
foo
<caret>bar
```

### ED-CARET-011 case 2: Ctrl+Left at a line start stops at the previous line end
settings: default
language: plain
```before
foo
<caret>bar
```
keys: Ctrl+Left
```after
foo<caret>
bar
```

### ED-CARET-011 case 3: Ctrl+Right at the end of the document does nothing
settings: default
language: plain
```before
foo<caret>
```
keys: Ctrl+Right
```after
foo<caret>
```

### ED-CARET-011 case 4: Ctrl+Left at the start of the document does nothing
settings: default
language: plain
```before
<caret>foo
```
keys: Ctrl+Left
```after
<caret>foo
```

### ED-CARET-012 case 1: camel humps on
settings: Use "CamelHumps" words=on
language: plain
```before
<caret>fooBarBaz
```
keys: Ctrl+Right, Ctrl+Right
```after
fooBar<caret>Baz
```

### ED-CARET-012 case 2: camel humps off
settings: default
language: plain
```before
<caret>fooBarBaz
```
keys: Ctrl+Right
```after
fooBarBaz<caret>
```

### ED-CARET-012 case 3: upper-case run followed by a word
settings: Use "CamelHumps" words=on
language: plain
```before
<caret>HTMLParser
```
keys: Ctrl+Right
```after
HTML<caret>Parser
```

### ED-CARET-012 case 4: underscore humps
settings: Use "CamelHumps" words=on
language: plain
```before
<caret>foo_bar
```
keys: Ctrl+Right
```after
foo<caret>_bar
```

### ED-CARET-012 case 5: Ctrl+Shift+Left selects back by humps
settings: Use "CamelHumps" words=on
language: plain
```before
fooBarBaz<caret>
```
keys: Ctrl+Shift+Left
```after
fooBar<selection><caret>Baz</selection>
```

### ED-CARET-015 case 1: smart Home toggles first non-blank and column 0
settings: default
language: plain
```before
    f<caret>oo   
```
keys: Home
```after
    <caret>foo   
```

### ED-CARET-015 case 2: second Home goes to column 0
settings: default
language: plain
```before
    f<caret>oo   
```
keys: Home, Home
```after
<caret>    foo   
```

### ED-CARET-015 case 3: third Home returns to the first non-blank
settings: default
language: plain
```before
    f<caret>oo   
```
keys: Home, Home, Home
```after
    <caret>foo   
```

### ED-CARET-015 case 4: Home with smart Home off
settings: Home moves caret to first non-whitespace character=off
language: plain
```before
    f<caret>oo
```
keys: Home
```after
<caret>    foo
```

### ED-CARET-015 case 5: End stops before trailing blanks first
settings: default
language: plain
```before
<caret>foo   
```
keys: End
```after
foo<caret>   
```

### ED-CARET-015 case 6: second End reaches the real line end
settings: default
language: plain
```before
<caret>foo   
```
keys: End, End
```after
foo   <caret>
```

### ED-CARET-015 case 7: Shift+End selects to before trailing blanks
settings: default
language: plain
```before
<caret>foo   
```
keys: Shift+End
```after
<selection>foo<caret></selection>   
```

### ED-CARET-016 case 1: Ctrl+Home removes secondary carets
settings: default
language: plain
```before
a<caret>b
c<caret>d
e<caret>f
```
keys: Ctrl+Home
```after
<caret>ab
cd
ef
```

### ED-CARET-016 case 2: Ctrl+Shift+End extends the selection to the document end
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Ctrl+Shift+End
```after
a<selection>b
cd<caret></selection>
```

### ED-CARET-017 case 1: Page Down with a 3-line viewport
settings: viewport-lines=3
language: plain
```before
a<caret>0
b1
c2
d3
e4
f5
```
keys: Page Down
```after
a0
b1
c2
d<caret>3
e4
f5
```

### ED-CARET-020 case 1: overwrite mode replaces the next character
settings: default
language: plain
```before
<caret>abc
```
keys: Insert, type "x"
```after
x<caret>bc
```

### ED-CARET-020 case 2: overwrite mode at a line end inserts
settings: default
language: plain
```before
ab<caret>
cd
```
keys: Insert, type "x"
```after
abx<caret>
cd
```

### ED-CARET-030 case 1: Select All
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Ctrl+A
```after
<selection>ab
cd<caret></selection>
```

### ED-CARET-030 case 2: Select All removes secondary carets
settings: default
language: plain
```before
a<caret>b
c<caret>d
```
keys: Ctrl+A
```after
<selection>ab
cd<caret></selection>
```

### ED-CARET-031 case 1: Extend Selection selects the word
settings: default
language: plain
```before
foo ba<caret>r baz
```
keys: Ctrl+W
```after
foo <selection>bar<caret></selection> baz
```

### ED-CARET-032 case 1: caret at the end of a word selects that word
settings: default
language: plain
```before
foo bar<caret> baz
```
keys: Ctrl+W
```after
foo <selection>bar<caret></selection> baz
```

### ED-CARET-031 case 2: Java string literal, three steps
settings: default
language: java
```before
class A { String s = "ab<caret>c d"; }
```
keys: Ctrl+W, Ctrl+W, Ctrl+W
```after
class A { String s = <selection>"abc d"<caret></selection>; }
```

### ED-CARET-031 case 3: Java string literal, two steps select the contents
settings: default
language: java
```before
class A { String s = "ab<caret>c d"; }
```
keys: Ctrl+W, Ctrl+W
```after
class A { String s = "<selection>abc d<caret></selection>"; }
```

### ED-CARET-036 case 1: Shrink Selection picks the word at the caret inside the selection
settings: default
language: java
```before
class A { String s = "ab<caret>c d"; }
```
keys: Ctrl+W, Ctrl+W, Ctrl+Shift+W
```after
class A { String s = "abc <selection>d<caret></selection>"; }
```

### ED-CARET-036 case 2: Shrink Selection with nothing smaller collapses to the caret
settings: default
language: plain
```before
foo <selection>bar<caret></selection> baz
```
keys: Ctrl+Shift+W
```after
foo bar<caret> baz
```

### ED-CARET-052 case 1: sticky selection extends on plain moves
settings: default
language: plain
```before
<caret>abcdef
```
keys: action EditorToggleStickySelection, Right, Right, Right
```after
<selection>abc<caret></selection>def
```

### ED-CARET-063 case 1: carets at the ends of selected lines
settings: default
language: plain
```before
<selection>a
bb
c<caret></selection>
```
keys: Alt+Shift+G
```after
a<caret>
bb<caret>
c<caret>
```

### ED-CARET-063 case 2: a selection ending at a line start excludes that line
settings: default
language: plain
```before
<selection>a
bb
<caret></selection>c
```
keys: Alt+Shift+G
```after
a<caret>
bb<caret>
c
```

### ED-CARET-074 case 1: Add Selection for Next Occurrence, first press selects the word
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Alt+J
```after
<selection>foo<caret></selection> foo foo
```

### ED-CARET-074 case 2: second press adds the next match
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Alt+J, Alt+J
```after
<selection>foo<caret></selection> <selection>foo<caret></selection> foo
```

### ED-CARET-074 case 3: whole-word matching after an empty-selection start
settings: default
language: plain
```before
<caret>foo foobar foo
```
keys: Alt+J, Alt+J
```after
<selection>foo<caret></selection> foobar <selection>foo<caret></selection>
```

### ED-CARET-074 case 4: a manual selection matches substrings
settings: default
language: plain
```before
<selection>foo<caret></selection> foobar foo
```
keys: Alt+J
```after
<selection>foo<caret></selection> <selection>foo<caret></selection>bar foo
```

### ED-CARET-076 case 1: Unselect Occurrence removes the newest caret
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Alt+J, Alt+J, Alt+J, Alt+Shift+J
```after
<selection>foo<caret></selection> <selection>foo<caret></selection> foo
```

### ED-CARET-076 case 2: with one caret only the selection is removed
settings: default
language: plain
```before
<selection>foo<caret></selection> bar
```
keys: Alt+Shift+J
```after
foo<caret> bar
```

### ED-CARET-077 case 1: Select All Occurrences
settings: default
language: plain
```before
a b <caret>a b a
```
keys: Ctrl+Alt+Shift+J
```after
<selection>a<caret></selection> b <selection>a<caret></selection> b <selection>a<caret></selection>
```

### ED-CARET-077 case 2: whole-word from an empty selection
settings: default
language: plain
```before
<caret>ab abc ab
```
keys: Ctrl+Alt+Shift+J
```after
<selection>ab<caret></selection> abc <selection>ab<caret></selection>
```

### ED-CARET-090 case 1: Escape keeps only the oldest caret
settings: default
language: plain
```before
<caret>a b<caret> c<caret>
```
keys: Escape
```after
<caret>a b c
```

### ED-CARET-090 case 2: Escape clears a selection and keeps the caret
settings: default
language: plain
```before
<selection>ab<caret></selection>c
```
keys: Escape
```after
ab<caret>c
```

### ED-CARET-090 case 3: Escape with one caret and no selection does nothing
settings: default
language: plain
```before
a<caret>bc
```
keys: Escape
```after
a<caret>bc
```

### ED-CARET-051 case 1: from after a parenthesis to its partner
settings: default
language: java
```before
class A { void f() { g(a, b)<caret>; } }
```
keys: Ctrl+Shift+M
```after
class A { void f() { g<caret>(a, b); } }
```

### ED-CARET-051 case 2: from before a parenthesis to after its partner
settings: default
language: java
```before
class A { void f() { g<caret>(a, b); } }
```
keys: Ctrl+Shift+M
```after
class A { void f() { g(a, b)<caret>; } }
```

### ED-CARET-070 case 1: typing at two carets
settings: default
language: plain
```before
<caret>a
<caret>b
```
keys: type "xy"
```after
xy<caret>a
xy<caret>b
```

### ED-CARET-070 case 2: carets that meet merge
settings: default
language: plain
```before
<caret>a<caret>b
```
keys: Left
```after
<caret>ab
```
