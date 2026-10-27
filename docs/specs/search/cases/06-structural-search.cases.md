# Cases for 06-structural-search.md (LATER)

### SRCH-SSR-001 case 1: simple template
settings: default
language: java
```before
--- A.java
class A { void f() { System.out.println(1); System.out.println("x"); } }
```
keys: Search Structurally…, template `System.out.println($arg$);`, run
```after
list: A.java:1 two matches (both println calls); tool window title "fragments matching template 'System.out.println($arg$);'"
```

### SRCH-SSR-002 case 1: Text modifier on a variable
settings: variable arg: Text = `\d+`
language: java
```before
--- A.java
class A { void f() { System.out.println(1); System.out.println("x"); } }
```
keys: run the template of the previous case with the modifier
```after
list: one match (println(1))
```

### SRCH-SSR-002 case 2: a repeated variable must match equal code
settings: default
language: java
```before
--- A.java
class A { boolean f(int a, int b) { return a == a || a == b; } }
```
keys: template `$x$ == $x$`
```after
list: one match (a == a)
```
