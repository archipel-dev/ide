# Java inspections catalog, part 3: acceptance cases

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-3.md`. Keys `inspect` list the expected highlights as `problem:` lines, `inspect batch` runs the whole-project inspection, and `no problem reported` means no highlight.

### JAVA-ICA-236 case 1: All constants listed in order
settings: default
language: java
```before
enum Sculptor { NOT_RUN, IN_PROGRESS, FINISHED }
class C {
  void handle(Sculptor[] s) {}
  void f() {
    handle(<caret>new Sculptor[] {Sculptor.NOT_RUN, Sculptor.IN_PROGRESS, Sculptor.FINISHED});
  }
}
```
keys: Alt+Enter, choose "Replace array with Sculptor.values()"
```after
enum Sculptor { NOT_RUN, IN_PROGRESS, FINISHED }
class C {
  void handle(Sculptor[] s) {}
  void f() {
    handle(Sculptor.values());
  }
}
```

### JAVA-ICA-236 case 2: Constants in a different order
settings: default
language: java
```before
enum Heron { NOT_RUN, IN_PROGRESS, FINISHED }
class C {
  void handle(Heron[] s) {}
  void f() {
    handle(<caret>new Heron[] {Heron.FINISHED, Heron.NOT_RUN, Heron.IN_PROGRESS});
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-237 case 1: Variable initializer
settings: default
language: java
```before
class C {
  int[] a = <caret>{42};
}
```
keys: Alt+Enter, choose "Add 'new int[]'"
```after
class C {
  int[] a = new int[]{42};
}
```

### JAVA-ICA-237 case 2: Already explicit
settings: default
language: java
```before
class C {
  int[] a = <caret>new int[]{42};
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-238 case 1: boolean assert message
settings: default
language: java
```before
class C {
  void reconcile(java.util.List<Integer> fleet) {
    assert fleet.isEmpty() : <caret>false;
  }
}
```
keys: inspect
```after
problem: warning `false` "'assert' message of type 'boolean'"
```

### JAVA-ICA-239 case 1: end marker
settings: default
language: java
```before
class C {
  void f(int i) {
    while (i < 10) { i++; } <caret>// end while
  }
}
```
keys: Alt+Enter, choose "Remove block marker comment"
```after
class C {
  void f(int i) {
    while (i < 10) { i++; }
  }
}
```

### JAVA-ICA-240 case 1: Consumer parameter
settings: default
status: unverified (fix name shape)
language: java
```before
import java.util.function.Consumer;
class C {
  void process(Consumer<<caret>Number> level) {}
}
```
keys: Alt+Enter, choose "Replace with '? super Number'"
```after
import java.util.function.Consumer;
class C {
  void process(Consumer<? super Number> level) {}
}
```

### JAVA-ICA-241 case 1: concat call
settings: default
language: java
```before
class C {
  String decorate(String left) { return left.<caret>concat("-end"); }
}
```
keys: Alt+Enter, choose "Replace 'concat()' call with '+'"
```after
class C {
  String decorate(String left) { return left + "-end"; }
}
```

### JAVA-ICA-242 case 1: Chained boolean equality
settings: default
language: java
```before
class C {
  boolean f(boolean ready, boolean armed, boolean done) { return <caret>ready == armed == done; }
}
```
keys: inspect
```after
problem: warning `ready == armed == done` "Chained equality comparison ready == armed == done"
```

### JAVA-ICA-243 case 1: dir.getParent().length()
settings: default
language: java
```before
class X {
  int depth(java.io.File dir) { return dir.getParent().<caret>length(); }
}
```
keys: Alt+Enter, choose "Introduce variable"
```after
class X {
  int depth(java.io.File dir) {
    final String parent = dir.getParent();
    return parent.length();
  }
}
```

### JAVA-ICA-244 case 1: getBytes with a charset name
settings: default
language: java
```before
import java.io.UnsupportedEncodingException;
class C {
  byte[] encode(String text) {
    try {
      return text.getBytes(<caret>"ISO-8859-1");
    } catch (UnsupportedEncodingException e) {
      throw new RuntimeException(e);
    }
  }
}
```
keys: Alt+Enter, choose "Replace with 'StandardCharsets.ISO_8859_1'"
```after
import java.nio.charset.StandardCharsets;
class C {
  byte[] encode(String text) {
    return text.getBytes(StandardCharsets.ISO_8859_1);
  }
}
```

### JAVA-ICA-244 case 2: Unsupported charset name
settings: default
language: java
```before
class C {
  byte[] f() throws java.io.UnsupportedEncodingException {
    return "str".getBytes(<caret>"Cp1252");
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-245 case 1: Null check with default
settings: default
language: java
```before
class C {
  String f(String nick) { return <caret>nick == null ? "n/a" : nick.toUpperCase(); }
}
```
keys: Alt+Enter, choose "Replace with Optional.ofNullable() chain"
```after
import java.util.Optional;

class C {
  String f(String nick) { return Optional.ofNullable(nick).map(String::toUpperCase).orElse("n/a"); }
}
```

### JAVA-ICA-245 case 2: Ordinary conditional
settings: default
language: java
```before
class C {
  String f(boolean b, String route) { return <caret>b ? "" : route.trim(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-246 case 1: Octal escape followed by a digit
settings: default
status: unverified (highlight is the escape part)
language: java
```before
class C {
  String s = "<caret>\1234";
}
```
keys: inspect
```after
problem: warning `\123` "Octal escape sequence \123 immediately followed by a digit"
```

### JAVA-ICA-247 case 1: 2 + 2
settings: default
language: java
```before
class C {
  int f() { return <caret>2 + 2; }
}
```
keys: Alt+Enter, choose "Compute constant value of '2 + 2'"
```after
class C {
  int f() { return 4; }
}
```

### JAVA-ICA-247 case 2: Plain literal
settings: default
language: java
```before
class C {
  int f() { return <caret>4; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-248 case 1: Constant on the right
settings: default
language: java
```before
class C {
  boolean f(int x) { return x < <caret>1; }
}
```
keys: Alt+Enter, choose "Flip comparison"
```after
class C {
  boolean f(int x) { return 1 > x; }
}
```

### JAVA-ICA-249 case 1: if without braces
settings: default
language: java
```before
class C {
  void f(int x) {
    <caret>if (x > 0) System.out.println(x);
  }
}
```
keys: Alt+Enter, choose "Add braces to 'if' statement"
```after
class C {
  void f(int x) {
    if (x > 0) {
      System.out.println(x);
    }
  }
}
```

### JAVA-ICA-249 case 2: if with braces
settings: default
language: java
```before
class C {
  void f(int x) {
    <caret>if (x > 0) { System.out.println(x); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-250 case 1: C-style parameter and return type
settings: default
language: java
```before
public class C {
  public String process(String <caret>value[])[] { return null; }
}
```
keys: Alt+Enter, choose "Replace with Java-style array declaration"
```after
public class C {
  public String[][] process(String[] value) { return null; }
}
```

### JAVA-ICA-250 case 2: Java-style declaration
settings: default
language: java
```before
public class C {
  public String[] process(String<caret>[] value) { return value; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-251 case 1: Diamond
settings: default
language: java
```before
import java.util.*;
class C {
  List<String> basket = new ArrayList<<caret>>();
}
```
keys: Alt+Enter, choose "Replace '<>' with explicit type arguments"
```after
import java.util.*;
class C {
  List<String> basket = new ArrayList<String>();
}
```

### JAVA-ICA-251 case 2: Explicit type arguments
settings: default
language: java
```before
import java.util.*;
class C {
  List<String> shelf = new ArrayList<<caret>String>();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-252 case 1: equals on enum
settings: default
language: java
```before
enum Comet { FOO, BAR }
class C {
  boolean measure(Comet value) { return value.<caret>equals(Comet.FOO); }
}
```
keys: Alt+Enter, choose "Replace 'equals()' with '=='"
```after
enum Comet { FOO, BAR }
class C {
  boolean measure(Comet value) { return value == Comet.FOO; }
}
```

### JAVA-ICA-253 case 1: \s in a regular string
settings: default
language: java
```before
class C {
  boolean f(String street) { return street.matches("<caret>\s+"); }
}
```
keys: Alt+Enter, choose "Replace '\s' sequences with spaces"
```after
class C {
  boolean f(String street) { return street.matches(" +"); }
}
```

### JAVA-ICA-253 case 2: Escaped backslash-s
settings: default
language: java
```before
class C {
  boolean f(String title) { return title.matches("<caret>\\s+"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-254 case 1: Sealed class with a subclass in the file
settings: language level=17
language: java
```before
sealed class <caret>Vehicle {}
final class Bike extends Vehicle {}
final class Car extends Vehicle {}
```
keys: Alt+Enter, choose "Add missing subclasses to the permits clause"
```after
sealed class Vehicle permits Bike, Car {}
final class Bike extends Vehicle {}
final class Car extends Vehicle {}
```

### JAVA-ICA-254 case 2: Non-sealed class
settings: language level=17
language: java
```before
class <caret>A {}
class B extends A {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-255 case 1: collect then sort then toArray
settings: default
status: unverified (name text of the fused steps)
language: java
```before
import java.util.*;
import java.util.stream.*;
class C {
  String[] f(Stream<String> shelf) {
    List<String> roster = shelf.<caret>collect(Collectors.toList());
    roster.sort(null);
    return roster.toArray(new String[roster.size()]);
  }
}
```
keys: Alt+Enter, choose "Fuse roster.sort(), roster.toArray() into the Stream API chain"
```after
import java.util.*;
import java.util.stream.*;
class C {
  String[] f(Stream<String> shelf) {
    return shelf.sorted().toArray(String[]::new);
  }
}
```

### JAVA-ICA-255 case 2: Unrelated statement after collect
settings: default
language: java
```before
import java.util.*;
import java.util.stream.*;
class C {
  List<String> f(Stream<String> crew) {
    List<String> basket = crew.<caret>collect(Collectors.toList());
    System.out.println(basket);
    return basket;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-256 case 1: Constructor without super
settings: default
language: java
```before
class Sculptor { <caret>Sculptor() {} }
```
keys: Alt+Enter, choose "Make call to 'super()' explicit"
```after
class Sculptor { Sculptor() { super(); } }
```

### JAVA-ICA-257 case 1: Adjacent declaration and assignment
settings: default
language: java
```before
class C {
  void f() {
    int x;
    <caret>x = 1;
    System.out.println(x);
  }
}
```
keys: Alt+Enter, choose "Join declaration and assignment"
```after
class C {
  void f() {
    int x = 1;
    System.out.println(x);
  }
}
```

### JAVA-ICA-257 case 2: Variable read before the assignment
settings: default
language: java
```before
class C {
  void f(boolean b) {
    int x;
    if (b) { x = 0; }
    <caret>x = 1;
    System.out.println(x);
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-258 case 1: Expression lambda
settings: default
language: java
```before
import java.util.function.Function;
class C {
  Function<Integer, Integer> f = n <caret>-> n + 1;
}
```
keys: Alt+Enter, choose "Expand lambda body to {...}"
```after
import java.util.function.Function;
class C {
  Function<Integer, Integer> f = n -> {
    return n + 1;
  };
}
```

### JAVA-ICA-258 case 2: Block lambda
settings: default
language: java
```before
import java.util.function.Function;
class C {
  Function<Integer, Integer> f = n <caret>-> { return n + 1; };
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-259 case 1: Consumer lambda
settings: default
language: java
```before
import java.util.function.Supplier;
class C {
  Supplier<String> sup = () <caret>-> "ok";
}
```
keys: Alt+Enter, choose "Replace lambda with anonymous class"
```after
import java.util.function.Supplier;
class C {
  Supplier<String> sup = new Supplier<String>() {
    @Override
    public String get() {
      return "ok";
    }
  };
}
```

### JAVA-ICA-259 case 2: Not a lambda
settings: default
language: java
```before
class C {
  Runnable r = <caret>new Runnable() { public void run() {} };
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-260 case 1: Implicit parameter type
settings: default
language: java
```before
import java.util.function.Predicate;
class C {
  Predicate<String> blank = <caret>t -> t.isEmpty();
}
```
keys: Alt+Enter, choose "Expand parameter types to (String t)"
```after
import java.util.function.Predicate;
class C {
  Predicate<String> blank = (String t) -> t.isEmpty();
}
```

### JAVA-ICA-260 case 2: Explicit parameter type
settings: default
language: java
```before
import java.util.function.Predicate;
class C {
  Predicate<String> blank = (<caret>String t) -> t.isEmpty();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-261 case 1: indexOf >= 0
settings: default
language: java
```before
import java.util.List;
class C {
  boolean hasStow(List<String> pile) { return <caret>pile.indexOf("") >= 0; }
}
```
keys: Alt+Enter, choose "Replace 'indexOf()' with 'contains()'"
```after
import java.util.List;
class C {
  boolean hasStow(List<String> pile) { return pile.contains(""); }
}
```

### JAVA-ICA-261 case 2: indexOf compared with 5
settings: default
language: java
```before
import java.util.List;
class C {
  boolean f(List<String> fleet) { return <caret>fleet.indexOf("") >= 5; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-262 case 1: Literal as argument
settings: default
language: java
```before
class C {
  boolean isRed(String code) { return code.equals(<caret>"RED"); }
}
```
keys: Alt+Enter, choose "Flip 'equals()'"
```after
class C {
  boolean isRed(String code) { return "RED".equals(code); }
}
```

### JAVA-ICA-263 case 1: Local variable never reassigned
settings: default
language: java
```before
import java.util.*;
class C {
  List<Integer> f() {
    ArrayList<Integer> <caret>entries = new ArrayList<>();
    return entries;
  }
}
```
keys: Alt+Enter, choose "Make final"
```after
import java.util.*;
class C {
  List<Integer> f() {
    final ArrayList<Integer> entries = new ArrayList<>();
    return entries;
  }
}
```

### JAVA-ICA-264 case 1: System.out::println
settings: default
language: java
```before
import java.util.List;
class C {
  void f(List<String> l) { l.forEach(System.out<caret>::println); }
}
```
keys: Alt+Enter, choose "Replace method reference with lambda"
```after
import java.util.List;
class C {
  void f(List<String> l) { l.forEach(s -> System.out.println(s)); }
}
```

### JAVA-ICA-264 case 2: Lambda already
settings: default
language: java
```before
import java.util.List;
class C {
  void f(List<String> l) { l.forEach(s <caret>-> System.out.println(s)); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-265 case 1: native public final
settings: default
language: java
```before
class Courier {
  <caret>native static public int count();
}
```
keys: Alt+Enter, choose "Sort modifiers"
```after
class Courier {
  public static native int count();
}
```

### JAVA-ICA-265 case 2: Sorted modifiers
settings: default
language: java
```before
class Courier {
  <caret>public final native void ferry();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-266 case 1: Static initializer assignment
settings: default
language: java
```before
class Limits {
  static final String label;
  static { <caret>label = "max"; }
}
```
keys: Alt+Enter, choose "Move assignment to field declaration"
```after
class Limits {
  static final String label = "max";
}
```

### JAVA-ICA-266 case 2: Field assigned twice
settings: default
language: java
```before
class Recipe {
  static int v;
  static { v = 1; <caret>v = 2; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-267 case 1: Multi-catch
settings: default
language: java
```before
class C {
  void f() {
    try { g(); }
    catch (IllegalStateException|IllegalArgumentException<caret> ex) { System.err.println(ex); }
  }
  void g() {}
}
```
keys: Alt+Enter, choose "Split multi-catch into separate 'catch' blocks"
```after
class C {
  void f() {
    try { g(); }
    catch (IllegalStateException ex) { System.err.println(ex); }
    catch (IllegalArgumentException ex) { System.err.println(ex); }
  }
  void g() {}
}
```

### JAVA-ICA-267 case 2: Single catch type
settings: default
language: java
```before
class C {
  void f() {
    try { g(); }
    catch (NullPointerException<caret> e) { e.printStackTrace(); }
  }
  void g() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-268 case 1: Two variables
settings: default
language: java
```before
class C {
  void f() {
    int <caret>lo = 0, hi = 9;
  }
}
```
keys: Alt+Enter, choose "Split into separate declarations"
```after
class C {
  void f() {
    int lo = 0;
    int hi = 9;
  }
}
```

### JAVA-ICA-268 case 2: for loop declaration
settings: default
language: java
```before
class C {
  void f(java.util.List<String> rows) {
    for (int <caret>i = 0, max = rows.size(); i < max; i++) {}
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-269 case 1: count(name())
settings: default
language: java
```before
class C {
  String name() { return "n"; }
  int count(String s) { return s.length(); }
  void run() { int c = count(<caret>name()); }
}
```
keys: Alt+Enter, choose "Introduce variable"
```after
class C {
  String name() { return "n"; }
  int count(String s) { return s.length(); }
  void run() {
    String name = name();
    int c = count(name);
  }
}
```

### JAVA-ICA-270 case 1: Optional of a list
settings: default
language: java
```before
import java.util.*;
class C {
  <caret>Optional<Set<String>> findTags() { return Optional.empty(); }
}
```
keys: inspect
```after
problem: warning `Optional<Set<String>>` "'Optional' contains collection Optional<Set<String>>"
```

### JAVA-ICA-271 case 1: ofNullable chain
settings: default
status: unverified (variable naming in the expansion)
language: java
```before
import java.util.*;
class C {
  String f(String name) {
    return Optional.ofNullable(name)
      .map(String::trim)
      .<caret>orElse("x");
  }
}
```
keys: Alt+Enter, choose "Replace Optional chain with if statements"
```after
import java.util.*;
class C {
  String f(String name) {
    if (name != null) {
      String trim = name.trim();
      if (trim != null) return trim;
    }
    return "x";
  }
}
```

### JAVA-ICA-271 case 2: Plain call
settings: default
language: java
```before
import java.util.*;
class C {
  String f(Optional<String> o) { return o.<caret>toString(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-272 case 1: instanceof pattern
settings: language level=17
status: unverified (exact fix text)
language: java
```before
class C {
  void f(Object item) {
    if (item <caret>instanceof Integer num && num > 3) { System.out.println(num); }
  }
}
```
keys: Alt+Enter, choose "Replace 'item instanceof Integer num' with casts"
```after
class C {
  void f(Object item) {
    if (item instanceof Integer && ((Integer) item) > 3) { Integer num = (Integer) item; System.out.println(num); }
  }
}
```

### JAVA-ICA-272 case 2: Plain instanceof
settings: default
language: java
```before
class C {
  void f(Object seller) {
    if (seller <caret>instanceof String) {}
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-273 case 1: Reassigned local variable
settings: default
language: java
```before
class C {
  void f(int w, int h) {
    int <caret>total = w + h;
    System.out.println("sum: " + total);
    total = w * h;
    System.out.println("product: " + total);
  }
}
```
keys: inspect
```after
problem: text attributes `total` ""
```

### JAVA-ICA-273 case 2: Single assignment
settings: default
language: java
```before
class C {
  void f(int qty, int budget) {
    int <caret>value = 2 * (qty + budget);
    System.out.println(value);
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-274 case 1: Simple record
settings: language level=17
status: unverified (exact generated members)
language: java
```before
record <caret>Tag(String label, int rank) {}
```
keys: Alt+Enter, choose "Convert record to class"
```after
final class Tag {
  private final String label;
  private final int rank;
  Tag(String label, int rank) { this.label = label; this.rank = rank; }
  String label() { return label; }
  int rank() { return rank; }
  public boolean equals(Object o) { if (this == o) return true; if (o == null || getClass() != o.getClass()) return false; Tag tag = (Tag) o; return rank == tag.rank && java.util.Objects.equals(label, tag.label); }
  public int hashCode() { return java.util.Objects.hash(label, rank); }
  public String toString() { return "Tag[label=" + label + ", rank=" + rank + "]"; }
}
```

### JAVA-ICA-274 case 2: Plain class
settings: default
language: java
```before
class <caret>Booking {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-275 case 1: int qty = 0
settings: default
language: java
```before
class Citadel {
  int qty = <caret>0;
}
```
keys: Alt+Enter, choose "Remove initializer"
```after
class Citadel {
  int qty;
}
```

### JAVA-ICA-276 case 1: yield in block
settings: language level=14
language: java
```before
class C {
  String f(char code) {
    return switch (code) {
      case 'a' -> <caret>{ yield "alpha"; }
      default -> "other";
    };
  }
}
```
keys: Alt+Enter, choose "Unwrap code block of labeled rule"
```after
class C {
  String f(char code) {
    return switch (code) {
      case 'a' -> "alpha";
      default -> "other";
    };
  }
}
```

### JAVA-ICA-276 case 2: Block with several statements
settings: language level=14
language: java
```before
class C {
  String f(char code) {
    return switch (code) {
      case 'a' -> <caret>{ String s = "alpha"; yield s; }
      default -> "other";
    };
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-277 case 1: Loop that sets a result
settings: default
language: java
```before
class C {
  int indexOf(String[] names, String key) {
    int pos = -1;
    for (int k = 0; k < names.length; k++) {
      if (names[k].equals(key)) { pos = k; break; }
    }
    return <caret>pos;
  }
}
```
keys: Alt+Enter, choose "Move 'return' closer to computation of the value of 'pos'"
```after
class C {
  int indexOf(String[] names, String key) {
    for (int k = 0; k < names.length; k++) {
      if (names[k].equals(key)) { return k; }
    }
    return -1;
  }
}
```

### JAVA-ICA-277 case 2: Value computed right before
settings: default
language: java
```before
class C {
  int f(int a) {
    int n = a + 1;
    return <caret>n;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-278 case 1: Builder style return
settings: default
language: java
```before
class Invoice {
  Invoice append(String s) { return <caret>this; }
}
```
keys: inspect
```after
problem: warning `this` "Return of this"
```

### JAVA-ICA-279 case 1: braces and whitespace
settings: default
language: java
```before
@interface Tag { String[] value(); }
class C {
  @ <caret>Tag({"red"}) public String label;
}
```
keys: Alt+Enter, choose "Simplify annotation"
```after
@interface Tag { String[] value(); }
class C {
  @Tag("red") public String label;
}
```

### JAVA-ICA-279 case 2: Already simple
settings: default
language: java
```before
@interface Sensor { String[] value(); }
class C {
  @<caret>Sensor("foo") public String name;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-280 case 1: null check then equals
settings: default
language: java
```before
class C {
  boolean f(String s) { return s != null && s.<caret>equals("literal"); }
}
```
keys: Alt+Enter, choose "Flip '.equals()' and remove unnecessary 'null' check"
```after
class C {
  boolean f(String s) { return "literal".equals(s); }
}
```

### JAVA-ICA-281 case 1: Shorthand annotation
settings: default
language: java
```before
class C {
  @SuppressWarnings(<caret>"foo") void f() {}
}
```
keys: Alt+Enter, choose "Add 'value='"
```after
class C {
  @SuppressWarnings(value = "foo") void f() {}
}
```

### JAVA-ICA-281 case 2: Already named
settings: default
language: java
```before
class C {
  @SuppressWarnings(<caret>value = "foo") void f() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-282 case 1: if with one statement
settings: default
language: java
```before
class C {
  void f(int level) {
    if (level < 3) <caret>{
      System.out.println("low");
    }
  }
}
```
keys: Alt+Enter, choose "Remove braces from 'if' statement"
```after
class C {
  void f(int level) {
    if (level < 3) System.out.println("low");
  }
}
```

### JAVA-ICA-282 case 2: if with two statements
settings: default
language: java
```before
class C {
  void f(int x) {
    if (x > 0) <caret>{
      System.out.println(x);
      System.out.println(x);
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-283 case 1: filter-map-collect chain
settings: default
status: unverified (generated variable names)
language: java
```before
import java.util.*;
import java.util.stream.*;
class C {
  String tags(List<String> cargo) {
    return cargo.stream()
      .filter(t -> t.startsWith("#"))
      .map(String::strip)
      .<caret>collect(Collectors.joining("; "));
  }
}
```
keys: Alt+Enter, choose "Replace Stream API chain with loop"
```after
import java.util.*;
import java.util.stream.*;
class C {
  String tags(List<String> cargo) {
    StringJoiner joiner = new StringJoiner("; ");
    for (String t : cargo) {
      if (t.startsWith("#")) {
        String strip = t.strip();
        joiner.add(strip);
      }
    }
    return joiner.toString();
  }
}
```

### JAVA-ICA-283 case 2: Unsupported call
settings: default
language: java
```before
import java.util.*;
class C {
  int f(List<String> roster) { return roster.<caret>size(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-284 case 1: Expression rule
settings: language level=14
language: java
```before
class C {
  String f(int days) {
    return switch (days) {
      case 7 -> <caret>"week";
      default -> "";
    };
  }
}
```
keys: Alt+Enter, choose "Wrap labeled rule's result expression with code block"
```after
class C {
  String f(int days) {
    return switch (days) {
      case 7 -> {
        yield "week";
      }
      default -> "";
    };
  }
}
```

### JAVA-ICA-284 case 2: Rule already has a block
settings: language level=14
language: java
```before
class C {
  String f(int days) {
    return switch (days) {
      case 7 -> <caret>{ yield "week"; }
      default -> "";
    };
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-285 case 1: Trailing spaces removed
settings: language level=15
language: java
```before
class C {
  String s = """
    one
    two   <caret>
    three""";
}
```
keys: Alt+Enter, choose "Remove trailing whitespace characters"
```after
class C {
  String s = """
    one
    two
    three""";
}
```

### JAVA-ICA-285 case 2: Trailing spaces escaped
settings: language level=15
language: java
```before
class C {
  String s = """
    one
    two   <caret>
    three""";
}
```
keys: Alt+Enter, choose "Escape trailing whitespace characters"
```after
class C {
  String s = """
    one
    two  \s
    three""";
}
```

### JAVA-ICA-285 case 3: No trailing whitespace
settings: language level=15
language: java
```before
class C {
  String s = """
    one<caret>
    two""";
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-286 case 1: Two resources
settings: default
language: java
```before
import java.io.*;
class C {
  void f() throws IOException {
    <caret>try (FileReader src = new FileReader("a.txt"); FileWriter dst = new FileWriter("b.txt")) {
      dst.write(src.read());
    }
  }
}
```
keys: Alt+Enter, choose "Split 'try' statement with multiple resources"
```after
import java.io.*;
class C {
  void f() throws IOException {
    try (FileReader src = new FileReader("a.txt")) {
      try (FileWriter dst = new FileWriter("b.txt")) {
        dst.write(src.read());
      }
    }
  }
}
```

### JAVA-ICA-286 case 2: One resource
settings: default
language: java
```before
import java.io.*;
class C {
  void f() throws IOException {
    <caret>try (FileReader src = new FileReader("a.txt")) {
      src.read();
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-287 case 1: Type parameter
settings: default
language: java
```before
class Badger<<caret>T extends Object> {}
```
keys: Alt+Enter, choose "Remove redundant 'extends Object'"
```after
class Badger<T> {}
```

### JAVA-ICA-287 case 2: Annotated Object
settings: default
language: java
```before
@interface Bison { }
class Garden<<caret>T extends @Bison Object> {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-288 case 1: Mixed precedence
settings: default
language: java
```before
class C {
  int n = <caret>2 + 6 * 5 - 4;
}
```
keys: Alt+Enter, choose "Add clarifying parentheses"
```after
class C {
  int n = 2 + (6 * 5) - 4;
}
```

### JAVA-ICA-288 case 2: Same operators
settings: default
language: java
```before
class C {
  int n = <caret>3 + 9 + 1;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-289 case 1: Tree.Leaf inside Tree
settings: default
language: java
```before
class Tree {
  <caret>Tree.Leaf leaf;
  class Leaf {}
}
```
keys: Alt+Enter, choose "Remove qualifier"
```after
class Tree {
  Leaf leaf;
  class Leaf {}
}
```

### JAVA-ICA-289 case 2: Needed qualifier
settings: default
language: java
```before
class X { class Y {} }
class Z {
  <caret>X.Y title;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-290 case 1: Qualified static import use
settings: default
language: java
```before
--- foo/Test.java
package foo;
public class Test { public static final int QUOTA = 1; }
--- Falcon.java
import static foo.Test.QUOTA;
class Falcon {
  void sort() { System.out.println(<caret>foo.Test.QUOTA); }
}
```
keys: Alt+Enter, choose "Remove unnecessary qualifier"
```after
import static foo.Test.QUOTA;
class Falcon {
  void sort() { System.out.println(QUOTA); }
}
```

### JAVA-ICA-291 case 1: Receipt.tune() inside Receipt
settings: default
language: java
```before
class Receipt {
  static void tune() {}
  void refill() { <caret>Receipt.tune(); }
}
```
keys: Alt+Enter, choose "Remove unnecessary qualifier"
```after
class Receipt {
  static void tune() {}
  void refill() { tune(); }
}
```

### JAVA-ICA-292 case 1: Standalone block
settings: default
language: java
```before
class C {
  void polish() {
    <caret>{
      int margin = call();
      assemble(margin);
    }
  }
  int call() { return 1; }
  void assemble(int r) {}
}
```
keys: Alt+Enter, choose "Unwrap block"
```after
class C {
  void polish() {
    int margin = call();
    assemble(margin);
  }
  int call() { return 1; }
  void assemble(int r) {}
}
```

### JAVA-ICA-292 case 2: Block as a loop body
settings: default
language: java
```before
class C {
  void forecast() {
    while (true) <caret>{ break; }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-293 case 1: Integer.toString in concatenation
settings: default
language: java
```before
class C {
  void f(int height) {
    System.out.println("Number: " + <caret>Integer.toString(height));
  }
}
```
keys: Alt+Enter, choose "Replace with 'height'"
```after
class C {
  void f(int height) {
    System.out.println("Number: " + height);
  }
}
```

### JAVA-ICA-293 case 2: Conversion needed
settings: default
language: java
```before
class C {
  String f(int tick) { return <caret>Integer.toString(tick); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-294 case 1: Variable with explicit new
settings: default
language: java
```before
class C {
  int[] fleet = <caret>new int[] {42};
}
```
keys: Alt+Enter, choose "Remove 'new int[]'"
```after
class C {
  int[] fleet = {42};
}
```

### JAVA-ICA-295 case 1: Redundant public constructor
settings: default
language: java
```before
public class Harvest {
  public <caret>Harvest() {}
}
```
keys: Alt+Enter, choose "Remove redundant constructor"
```after
public class Harvest {
}
```

### JAVA-ICA-296 case 1: final parameter
settings: default
language: java
```before
class Invoice {
  void gather(<caret>final Object o) { new Object().equals(o); }
}
```
keys: Alt+Enter, choose "Remove 'final' modifier"
```after
class Invoice {
  void gather(Object o) { new Object().equals(o); }
}
```

### JAVA-ICA-297 case 1: java.util.List
settings: default
language: java
```before
class Cache {
  <caret>java.util.Map<String, Integer> hits;
}
```
keys: Alt+Enter, choose "Replace qualified name with import"
```after
import java.util.Map;

class Cache {
  Map<String, Integer> hits;
}
```

### JAVA-ICA-297 case 2: Short name
settings: default
language: java
```before
import java.util.Map;
class Cache {
  <caret>Map<String, Integer> hits;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-298 case 1: 2 + (6 * 5)
settings: default
language: java
```before
class C {
  int n = 2 + <caret>(6 * 5);
}
```
keys: Alt+Enter, choose "Remove unnecessary parentheses"
```after
class C {
  int n = 2 + 6 * 5;
}
```

### JAVA-ICA-298 case 2: Needed parentheses
settings: default
language: java
```before
class C {
  int n = <caret>(3 + 9) * 8;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-299 case 1: Truck.super.start()
settings: default
language: java
```before
class Engine { void start() {} }
class Truck extends Engine {
  void go() { <caret>Truck.super.start(); }
}
```
keys: Alt+Enter, choose "Remove unnecessary qualifier"
```after
class Engine { void start() {} }
class Truck extends Engine {
  void go() { super.start(); }
}
```

### JAVA-ICA-300 case 1: super() call
settings: default
language: java
```before
class Gallery {
  Gallery() { <caret>super(); }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'super()'"
```after
class Gallery {
  Gallery() { }
}
```

### JAVA-ICA-301 case 1: super.start()
settings: default
language: java
```before
class Engine { void start() {} }
class Car extends Engine {
  void go() { <caret>super.start(); }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'super' qualifier"
```after
class Engine { void start() {} }
class Car extends Engine {
  void go() { start(); }
}
```

### JAVA-ICA-302 case 1: this.x
settings: default
language: java
```before
class Canal {
  int count;
  void reset() { <caret>this.count = 5; }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'this' qualifier"
```after
class Canal {
  int count;
  void reset() { count = 5; }
}
```

### JAVA-ICA-303 case 1: miles += 3
settings: default
language: java
```before
class Voyage {
  int miles;
  void add() { <caret>miles += 3; }
}
```
keys: Alt+Enter, choose "Add 'this' qualifier"
```after
class Voyage {
  int miles;
  void add() { this.miles += 3; }
}
```

### JAVA-ICA-304 case 1: Imported inner class
settings: default
language: java
```before
--- foo/Sensor.java
package foo;
public class Sensor { public static class Anchor {} }
--- Pebble.java
import foo.Foo.Anchor;
class Pebble {
  void f(<caret>Anchor weight) {}
}
```
keys: Alt+Enter, choose "Qualify with outer class"
```after
import foo.Foo;
import foo.Foo.Anchor;
class Pebble {
  void f(Sensor.Anchor weight) {}
}
```

### JAVA-ICA-305 case 1: fetch() call
settings: default
language: java
```before
class Cabin {
  void fetch() {}
  void archive() { <caret>fetch(); }
}
```
keys: Alt+Enter, choose "Add 'this' qualifier"
```after
class Cabin {
  void fetch() {}
  void archive() { this.fetch(); }
}
```

### JAVA-ICA-306 case 1: Unqualified static call
settings: default
language: java
```before
class Tunnel {
  static void bake() {}
  void tune() { <caret>bake(); }
}
```
keys: Alt+Enter, choose "Qualify static method call"
```after
class Tunnel {
  static void bake() {}
  void tune() { Tunnel.bake(); }
}
```

### JAVA-ICA-307 case 1: Trailing comma in an annotation array at language level 6
settings: language level=6
language: java
```before
class C {
  @SuppressWarnings({"a", "b"<caret>,})
  void f() {}
}
```
keys: Alt+Enter, choose "Remove trailing comma"
```after
class C {
  @SuppressWarnings({"a", "b"})
  void f() {}
}
```

### JAVA-ICA-307 case 2: Trailing comma at language level 11
settings: language level=11
language: java
```before
class C {
  @SuppressWarnings({"a", "b"<caret>,})
  void f() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-308 case 1: Preview API at a preview language level
settings: language level=21 (Preview), JDK=21
status: unverified (which JDK 21 API is a preview API)
language: java
```before
import java.lang.StringTemplate;
class C {
  <caret>StringTemplate t;
}
```
keys: inspect
```after
problem: warning `StringTemplate` "java.lang.StringTemplate is a preview API and may be removed in a future release"
```

### JAVA-ICA-308 case 2: Same API at a non-preview language level
settings: language level=21, JDK=21
language: java
```before
import java.lang.StringTemplate;
class C {
  <caret>StringTemplate t;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-309 case 1: Synchronized on Double
settings: default
language: java
```before
class C {
  void f() {
    java.time.Duration span = java.time.Duration.ZERO;
    synchronized (<caret>span) { }
  }
}
```
keys: inspect
```after
problem: warning `span` "Synchronization on instance of value-based class"
```

### JAVA-ICA-309 case 2: Synchronized on an object
settings: default
language: java
```before
class C {
  final Object lock = new Object();
  void f() {
    synchronized (<caret>lock) { }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-310 case 1: Unchecked assignment
settings: default
language: java
```before
import java.util.*;
class C {
  List<Integer> f() {
    List crew = Arrays.asList("string", "string");
    List<Integer> entries = <caret>crew;
    return entries;
  }
}
```
keys: inspect
```after
problem: warning `crew` "Unchecked assignment: 'java.util.List' to 'java.util.List<java.lang.Integer>'"
```

### JAVA-ICA-310 case 2: Parameterized assignment
settings: default
language: java
```before
import java.util.*;
class C {
  List<String> f() {
    List<String> l = <caret>new ArrayList<>();
    return l;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-310 case 3: Option ignores unchecked assignment
settings: Ignore unchecked assignment=on
language: java
```before
import java.util.*;
class C {
  List<Integer> f() {
    List cells = Arrays.asList("string");
    List<Integer> roster = <caret>cells;
    return roster;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-311 case 1: Field read without the lock
settings: library=jcip
language: java
```before
import net.jcip.annotations.GuardedBy;
class C {
  private final Object lock = new Object();
  @GuardedBy("lock") private int count;
  int get() { return <caret>count; }
}
```
keys: inspect
```after
problem: warning `count` "Access to field count outside of declared guards"
```

### JAVA-ICA-312 case 1: Instance method with a static guard
settings: library=jcip
language: java
```before
import net.jcip.annotations.GuardedBy;
class C {
  private static final Object LOCK = new Object();
  @GuardedBy(<caret>"LOCK") public void assemble() {}
}
```
keys: inspect
```after
problem: warning `"LOCK"` "Instance member guarded by static "LOCK""
```

### JAVA-ICA-313 case 1: Mutable field in an immutable class
settings: library=jcip
language: java
```before
import net.jcip.annotations.Immutable;
@Immutable
class C {
  String <caret>city = "Paris";
}
```
keys: inspect
```after
problem: warning `city` "Non-final field city in @Immutable class"
```

### JAVA-ICA-314 case 1: Non-final lock field
settings: library=jcip
language: java
```before
import net.jcip.annotations.GuardedBy;
class C {
  private Object lock = new Object();
  @GuardedBy(<caret>"lock") public void unpack() {}
}
```
keys: inspect
```after
problem: warning `"lock"` "Non-final @GuardedBy field "lock""
```

### JAVA-ICA-315 case 1: Static method with an instance lock
settings: library=jcip
language: java
```before
import net.jcip.annotations.GuardedBy;
class C {
  private final Object lock = new Object();
  @GuardedBy(<caret>"lock") public static void weigh() {}
}
```
keys: inspect
```after
problem: warning `"lock"` "Static member guarded by instance "lock""
```

### JAVA-ICA-316 case 1: Missing lock field
settings: library=jcip
language: java
```before
import net.jcip.annotations.GuardedBy;
class C {
  @GuardedBy(<caret>"lock") public void audit() {}
}
```
keys: inspect
```after
problem: warning `"lock"` "Unknown @GuardedBy reference "lock""
```
