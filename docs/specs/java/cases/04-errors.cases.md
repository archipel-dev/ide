# Java error highlighting: acceptance cases

Format: `docs/specs/java/cases/README.md`. Each case restates the requirement of the same ID in
`docs/specs/java/04-error-catalog.md` (and `02-resolve.md` for the walk rules); the executable runner is
`crates/archipel-ide-java/tests/java_cases.rs`. All cases use `keys: inspect`: the file is analyzed as the
daemon does while typing (unsaved text), `after` lists the highlights. Several of them are *broken* files, the
state of a buffer between two keystrokes (a statement without its `;`, a call without its `)`): the references
that are intact must still be marked (JAVA-ERR-002 per-reference tolerance).

### JAVA-ERR-007 case 1: A call of a method the class does not have
language: java
```before
class App {
  void run() {
    foo();
  }
}
```
keys: inspect
```after
problem: error `foo` "Cannot resolve method 'foo()'"
```

### JAVA-ERR-007 case 2: The message prints the argument types as short names
language: java
```before
class App {
  void run(int count) {
    foo(1, "a", count, unknown);
  }
}
```
keys: inspect
```after
problem: error `foo` "Cannot resolve method 'foo(int, String, int, ?)'"
problem: error `unknown` "Cannot resolve symbol 'unknown'"
```

### JAVA-ERR-007 case 3: The call is marked while the statement is still unterminated
language: java
```before
class App {
  void run() {
    foo()
  }
}
```
keys: inspect
```after
problem: error `foo` "Cannot resolve method 'foo()'"
```

### JAVA-ERR-007 case 4: The call is marked while its argument list is still open
language: java
```before
class App {
  void run() {
    foo(
  }
}
```
keys: inspect
```after
problem: error `foo` "Cannot resolve method 'foo()'"
```

### JAVA-ERR-007 case 5: A missing method on a library-typed receiver, also through a call chain
language: java
```before
import java.util.List;

class App {
  void run(List<String> names, StringBuilder out) {
    names.get(0).nope();
    out.append(1).nope();
  }
}
```
keys: inspect
```after
problem: error `nope` "Cannot resolve method 'nope()'"
problem: error `nope` "Cannot resolve method 'nope()'"
```

### JAVA-ERR-011 case 1: A missing static method on a class qualifier names the class
language: java
```before
class App {
  int run() {
    return Math.nope(1);
  }
}
```
keys: inspect
```after
problem: error `nope` "Cannot resolve method 'nope(int)' in 'Math'"
```

### JAVA-ERR-012 case 1: A type that does not exist
language: java
```before
class App {
  void run() {
    Bar b;
  }
}
```
keys: inspect
```after
problem: error `Bar` "Cannot resolve symbol 'Bar'"
```

### JAVA-ERR-012 case 2: The declaration is marked while the statement is unterminated
language: java
```before
class App {
  void run() {
    Bar b
  }
}
```
keys: inspect
```after
problem: error `Bar` "Cannot resolve symbol 'Bar'"
```

### JAVA-ERR-012 case 3: A class name being completed inside a declaration with `new`
language: java
```before
import java.util.List;

class App {
  void run() {
    List<String> more = new ArrayLi
  }
}
```
keys: inspect
```after
problem: error `ArrayLi` "Cannot resolve symbol 'ArrayLi'"
```

### JAVA-ERR-012 case 4: An unknown variable and an unknown constructor class
language: java
```before
class App {
  void run() {
    int y = zzz + 1;
    Object o = new Unknown(1);
  }
}
```
keys: inspect
```after
problem: error `zzz` "Cannot resolve symbol 'zzz'"
problem: error `Unknown` "Cannot resolve symbol 'Unknown'"
```

### JAVA-ERR-012 case 5: Only the first unresolved segment of a qualified name is marked (JAVA-RES-005)
language: java
```before
class App {
  java.utl.List<String> items;
}
```
keys: inspect
```after
problem: error `utl` "Cannot resolve symbol 'utl'"
```

### JAVA-ERR-012 case 6: An unknown annotation type is marked, nothing else in the class is
language: java
```before
class App {
  @Te
  void run() {
    foo();
  }
}
```
keys: inspect
```after
problem: error `Te` "Cannot resolve symbol 'Te'"
```

### JAVA-ERR-002 case 1: A correct file has no problem (generics, lambdas, inner classes, static imports, library supertypes)
language: java
```before
import static java.util.Collections.emptyList;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.function.Function;

class App extends ArrayList<String> implements Comparable<App> {
  static class Inner {
    int size() { return 1; }
  }

  interface Visitor<T> {
    T visit(Inner inner);
  }

  public int compareTo(App other) { return 0; }

  <T> List<T> wrap(T value, Function<T, String> render) {
    List<T> out = new ArrayList<>();
    out.add(value);
    String shown = render.apply(value);
    Map<String, List<T>> index = Map.of(shown, out);
    index.get(shown).forEach(item -> System.out.println(item));
    return out.isEmpty() ? emptyList() : out;
  }

  int run() {
    Inner inner = new Inner();
    Visitor<Integer> v = i -> i.size();
    StringBuilder sb = new StringBuilder();
    sb.append(v.visit(inner)).append("x").reverse();
    add("a");
    return size() + get(0).length() + sb.length();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ERR-002 case 2: Members that an unresolved supertype could supply are not reported
language: java
```before
class App extends Missing {
  void run() {
    inherited();
    this.alsoInherited();
  }
}
```
keys: inspect
```after
problem: error `Missing` "Cannot resolve symbol 'Missing'"
```

### JAVA-ERR-002 case 3: Members that an annotation processor could generate are not reported
language: java
```before
import lombok.Getter;

@Getter
class App {
  int count;

  int run() {
    return getCount();
  }
}
```
keys: inspect
```after
problem: error `lombok` "Cannot resolve symbol 'lombok'"
```
