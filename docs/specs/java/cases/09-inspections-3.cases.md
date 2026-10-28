# Java inspections catalog, part 3: acceptance cases

Format: `docs/specs/editor/cases/README.md`. Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-3.md`. Keys `Alt+Enter, choose "<fix>"` mean: press Alt+Enter at the caret and pick the quick fix with that exact name from the popup. Keys `inspect` run the inspection over the file (the highlight is what the editor shows); `problem:` lines list the expected highlight. `after: no problem reported` means the inspection must not highlight anything in the before text.

### JAVA-ICA-236 case 1: All constants listed in order
settings: default
language: java
```before
enum States { NOT_RUN, IN_PROGRESS, FINISHED }
class C {
  void handle(States[] s) {}
  void f() {
    handle(<caret>new States[] {States.NOT_RUN, States.IN_PROGRESS, States.FINISHED});
  }
}
```
keys: Alt+Enter, choose "Replace array with States.values()"
```after
enum States { NOT_RUN, IN_PROGRESS, FINISHED }
class C {
  void handle(States[] s) {}
  void f() {
    handle(States.values());
  }
}
```

### JAVA-ICA-236 case 2: Constants in a different order
settings: default
language: java
```before
enum States { NOT_RUN, IN_PROGRESS, FINISHED }
class C {
  void handle(States[] s) {}
  void f() {
    handle(<caret>new States[] {States.FINISHED, States.NOT_RUN, States.IN_PROGRESS});
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
  void foo(java.util.List<Integer> myList) {
    assert myList.isEmpty() : <caret>false;
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
  void process(Consumer<<caret>Number> consumer) {}
}
```
keys: Alt+Enter, choose "Replace with '? super Number'"
```after
import java.util.function.Consumer;
class C {
  void process(Consumer<? super Number> consumer) {}
}
```

### JAVA-ICA-241 case 1: concat call
settings: default
language: java
```before
class C {
  String foo(String name) { return name.<caret>concat("foo"); }
}
```
keys: Alt+Enter, choose "Replace 'concat()' call with '+'"
```after
class C {
  String foo(String name) { return name + "foo"; }
}
```

### JAVA-ICA-242 case 1: Chained boolean equality
settings: default
language: java
```before
class C {
  boolean f(boolean a, boolean b, boolean c) { return <caret>a == b == c; }
}
```
keys: inspect
```after
problem: warning `a == b == c` "Chained equality comparison a == b == c"
```

### JAVA-ICA-243 case 1: f.getName().length()
settings: default
language: java
```before
class X {
  int foo(java.io.File f) { return f.getName().<caret>length(); }
}
```
keys: Alt+Enter, choose "Introduce variable"
```after
class X {
  int foo(java.io.File f) {
    final String name = f.getName();
    return name.length();
  }
}
```

### JAVA-ICA-244 case 1: getBytes with a charset name
settings: default
language: java
```before
import java.io.UnsupportedEncodingException;
class C {
  byte[] f() {
    try {
      return "str".getBytes(<caret>"UTF-8");
    } catch (UnsupportedEncodingException e) {
      throw new RuntimeException(e);
    }
  }
}
```
keys: Alt+Enter, choose "Replace with 'StandardCharsets.UTF_8'"
```after
import java.nio.charset.StandardCharsets;
class C {
  byte[] f() {
    return "str".getBytes(StandardCharsets.UTF_8);
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
  String f(String str) { return <caret>str == null ? "" : str.trim(); }
}
```
keys: Alt+Enter, choose "Replace with Optional.ofNullable() chain"
```after
import java.util.Optional;

class C {
  String f(String str) { return Optional.ofNullable(str).map(String::trim).orElse(""); }
}
```

### JAVA-ICA-245 case 2: Ordinary conditional
settings: default
language: java
```before
class C {
  String f(boolean b, String str) { return <caret>b ? "" : str.trim(); }
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
  List<String> list = new ArrayList<<caret>>();
}
```
keys: Alt+Enter, choose "Replace '<>' with explicit type arguments"
```after
import java.util.*;
class C {
  List<String> list = new ArrayList<String>();
}
```

### JAVA-ICA-251 case 2: Explicit type arguments
settings: default
language: java
```before
import java.util.*;
class C {
  List<String> list = new ArrayList<<caret>String>();
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
enum MyEnum { FOO, BAR }
class C {
  boolean foo(MyEnum value) { return value.<caret>equals(MyEnum.FOO); }
}
```
keys: Alt+Enter, choose "Replace 'equals()' with '=='"
```after
enum MyEnum { FOO, BAR }
class C {
  boolean foo(MyEnum value) { return value == MyEnum.FOO; }
}
```

### JAVA-ICA-253 case 1: \s in a regular string
settings: default
language: java
```before
class C {
  boolean f(String str) { return str.matches("<caret>\s+"); }
}
```
keys: Alt+Enter, choose "Replace '\s' sequences with spaces"
```after
class C {
  boolean f(String str) { return str.matches(" +"); }
}
```

### JAVA-ICA-253 case 2: Escaped backslash-s
settings: default
language: java
```before
class C {
  boolean f(String str) { return str.matches("<caret>\\s+"); }
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
sealed class <caret>A {}
final class B extends A {}
```
keys: Alt+Enter, choose "Add missing subclasses to the permits clause"
```after
sealed class A permits B {}
final class B extends A {}
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
  String[] f(Stream<String> stream) {
    List<String> list = stream.<caret>collect(Collectors.toList());
    list.sort(null);
    return list.toArray(new String[list.size()]);
  }
}
```
keys: Alt+Enter, choose "Fuse list.sort(), list.toArray() into the Stream API chain"
```after
import java.util.*;
import java.util.stream.*;
class C {
  String[] f(Stream<String> stream) {
    return stream.sorted().toArray(String[]::new);
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
  List<String> f(Stream<String> stream) {
    List<String> list = stream.<caret>collect(Collectors.toList());
    System.out.println(list);
    return list;
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
class Foo { <caret>Foo() {} }
```
keys: Alt+Enter, choose "Make call to 'super()' explicit"
```after
class Foo { Foo() { super(); } }
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
import java.util.function.Consumer;
class C {
  Consumer<String> c = s <caret>-> System.out.println(s);
}
```
keys: Alt+Enter, choose "Replace lambda with anonymous class"
```after
import java.util.function.Consumer;
class C {
  Consumer<String> c = new Consumer<String>() {
    @Override
    public void accept(String s) {
      System.out.println(s);
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
import java.util.function.Function;
class C {
  Function<String, Integer> length = <caret>a -> a.length();
}
```
keys: Alt+Enter, choose "Expand parameter types to (String a)"
```after
import java.util.function.Function;
class C {
  Function<String, Integer> length = (String a) -> a.length();
}
```

### JAVA-ICA-260 case 2: Explicit parameter type
settings: default
language: java
```before
import java.util.function.Function;
class C {
  Function<String, Integer> length = (<caret>String a) -> a.length();
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
  boolean hasEmptyString(List<String> list) { return <caret>list.indexOf("") >= 0; }
}
```
keys: Alt+Enter, choose "Replace 'indexOf()' with 'contains()'"
```after
import java.util.List;
class C {
  boolean hasEmptyString(List<String> list) { return list.contains(""); }
}
```

### JAVA-ICA-261 case 2: indexOf compared with 5
settings: default
language: java
```before
import java.util.List;
class C {
  boolean f(List<String> list) { return <caret>list.indexOf("") >= 5; }
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
  boolean isFoo(String value) { return value.equals(<caret>"foo"); }
}
```
keys: Alt+Enter, choose "Flip 'equals()'"
```after
class C {
  boolean isFoo(String value) { return "foo".equals(value); }
}
```

### JAVA-ICA-263 case 1: Local variable never reassigned
settings: default
language: java
```before
import java.util.*;
class C {
  List<Integer> f() {
    ArrayList<Integer> <caret>list = new ArrayList<>();
    return list;
  }
}
```
keys: Alt+Enter, choose "Make final"
```after
import java.util.*;
class C {
  List<Integer> f() {
    final ArrayList<Integer> list = new ArrayList<>();
    return list;
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
class Foo {
  <caret>native public final void foo();
}
```
keys: Alt+Enter, choose "Sort modifiers"
```after
class Foo {
  public final native void foo();
}
```

### JAVA-ICA-265 case 2: Sorted modifiers
settings: default
language: java
```before
class Foo {
  <caret>public final native void foo();
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
class MyClass {
  static final int intConstant;
  static { <caret>intConstant = 10; }
}
```
keys: Alt+Enter, choose "Move assignment to field declaration"
```after
class MyClass {
  static final int intConstant = 10;
}
```

### JAVA-ICA-266 case 2: Field assigned twice
settings: default
language: java
```before
class MyClass {
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
    catch (NullPointerException|IndexOutOfBoundsException<caret> e) { e.printStackTrace(); }
  }
  void g() {}
}
```
keys: Alt+Enter, choose "Split multi-catch into separate 'catch' blocks"
```after
class C {
  void f() {
    try { g(); }
    catch (NullPointerException e) { e.printStackTrace(); }
    catch (IndexOutOfBoundsException e) { e.printStackTrace(); }
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
    int <caret>x = 1, y = 2;
  }
}
```
keys: Alt+Enter, choose "Split into separate declarations"
```after
class C {
  void f() {
    int x = 1;
    int y = 2;
  }
}
```

### JAVA-ICA-268 case 2: for loop declaration
settings: default
language: java
```before
class C {
  void f(java.util.List<String> list) {
    for (int <caret>i = 0, max = list.size(); i < max; i++) {}
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-269 case 1: f(y())
settings: default
language: java
```before
class C {
  int y() { return 1; }
  int f(int x) { return 2 * x; }
  void foo() { int x = f(<caret>y()); }
}
```
keys: Alt+Enter, choose "Introduce variable"
```after
class C {
  int y() { return 1; }
  int f(int x) { return 2 * x; }
  void foo() {
    int y = y();
    int x = f(y);
  }
}
```

### JAVA-ICA-270 case 1: Optional of a list
settings: default
language: java
```before
import java.util.*;
class C {
  <caret>Optional<List<Integer>> foo() { return Optional.empty(); }
}
```
keys: inspect
```after
problem: warning `Optional<List<Integer>>` "'Optional' contains collection Optional<List<Integer>>"
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
  void f(Object object) {
    if (object <caret>instanceof String txt && txt.length() == 1) { System.out.println(txt); }
  }
}
```
keys: Alt+Enter, choose "Replace 'object instanceof String txt' with casts"
```after
class C {
  void f(Object object) {
    if (object instanceof String && ((String) object).length() == 1) { String txt = (String) object; System.out.println(txt); }
  }
}
```

### JAVA-ICA-272 case 2: Plain instanceof
settings: default
language: java
```before
class C {
  void f(Object object) {
    if (object <caret>instanceof String) {}
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
  void f(int height, int width) {
    int <caret>value = 2 * (height + width);
    System.out.println("perimeter: " + value);
    value = height * width;
    System.out.println("area: " + value);
  }
}
```
keys: inspect
```after
problem: text attributes `value` ""
```

### JAVA-ICA-273 case 2: Single assignment
settings: default
language: java
```before
class C {
  void f(int height, int width) {
    int <caret>value = 2 * (height + width);
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
record <caret>Point(int x, int y) {}
```
keys: Alt+Enter, choose "Convert record to class"
```after
final class Point {
  private final int x;
  private final int y;
  Point(int x, int y) { this.x = x; this.y = y; }
  int x() { return x; }
  int y() { return y; }
  public boolean equals(Object o) { if (this == o) return true; if (o == null || getClass() != o.getClass()) return false; Point point = (Point) o; return x == point.x && y == point.y; }
  public int hashCode() { return java.util.Objects.hash(x, y); }
  public String toString() { return "Point[x=" + x + ", y=" + y + "]"; }
}
```

### JAVA-ICA-274 case 2: Plain class
settings: default
language: java
```before
class <caret>Point {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-275 case 1: int foo = 0
settings: default
language: java
```before
class Foo {
  int foo = <caret>0;
}
```
keys: Alt+Enter, choose "Remove initializer"
```after
class Foo {
  int foo;
}
```

### JAVA-ICA-276 case 1: yield in block
settings: language level=14
language: java
```before
class C {
  String f(int n) {
    return switch (n) {
      case 1 -> <caret>{ yield Integer.toString(n); }
      default -> "default";
    };
  }
}
```
keys: Alt+Enter, choose "Unwrap code block of labeled rule"
```after
class C {
  String f(int n) {
    return switch (n) {
      case 1 -> Integer.toString(n);
      default -> "default";
    };
  }
}
```

### JAVA-ICA-276 case 2: Block with several statements
settings: language level=14
language: java
```before
class C {
  String f(int n) {
    return switch (n) {
      case 1 -> <caret>{ String s = "a"; yield s; }
      default -> "default";
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
  int find(int[] a, int b) {
    int n = -1;
    for (int i = 0; i < a.length; i++) {
      if (a[i] == b) { n = i; break; }
    }
    return <caret>n;
  }
}
```
keys: Alt+Enter, choose "Move 'return' closer to computation of the value of 'n'"
```after
class C {
  int find(int[] a, int b) {
    for (int i = 0; i < a.length; i++) {
      if (a[i] == b) { return i; }
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
class Builder {
  Builder append(String s) { return <caret>this; }
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
@interface Foo { String[] value(); }
class C {
  @ <caret>Foo({"foo"}) public String name;
}
```
keys: Alt+Enter, choose "Simplify annotation"
```after
@interface Foo { String[] value(); }
class C {
  @Foo("foo") public String name;
}
```

### JAVA-ICA-279 case 2: Already simple
settings: default
language: java
```before
@interface Foo { String[] value(); }
class C {
  @<caret>Foo("foo") public String name;
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
  void f(int x) {
    if (x > 0) <caret>{
      System.out.println("x is positive");
    }
  }
}
```
keys: Alt+Enter, choose "Remove braces from 'if' statement"
```after
class C {
  void f(int x) {
    if (x > 0) System.out.println("x is positive");
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
  String joinNonEmpty(List<String> list) {
    return list.stream()
      .filter(s -> !s.isEmpty())
      .map(String::trim)
      .<caret>collect(Collectors.joining(", "));
  }
}
```
keys: Alt+Enter, choose "Replace Stream API chain with loop"
```after
import java.util.*;
import java.util.stream.*;
class C {
  String joinNonEmpty(List<String> list) {
    StringJoiner joiner = new StringJoiner(", ");
    for (String s : list) {
      if (!s.isEmpty()) {
        String trim = s.trim();
        joiner.add(trim);
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
  int f(List<String> list) { return list.<caret>size(); }
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
  String f(int errorCode) {
    return switch (errorCode) {
      case 404 -> <caret>"Not found!";
      default -> "";
    };
  }
}
```
keys: Alt+Enter, choose "Wrap labeled rule's result expression with code block"
```after
class C {
  String f(int errorCode) {
    return switch (errorCode) {
      case 404 -> {
        yield "Not found!";
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
  String f(int errorCode) {
    return switch (errorCode) {
      case 404 -> <caret>{ yield "Not found!"; }
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
    <caret>try (FileInputStream in = new FileInputStream("in.txt"); FileOutputStream out = new FileOutputStream("out.txt")) {
      in.read();
    }
  }
}
```
keys: Alt+Enter, choose "Split 'try' statement with multiple resources"
```after
import java.io.*;
class C {
  void f() throws IOException {
    try (FileInputStream in = new FileInputStream("in.txt")) {
      try (FileOutputStream out = new FileOutputStream("out.txt")) {
        in.read();
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
    <caret>try (FileInputStream in = new FileInputStream("in.txt")) {
      in.read();
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
class ClassA<<caret>T extends Object> {}
```
keys: Alt+Enter, choose "Remove redundant 'extends Object'"
```after
class ClassA<T> {}
```

### JAVA-ICA-287 case 2: Annotated Object
settings: default
language: java
```before
@interface NotNull { }
class MyClass<<caret>T extends @NotNull Object> {}
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
  int n = <caret>3 + 9 * 8 + 1;
}
```
keys: Alt+Enter, choose "Add clarifying parentheses"
```after
class C {
  int n = 3 + (9 * 8) + 1;
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

### JAVA-ICA-289 case 1: X.Y inside X
settings: default
language: java
```before
class X {
  <caret>X.Y foo;
  class Y {}
}
```
keys: Alt+Enter, choose "Remove qualifier"
```after
class X {
  Y foo;
  class Y {}
}
```

### JAVA-ICA-289 case 2: Needed qualifier
settings: default
language: java
```before
class X { class Y {} }
class Z {
  <caret>X.Y foo;
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
public class Test { public static final int WIDTH = 1; }
--- Bar.java
import static foo.Test.WIDTH;
class Bar {
  void bar() { System.out.println(<caret>foo.Test.WIDTH); }
}
```
keys: Alt+Enter, choose "Remove unnecessary qualifier"
```after
import static foo.Test.WIDTH;
class Bar {
  void bar() { System.out.println(WIDTH); }
}
```

### JAVA-ICA-291 case 1: Foo.foo() inside Foo
settings: default
language: java
```before
class Foo {
  static void foo() {}
  void bar() { <caret>Foo.foo(); }
}
```
keys: Alt+Enter, choose "Remove unnecessary qualifier"
```after
class Foo {
  static void foo() {}
  void bar() { foo(); }
}
```

### JAVA-ICA-292 case 1: Standalone block
settings: default
language: java
```before
class C {
  void foo() {
    <caret>{
      int result = call();
      analyze(result);
    }
  }
  int call() { return 1; }
  void analyze(int r) {}
}
```
keys: Alt+Enter, choose "Unwrap block"
```after
class C {
  void foo() {
    int result = call();
    analyze(result);
  }
  int call() { return 1; }
  void analyze(int r) {}
}
```

### JAVA-ICA-292 case 2: Block as a loop body
settings: default
language: java
```before
class C {
  void foo() {
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
  void f(int count) {
    System.out.println("Number: " + <caret>Integer.toString(count));
  }
}
```
keys: Alt+Enter, choose "Replace with 'count'"
```after
class C {
  void f(int count) {
    System.out.println("Number: " + count);
  }
}
```

### JAVA-ICA-293 case 2: Conversion needed
settings: default
language: java
```before
class C {
  String f(int count) { return <caret>Integer.toString(count); }
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
  int[] foo = <caret>new int[] {42};
}
```
keys: Alt+Enter, choose "Remove 'new int[]'"
```after
class C {
  int[] foo = {42};
}
```

### JAVA-ICA-295 case 1: Redundant public constructor
settings: default
language: java
```before
public class Foo {
  public <caret>Foo() {}
}
```
keys: Alt+Enter, choose "Remove redundant constructor"
```after
public class Foo {
}
```

### JAVA-ICA-296 case 1: final parameter
settings: default
language: java
```before
class Foo {
  void bar(<caret>final Object o) { new Object().equals(o); }
}
```
keys: Alt+Enter, choose "Remove 'final' modifier"
```after
class Foo {
  void bar(Object o) { new Object().equals(o); }
}
```

### JAVA-ICA-297 case 1: java.util.List
settings: default
language: java
```before
class ListWrapper {
  <caret>java.util.List<Integer> l;
}
```
keys: Alt+Enter, choose "Replace qualified name with import"
```after
import java.util.List;

class ListWrapper {
  List<Integer> l;
}
```

### JAVA-ICA-297 case 2: Short name
settings: default
language: java
```before
import java.util.List;
class ListWrapper {
  <caret>List<Integer> l;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-298 case 1: 3 + (9 * 8)
settings: default
language: java
```before
class C {
  int n = 3 + <caret>(9 * 8);
}
```
keys: Alt+Enter, choose "Remove unnecessary parentheses"
```after
class C {
  int n = 3 + 9 * 8;
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

### JAVA-ICA-299 case 1: Bar.super.foo()
settings: default
language: java
```before
class Foo { void foo() {} }
class Bar extends Foo {
  void bar() { <caret>Bar.super.foo(); }
}
```
keys: Alt+Enter, choose "Remove unnecessary qualifier"
```after
class Foo { void foo() {} }
class Bar extends Foo {
  void bar() { super.foo(); }
}
```

### JAVA-ICA-300 case 1: super() call
settings: default
language: java
```before
class Foo {
  Foo() { <caret>super(); }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'super()'"
```after
class Foo {
  Foo() { }
}
```

### JAVA-ICA-301 case 1: super.foo()
settings: default
language: java
```before
class Foo { void foo() {} }
class Bar extends Foo {
  void bar() { <caret>super.foo(); }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'super' qualifier"
```after
class Foo { void foo() {} }
class Bar extends Foo {
  void bar() { foo(); }
}
```

### JAVA-ICA-302 case 1: this.x
settings: default
language: java
```before
class Foo {
  int x;
  void foo() { <caret>this.x = 2; }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'this' qualifier"
```after
class Foo {
  int x;
  void foo() { x = 2; }
}
```

### JAVA-ICA-303 case 1: foo += 1
settings: default
language: java
```before
class Foo {
  int foo;
  void bar() { <caret>foo += 1; }
}
```
keys: Alt+Enter, choose "Add 'this' qualifier"
```after
class Foo {
  int foo;
  void bar() { this.foo += 1; }
}
```

### JAVA-ICA-304 case 1: Imported inner class
settings: default
language: java
```before
--- foo/Foo.java
package foo;
public class Foo { public static class Bar {} }
--- Baz.java
import foo.Foo.Bar;
class Baz {
  void f(<caret>Bar bar) {}
}
```
keys: Alt+Enter, choose "Qualify with outer class"
```after
import foo.Foo;
import foo.Foo.Bar;
class Baz {
  void f(Foo.Bar bar) {}
}
```

### JAVA-ICA-305 case 1: bar() call
settings: default
language: java
```before
class Foo {
  void bar() {}
  void foo() { <caret>bar(); }
}
```
keys: Alt+Enter, choose "Add 'this' qualifier"
```after
class Foo {
  void bar() {}
  void foo() { this.bar(); }
}
```

### JAVA-ICA-306 case 1: Unqualified static call
settings: default
language: java
```before
class Foo {
  static void foo() {}
  void bar() { <caret>foo(); }
}
```
keys: Alt+Enter, choose "Qualify static method call"
```after
class Foo {
  static void foo() {}
  void bar() { Foo.foo(); }
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
    Double d = 20.0;
    synchronized (<caret>d) { }
  }
}
```
keys: inspect
```after
problem: warning `d` "Synchronization on instance of value-based class"
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
    List items = Arrays.asList("string", "string");
    List<Integer> numbers = <caret>items;
    return numbers;
  }
}
```
keys: inspect
```after
problem: warning `items` "Unchecked assignment: 'java.util.List' to 'java.util.List<java.lang.Integer>'"
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
    List items = Arrays.asList("string");
    List<Integer> numbers = <caret>items;
    return numbers;
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
  @GuardedBy(<caret>"LOCK") public void bar() {}
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
  String <caret>bar = "foo";
}
```
keys: inspect
```after
problem: warning `bar` "Non-final field bar in @Immutable class"
```

### JAVA-ICA-314 case 1: Non-final lock field
settings: library=jcip
language: java
```before
import net.jcip.annotations.GuardedBy;
class C {
  private Object lock = new Object();
  @GuardedBy(<caret>"lock") public void bar() {}
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
  @GuardedBy(<caret>"lock") public static void bar() {}
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
  @GuardedBy(<caret>"lock") public void bar() {}
}
```
keys: inspect
```after
problem: warning `"lock"` "Unknown @GuardedBy reference "lock""
```
