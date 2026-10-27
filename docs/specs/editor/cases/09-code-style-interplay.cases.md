# Cases for 09 code style inputs

### ED-STYLE-010 case 1: detected 2-space indent is used by Enter
settings: default
status: unverified (detection runs in the background; the case assumes it has finished)
language: java
```before
class A {
  int x;
  int y;
  void f() {<caret>
  }
}
```
keys: Enter
```after
class A {
  int x;
  int y;
  void f() {
    <caret>
  }
}
```

### ED-STYLE-010 case 2: detection switched off uses the scheme indent
settings: Detect and use existing file indents for editing=off
language: java
```before
class A {
  int x;
  int y;
  void f() {<caret>
  }
}
```
keys: Enter
```after
class A {
  int x;
  int y;
  void f() {
      <caret>
  }
}
```

### ED-STYLE-001 case 1: indent size 2 from the scheme
settings: Indent=2, Detect and use existing file indents for editing=off
language: plain
```before
a<caret>
```
keys: Tab
```after
a <caret>
```

### ED-STYLE-001 case 2: indent size 2, at a stop
settings: Indent=2, Detect and use existing file indents for editing=off
language: plain
```before
ab<caret>
```
keys: Tab
```after
ab  <caret>
```

### ED-STYLE-020 case 1: tab characters for Tab
settings: Use tab character=on
language: plain
```before
<caret>
```
keys: Tab
```after
\t<caret>
```

### ED-STYLE-020 case 2: tab characters, indent selection
settings: Use tab character=on
language: plain
```before
<selection>a
b<caret></selection>
```
keys: Tab
```after
<selection>\ta
\tb<caret></selection>
```
