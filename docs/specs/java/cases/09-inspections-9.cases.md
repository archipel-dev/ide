# Cases for 09-inspections-catalog-9 (JAVA-ICC)

Format: see `docs/specs/editor/cases/README.md` and the header of `09-inspections-7.cases.md` (`inspect`, `problem:` lines,
`no problem reported`). Double-brace tokens carry the short name and are replaced by the requirement ID of that inspection.
Cases for default-on inspections whose editor level is INFORMATION use `keys: Alt+Enter, choose "<fix>"` (the intention form).

### JAVA-ICC-218 case 1: narrow the variable
settings: default
language: java
```before
class A {
  void f() {
    Object x = " string ";
    System.out.println(((<caret>String) x).trim());
  }
}
```
keys: Alt+Enter, choose "Change type of 'x' to 'String' and remove cast"
```after
class A {
  void f() {
    String x = " string ";
    System.out.println(x.trim());
  }
}
```

### JAVA-ICC-218 case 2: variable assigned a non-String elsewhere
settings: default
language: java
```before
class A {
  void f(boolean b) {
    Object x = " string ";
    if (b) x = 1;
    System.out.println(((<caret>String) x).trim());
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-219 case 1: reuse existing variable
settings: default
status: unverified (the replaced text in the fix name may be shown without parentheses)
language: java
```before
class A {
  void foo(Object obj) {
    String s = (String) obj;
    System.out.println(((<caret>String) obj).trim());
  }
}
```
keys: Alt+Enter, choose "Replace '(String) obj' with 's'"
```after
class A {
  void foo(Object obj) {
    String s = (String) obj;
    System.out.println(s.trim());
  }
}
```

### JAVA-ICC-219 case 2: variable reassigned in between
settings: default
language: java
```before
class A {
  void foo(Object obj, Object other) {
    String s = (String) obj;
    obj = other;
    System.out.println(((<caret>String) obj).trim());
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-220 case 1: null check before instanceof
settings: default
language: java
```before
class A {
  boolean f(Object obj) { return <caret>obj != null && obj instanceof String; }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'obj != null' condition"
```after
class A {
  boolean f(Object obj) { return obj instanceof String; }
}
```

### JAVA-ICC-220 case 2: unrelated conditions
settings: default
language: java
```before
class A {
  boolean f(int a, int b) { return <caret>a > 0 && b > 0; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-221 case 1: merge branches
settings: default
status: unverified (fix name wording: `Merge with ''{0}''`)
language: java
```before
class A {
  void f(int n) {
    switch (n) {
      case 1:
        System.out.println(n);
        break;
      case 2:
        <caret>System.out.println(n);
        break;
      default:
        System.out.println("default");
    }
  }
}
```
keys: Alt+Enter, choose "Merge with 'case 1'"
```after
class A {
  void f(int n) {
    switch (n) {
      case 1:
      case 2:
        System.out.println(n);
        break;
      default:
        System.out.println("default");
    }
  }
}
```

### JAVA-ICC-221 case 2: different branches
settings: default
language: java
```before
class A {
  void f(int n) {
    switch (n) {
      case 1:
        System.out.println(1);
        break;
      case 2:
        <caret>System.out.println(2);
        break;
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-222 case 1: introduce variable
settings: default
status: unverified (variable name suggestion and complexity threshold: expression must score above 70)
language: java
```before
class A {
  int f(int a, int b) {
    int x = <caret>Math.max(a * b, 10) + 1;
    int y = Math.max(a * b, 10) + 2;
    return x + y;
  }
}
```
keys: Alt+Enter, choose "Introduce variable for 'Math.max(a * b, 10)'"
```after
class A {
  int f(int a, int b) {
    int max = Math.max(a * b, 10);
    int x = max + 1;
    int y = max + 2;
    return x + y;
  }
}
```

### JAVA-ICC-222 case 2: single occurrence
settings: default
language: java
```before
class A {
  int f(int a, int b) { return <caret>Math.max(a * b, 10); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-223 case 1: orElseGet(() -> null)
settings: default
language: java
```before
import java.util.Optional;
class A {
  String f(Optional<String> o) { return o.orElseGet(<caret>() -> null); }
}
```
keys: Alt+Enter, choose "Use 'orElse' method without lambda"
```after
import java.util.Optional;
class A {
  String f(Optional<String> o) { return o.orElse(null); }
}
```

### JAVA-ICC-223 case 2: non-trivial lambda
settings: default
language: java
```before
import java.util.Optional;
class A {
  String f(Optional<String> o) { return o.orElseGet(<caret>() -> "x".trim()); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-224 case 1: x > 2 && x < 4
settings: default
language: java
```before
class A {
  boolean f(int x) { return <caret>x > 2 && x < 4; }
}
```
keys: Alt+Enter, choose "Replace with 'x == 3'"
```after
class A {
  boolean f(int x) { return x == 3; }
}
```

### JAVA-ICC-224 case 2: genuine range
settings: default
language: java
```before
class A {
  boolean f(int x) { return <caret>x > 2 && x < 10; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-225 case 1: loop to Arrays.fill
settings: default
language: java
```before
class A {
  void f(int[] array) {
    <caret>for (int i = 0; i < array.length; i++) { array[i] = 0; }
  }
}
```
keys: Alt+Enter, choose "Replace loop with 'Arrays.fill()' method call"
```after
import java.util.Arrays;
class A {
  void f(int[] array) {
    Arrays.fill(array, 0);
  }
}
```

### JAVA-ICC-225 case 2: index-dependent value with option off
settings: default (Suggest 'Arrays.setAll()' = off)
language: java
```before
class A {
  int calc(int i) { return i * 2; }
  void f(int[] array) {
    <caret>for (int i = 0; i < array.length; i++) { array[i] = calc(i); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-228 case 1: conditional min
settings: default
status: unverified (fix name shows `{0}` as `Math.min`)
language: java
```before
class A {
  public int min(int a, int b) { return <caret>b < a ? b : a; }
}
```
keys: Alt+Enter, choose "Replace with 'Math.min' call"
```after
class A {
  public int min(int a, int b) { return Math.min(a, b); }
}
```

### JAVA-ICC-228 case 2: different values returned
settings: default
language: java
```before
class A {
  public int f(int a, int b) { return <caret>b < a ? b : 0; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-229 case 1: map before count
settings: default
language: java
```before
import java.util.List;
class A {
  long f(List<String> list) { return list.stream().filter(s -> !s.isEmpty()).<caret>map(s -> s.trim()).count(); }
}
```
keys: inspect
```after
problem: WARNING map "The 'map()' call does not change the final count and might be optimized out."
```

### JAVA-ICC-229 case 2: filter before count
settings: default
language: java
```before
import java.util.List;
class A {
  long f(List<String> list) { return list.stream().<caret>filter(s -> !s.isEmpty()).count(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-230 case 1: nested non-strict comparison
settings: default
language: java
```before
class A {
  void f(int x) {
    if (x >= 10) {
      if (x <caret><= 10) { System.out.println(); }
    }
  }
}
```
keys: Alt+Enter, choose "Replace with '=='"
```after
class A {
  void f(int x) {
    if (x >= 10) {
      if (x == 10) { System.out.println(); }
    }
  }
}
```

### JAVA-ICC-230 case 2: independent comparison
settings: default
language: java
```before
class A {
  void f(int x) {
    if (x >= 10) {
      if (x <caret><= 20) { System.out.println(); }
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-231 case 1: requireNonNull of a literal
settings: default
language: java
```before
import java.util.Objects;
class A {
  String f() { final String greeting = Objects.<caret>requireNonNull("Hi!"); return greeting; }
}
```
keys: Alt+Enter, choose "Remove redundant null-check"
```after
import java.util.Objects;
class A {
  String f() { final String greeting = "Hi!"; return greeting; }
}
```

### JAVA-ICC-231 case 2: variable argument
settings: default
language: java
```before
import java.util.Objects;
class A {
  String f(String s) { return Objects.<caret>requireNonNull(s); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-232 case 1: indexed array initializer
settings: default
language: java
```before
class A {
  void f() { System.out.println(<caret>new int[] {1, 2, 3, 4, 5}[2]); }
}
```
keys: Alt+Enter, choose "Replace with '3'"
```after
class A {
  void f() { System.out.println(3); }
}
```

### JAVA-ICC-232 case 2: array variable
settings: default
language: java
```before
class A {
  void f(int[] a) { System.out.println(<caret>a[2]); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-233 case 1: asList(new String[]{...})
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> f() { return Arrays.asList(<caret>new String[]{"Hello", "world"}); }
}
```
keys: Alt+Enter, choose "Remove explicit array creation"
```after
import java.util.*;
class A {
  List<String> f() { return Arrays.asList("Hello", "world"); }
}
```

### JAVA-ICC-233 case 2: array variable
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> f(String[] a) { return Arrays.asList(<caret>a); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-235 case 1: isInstance
settings: default
status: unverified (fix name `{0}` text)
language: java
```before
class A {
  boolean f(Object o) { return String.class.<caret>isInstance(o); }
}
```
keys: Alt+Enter, choose "Replace with 'o instanceof String'"
```after
class A {
  boolean f(Object o) { return o instanceof String; }
}
```

### JAVA-ICC-235 case 2: class variable
settings: default
language: java
```before
class A {
  boolean f(Object o, Class<?> c) { return c.<caret>isInstance(o); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-236 case 1: containsAll(singletonList)
settings: default
status: unverified (fix name `{0}` text)
language: java
```before
import java.util.*;
class A {
  boolean f(Collection<String> collection) { return collection.<caret>containsAll(Collections.singletonList("x")); }
}
```
keys: Alt+Enter, choose "Replace with 'contains'"
```after
import java.util.*;
class A {
  boolean f(Collection<String> collection) { return collection.contains("x"); }
}
```

### JAVA-ICC-236 case 2: containsAll with a collection
settings: default
language: java
```before
import java.util.*;
class A {
  boolean f(Collection<String> collection, Collection<String> other) { return collection.<caret>containsAll(other); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-237 case 1: thenComparing(comparing(...))
settings: default
language: java
```before
import java.util.*;
import java.util.function.Function;
class A {
  Comparator<String> f(Comparator<String> c, Function<String, String> function) { return c.thenComparing(Comparator.<caret>comparing(function)); }
}
```
keys: Alt+Enter, choose "Remove 'comparing()' call"
```after
import java.util.*;
import java.util.function.Function;
class A {
  Comparator<String> f(Comparator<String> c, Function<String, String> function) { return c.thenComparing(function); }
}
```

### JAVA-ICC-237 case 2: plain thenComparing
settings: default
language: java
```before
import java.util.*;
import java.util.function.Function;
class A {
  Comparator<String> f(Comparator<String> c, Function<String, String> function) { return c.<caret>thenComparing(function); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-239 case 1: embedded literal in a string template
settings: language level 21 preview
language: java
```before
class A {
  void f() { System.out.println(STR."Hello \{<caret>"world"}"); }
}
```
keys: Alt+Enter, choose "Inline embedded expression"
```after
class A {
  void f() { System.out.println(STR."Hello world"); }
}
```

### JAVA-ICC-239 case 2: variable embedded
settings: language level 21 preview
language: java
```before
class A {
  void f(String w) { System.out.println(STR."Hello \{<caret>w}"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-240 case 1: \\b in replacement
settings: default
language: java
```before
class A {
  String f(String string) { return string.replaceAll("a", "<caret>\\b"); }
}
```
keys: Alt+Enter, choose "Remove redundant escape"
```after
class A {
  String f(String string) { return string.replaceAll("a", "b"); }
}
```

### JAVA-ICC-240 case 2: escaped dollar
settings: default
language: java
```before
class A {
  String f(String string) { return string.replaceAll("a", "<caret>\\$"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-241 case 1: new File inside FileInputStream
settings: default
language: java
```before
import java.io.*;
class A {
  InputStream f() throws IOException { return new FileInputStream(<caret>new File("in.txt")); }
}
```
keys: Alt+Enter, choose "Replace with file name"
```after
import java.io.*;
class A {
  InputStream f() throws IOException { return new FileInputStream("in.txt"); }
}
```

### JAVA-ICC-241 case 2: File variable
settings: default
language: java
```before
import java.io.*;
class A {
  InputStream f(File file) throws IOException { return new FileInputStream(<caret>file); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-242 case 1: ChronoField call
settings: default
status: unverified (fix name `{0}` text)
language: java
```before
import java.time.*; import java.time.temporal.ChronoField;
class A {
  int f(LocalTime localTime) { return localTime.<caret>get(ChronoField.MINUTE_OF_HOUR); }
}
```
keys: Alt+Enter, choose "Replace with 'getMinute()' call"
```after
import java.time.*; import java.time.temporal.ChronoField;
class A {
  int f(LocalTime localTime) { return localTime.getMinute(); }
}
```

### JAVA-ICC-242 case 2: redundant creation
settings: default
status: unverified (fix name text)
language: java
```before
import java.time.*;
class A {
  LocalDateTime f() { LocalDateTime now = LocalDateTime.now(); return LocalDateTime.<caret>from(now); }
}
```
keys: Alt+Enter, choose "Remove 'LocalDateTime.from' call"
```after
import java.time.*;
class A {
  LocalDateTime f() { LocalDateTime now = LocalDateTime.now(); return now; }
}
```

### JAVA-ICC-243 case 1: length check before loop
settings: default
language: java
```before
class A {
  void f(String[] array) {
    if (<caret>array.length != 0) {
      for (String str : array) { System.out.println(str); }
    }
  }
}
```
keys: Alt+Enter, choose "Unwrap 'if' statement"
```after
class A {
  void f(String[] array) {
    for (String str : array) { System.out.println(str); }
  }
}
```

### JAVA-ICC-243 case 2: other statements guarded
settings: default
language: java
```before
class A {
  void f(String[] array) {
    if (<caret>array.length != 0) {
      System.out.println("x");
      for (String str : array) { System.out.println(str); }
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-244 case 1: map(x -> x)
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> f(List<String> l) { return l.stream().<caret>map(x -> x).toList(); }
}
```
keys: Alt+Enter, choose "Remove 'map()' call"
```after
import java.util.*;
class A {
  List<String> f(List<String> l) { return l.stream().toList(); }
}
```

### JAVA-ICC-244 case 2: meaningful map
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> f(List<String> l) { return l.stream().<caret>map(x -> x.trim()).toList(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-246 case 1: unmodifiable of singletonList
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> x = Collections.<caret>unmodifiableList(Collections.singletonList("abc"));
}
```
keys: Alt+Enter, choose "Unwrap argument"
```after
import java.util.*;
class A {
  List<String> x = Collections.singletonList("abc");
}
```

### JAVA-ICC-246 case 2: mutable argument
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> x = Collections.<caret>unmodifiableList(new ArrayList<>());
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-247 case 1: search string absent
settings: default
language: java
```before
class A {
  String f(String value) { return "hello".<caret>replace("$value$", value); }
}
```
keys: Alt+Enter, choose "Remove redundant call"
```after
class A {
  String f(String value) { return "hello"; }
}
```

### JAVA-ICC-247 case 2: search string present
settings: default
language: java
```before
class A {
  String f(String value) { return "hello".<caret>replace("l", value); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-248 case 1: groupingBy with maxBy and get
settings: default
language: java
```before
import java.util.*; import java.util.stream.*;
class A {
  Object f() {
    return Stream.of("a").collect(Collectors.<caret>groupingByConcurrent(String::length,
      Collectors.collectingAndThen(Collectors.maxBy(String::compareTo), Optional::get)));
  }
}
```
keys: Alt+Enter, choose "Use 'Collectors.toConcurrentMap()' collector"
```after
import java.util.*; import java.util.function.*; import java.util.stream.*;
class A {
  Object f() {
    return Stream.of("a").collect(Collectors.toConcurrentMap(String::length, Function.identity(), BinaryOperator.maxBy(String::compareTo)));
  }
}
```

### JAVA-ICC-248 case 2: plain groupingBy
settings: default
language: java
```before
import java.util.*; import java.util.stream.*;
class A {
  Object f() { return Stream.of("a").collect(Collectors.<caret>groupingBy(String::length)); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-249 case 1: map(x -> true).orElse(false)
settings: default
language: java
```before
import java.util.Optional;
class A {
  boolean f(Optional<String> optional) { return optional.map(x -> true).<caret>orElse(false); }
}
```
keys: Alt+Enter, choose "Simplify optional chain to 'isPresent()'"
```after
import java.util.Optional;
class A {
  boolean f(Optional<String> optional) { return optional.isPresent(); }
}
```

### JAVA-ICC-249 case 2: meaningful map
settings: default
language: java
```before
import java.util.Optional;
class A {
  boolean f(Optional<String> optional) { return optional.map(x -> x.isEmpty()).<caret>orElse(false); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-250 case 1: stream().forEach
settings: default
status: unverified (fix text: `Replace ''stream().{0}()'' with ''{1}()''{2}`)
language: java
```before
import java.util.*;
class A {
  void f(List<String> list) { list.<caret>stream().forEach(System.out::println); }
}
```
keys: Alt+Enter, choose "Replace 'stream().forEach()' with 'forEach()'"
```after
import java.util.*;
class A {
  void f(List<String> list) { list.forEach(System.out::println); }
}
```

### JAVA-ICC-250 case 2: filter then forEach
settings: default
language: java
```before
import java.util.*;
class A {
  void f(List<String> list) { list.<caret>stream().filter(s -> !s.isEmpty()).forEach(System.out::println); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-251 case 1: appends then toString
settings: default
status: unverified (fix name `Replace ''{0}'' with ''{1}''` text)
language: java
```before
class A {
  String f(int i) {
    StringBuilder <caret>result = new StringBuilder();
    result.append("i = ");
    result.append(i);
    result.append(";");
    return result.toString();
  }
}
```
keys: Alt+Enter, choose "Replace with 'String'"
```after
class A {
  String f(int i) {
    String result = "i = " + i + ";";
    return result;
  }
}
```

### JAVA-ICC-251 case 2: appends in a loop
settings: default
language: java
```before
class A {
  String f(String[] parts) {
    StringBuilder <caret>result = new StringBuilder();
    for (String p : parts) result.append(p);
    return result.toString();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-252 case 1: new String(literal)
settings: default
status: unverified (exact fix name for the constructor case)
language: java
```before
class A {
  void f() { System.out.println(<caret>new String("message")); }
}
```
keys: Alt+Enter, choose "Remove redundant 'String()' call"
```after
class A {
  void f() { System.out.println("message"); }
}
```

### JAVA-ICC-252 case 2: constructor option on
settings: Do not report String constructor calls = on
language: java
```before
class A {
  void f() { System.out.println(<caret>new String("message")); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-253 case 1: empty string prefix
settings: default
language: java
```before
class A {
  void foo(int x, int y) { String s = <caret>"" + x + " ; " + y; }
}
```
keys: Alt+Enter, choose "Remove empty string operand"
```after
class A {
  void foo(int x, int y) { String s = x + " ; " + y; }
}
```

### JAVA-ICC-253 case 2: option on and removal needs conversion
settings: default
language: java
```before
class A {
  void foo(int x, int y) { String s = <caret>"" + x + y; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-254 case 1: break in last rule block
settings: language level 14
language: java
```before
class A {
  enum E { A, B }
  void f(E e) {
    switch (e) {
      case A -> { System.out.println("A"); <caret>break; }
      default -> { System.out.println("Default"); }
    }
  }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'break'"
```after
class A {
  enum E { A, B }
  void f(E e) {
    switch (e) {
      case A -> { System.out.println("A"); }
      default -> { System.out.println("Default"); }
    }
  }
}
```

### JAVA-ICC-254 case 2: break needed
settings: default
language: java
```before
class A {
  void f(int e) {
    switch (e) {
      case 1: System.out.println("A"); <caret>break;
      default: System.out.println("Default");
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-255 case 1: continue as last statement
settings: default
language: java
```before
import java.util.List;
class A {
  void f(List<String> elements) {
    for (String element : elements) { System.out.println(); <caret>continue; }
  }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'continue'"
```after
import java.util.List;
class A {
  void f(List<String> elements) {
    for (String element : elements) { System.out.println(); }
  }
}
```

### JAVA-ICC-255 case 2: continue in the middle
settings: default
language: java
```before
import java.util.List;
class A {
  void f(List<String> elements) {
    for (String element : elements) { if (element.isEmpty()) <caret>continue; System.out.println(element); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-256 case 1: default in enum switch expression
settings: language level 14
language: java
```before
class A {
  enum E { A, B }
  int foo(E e) { return switch (e) { case A -> 1; case B -> 2; <caret>default -> 3; }; }
}
```
keys: Alt+Enter, choose "Remove 'default' branch"
```after
class A {
  enum E { A, B }
  int foo(E e) { return switch (e) { case A -> 1; case B -> 2; }; }
}
```

### JAVA-ICC-256 case 2: constant not covered
settings: language level 14
language: java
```before
class A {
  enum E { A, B, C }
  int foo(E e) { return switch (e) { case A -> 1; case B -> 2; <caret>default -> 3; }; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-257 case 1: break label
settings: default
language: java
```before
class A {
  boolean shouldBreak() { return true; }
  void f() {
    label: for (int i = 0; i < 10; i++) { if (shouldBreak()) break <caret>label; }
  }
}
```
keys: Alt+Enter, choose "Remove label"
```after
class A {
  boolean shouldBreak() { return true; }
  void f() {
    label: for (int i = 0; i < 10; i++) { if (shouldBreak()) break; }
  }
}
```

### JAVA-ICC-257 case 2: label needed for outer loop
settings: default
language: java
```before
class A {
  void f() {
    label: for (int i = 0; i < 10; i++) { for (int j = 0; j < 10; j++) { if (j == i) break <caret>label; } }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-258 case 1: continue LABEL as last statement
settings: default
language: java
```before
class A {
  void f(int a, int b) {
    LABEL: while (a > b) { System.out.println("Hello"); continue <caret>LABEL; }
  }
}
```
keys: Alt+Enter, choose "Remove label"
```after
class A {
  void f(int a, int b) {
    LABEL: while (a > b) { System.out.println("Hello"); continue; }
  }
}
```

### JAVA-ICC-258 case 2: label needed
settings: default
language: java
```before
class A {
  void f() {
    LABEL: for (int i = 0; i < 3; i++) { for (int j = 0; j < 3; j++) { if (j == 1) continue <caret>LABEL; } }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-260 case 1: escaped single quote
settings: default
language: java
```before
class A {
  String s = "<caret>\'Scare\' quotes";
}
```
keys: Alt+Enter, choose "Remove unnecessary escaping"
```after
class A {
  String s = "'Scare' quotes";
}
```

### JAVA-ICC-260 case 2: escaped tab is kept
settings: default
language: java
```before
class A {
  String s = "a<caret>\tb";
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-262 case 1: inherited field vs parameter
settings: inspection AmbiguousFieldAccess enabled
language: java
```before
class First { protected String ambiguous; }
class Second {
  void foo(String ambiguous) {
    new First() { { System.out.println(<caret>ambiguous); } };
  }
}
```
keys: Alt+Enter, choose "Add 'super' qualifier to field access"
```after
class First { protected String ambiguous; }
class Second {
  void foo(String ambiguous) {
    new First() { { System.out.println(super.ambiguous); } };
  }
}
```

### JAVA-ICC-262 case 2: no surrounding variable of that name
settings: inspection AmbiguousFieldAccess enabled
language: java
```before
class First { protected String ambiguous; }
class Second {
  void foo(String other) {
    new First() { { System.out.println(<caret>ambiguous); } };
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-263 case 1: inherited method vs outer method
settings: inspection AmbiguousMethodCall enabled
language: java
```before
class Parent { void ambiguous() {} }
class Example {
  void ambiguous() {}
  class Inner extends Parent { void example() { <caret>ambiguous(); } }
}
```
keys: Alt+Enter, choose "Add 'super' qualifier to method call"
```after
class Parent { void ambiguous() {} }
class Example {
  void ambiguous() {}
  class Inner extends Parent { void example() { super.ambiguous(); } }
}
```

### JAVA-ICC-263 case 2: no outer method
settings: inspection AmbiguousMethodCall enabled
language: java
```before
class Parent { void ambiguous() {} }
class Example {
  class Inner extends Parent { void example() { <caret>ambiguous(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-264 case 1: field hides parameter
settings: inspection AnonymousClassVariableHidesContainingMethodVariable enabled
language: java
```before
class Test {
  public Test(String value) {
    Object foo = new Object() { private String <caret>value = "TEST"; };
  }
}
```
keys: inspect
```after
problem: WARNING value "Anonymous class field value hides variable in containing method"
```

### JAVA-ICC-264 case 2: different name
settings: inspection AnonymousClassVariableHidesContainingMethodVariable enabled
language: java
```before
class Test {
  public Test(String value) {
    Object foo = new Object() { private String <caret>other = "TEST"; };
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-265 case 1: public method returning a private class
settings: default
language: java
```before
public class Parent {
  public <caret>Child getChild() { return new Child(); }
  private class Child {}
}
```
keys: inspect
```after
problem: WARNING Child "Class Child is exposed outside its defined visibility scope"
```

### JAVA-ICC-265 case 2: private method
settings: default
language: java
```before
public class Parent {
  private <caret>Child getChild() { return new Child(); }
  private class Child {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-266 case 1: same name in subclass
settings: inspection FieldHidesSuperclassField enabled
language: java
```before
class Parent { Parent parent; }
class Child extends Parent { Child <caret>parent; }
```
keys: inspect
```after
problem: WARNING parent "Field parent hides field in superclass"
```

### JAVA-ICC-266 case 2: private superclass field
settings: inspection FieldHidesSuperclassField enabled
language: java
```before
class Parent { private Parent parent; }
class Child extends Parent { Child <caret>parent; }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-267 case 1: inner field hides outer field
settings: inspection InnerClassVariableHidesOuterClassVariable enabled
language: java
```before
class Outer {
  private String name;
  class Inner { private String <caret>name; }
}
```
keys: inspect
```after
problem: WARNING name "Inner class field name hides outer class field"
```

### JAVA-ICC-267 case 2: static nested class and instance outer field
settings: inspection InnerClassVariableHidesOuterClassVariable enabled
status: unverified (static nested classes cannot see instance fields; whether the inspection treats them as not visible)
language: java
```before
class Outer {
  private String name;
  static class Inner { private String <caret>name; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-268 case 1: exports to itself
settings: language level 9
language: java
```before
--- module-info.java
module com.mycomp { exports com.mycomp.main to <caret>com.mycomp; }
```
keys: Alt+Enter, choose "Delete directive"
```after
--- module-info.java
module com.mycomp { }
```

### JAVA-ICC-268 case 2: exports to another module
settings: language level 9
language: java
```before
--- module-info.java
module com.mycomp { exports com.mycomp.main to <caret>com.other; }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-269 case 1: ServiceLoader without uses
settings: language level 9; module-info.java present without `uses`
language: java
```before
--- module-info.java
module m { }
--- A.java
import java.util.ServiceLoader;
class A {
  Object f() { return ServiceLoader.load(<caret>Runnable.class); }
}
```
keys: Alt+Enter, choose "Add 'uses java.lang.Runnable' directive to module-info.java"
```after
--- module-info.java
module m { uses java.lang.Runnable; }
--- A.java
import java.util.ServiceLoader;
class A {
  Object f() { return ServiceLoader.load(Runnable.class); }
}
```

### JAVA-ICC-269 case 2: uses declared
settings: language level 9
language: java
```before
--- module-info.java
module m { uses java.lang.Runnable; }
--- A.java
import java.util.ServiceLoader;
class A {
  Object f() { return ServiceLoader.load(<caret>Runnable.class); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-270 case 1: empty module body
settings: language level 9; A.java imports `java.sql.Connection`
status: unverified (requires derived from imports of the module's sources)
language: java
```before
--- module-info.java
module <caret>m { }
```
keys: Alt+Enter, choose "Fill in module dependencies"
```after
--- module-info.java
module m { requires java.sql; }
```

### JAVA-ICC-270 case 2: body has a comment
settings: language level 9
language: java
```before
--- module-info.java
module <caret>m { // no dependencies
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-271 case 1: lambda parameter named like a field
settings: inspection LambdaParameterHidingMemberVariable enabled
language: java
```before
import java.util.List;
public class MyClass {
  public Object foo;
  void sort(List<Integer> list) { list.sort((<caret>foo, bar) -> 0); }
}
```
keys: inspect
```after
problem: WARNING foo "Lambda parameter foo hides field in class 'MyClass'"
```

### JAVA-ICC-271 case 2: different names
settings: inspection LambdaParameterHidingMemberVariable enabled
language: java
```before
import java.util.List;
public class MyClass {
  public Object foo;
  void sort(List<Integer> list) { list.sort((<caret>a, b) -> 0); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-272 case 1: local hides field
settings: inspection LocalVariableHidingMemberVariable enabled
language: java
```before
public class Foo {
  public Object foo;
  void bar() { Object <caret>foo; }
}
```
keys: inspect
```after
problem: WARNING foo "Local variable foo hides field in class 'Foo'"
```

### JAVA-ICC-272 case 2: static method hiding instance field
settings: inspection LocalVariableHidingMemberVariable enabled
language: java
```before
public class Foo {
  public Object foo;
  static void bar() { Object <caret>foo; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-273 case 1: foo(Number) vs foo(int)
settings: inspection MethodOverloadsParentMethod enabled
language: java
```before
public class Foo { void foo(int x) {} }
public class Bar extends Foo { void <caret>foo(Number x) {} }
```
keys: inspect
```after
problem: WARNING foo "Method foo() overloads a compatible method of a superclass, when overriding might have been intended"
```

### JAVA-ICC-273 case 2: an override
settings: inspection MethodOverloadsParentMethod enabled
language: java
```before
public class Foo { void foo(int x) {} }
public class Bar extends Foo { void <caret>foo(int x) {} }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-274 case 1: private super method
settings: inspection MethodOverridesInaccessibleMethodOfSuper enabled
status: unverified (message text not in the bundle excerpt read)
language: java
```before
public class Super { private void test() { } }
public class Sub extends Super { void <caret>test() { } }
```
keys: inspect
```after
problem: WARNING test "(message: method overrides an inaccessible method of the superclass)"
```

### JAVA-ICC-274 case 2: unrelated name
settings: inspection MethodOverridesInaccessibleMethodOfSuper enabled
language: java
```before
public class Super { private void test() { } }
public class Sub extends Super { void <caret>other() { } }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-275 case 1: static hides static
settings: inspection MethodOverridesStaticMethod enabled
language: java
```before
class Parent { static void method() {} }
class Example extends Parent { static void <caret>method() {} }
```
keys: inspect
```after
problem: WARNING method "Method method() tries to override a static method of a superclass"
```

### JAVA-ICC-275 case 2: different signature
settings: inspection MethodOverridesStaticMethod enabled
language: java
```before
class Parent { static void method() {} }
class Example extends Parent { static void <caret>method(int i) {} }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-276 case 1: constructor-less method parameter
settings: inspection ParameterHidingMemberVariable enabled
language: java
```before
class Main {
  private String value;
  void set(String <caret>value) { this.value = value; }
}
```
keys: inspect
```after
problem: WARNING value "Parameter value hides field in class 'Main'"
```

### JAVA-ICC-276 case 2: static method parameter vs instance field
settings: inspection ParameterHidingMemberVariable enabled
language: java
```before
class Main {
  private String value;
  static void set(String <caret>value) { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-277 case 1: pattern variable named like a field
settings: language level 17
language: java
```before
class Pointless {
  java.awt.Point p = new java.awt.Point();
  public void test(Object a) {
    if (a instanceof java.awt.Point <caret>p) { System.out.println(p.x); }
  }
}
```
keys: inspect
```after
problem: WARNING p "Pattern variable p hides field in class 'Pointless'"
```

### JAVA-ICC-277 case 2: different name
settings: language level 17
language: java
```before
class Pointless {
  java.awt.Point p = new java.awt.Point();
  public void test(Object a) {
    if (a instanceof java.awt.Point <caret>q) { System.out.println(q.x); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-278 case 1: public constructor in package-private class
settings: inspection PublicConstructorInNonPublicClass enabled
language: java
```before
class House {
  <caret>public House() {}
}
```
keys: Alt+Enter, choose "Make constructor private"
```after
class House {
  House() {}
}
```

### JAVA-ICC-278 case 2: public class
settings: inspection PublicConstructorInNonPublicClass enabled
language: java
```before
public class House {
  <caret>public House() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-279 case 1: method type parameter hides class type parameter
settings: default
language: java
```before
import java.util.*;
abstract class MyList<T> extends AbstractList<T> {
  private List<T> elements;
  public <<caret>T> T[] toArray(T[] array) { return elements.toArray(array); }
}
```
keys: inspect
```after
problem: WARNING T "Type parameter T hides type parameter 'T'"
```

### JAVA-ICC-279 case 2: distinct names
settings: default
language: java
```before
import java.util.*;
abstract class MyList<T> extends AbstractList<T> {
  private List<T> elements;
  public <<caret>E> E[] toArray(E[] array) { return elements.toArray(array); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-280 case 1: class without toString
settings: inspection ClassHasNoToStringMethod enabled
language: java
```before
class <caret>Point { int x; }
```
keys: inspect
```after
problem: WARNING Point "Class 'Point' does not override 'toString()' method"
```

### JAVA-ICC-280 case 2: record is ignored by default
settings: inspection ClassHasNoToStringMethod enabled; language level 16
language: java
```before
record <caret>Point(int x) { }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-281 case 1: field missing from toString
settings: inspection FieldNotUsedInToString enabled
language: java
```before
public class Relevant {
  private String <caret>name;
  private int index;
  @Override public String toString() { return "Relevant{" + "index=" + index + '}'; }
}
```
keys: inspect
```after
problem: WARNING name "Field 'name' is not used in 'toString()' method"
```

### JAVA-ICC-281 case 2: field used
settings: inspection FieldNotUsedInToString enabled
language: java
```before
public class Relevant {
  private int <caret>index;
  @Override public String toString() { return "Relevant{" + "index=" + index + '}'; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-282 case 1: setScale with ROUND_FLOOR
settings: default
language: java
```before
import java.math.BigDecimal;
class A {
  Object f() { return new BigDecimal("42").<caret>setScale(2, BigDecimal.ROUND_FLOOR); }
}
```
keys: Alt+Enter, choose "Use 'RoundingMode' enum constant"
```after
import java.math.BigDecimal;
import java.math.RoundingMode;
class A {
  Object f() { return new BigDecimal("42").setScale(2, RoundingMode.FLOOR); }
}
```

### JAVA-ICC-282 case 2: RoundingMode argument
settings: default
language: java
```before
import java.math.*;
class A {
  Object f() { return new BigDecimal("42").<caret>setScale(2, RoundingMode.FLOOR); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-283 case 1: EMPTY_LIST
settings: inspection CollectionsFieldAccessReplaceableByMethodCall enabled
language: java
```before
import java.util.*;
class A {
  List<Integer> emptyList = Collections.<caret>EMPTY_LIST;
}
```
keys: Alt+Enter, choose "Replace with 'Collections.emptyList()'"
```after
import java.util.*;
class A {
  List<Integer> emptyList = Collections.emptyList();
}
```

### JAVA-ICC-283 case 2: method call already
settings: inspection CollectionsFieldAccessReplaceableByMethodCall enabled
language: java
```before
import java.util.*;
class A {
  List<Integer> emptyList = Collections.<caret>emptyList();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-285 case 1: indexOf >= 0
settings: default
status: unverified (the shown `{0}` text of the comparison)
language: java
```before
class A {
  boolean b = "abcd".<caret>indexOf("e") >= 0;
}
```
keys: Alt+Enter, choose "Replace 'indexOf() >= 0' with 'contains()'"
```after
class A {
  boolean b = "abcd".contains("e");
}
```

### JAVA-ICC-285 case 2: index used
settings: default
language: java
```before
class A {
  int i = "abcd".<caret>indexOf("e");
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-286 case 1: trailing array parameter
settings: default
language: java
```before
class A {
  void <caret>process(String name, Object[] objects) {}
}
```
keys: Alt+Enter, choose "Convert to varargs method"
```after
class A {
  void process(String name, Object... objects) {}
}
```

### JAVA-ICC-286 case 2: array not last
settings: default
language: java
```before
class A {
  void <caret>process(Object[] objects, String name) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-288 case 1: local StringBuffer
settings: default
language: java
```before
class A {
  String f() { StringBuffer <caret>sb = new StringBuffer(); sb.append("a"); return sb.toString(); }
}
```
keys: Alt+Enter, choose "Replace with 'StringBuilder'"
```after
class A {
  String f() { StringBuilder sb = new StringBuilder(); sb.append("a"); return sb.toString(); }
}
```

### JAVA-ICC-288 case 2: StringBuffer passed to an API requiring it
settings: default
language: java
```before
class A {
  void take(StringBuffer b) {}
  void f() { StringBuffer <caret>sb = new StringBuffer(); take(sb); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-291 case 1: iterator while loop
settings: default
language: java
```before
import java.util.*;
class A {
  void f(Collection<Object> c) {
    Iterator<Object> it = c.iterator();
    <caret>while (it.hasNext()) { Object obj = it.next(); System.out.println(obj); }
  }
}
```
keys: Alt+Enter, choose "Replace with enhanced 'for'"
```after
import java.util.*;
class A {
  void f(Collection<Object> c) {
    for (Object obj : c) { System.out.println(obj); }
  }
}
```

### JAVA-ICC-291 case 2: iterator also used for remove
settings: default
language: java
```before
import java.util.*;
class A {
  void f(Collection<Object> c) {
    Iterator<Object> it = c.iterator();
    <caret>while (it.hasNext()) { Object obj = it.next(); it.remove(); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-292 case 1: explicit type argument
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> list = new ArrayList<<caret>String>();
}
```
keys: Alt+Enter, choose "Replace with <>"
```after
import java.util.*;
class A {
  List<String> list = new ArrayList<>();
}
```

### JAVA-ICC-292 case 2: already diamond
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> list = new <caret>ArrayList<>();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-293 case 1: ternary null-safe equals with option off
settings: default
status: unverified (import handling and fix `{0}` text)
language: java
```before
class A {
  boolean f(Object a, Object b) { return <caret>a == null ? b == null : a.equals(b); }
}
```
keys: Alt+Enter, choose "Replace with 'Objects.equals()'"
```after
class A {
  boolean f(Object a, Object b) { return java.util.Objects.equals(a, b); }
}
```

### JAVA-ICC-293 case 2: a != null && a.equals(b) with option off
settings: default
language: java
```before
class A {
  boolean f(Object a, Object b) { return <caret>a != null && a.equals(b); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-294 case 1: final generic varargs method
settings: default
language: java
```before
import java.util.*;
public class Foo<T> {
  private List<T> list = new ArrayList<>();
  public final void safeVarargs(T... <caret>elements) { Collections.addAll(list, elements); }
}
```
keys: Alt+Enter, choose "Annotate as '@SafeVarargs'"
```after
import java.util.*;
public class Foo<T> {
  private List<T> list = new ArrayList<>();
  @SafeVarargs
  public final void safeVarargs(T... elements) { Collections.addAll(list, elements); }
}
```

### JAVA-ICC-294 case 2: reifiable element type
settings: default
language: java
```before
public class Foo {
  public final void f(String... <caret>elements) { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-295 case 1: close in finally
settings: default
language: java
```before
import java.io.*;
class A {
  void f(String fileName) throws IOException {
    PrintStream printStream = new PrintStream(fileName);
    <caret>try { printStream.print(true); } finally { printStream.close(); }
  }
}
```
keys: Alt+Enter, choose "Replace with 'try' with resources"
```after
import java.io.*;
class A {
  void f(String fileName) throws IOException {
    try (PrintStream printStream = new PrintStream(fileName)) { printStream.print(true); }
  }
}
```

### JAVA-ICC-295 case 2: finally does something else
settings: default
language: java
```before
import java.io.*;
class A {
  void f(String fileName) throws IOException {
    PrintStream printStream = new PrintStream(fileName);
    <caret>try { printStream.print(true); } finally { System.out.println(); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-296 case 1: two identical catches
settings: default
language: java
```before
import java.io.*; import java.net.*;
class A {
  void doSmth() throws IOException, URISyntaxException {}
  void f() {
    try { doSmth(); }
    catch (IOException e) { e.printStackTrace(); }
    catch (<caret>URISyntaxException e) { e.printStackTrace(); }
  }
}
```
keys: Alt+Enter, choose "Collapse 'catch' blocks"
```after
import java.io.*; import java.net.*;
class A {
  void doSmth() throws IOException, URISyntaxException {}
  void f() {
    try { doSmth(); }
    catch (IOException | URISyntaxException e) { e.printStackTrace(); }
  }
}
```

### JAVA-ICC-296 case 2: different bodies
settings: default
language: java
```before
import java.io.*; import java.net.*;
class A {
  void doSmth() throws IOException, URISyntaxException {}
  void f() {
    try { doSmth(); }
    catch (IOException e) { e.printStackTrace(); }
    catch (<caret>URISyntaxException e) { throw new RuntimeException(e); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-297 case 1: anonymous Runnable calling a method
settings: default
language: java
```before
class A {
  void f() {
    Runnable r = new <caret>Runnable() { @Override public void run() { System.out.println(); } };
  }
}
```
keys: Alt+Enter, choose "Replace with method reference"
```after
class A {
  void f() {
    Runnable r = System.out::println;
  }
}
```

### JAVA-ICC-297 case 2: body with two statements
settings: default
language: java
```before
class A {
  void f() {
    Runnable r = new <caret>Runnable() { @Override public void run() { System.out.println(); System.out.println(); } };
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-298 case 1: anonymous Thread
settings: default
status: unverified (fix text uses `{0}`; body braces kept)
language: java
```before
class A {
  void f() {
    new <caret>Thread() { @Override public void run() { System.out.println("Hello from thread!"); } }.start();
  }
}
```
keys: Alt+Enter, choose "Replace with new Thread(Runnable)"
```after
class A {
  void f() {
    new Thread(() -> { System.out.println("Hello from thread!"); }).start();
  }
}
```

### JAVA-ICC-298 case 2: Thread with extra members
settings: default
language: java
```before
class A {
  void f() {
    new <caret>Thread() { int x; @Override public void run() { System.out.println(x); } }.start();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-300 case 1: comparing by getter
settings: default
language: java
```before
import java.util.*;
class A {
  static class Person { String getName() { return ""; } }
  void f(List<Person> myList) { myList.sort(<caret>(person1, person2) -> person1.getName().compareTo(person2.getName())); }
}
```
keys: Alt+Enter, choose "Replace with 'Comparator.comparing(Person::getName)'"
```after
import java.util.*;
class A {
  static class Person { String getName() { return ""; } }
  void f(List<Person> myList) { myList.sort(Comparator.comparing(Person::getName)); }
}
```

### JAVA-ICC-300 case 2: comparing different getters
settings: default
language: java
```before
import java.util.*;
class A {
  static class Person { String getName() { return ""; } String getTitle() { return ""; } }
  void f(List<Person> myList) { myList.sort(<caret>(p1, p2) -> p1.getName().compareTo(p2.getTitle())); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-301 case 1: anonymous Runnable
settings: default
language: java
```before
class A {
  void f() { new Thread(new <caret>Runnable() { @Override public void run() { System.out.println("run"); } }); }
}
```
keys: Alt+Enter, choose "Replace with lambda"
```after
class A {
  void f() { new Thread(() -> { System.out.println("run"); }); }
}
```

### JAVA-ICC-301 case 2: anonymous class with a field
settings: default
language: java
```before
class A {
  void f() { new Thread(new <caret>Runnable() { int x; @Override public void run() { System.out.println(x); } }); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-303 case 1: loop to allMatch
settings: default
status: unverified (fix name wording)
language: java
```before
import java.util.List;
class A {
  boolean check(List<String> data) {
    <caret>for (String e : data) {
      String trimmed = e.trim();
      if (!trimmed.startsWith("xyz")) { return false; }
    }
    return true;
  }
}
```
keys: Alt+Enter, choose "Replace with allMatch"
```after
import java.util.List;
class A {
  boolean check(List<String> data) {
    return data.stream().map(String::trim).allMatch(trimmed -> trimmed.startsWith("xyz"));
  }
}
```

### JAVA-ICC-303 case 2: loop with a break
settings: default
language: java
```before
import java.util.List;
class A {
  int check(List<String> data) {
    int n = 0;
    <caret>for (String e : data) { n++; if (n > 3) break; System.out.println(e); }
    return n;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-304 case 1: repeated startsWith
settings: default
language: java
```before
class A {
  boolean allStartWith(String a, String b, String c, String d, String prefix) {
    return <caret>a.startsWith(prefix) && b.startsWith(prefix) && c.startsWith(prefix) && d.startsWith(prefix);
  }
}
```
keys: Alt+Enter, choose "Fold expression"
```after
import java.util.stream.Stream;
class A {
  boolean allStartWith(String a, String b, String c, String d, String prefix) {
    return Stream.of(a, b, c, d).allMatch(s -> s.startsWith(prefix));
  }
}
```

### JAVA-ICC-304 case 2: two operands only
settings: default
language: java
```before
class A {
  boolean f(String a, String b, String prefix) { return <caret>a.startsWith(prefix) && b.startsWith(prefix); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-306 case 1: iterator remove loop
settings: default
language: java
```before
import java.util.*;
class A {
  boolean shouldBeRemoved(String s) { return s.isEmpty(); }
  void f(Collection<String> collection) {
    <caret>for (Iterator<String> it = collection.iterator(); it.hasNext(); ) {
      String aValue = it.next();
      if (shouldBeRemoved(aValue)) { it.remove(); }
    }
  }
}
```
keys: Alt+Enter, choose "Replace loop with 'Collection.removeIf()'"
```after
import java.util.*;
class A {
  boolean shouldBeRemoved(String s) { return s.isEmpty(); }
  void f(Collection<String> collection) {
    collection.removeIf(aValue -> shouldBeRemoved(aValue));
  }
}
```

### JAVA-ICC-306 case 2: other side effects in the loop
settings: default
language: java
```before
import java.util.*;
class A {
  void f(Collection<String> collection) {
    <caret>for (Iterator<String> it = collection.iterator(); it.hasNext(); ) {
      String aValue = it.next();
      System.out.println(aValue);
      if (aValue.isEmpty()) { it.remove(); }
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-307 case 1: set(i, f(get(i)))
settings: default
language: java
```before
import java.util.*;
class A {
  void f(List<String> strings) {
    <caret>for (int i = 0; i < strings.size(); i++) { String str = strings.get(i).toLowerCase(); strings.set(i, str); }
  }
}
```
keys: Alt+Enter, choose "Replace loop with 'List.replaceAll()'"
```after
import java.util.*;
class A {
  void f(List<String> strings) {
    strings.replaceAll(String::toLowerCase);
  }
}
```

### JAVA-ICC-307 case 2: index used for another purpose
settings: default
language: java
```before
import java.util.*;
class A {
  void f(List<String> strings) {
    <caret>for (int i = 0; i < strings.size(); i++) { strings.set(i, strings.get(i) + i); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-308 case 1: Collections.sort with comparator
settings: default
language: java
```before
import java.util.*;
class A {
  void f(List<String> list, Comparator<String> cmp) { Collections.<caret>sort(list, cmp); }
}
```
keys: Alt+Enter, choose "Replace with 'List.sort()'"
```after
import java.util.*;
class A {
  void f(List<String> list, Comparator<String> cmp) { list.sort(cmp); }
}
```

### JAVA-ICC-308 case 2: list.sort already
settings: default
language: java
```before
import java.util.*;
class A {
  void f(List<String> list, Comparator<String> cmp) { list.<caret>sort(cmp); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-309 case 1: containsKey ? get : default
settings: default
language: java
```before
import java.util.*;
class A {
  String f(Map<String, String> map, String key) { return <caret>map.containsKey(key) ? map.get(key) : "default"; }
}
```
keys: Alt+Enter, choose "Replace with 'getOrDefault()' call"
```after
import java.util.*;
class A {
  String f(Map<String, String> map, String key) { return map.getOrDefault(key, "default"); }
}
```

### JAVA-ICC-309 case 2: different keys
settings: default
language: java
```before
import java.util.*;
class A {
  String f(Map<String, String> map, String key, String k2) { return <caret>map.containsKey(key) ? map.get(k2) : "default"; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-310 case 1: entrySet().forEach
settings: default
status: unverified (fix text `{0}` and lambda body shape)
language: java
```before
import java.util.*;
class A {
  void print(Map<String, Integer> map) {
    map.entrySet().<caret>forEach(entry -> { String str = entry.getKey(); System.out.println(str + ":" + entry.getValue()); });
  }
}
```
keys: Alt+Enter, choose "Replace with 'Map.forEach()'"
```after
import java.util.*;
class A {
  void print(Map<String, Integer> map) {
    map.forEach((str, value) -> System.out.println(str + ":" + value));
  }
}
```

### JAVA-ICC-310 case 2: entry used as a whole
settings: default
language: java
```before
import java.util.*;
class A {
  void print(Map<String, Integer> map, List<Object> out) {
    map.entrySet().<caret>forEach(entry -> out.add(entry));
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-311 case 1: x -> x
settings: default
language: java
```before
import java.util.function.Function;
class A {
  Function<String, String> f = <caret>x -> x;
}
```
keys: Alt+Enter, choose "Replace lambda expression with 'Function.identity()'"
```after
import java.util.function.Function;
class A {
  Function<String, String> f = Function.identity();
}
```

### JAVA-ICC-311 case 2: transforming lambda
settings: default
language: java
```before
import java.util.function.Function;
class A {
  Function<String, String> f = <caret>x -> x.trim();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-312 case 1: forEach adding to a list
settings: default
status: unverified (fix text; collect form)
language: java
```before
import java.util.*;
class A {
  List<String> findNStrings(List<String> list, int n) {
    List<String> other = new ArrayList<>();
    list.<caret>forEach(s -> { if (s.length() > n) other.add(s); });
    return other;
  }
}
```
keys: Alt+Enter, choose "Replace with 'collect'"
```after
import java.util.*;
import java.util.stream.*;
class A {
  List<String> findNStrings(List<String> list, int n) {
    List<String> other = list.stream().filter(s -> s.length() > n).collect(Collectors.toList());
    return other;
  }
}
```

### JAVA-ICC-312 case 2: plain forEach
settings: default
language: java
```before
import java.util.*;
class A {
  void f(List<String> list) { list.<caret>forEach(s -> System.out.println(s)); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-314 case 1: manual long hash
settings: language level 8
language: java
```before
class A {
  int f(long var) { int result = (int)(<caret>var ^ (var >>> 32)); return result; }
}
```
keys: Alt+Enter, choose "Replace with 'Long.hashCode(var)'"
```after
class A {
  int f(long var) { int result = Long.hashCode(var); return result; }
}
```

### JAVA-ICC-314 case 2: different shift
settings: language level 8
language: java
```before
class A {
  int f(long var) { int result = (int)(<caret>var ^ (var >>> 16)); return result; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-315 case 1: unmodifiableList(asList)
settings: language level 9
language: java
```before
import java.util.*;
class A {
  List<Integer> even = Collections.<caret>unmodifiableList(Arrays.asList(2, 4, 6, 8, 10, 2));
}
```
keys: Alt+Enter, choose "Replace with 'List.of()' call"
```after
import java.util.*;
class A {
  List<Integer> even = List.of(2, 4, 6, 8, 10, 2);
}
```

### JAVA-ICC-315 case 2: level 8
settings: language level 8
language: java
```before
import java.util.*;
class A {
  List<Integer> even = Collections.<caret>unmodifiableList(Arrays.asList(2, 4, 6));
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-316 case 1: requires transitive an automatic module
settings: language level 9; library `drivers.corp.org` is an automatic module
language: java
```before
--- module-info.java
module org.printer { requires transitive <caret>drivers.corp.org; }
```
keys: inspect
```after
problem: WARNING drivers.corp.org "'requires transitive' directive for an automatic module"
```

### JAVA-ICC-316 case 2: plain requires with default option
settings: language level 9; library `drivers.corp.org` is an automatic module
language: java
```before
--- module-info.java
module org.printer { requires <caret>drivers.corp.org; }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-317 case 1: if/else null check
settings: language level 9
status: unverified (import style and lambda vs method reference)
language: java
```before
class A {
  void save(Object o) {}
  void f(Object message) {
    <caret>if (message == null) { save(new Object()); } else { save(message); }
  }
}
```
keys: Alt+Enter, choose "Replace 'if' statement with 'requireNonNullElseGet()' call"
```after
class A {
  void save(Object o) {}
  void f(Object message) {
    save(java.util.Objects.requireNonNullElseGet(message, () -> new Object()));
  }
}
```

### JAVA-ICC-317 case 2: unrelated branches
settings: language level 9
language: java
```before
class A {
  void save(Object o) {}
  void log(Object o) {}
  void f(Object message) {
    <caret>if (message == null) { log(null); } else { save(message); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-318 case 1: replace with var
settings: language level 10
language: java
```before
import java.io.InputStream;
class A {
  void test(InputStream s) { try (<caret>InputStream in = s) {} catch (Exception e) {} }
}
```
keys: Alt+Enter, choose "Replace explicit type with 'var'"
```after
import java.io.InputStream;
class A {
  void test(InputStream s) { try (var in = s) {} catch (Exception e) {} }
}
```

### JAVA-ICC-318 case 2: option on, initializer not obvious
settings: language level 10; Only report when the variable type is obvious from the initializer = on
language: java
```before
class A {
  void f() { <caret>int codePointAt = "hello".codePointAt(1); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-319 case 1: var to explicit type
settings: language level 10
language: java
```before
class A {
  void f() { <caret>var str = "Hello"; }
}
```
keys: Alt+Enter, choose "Replace 'var' with explicit type"
```after
class A {
  void f() { String str = "Hello"; }
}
```

### JAVA-ICC-319 case 2: explicit already
settings: language level 10
language: java
```before
class A {
  void f() { <caret>String str = "Hello"; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-320 case 1: readAllBytes
settings: language level 11
language: java
```before
import java.nio.file.*; import java.nio.charset.StandardCharsets; import java.io.IOException;
class A {
  String f() throws IOException { return new String(Files.<caret>readAllBytes(Paths.get("in.txt")), StandardCharsets.ISO_8859_1); }
}
```
keys: Alt+Enter, choose "Replace with 'Files.readString()'"
```after
import java.nio.file.*; import java.nio.charset.StandardCharsets; import java.io.IOException;
class A {
  String f() throws IOException { return Files.readString(Paths.get("in.txt"), StandardCharsets.ISO_8859_1); }
}
```

### JAVA-ICC-320 case 2: bytes not converted to String
settings: language level 11
language: java
```before
import java.nio.file.*; import java.io.IOException;
class A {
  byte[] f() throws IOException { return Files.<caret>readAllBytes(Paths.get("in.txt")); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-321 case 1: switch expression to old style
settings: language level 14
language: java
```before
class A {
  boolean f(int condition) {
    boolean even = <caret>switch (condition) { case 1, 3, 5, 7, 9 -> false; default -> true; };
    return even;
  }
}
```
keys: Alt+Enter, choose "Replace with old style 'switch' statement"
```after
class A {
  boolean f(int condition) {
    boolean even;
    switch (condition) {
      case 1:
      case 3:
      case 5:
      case 7:
      case 9:
        even = false;
        break;
      default:
        even = true;
        break;
    }
    return even;
  }
}
```

### JAVA-ICC-321 case 2: old style switch
settings: language level 14
language: java
```before
class A {
  void f(int c) {
    <caret>switch (c) { case 1: System.out.println(); break; default: break; }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-322 case 1: returning switch statement
settings: language level 14
language: java
```before
class A {
  double getPrice(String fruit) {
    <caret>switch (fruit) {
      case "Apple": return 1.0;
      case "Orange": return 1.5;
      case "Mango": return 2.0;
      default: throw new IllegalArgumentException();
    }
  }
}
```
keys: Alt+Enter, choose "Migrate to enhanced switch"
```after
class A {
  double getPrice(String fruit) {
    return switch (fruit) {
      case "Apple" -> 1.0;
      case "Orange" -> 1.5;
      case "Mango" -> 2.0;
      default -> throw new IllegalArgumentException();
    };
  }
}
```

### JAVA-ICC-322 case 2: fallthrough with side effects
settings: language level 14
status: unverified (switches with fall-through are converted only when it can be expressed with multiple labels)
language: java
```before
class A {
  void f(int c) {
    <caret>switch (c) {
      case 1: System.out.println("1");
      case 2: System.out.println("2"); break;
      default: break;
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-323 case 1: text block to string literal
settings: language level 15
status: unverified (concatenation layout)
language: java
```before
class A {
  String s = <caret>"""
    hello
    world
    """;
}
```
keys: Alt+Enter, choose "Replace with regular string literal"
```after
class A {
  String s = "hello\n" +
    "world\n";
}
```

### JAVA-ICC-323 case 2: regular literal
settings: language level 15
language: java
```before
class A {
  String s = <caret>"hello";
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-324 case 1: html concatenation
settings: language level 15
language: java
```before
class A {
  String html = <caret>"<html>\n" +
    "  <body>\n" +
    "  </body>\n" +
    "</html>\n";
}
```
keys: Alt+Enter, choose "Replace with text block"
```after
class A {
  String html = """
    <html>
      <body>
      </body>
    </html>
    """;
}
```

### JAVA-ICC-324 case 2: only one newline
settings: language level 15
language: java
```before
class A {
  String s = <caret>"a\n" + "b";
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-325 case 1: immutable point class
settings: language level 16
status: unverified (with accessor renaming the getters become `x()`/`y()` accessors and callers are renamed; the declaration shown is the result)
language: java
```before
class <caret>Point {
  private final double x;
  private final double y;
  Point(double x, double y) { this.x = x; this.y = y; }
  double getX() { return x; }
  double getY() { return y; }
}
```
keys: Alt+Enter, choose "Convert to record class"
```after
record Point(double x, double y) { }
```

### JAVA-ICC-325 case 2: non-final field
settings: language level 16
language: java
```before
class <caret>Point {
  private double x;
  Point(double x) { this.x = x; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-326 case 1: cast after instanceof
settings: language level 16
language: java
```before
class A {
  void f(Object obj) {
    if (obj instanceof String) {
      String <caret>str = (String) obj;
      System.out.println(str);
    }
  }
}
```
keys: Alt+Enter, choose "Replace 'str' with pattern variable"
```after
class A {
  void f(Object obj) {
    if (obj instanceof String str) {
      System.out.println(str);
    }
  }
}
```

### JAVA-ICC-326 case 2: language level 15
settings: language level 15
language: java
```before
class A {
  void f(Object obj) {
    if (obj instanceof String) {
      String <caret>str = (String) obj;
      System.out.println(str);
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-327 case 1: accessors extracted to locals
settings: language level 21
language: java
```before
record Point(int x, int y) {
  static void printSum(Object obj) {
    if (obj instanceof <caret>Point p) {
      int x = p.x();
      int y = p.y();
      System.out.println(x + y);
    }
  }
}
```
keys: Alt+Enter, choose "Replace with record pattern"
```after
record Point(int x, int y) {
  static void printSum(Object obj) {
    if (obj instanceof Point(int x, int y)) {
      System.out.println(x + y);
    }
  }
}
```

### JAVA-ICC-327 case 2: pattern variable used as a whole
settings: language level 21
language: java
```before
record Point(int x, int y) {
  static void printSum(Object obj) {
    if (obj instanceof <caret>Point p) {
      int x = p.x();
      System.out.println(p);
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-328 case 1: max(min())
settings: language level 21 with Math.clamp available (JDK 21)
status: unverified (`{0}` text of the fix name)
language: java
```before
class A {
  int f(int input) {
    int min = 5, max = 10;
    input = Math.<caret>max(Math.min(max, input), min);
    return input;
  }
}
```
keys: Alt+Enter, choose "Replace with 'Math.clamp()'"
```after
class A {
  int f(int input) {
    int min = 5, max = 10;
    input = Math.clamp(input, min, max);
    return input;
  }
}
```

### JAVA-ICC-328 case 2: only min
settings: language level 21
language: java
```before
class A {
  int f(int input) { return Math.<caret>min(10, input); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-329 case 1: get(0)
settings: language level 21
status: unverified (fix name `{0}` text)
language: java
```before
import java.util.List;
class A {
  String f(List<String> list) { return list.<caret>get(0); }
}
```
keys: Alt+Enter, choose "Replace with 'getFirst'"
```after
import java.util.List;
class A {
  String f(List<String> list) { return list.getFirst(); }
}
```

### JAVA-ICC-329 case 2: get(1)
settings: language level 21
language: java
```before
import java.util.List;
class A {
  String f(List<String> list) { return list.<caret>get(1); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-330 case 1: concatenation to template
settings: language level 21 preview
language: java
```before
class A {
  String f(String name) { return <caret>"Hello, " + name + ". You are " + 29 + " years old."; }
}
```
keys: Alt+Enter, choose "Replace with string template"
```after
class A {
  String f(String name) { return STR."Hello, \{name}. You are 29 years old."; }
}
```

### JAVA-ICC-330 case 2: constant only
settings: language level 21 preview
language: java
```before
class A {
  String f() { return <caret>"Hello"; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-331 case 1: template to concatenation
settings: language level 21 preview
language: java
```before
class A {
  String f(String name) { return <caret>STR."Hello, \{name}. You are 29 years old."; }
}
```
keys: Alt+Enter, choose "Replace with string concatenation"
```after
class A {
  String f(String name) { return "Hello, " + name + ". You are 29 years old."; }
}
```

### JAVA-ICC-331 case 2: plain literal
settings: language level 21 preview
language: java
```before
class A {
  String f() { return <caret>"Hello"; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-332 case 1: simple Javadoc
settings: language level 23
language: java
```before
/**
 * For blubbering of the florz.
 */
class <caret>FlorzBlubber { }
```
keys: Alt+Enter, choose "Convert to Markdown documentation comment"
```after
/// For blubbering of the florz.
class FlorzBlubber { }
```

### JAVA-ICC-332 case 2: ordinary comment
settings: language level 23
language: java
```before
/* not a doc comment */
class <caret>FlorzBlubber { }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-333 case 1: hello world class
settings: language level 25
status: unverified (`main` signature relaxation and formatting)
language: java
```before
public class <caret>Sample {
  public static void main(String[] args) { System.out.println("Hello, world!"); }
}
```
keys: Alt+Enter, choose "Convert into compact source file"
```after
void main() {
  IO.println("Hello, world!");
}
```

### JAVA-ICC-333 case 2: class with a constructor and state
settings: language level 25
status: unverified (whether fields block the conversion)
language: java
```before
public class <caret>Sample {
  private int x;
  public static void main(String[] args) { System.out.println("Hello"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-334 case 1: compact source file
settings: language level 25; file `Sample.java`
language: java
```before
void <caret>main() { String a = "Hello, world!"; }
```
keys: Alt+Enter, choose "Convert into a regular class"
```after
public class Sample {
  void main() { String a = "Hello, world!"; }
}
```

### JAVA-ICC-334 case 2: ordinary class file
settings: language level 25
language: java
```before
public class Sample { void <caret>main() { } }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-335 case 1: IO.println
settings: language level 25
status: unverified (fix text `{0}`)
language: java
```before
class A {
  void f() { IO.<caret>println("Hello, World!"); }
}
```
keys: Alt+Enter, choose "Replace with 'System.out.println'"
```after
class A {
  void f() { System.out.println("Hello, World!"); }
}
```

### JAVA-ICC-335 case 2: System.out
settings: language level 25
language: java
```before
class A {
  void f() { System.out.<caret>println("Hello, World!"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-336 case 1: System.out.println to IO
settings: language level 25
status: unverified (fix text `{0}`)
language: java
```before
class A {
  void f() { System.out.<caret>println("Hello, World!"); }
}
```
keys: Alt+Enter, choose "Replace with 'IO.println'"
```after
class A {
  void f() { IO.println("Hello, World!"); }
}
```

### JAVA-ICC-336 case 2: option for regular classes off
settings: language level 25; Report in regular classes = off
language: java
```before
class A {
  void f() { System.out.<caret>println("Hello, World!"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-337 case 1: ternary on compare
settings: language level 26 (Comparator.max/min available)
status: unverified (API level and fix text)
language: java
```before
import java.util.Comparator;
class A {
  String f(Comparator<String> comp, String a, String b) { return <caret>comp.compare(a, b) > 0 ? a : b; }
}
```
keys: Alt+Enter, choose "Replace with 'comp.max(a, b)'"
```after
import java.util.Comparator;
class A {
  String f(Comparator<String> comp, String a, String b) { return comp.max(a, b); }
}
```

### JAVA-ICC-337 case 2: different results
settings: language level 26
language: java
```before
import java.util.Comparator;
class A {
  String f(Comparator<String> comp, String a, String b) { return <caret>comp.compare(a, b) > 0 ? a : "x"; }
}
```
keys: inspect
```after
no problem reported
```
