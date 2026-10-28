# Java resolve: acceptance cases

Format: `docs/specs/java/cases/README.md`; requirements in `docs/specs/java/02-resolve.md`. The cases observe
resolution through the errors it produces (`keys: inspect`): a name that resolves gives no problem, a name that
does not is marked by the first unresolved segment.

### JAVA-RES-005 case 1: The first unresolved segment is the only one marked
language: java
```before
class App {
  void run() {
    Collections.emptyList();
    zork.nothere.Foo f;
  }
}
```
keys: inspect
```after
problem: error `Collections` "Cannot resolve symbol 'Collections'"
problem: error `zork` "Cannot resolve symbol 'zork'"
```

### JAVA-RES-007 case 1: Own members hide inherited ones and inherited members of library classes resolve
language: java
```before
import java.util.ArrayList;

class Names extends ArrayList<String> {
  int size() { return 0; }

  int run() {
    add("a");
    return size() + get(0).length() + hashCode();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-RES-011 case 1: An unqualified call is resolved at the innermost class that has the name
language: java
```before
class Outer {
  void helper() {}

  class Inner {
    void run() {
      helper();
      missing();
    }
  }
}
```
keys: inspect
```after
problem: error `missing` "Cannot resolve method 'missing()'"
```

### JAVA-RES-018 case 1: Imports are looked up in stage order: single import, then the same package, then on demand
language: java
```before
import java.util.*;
import java.sql.Date;

class App {
  Date when;
  List<String> names;
  Map<String, Integer> counts;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-RES-021 case 1: Static single imports resolve fields and methods
language: java
```before
import static java.lang.Math.PI;
import static java.lang.Math.max;

class App {
  double run() {
    return max(PI, 3);
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-RES-020 case 1: `java.lang` is an implicit on-demand import
language: java
```before
class App {
  String run(Object o) {
    return Integer.toString(Math.abs(o.hashCode())) + StringBuilder.class.getName();
  }
}
```
keys: inspect
```after
no problem reported
```
