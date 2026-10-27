# Cases for 02 typing and typed handlers

### ED-TYPE-001 case 1: typing replaces a selection
settings: default
language: plain
```before
a<selection>bc<caret></selection>d
```
keys: type "x"
```after
ax<caret>d
```

### ED-TYPE-001 case 2: typing at two carets
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

### ED-TYPE-001 case 3: typing at three carets with different selections
settings: default
language: plain
```before
<selection>aa<caret></selection> b<caret> <selection>cc<caret></selection>
```
keys: type "z"
```after
z<caret> bz<caret> z<caret>
```

### ED-TYPE-004 case 1: typing a character outside the BMP
settings: default
language: plain
```before
a<caret>b
```
keys: type "😀"
```after
a😀<caret>b
```

### ED-TYPE-003 case 1: overwrite mode replaces
settings: default
language: plain
```before
<caret>abc
```
keys: Insert, type "xy"
```after
xy<caret>c
```

### ED-TYPE-003 case 2: overwrite mode does not eat the line break
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

### ED-TYPE-010 case 1: closing parenthesis is inserted
settings: default
language: java
```before
class A { void f() { foo<caret> } }
```
keys: type "("
```after
class A { void f() { foo(<caret>) } }
```

### ED-TYPE-010 case 2: no pairing before an identifier
settings: default
language: java
```before
class A { void f() { foo<caret>bar } }
```
keys: type "("
```after
class A { void f() { foo(<caret>bar } }
```

### ED-TYPE-010 case 3: square bracket
settings: default
language: java
```before
class A { int[] a = new int<caret>; }
```
keys: type "["
```after
class A { int[] a = new int[<caret>]; }
```

### ED-TYPE-010 case 4: opening brace before a closing brace
settings: default
language: java
```before
class A { void f() <caret>}
```
keys: type "{"
```after
class A { void f() {<caret>}}
```

### ED-TYPE-010 case 5: plain text never pairs
settings: default
language: plain
```before
foo<caret>
```
keys: type "("
```after
foo(<caret>
```

### ED-TYPE-013 case 1: pairing switched off
settings: Insert paired brackets=off
language: java
```before
class A { void f() { foo<caret> } }
```
keys: type "("
```after
class A { void f() { foo(<caret> } }
```

### ED-TYPE-012 case 1: generics bracket after a class name
settings: default
language: java
```before
class A { List<caret> x; }
```
keys: type "<"
```after
class A { List<<caret>> x; }
```

### ED-TYPE-012 case 2: less-than in an expression is not paired
settings: default
language: java
```before
class A { boolean f(int a) { return a <caret> 1; } }
```
keys: type "<"
```after
class A { boolean f(int a) { return a <<caret> 1; } }
```

### ED-TYPE-020 case 1: overtype of a closing parenthesis
settings: default
language: java
```before
class A { void f() { foo(<caret>) } }
```
keys: type ")"
```after
class A { void f() { foo()<caret> } }
```

### ED-TYPE-020 case 2: overtype after typed arguments
settings: default
language: java
```before
class A { void f() { foo(a<caret>) } }
```
keys: type ")"
```after
class A { void f() { foo(a)<caret> } }
```

### ED-TYPE-020 case 3: no opener, the closer is inserted
settings: default
language: java
```before
class A { void f() { x<caret>) } }
```
keys: type ")"
```after
class A { void f() { x)<caret>) } }
```

### ED-TYPE-020 case 4: closing brace overtype
settings: default
language: java
```before
class A {<caret>}
```
keys: type "}"
```after
class A {}<caret>
```

### ED-TYPE-020 case 5: typing the pair from scratch
settings: default
language: java
```before
class A { void f() { <caret> } }
```
keys: type "(a)"
```after
class A { void f() { (a)<caret> } }
```

### ED-TYPE-025 case 1: overtype a closing quote
settings: default
language: java
```before
class A { String s = "abc<caret>"; }
```
keys: type "\""
```after
class A { String s = "abc"<caret>; }
```

### ED-TYPE-027 case 1: opening quote gets a partner
settings: default
language: java
```before
class A { String s = <caret>; }
```
keys: type "\""
```after
class A { String s = "<caret>"; }
```

### ED-TYPE-027 case 2: no partner before an identifier character
settings: default
language: java
```before
class A { String s = <caret>abc; }
```
keys: type "\""
```after
class A { String s = "<caret>abc; }
```

### ED-TYPE-027 case 3: inside a literal a quote is inserted alone
settings: default
language: java
```before
class A { String s = "ab<caret>c"; }
```
keys: type "\""
```after
class A { String s = "ab"<caret>c"; }
```

### ED-TYPE-027 case 4: character literal
settings: default
language: java
```before
class A { char c = <caret>; }
```
keys: type "'"
```after
class A { char c = '<caret>'; }
```

### ED-TYPE-026 case 1: an escaped quote is plain
settings: default
language: java
```before
class A { String s = "a\<caret>"; }
```
keys: type "\""
```after
class A { String s = "a\"<caret>"; }
```

### ED-TYPE-027 case 5: pairing switched off
settings: Insert pair quote=off
language: java
```before
class A { String s = <caret>; }
```
keys: type "\""
```after
class A { String s = "<caret>; }
```

### ED-TYPE-030 case 1: Tab leaves a typed parenthesis pair
settings: default
language: java
```before
class A { void f() { foo<caret> } }
```
keys: type "(a", Tab
```after
class A { void f() { foo(a)<caret> } }
```

### ED-TYPE-030 case 2: Tab leaves a typed quote pair
settings: default
language: java
```before
class A { String s = <caret>; }
```
keys: type "\"ab", Tab
```after
class A { String s = "ab"<caret>; }
```

### ED-TYPE-030 case 3: Tab out disabled
settings: Jump outside closing bracket/quote with Tab when typing=off
language: java
```before
class A { void f() { foo<caret> } }
```
keys: type "(a", Tab
```after
class A { void f() { foo(a  <caret>) } }
```

### ED-TYPE-035 case 1: surround with parentheses
settings: default
language: plain
```before
x <selection>abc<caret></selection> y
```
keys: type "("
```after
x (<selection>abc<caret></selection>) y
```

### ED-TYPE-035 case 2: surround with double quotes
settings: default
language: plain
```before
x <selection>abc<caret></selection> y
```
keys: type "\""
```after
x "<selection>abc<caret></selection>" y
```

### ED-TYPE-035 case 3: backwards selection keeps the caret at its end
settings: default
language: plain
```before
x <selection><caret>abc</selection> y
```
keys: type "["
```after
x [<selection><caret>abc</selection>] y
```

### ED-TYPE-035 case 4: selection already quoted is re-quoted, not nested
settings: default
language: plain
```before
x <selection>"abc"<caret></selection> y
```
keys: type "'"
```after
x '<selection>abc<caret></selection>' y
```

### ED-TYPE-035 case 5: surround switched off replaces the selection
settings: Surround selection on typing quote or brace=off
language: plain
```before
x <selection>abc<caret></selection> y
```
keys: type "("
```after
x (<caret> y
```

### ED-TYPE-036 case 1: comparison operator selection is replaced
settings: default
language: java
```before
class A { boolean f(int a, int b) { return a <selection>==<caret></selection> b; } }
```
keys: type "<"
```after
class A { boolean f(int a, int b) { return a <<caret> b; } }
```

### ED-TYPE-040 case 1: Backspace removes one character
settings: default
language: plain
```before
ab<caret>c
```
keys: Backspace
```after
a<caret>c
```

### ED-TYPE-040 case 2: Backspace joins lines
settings: default
language: plain
```before
ab
<caret>cd
```
keys: Backspace
```after
ab<caret>cd
```

### ED-TYPE-040 case 3: Backspace over a surrogate pair
settings: default
language: plain
```before
a😀<caret>b
```
keys: Backspace
```after
a<caret>b
```

### ED-TYPE-040 case 4: Backspace at the document start
settings: default
language: plain
```before
<caret>ab
```
keys: Backspace
```after
<caret>ab
```

### ED-TYPE-040 case 5: Backspace with a selection
settings: default
language: plain
```before
a<selection>bc<caret></selection>d
```
keys: Backspace
```after
a<caret>d
```

### ED-TYPE-042 case 1: Backspace at three carets
settings: default
language: plain
```before
ab<caret> cd<caret> ef<caret>
```
keys: Backspace
```after
a<caret> c<caret> e<caret>
```

### ED-TYPE-043 case 1: Backspace removes the paired closer
settings: default
language: java
```before
class A { void f() { foo(<caret>) } }
```
keys: Backspace
```after
class A { void f() { foo<caret> } }
```

### ED-TYPE-043 case 2: the closer of another pair is kept
settings: default
language: java
```before
class A { void f() { foo((<caret>) } }
```
keys: Backspace
```after
class A { void f() { foo(<caret>) } }
```

### ED-TYPE-045 case 1: Backspace in an empty string literal
settings: default
language: java
```before
class A { String s = "<caret>"; }
```
keys: Backspace
```after
class A { String s = <caret>; }
```

### ED-TYPE-045 case 2: Backspace after a closing quote deletes only it
settings: default
language: java
```before
class A { String s = "abc"<caret>; }
```
keys: Backspace
```after
class A { String s = "abc<caret>; }
```

### ED-TYPE-046 case 1: proper indent, empty indented line joins with the previous line
settings: default
language: java
```before
class A {
    int x;
    <caret>
}
```
keys: Backspace
```after
class A {
    int x;<caret>
}
```

### ED-TYPE-046 case 2: proper indent, over-indented line snaps to the proper indent
settings: default
language: java
```before
class A {
        <caret>int x;
}
```
keys: Backspace
```after
class A {
    <caret>int x;
}
```

### ED-TYPE-046 case 3: nearest indent position
settings: Unindent on Backspace=To nearest indent position
language: java
```before
class A {
      <caret>int x;
}
```
keys: Backspace
```after
class A {
    <caret>int x;
}
```

### ED-TYPE-046 case 4: disabled
settings: Unindent on Backspace=Disabled
language: java
```before
class A {
        <caret>int x;
}
```
keys: Backspace
```after
class A {
       <caret>int x;
}
```

### ED-TYPE-047 case 1: Hungry Backspace
settings: default
language: plain
```before
a   <caret>b
```
keys: action EditorHungryBackSpace
```after
a<caret>b
```

### ED-TYPE-047 case 2: Hungry Backspace over a line break
settings: default
language: plain
```before
a
   <caret>b
```
keys: action EditorHungryBackSpace
```after
a<caret>b
```

### ED-TYPE-048 case 1: Delete joins lines
settings: default
language: plain
```before
ab<caret>
cd
```
keys: Delete
```after
ab<caret>cd
```

### ED-TYPE-048 case 2: Delete at the end of the document
settings: default
language: plain
```before
ab<caret>
```
keys: Delete
```after
ab<caret>
```

### ED-TYPE-049 case 1: Ctrl+Backspace deletes the word before the caret
settings: default
language: plain
```before
foo bar<caret>
```
keys: Ctrl+Backspace
```after
foo <caret>
```

### ED-TYPE-049 case 2: Ctrl+Backspace eats the blank before the word as well
settings: default
language: plain
```before
foo <caret>bar
```
keys: Ctrl+Backspace
```after
<caret>bar
```

### ED-TYPE-049 case 3: Ctrl+Delete deletes to the end of the word
settings: default
language: plain
```before
<caret>foo bar
```
keys: Ctrl+Delete
```after
<caret> bar
```

### ED-TYPE-049 case 4: Ctrl+Backspace with camel humps
settings: Use "CamelHumps" words=on
language: plain
```before
fooBar<caret>
```
keys: Ctrl+Backspace
```after
foo<caret>
```

### ED-TYPE-049 case 5: Ctrl+Backspace stops at the line start
settings: default
language: plain
```before
ab
cd<caret>
```
keys: Ctrl+Backspace
```after
ab
<caret>
```

### ED-TYPE-060 case 1: Enter replaces a selection
settings: default
language: plain
```before
a<selection>bc<caret></selection>d
```
keys: Enter
```after
a
<caret>d
```

### ED-TYPE-061 case 1: plain text keeps the previous indent
settings: default
language: plain
```before
    abc<caret>
```
keys: Enter
```after
    abc
    <caret>
```

### ED-TYPE-061 case 2: only the whitespace before the caret is copied
settings: default
language: plain
```before
    ab<caret>c
```
keys: Enter
```after
    ab
    <caret>c
```

### ED-TYPE-061 case 3: a caret inside the indent copies only the part before it
settings: default
language: plain
```before
  <caret>  abc
```
keys: Enter
```after
  
  <caret>  abc
```

### ED-TYPE-062 case 1: Java indents after a statement in a block
settings: default
language: java
```before
class A {
    int x;<caret>
}
```
keys: Enter
```after
class A {
    int x;
    <caret>
}
```

### ED-TYPE-062 case 2: smart indent goes one level deeper after an opening brace
settings: default
language: java
```before
class A {
    void f() {<caret>
    }
}
```
keys: Enter
```after
class A {
    void f() {
        <caret>
    }
}
```

### ED-TYPE-062 case 3: smart indent off only copies the indent
settings: Smart indent=off
language: java
```before
class A {
    void f() {<caret>
    }
}
```
keys: Enter
```after
class A {
    void f() {
    <caret>
    }
}
```

### ED-TYPE-063 case 1: Enter in the leading whitespace of a non-blank line
settings: default
language: java
```before
class A {
    <caret>int x;
}
```
keys: Enter
```after
class A {
    
    <caret>int x;
}
```

### ED-TYPE-063 case 2: Enter in overwrite mode only moves
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Insert, Enter
```after
ab
<caret>cd
```

### ED-TYPE-063 case 3: Enter in overwrite mode on the last line appends a line break
settings: default
language: plain
```before
a<caret>b
```
keys: Insert, Enter
```after
ab
<caret>
```

### ED-TYPE-064 case 1: Enter between braces
settings: default
language: java
```before
class A {
    void f() {<caret>}
}
```
keys: Enter
```after
class A {
    void f() {
        <caret>
    }
}
```

### ED-TYPE-065 case 1: Enter after an unmatched brace adds the closer
settings: default
language: java
```before
class A {
    void f() {<caret>
}
```
keys: Enter
```after
class A {
    void f() {
        <caret>
    }
}
```

### ED-TYPE-065 case 2: setting off
settings: Insert pair '}'=off
language: java
```before
class A {
    void f() {<caret>
}
```
keys: Enter
```after
class A {
    void f() {
        <caret>
}
```

### ED-TYPE-066 case 1: Enter in the middle of a line comment continues it
settings: default
language: java
```before
class A {
    // foo<caret> bar
    int x;
}
```
keys: Enter
```after
class A {
    // foo
    // <caret>bar
    int x;
}
```

### ED-TYPE-066 case 2: Enter at the end of a line comment does not
settings: default
language: java
```before
class A {
    // foo<caret>
    int x;
}
```
keys: Enter
```after
class A {
    // foo
    <caret>
    int x;
}
```

### ED-TYPE-067 case 1: Enter inside a documentation comment
settings: default
language: java
```before
class A {
    /**
     * foo<caret>
     */
    void f() {}
}
```
keys: Enter
```after
class A {
    /**
     * foo
     * <caret>
     */
    void f() {}
}
```

### ED-TYPE-067 case 2: Enter after an unterminated doc comment start closes it
settings: default
language: java
```before
class A {
    /**<caret>
    void f() {}
}
```
keys: Enter
```after
class A {
    /**
     * <caret>
     */
    void f() {}
}
```

### ED-TYPE-067 case 3: Close block comment off
settings: Close block comment=off
language: java
```before
class A {
    /*<caret>
    void f() {}
}
```
keys: Enter
```after
class A {
    /*
    <caret>
    void f() {}
}
```

### ED-TYPE-069 case 1: Start New Line
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Shift+Enter
```after
ab
<caret>
cd
```

### ED-TYPE-069 case 2: Start New Line keeps the indent
settings: default
language: java
```before
class A {
    int <caret>x;
}
```
keys: Shift+Enter
```after
class A {
    int x;
    <caret>
}
```

### ED-TYPE-069 case 3: Start New Line Before Current
settings: default
language: java
```before
class A {
    int <caret>x;
}
```
keys: Ctrl+Alt+Enter
```after
class A {
    <caret>
    int x;
}
```

### ED-TYPE-070 case 1: Split Line keeps the caret in place
settings: default
language: plain
```before
ab<caret>cd
```
keys: Ctrl+Enter
```after
ab<caret>
cd
```

### ED-TYPE-070 case 2: Split Line with only blanks before the caret
settings: default
language: plain
```before
  <caret>cd
```
keys: Ctrl+Enter
```after
  
  <caret>cd
```

### ED-TYPE-080 case 1: Tab inserts spaces up to the next indent stop
settings: default
language: plain
```before
a<caret>
```
keys: Tab
```after
a   <caret>
```

### ED-TYPE-080 case 2: Tab at a stop inserts a full indent
settings: default
language: plain
```before
abcd<caret>
```
keys: Tab
```after
abcd    <caret>
```

### ED-TYPE-080 case 3: tab characters
settings: Use tab character=on
language: plain
```before
a<caret>
```
keys: Tab
```after
a\t<caret>
```

### ED-TYPE-081 case 1: Tab indents all selected lines
settings: default
language: plain
```before
<selection>a
b<caret></selection>
```
keys: Tab
```after
<selection>    a
    b<caret></selection>
```

### ED-TYPE-081 case 2: a selection ending at a line start excludes that line
settings: default
language: plain
```before
<selection>a
<caret></selection>b
```
keys: Tab
```after
<selection>    a
<caret></selection>b
```

### ED-TYPE-082 case 1: Shift+Tab without a selection
settings: default
language: plain
```before
    a<caret>
```
keys: Shift+Tab
```after
a<caret>
```

### ED-TYPE-082 case 2: Shift+Tab with less than one indent
settings: default
language: plain
```before
  a<caret>
```
keys: Shift+Tab
```after
a<caret>
```

### ED-TYPE-082 case 3: Shift+Tab on selected lines
settings: default
language: plain
```before
<selection>    a
        b<caret></selection>
```
keys: Shift+Tab
```after
<selection>a
    b<caret></selection>
```

### ED-TYPE-090 case 1: Complete Current Statement in plain text acts as Start New Line
settings: default
language: plain
```before
a<caret>b
cd
```
keys: Ctrl+Shift+Enter
```after
ab
<caret>
cd
```

### ED-TYPE-092 case 1: closes the parenthesis and block of an if
settings: default
language: java
```before
class A {
    void f(boolean a) {
        if (a<caret>
    }
}
```
keys: Ctrl+Shift+Enter
```after
class A {
    void f(boolean a) {
        if (a) {
            <caret>
        }
    }
}
```

### ED-TYPE-092 case 2: adds the missing parenthesis and semicolon
settings: default
language: java
```before
class A {
    void f() {
        foo(1<caret>
    }
}
```
keys: Ctrl+Shift+Enter
```after
class A {
    void f() {
        foo(1);
        <caret>
    }
}
```

### ED-TYPE-093 case 1: adds the semicolon
settings: default
language: java
```before
class A {
    void f() {
        int x = 1<caret>
    }
}
```
keys: Ctrl+Shift+Enter
```after
class A {
    void f() {
        int x = 1;
        <caret>
    }
}
```

### ED-TYPE-100 case 1: paste at two carets with two lines
settings: default
language: plain
```before
<caret>a
<caret>b
```
keys: paste "X\nY"
```after
X<caret>a
Y<caret>b
```

### ED-TYPE-100 case 2: paste at two carets with one line
settings: default
language: plain
```before
<caret>a
<caret>b
```
keys: paste "X"
```after
X<caret>a
X<caret>b
```

### ED-TYPE-100 case 3: paste replaces the selection
settings: default
language: plain
```before
a<selection>bc<caret></selection>d
```
keys: paste "X"
```after
aX<caret>d
```

### ED-TYPE-101 case 1: CRLF clipboard text
settings: default
language: plain
```before
<caret>
```
keys: paste "a\r\nb"
```after
a
b<caret>
```

### ED-TYPE-102 case 1: Indent each line
settings: default
language: java
```before
class A {
    void f() {
        <caret>
    }
}
```
keys: paste "a();\n        b();"
```after
class A {
    void f() {
        a();
        b();<caret>
    }
}
```

### ED-TYPE-102 case 2: None leaves the pasted lines as they are
settings: Reformat on paste=None
language: java
```before
class A {
    void f() {
        <caret>
    }
}
```
keys: paste "a();\nb();"
```after
class A {
    void f() {
        a();
b();<caret>
    }
}
```

### ED-TYPE-102 case 3: Indent block shifts the block, keeping relative indents
settings: Reformat on paste=Indent block
language: java
```before
class A {
    void f() {
        <caret>
    }
}
```
keys: paste "if (x) {\n    y();\n}"
```after
class A {
    void f() {
        if (x) {
            y();
        }<caret>
    }
}
```

### ED-TYPE-103 case 1: paste into a string literal escapes the text
settings: default
language: java
```before
class A { String s = "<caret>"; }
```
keys: paste "a\"b"
```after
class A { String s = "a\"b<caret>"; }
```

### ED-TYPE-104 case 1: a line copied with no selection is pasted above the caret line
settings: default
language: plain
```before
one<caret>
two
three
```
keys: Ctrl+C, Down, Down, Home, Ctrl+V
```after
one
two
one
<caret>three
```

### ED-TYPE-106 case 1: Paste as Plain Text keeps raw indentation
settings: default
language: java
```before
class A {
    void f() {
        <caret>
    }
}
```
clipboard: "a();\nb();"
keys: action EditorPasteSimple
```after
class A {
    void f() {
        a();
b();<caret>
    }
}
```

### ED-TYPE-037 case 1: replacing the quotes of a literal by typing over one of them
settings: default
language: java
```before
class A { String s = <selection>"</selection>ab"; }
```
keys: type "'"
```after
class A { String s = '<caret>ab'; }
```

### ED-TYPE-044 case 1: Backspace removes the generics pair
settings: default
language: java
```before
class A { List<<caret>> x; }
```
keys: Backspace
```after
class A { List<caret> x; }
```

### ED-TYPE-083 case 1: Indent Line or Selection without a selection
settings: default
language: plain
```before
a<caret>
```
keys: action EditorIndentLineOrSelection
```after
    a<caret>
```
