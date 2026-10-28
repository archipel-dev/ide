# Cases for 09-inspections-catalog-7 (JAVA-ICC)

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-7.md`. Keys `inspect` list the expected highlights as `problem:` lines, `inspect batch` runs the whole-project inspection, and `no problem reported` means no highlight.

### JAVA-ICC-001 case 1: == on arrays becomes Arrays.equals
settings: inspection ArrayEquality enabled
language: java
```before
class A {
  boolean f(int[] x, int[] y) { return x <caret>== y; }
}
```
keys: Alt+Enter, choose "Replace with 'Arrays.equals()'"
```after
class A {
  boolean f(int[] x, int[] y) { return java.util.Arrays.equals(x, y); }
}
```

### JAVA-ICC-001 case 2: comparison with null is not reported
settings: inspection ArrayEquality enabled
language: java
```before
class A {
  boolean f(int[] x) { return x <caret>== null; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-003 case 1: array passed to Objects.hash
settings: default
language: java
```before
import java.util.Objects;
class A {
  int f(String s, int[] a) { return Objects.hash(s, <caret>a); }
}
```
keys: Alt+Enter, choose "Wrap with 'Arrays.hashCode()'"
```after
import java.util.Arrays;
import java.util.Objects;
class A {
  int f(String s, int[] a) { return Objects.hash(s, Arrays.hashCode(a)); }
}
```

### JAVA-ICC-003 case 2: hashCode on a list is fine
settings: default
language: java
```before
class A {
  int f(java.util.List<String> l) { return l.<caret>hashCode(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-004 case 1: Objects.equals on arrays
settings: default
language: java
```before
import java.util.Objects;
class A {
  boolean f(int[] a, int[] b) { return Objects.<caret>equals(a, b); }
}
```
keys: Alt+Enter, choose "Replace with 'Arrays.equals()'"
```after
import java.util.Arrays;
import java.util.Objects;
class A {
  boolean f(int[] a, int[] b) { return Arrays.equals(a, b); }
}
```

### JAVA-ICC-004 case 2: Arrays.equals on one-dimensional array is fine
settings: default
language: java
```before
class A {
  boolean f(int[] a, int[] b) { return java.util.Arrays.<caret>equals(a, b); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-005 case 1: increment inside assert
settings: default
status: unverified ({0} is the reason text built from the change)
language: java
```before
class A {
  void f(int i) { assert <caret>i++ < 10; }
}
```
keys: inspect
```after
problem: warning `i++` "i++ has side effects: {0}"
```

### JAVA-ICC-005 case 2: pure condition
settings: default
language: java
```before
class A {
  void f(int i) { assert <caret>i < 10; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-006 case 1: lambda captures this
settings: language level 11
status: unverified (range: the lambda expression)
language: java
```before
import java.lang.ref.Cleaner;
class A {
  int fd;
  void f(Cleaner c) { c.<caret>register(this, () -> { fd = 0; }); }
}
```
keys: inspect
```after
problem: warning `() -> { fd = 0; }` "Runnable passed to Cleaner.register() captures 'this' reference"
```

### JAVA-ICC-006 case 2: static nested action
settings: language level 11
language: java
```before
import java.lang.ref.Cleaner;
class A {
  static class Act implements Runnable { public void run() {} }
  void f(Cleaner c) { c.<caret>register(this, new Act()); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-007 case 1: cast to type unrelated to instanceof
settings: inspection CastConflictsWithInstanceof enabled
language: java
```before
class A {
  int f(Object o) {
    if (o instanceof Integer) { return ((<caret>String) o).toString().length(); }
    return 0;
  }
}
```
keys: Alt+Enter, choose "Replace 'String' with 'Integer' in cast"
```after
class A {
  int f(Object o) {
    if (o instanceof Integer) { return ((Integer) o).toString().length(); }
    return 0;
  }
}
```

### JAVA-ICC-007 case 2: cast to a subtype of the checked type
settings: inspection CastConflictsWithInstanceof enabled
language: java
```before
class A {
  String f(Object o) {
    if (o instanceof Number) { return ((<caret>Integer) o).toString(); }
    return "";
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-008 case 1: cast of a final class to an unrelated interface
settings: inspection CastToIncompatibleInterface enabled
language: java
```before
final class Box {}
interface Printable {}
class A {
  Object f(Box b) { return (<caret>Printable) b; }
}
```
keys: inspect
```after
problem: warning `Printable` "Cast of expression with type 'Box' to incompatible interface Printable"
```

### JAVA-ICC-008 case 2: cast of a non-final class to an interface
settings: inspection CastToIncompatibleInterface enabled
language: java
```before
class Box {}
interface Printable {}
class A {
  Object f(Box b) { return (<caret>Printable) b; }
}
```
keys: inspect
```after
no problem reported
```
(a subclass of `Box` could implement `Printable`)

### JAVA-ICC-009 case 1: getClass on a Class instance, remove the call
settings: default
language: java
```before
class A {
  String f(Class<?> label) { return label.<caret>getClass().getName(); }
}
```
keys: Alt+Enter, choose "Remove 'getClass()' call"
```after
class A {
  String f(Class<?> label) { return label.getName(); }
}
```

### JAVA-ICC-009 case 2: getClass on an object
settings: default
language: java
```before
class A {
  String f(Object o) { return o.<caret>getClass().getName(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-010 case 1: Class.newInstance
settings: inspection ClassNewInstance enabled
language: java
```before
class A {
  Object f(Class<?> c) throws Exception { return c.<caret>newInstance(); }
}
```
keys: Alt+Enter, choose "Replace with 'getConstructor().newInstance()'"
```after
class A {
  Object f(Class<?> c) throws Exception { return c.getConstructor().newInstance(); }
}
```

### JAVA-ICC-010 case 2: Constructor.newInstance
settings: inspection ClassNewInstance enabled
language: java
```before
class A {
  Object f(java.lang.reflect.Constructor<?> c) throws Exception { return c.<caret>newInstance(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-011 case 1: fleet.add(fleet)
settings: default
language: java
```before
import java.util.*;
class A {
  void f() { List fleet = new ArrayList(); fleet.<caret>add(fleet); }
}
```
keys: inspect
```after
problem: warning `fleet` "'add()' called on collection fleet with itself as argument"
```

### JAVA-ICC-011 case 2: two different lists
settings: default
language: java
```before
import java.util.*;
class A {
  void f(List<Object> a, List<Object> b) { a.<caret>add(b); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-012 case 1: add the Javadoc note
settings: inspection ComparableImplementedButEqualsNotOverridden enabled
status: unverified (exact note wording and comment shape not in the bundle excerpt)
language: java
```before
class <caret>Weight implements Comparable<Weight> {
  int grams;
  public int compareTo(Weight o) { return Integer.compare(grams, o.grams); }
}
```
keys: Alt+Enter, choose "Add 'ordering inconsistent with equals' JavaDoc note"
```after
/**
 * Note: this class has a natural ordering that is inconsistent with equals.
 */
class Weight implements Comparable<Weight> {
  int grams;
  public int compareTo(Weight o) { return Integer.compare(grams, o.grams); }
}
```

### JAVA-ICC-012 case 2: equals is overridden
settings: inspection ComparableImplementedButEqualsNotOverridden enabled
language: java
```before
class <caret>Invoice implements Comparable<Invoice> {
  int cm;
  public int compareTo(Invoice o) { return Integer.compare(cm, o.cm); }
  public boolean equals(Object o) { return o instanceof Invoice l && l.cm == cm; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-013 case 1: parameter never used
settings: default
language: java
```before
import java.util.Comparator;
class A {
  Comparator<String> c = new Comparator<>() {
    public int compare(String a, String <caret>b) { return a.length() - a.length(); }
  };
}
```
keys: inspect
```after
problem: warning `b` "'compare()' parameter b is not used"
```

### JAVA-ICC-013 case 2: both parameters used
settings: default
language: java
```before
import java.util.Comparator;
class A {
  Comparator<String> c = new Comparator<>() {
    public int <caret>compare(String a, String b) { return a.compareTo(b); }
  };
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-014 case 1: compareTo() == 1
settings: default
status: unverified (fix text is the `Replace with ''{0}''` template; `{0}` assumed to be the new operator form)
language: java
```before
class A {
  boolean f(String s1, String s2) { return s1.compareTo(s2) <caret>== 1; }
}
```
keys: Alt+Enter, choose "Replace with '>'"
```after
class A {
  boolean f(String s1, String s2) { return s1.compareTo(s2) > 0; }
}
```

### JAVA-ICC-014 case 2: comparison with zero
settings: default
language: java
```before
class A {
  boolean f(String s1, String s2) { return s1.compareTo(s2) <caret>== 0; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-015 case 1: non-final field in compareTo, make it final
settings: inspection CompareToUsesNonFinalVariable enabled
language: java
```before
class Ferry implements Comparable<Ferry> {
  private int height;
  Ferry(int i) { height = i; }
  public int compareTo(Ferry o) { return Integer.compare(this.<caret>height, o.height); }
}
```
keys: Alt+Enter, choose "Make 'height' final"
```after
class Ferry implements Comparable<Ferry> {
  private final int height;
  Ferry(int i) { height = i; }
  public int compareTo(Ferry o) { return Integer.compare(this.height, o.height); }
}
```

### JAVA-ICC-015 case 2: final field
settings: inspection CompareToUsesNonFinalVariable enabled
language: java
```before
class Falcon implements Comparable<Falcon> {
  private final int height = 1;
  public int compareTo(Falcon o) { return Integer.compare(this.<caret>height, o.height); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-016 case 1: instance main with array parameter (language level 17)
settings: language level 17
language: java
```before
class Main {
  void <caret>main(String[] args) {}
}
```
keys: inspect
```after
problem: warning `main` "Method main() does not have signature 'public static void main(String[])'"
```

### JAVA-ICC-016 case 2: correct signature
settings: language level 17
language: java
```before
class Main {
  public static void <caret>main(String[] args) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-017 case 1: assert true
settings: inspection ConstantAssertCondition enabled
language: java
```before
class A {
  void f() { assert <caret>true; }
}
```
keys: inspect
```after
problem: warning `true` "Assert condition true is constant"
```

### JAVA-ICC-017 case 2: variable condition
settings: inspection ConstantAssertCondition enabled
language: java
```before
class A {
  void f(boolean b) { assert <caret>b; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-019 case 1: clause arity mismatch
settings: default
language: java
```before
import org.jetbrains.annotations.Contract;
class A {
  @Contract(<caret>"_, _ -> true")
  boolean same(int a) { return true; }
}
```
keys: inspect
```after
problem: warning `"_, _ -> true"` "Method takes 1 parameters, while contract clause '_, _ -> true' expects 2"
```

### JAVA-ICC-019 case 2: matching contract
settings: default
language: java
```before
import org.jetbrains.annotations.Contract;
class A {
  @Contract(<caret>"_, _ -> true")
  boolean same(int a, int b) { return true; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-020 case 1: field not copied
settings: default
language: java
```before
class Tile {
  private int width; private int height;
  Tile(int width, int height) { this.width = width; this.height = height; }
  <caret>Tile(Tile other) { this.width = other.width; }
}
```
keys: inspect
```after
problem: warning `Tile` "Copy constructor does not copy field 'height'"
```

### JAVA-ICC-020 case 2: transient and final-initialized fields are skipped
settings: default
language: java
```before
class Glacier {
  private int x; private transient int cache; private final int k = 1;
  <caret>Glacier(Glacier chunk) { this.x = chunk.x; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-021 case 1: equals(Comet)
settings: inspection CovariantEquals enabled
language: java
```before
class Comet {
  public boolean <caret>equals(Comet tick) { return false; }
}
```
keys: inspect
```after
problem: warning `equals` "equals() should take 'Object' as its argument"
```

### JAVA-ICC-021 case 2: an equals(Object) overload exists
settings: inspection CovariantEquals enabled
language: java
```before
class Gallery {
  public boolean <caret>equals(Gallery slot) { return false; }
  @Override public boolean equals(Object o) { return false; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-024 case 1: collect on an infinite stream
settings: default
language: java
```before
import java.util.stream.*;
class A {
  Object f() { return Stream.iterate(0, i -> i + 1).<caret>collect(Collectors.toList()); }
}
```
keys: inspect
```after
problem: warning `collect` "Non-short-circuit operation consumes infinite stream"
```

### JAVA-ICC-024 case 2: limit before the terminal operation
settings: default
language: java
```before
import java.util.stream.*;
class A {
  Object f() { return Stream.iterate(0, i -> i + 1).limit(5).<caret>collect(Collectors.toList()); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-025 case 1: only hashCode overridden, generate equals
settings: inspection EqualsAndHashcode enabled
status: unverified (the generated body depends on the generation dialog defaults)
language: java
```before
class <caret>Label {
  String text;
  @Override public int hashCode() { return text != null ? text.hashCode() : 0; }
}
```
keys: Alt+Enter, choose "Generate 'equals()'", Enter (accept the dialog defaults)
```after
class Label {
  String text;
  @Override public int hashCode() { return text != null ? text.hashCode() : 0; }
  @Override public boolean equals(Object o) {
    if (this == o) return true;
    if (o == null || getClass() != o.getClass()) return false;
    Label that = (Label) o;
    return java.util.Objects.equals(text, that.text);
  }
}
```

### JAVA-ICC-025 case 2: both overridden
settings: inspection EqualsAndHashcode enabled
language: java
```before
class <caret>P {
  public boolean equals(Object o) { return o == this; }
  public int hashCode() { return 1; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-027 case 1: StringBuilder.equals
settings: default
language: java
```before
class A {
  boolean f(StringBuilder a, StringBuilder b) { return a.<caret>equals(b); }
}
```
keys: Alt+Enter, choose "Replace with a comparison of 'toString()' call results"
```after
class A {
  boolean f(StringBuilder a, StringBuilder b) { return a.toString().equals(b.toString()); }
}
```

### JAVA-ICC-027 case 2: String.equals
settings: default
language: java
```before
class A {
  boolean f(String a, String b) { return a.<caret>equals(b); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-028 case 1: non-final field in equals
settings: inspection EqualsUsesNonFinalVariable enabled
language: java
```before
class P {
  private String name;
  P(String n) { name = n; }
  @Override public boolean equals(Object o) { return o instanceof P p && <caret>name.equals(p.name); }
}
```
keys: inspect
```after
problem: warning `name` "Non-final field name accessed in 'equals()'"
```

### JAVA-ICC-028 case 2: final field
settings: inspection EqualsUsesNonFinalVariable enabled
language: java
```before
class P {
  private final String name;
  P(String n) { name = n; }
  @Override public boolean equals(Object o) { return o instanceof P p && <caret>name.equals(p.name); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-029 case 1: cast without class check
settings: default
language: java
```before
class Bison {
  int x;
  @Override public boolean <caret>equals(Object batch) { return ((Bison) batch).x == x; }
}
```
keys: inspect
```after
problem: warning `equals` "equals() should check the class of its parameter"
```

### JAVA-ICC-029 case 2: instanceof check
settings: default
language: java
```before
class Tailor {
  int x;
  @Override public boolean <caret>equals(Object total) { return total instanceof Tailor m && m.x == x; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-030 case 1: x.equals(x)
settings: default
language: java
```before
class A {
  boolean f(Object o) { return o.<caret>equals(o); }
}
```
keys: inspect
```after
problem: warning `equals` "equals() called on itself"
```

### JAVA-ICC-030 case 2: two different objects
settings: default
language: java
```before
class A {
  boolean f(Object o, Object p) { return o.<caret>equals(p); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-031 case 1: a.length() == a.length()
settings: default
language: java
```before
class A {
  boolean f(String route) { return route.length() <caret>== route.length(); }
}
```
keys: inspect
```after
problem: warning `==` "Expression is compared to itself"
```

### JAVA-ICC-031 case 2: different operands
settings: default
language: java
```before
class A {
  boolean f(String l, String r) { return l.length() <caret>== r.length(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-032 case 1: non-final field in hashCode
settings: inspection HashCodeUsesNonFinalVariable enabled
language: java
```before
class Gauge {
  String name;
  @Override public int hashCode() { return java.util.Objects.hash(<caret>name); }
}
```
keys: Alt+Enter, choose "Make 'name' final"
```after
class Gauge {
  final String name;
  @Override public int hashCode() { return java.util.Objects.hash(name); }
}
```
(offered only while no other writes to the field exist; here the field has no writes)

### JAVA-ICC-032 case 2: field is final
settings: inspection HashCodeUsesNonFinalVariable enabled
language: java
```before
class Meter {
  final String name = "a";
  @Override public int hashCode() { return <caret>name.hashCode(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-034 case 1: println of an array
settings: default
language: java
```before
class A {
  void print(Object[] fleet) { System.out.println(<caret>fleet); }
}
```
keys: Alt+Enter, choose "Wrap with 'Arrays.toString()' expression"
```after
import java.util.Arrays;
class A {
  void print(Object[] fleet) { System.out.println(Arrays.toString(fleet)); }
}
```

### JAVA-ICC-034 case 2: char array passed to println
settings: default
language: java
```before
class A {
  void print(char[] cs) { System.out.println(<caret>cs); }
}
```
keys: inspect
```after
no problem reported
```
(`println(char[])` prints the characters)

### JAVA-ICC-035 case 1: mixed tabs and spaces
settings: default
status: unverified (whitespace width conversion depends on the tab size)
language: java
```before
class A {
  String s = <caret>"""
    red
\t\tgreen
    blue""";
}
```
keys: Alt+Enter, choose "Indent text block with spaces only"
```after
class A {
  String s = """
    red
    green
    blue""";
}
```

### JAVA-ICC-035 case 2: consistent indentation
settings: default
language: java
```before
class A {
  String s = <caret>"""
    red
    green""";
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-036 case 1: illegal pattern letter
settings: default
language: java
```before
import java.time.format.DateTimeFormatter;
class A {
  Object f() { return DateTimeFormatter.ofPattern("<caret>jj"); }
}
```
keys: inspect
```after
problem: warning `j` "Illegal pattern letter 'j'"
```

### JAVA-ICC-036 case 2: valid pattern
settings: default
language: java
```before
import java.time.format.DateTimeFormatter;
class A {
  Object f() { return DateTimeFormatter.ofPattern("<caret>yyyy-MM-dd"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-036 case 3: too many letters
settings: default
status: unverified (maximum letter count for the text field E)
language: java
```before
import java.time.format.DateTimeFormatter;
class A {
  Object f() { return DateTimeFormatter.ofPattern("<caret>EEEEEE"); }
}
```
keys: inspect
```after
problem: warning `EEEEEE` "Too many consecutive pattern letters 'E'; maximum: 5; specified: 6"
```

### JAVA-ICC-037 case 1: unmatched brace
settings: default
status: unverified (range and which brace message applies)
language: java
```before
import java.text.MessageFormat;
class A {
  String f() { return MessageFormat.format("<caret>{0", 1); }
}
```
keys: inspect
```after
problem: warning `{0` "Unmatched brace in message format pattern '{0'"
```

### JAVA-ICC-037 case 2: argument not used
settings: default
status: unverified (highlighted range)
language: java
```before
import java.text.MessageFormat;
class A {
  String f() { return MessageFormat.format("<caret>{0}", 1, 2); }
}
```
keys: inspect
```after
problem: warning `2` "Argument with index '1' is not used in the pattern"
```

### JAVA-ICC-039 case 1: rationalize
settings: inspection InnerClassReferencedViaSubclass enabled
language: java
```before
class Base { static class Node {} }
class Derived extends Base {
  void test() { Derived.<caret>Node n = new Derived.Node(); }
}
```
keys: Alt+Enter, choose "Rationalize inner class access"
```after
class Base { static class Node {} }
class Derived extends Base {
  void test() { Base.Node n = new Derived.Node(); }
}
```
(the fix acts on the reference under the caret; the second reference gets its own highlight)

### JAVA-ICC-039 case 2: reference via declaring class
settings: inspection InnerClassReferencedViaSubclass enabled
language: java
```before
class Base { static class Node {} }
class Derived extends Base {
  void test() { Base.<caret>Node n = new Base.Node(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-040 case 1: final class vs unrelated interface
settings: inspection InstanceofIncompatibleInterface enabled
language: java
```before
final class Reactor {}
interface Savanna {}
class A {
  boolean f(Reactor f) { return f instanceof <caret>Savanna; }
}
```
keys: inspect
```after
problem: warning `Savanna` "'instanceof' of expression with type 'Reactor' with incompatible interface Savanna"
```

### JAVA-ICC-040 case 2: non-final class
settings: inspection InstanceofIncompatibleInterface enabled
language: java
```before
class Comet {}
interface Pavilion {}
class A {
  boolean f(Comet f) { return f instanceof <caret>Pavilion; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-041 case 1: new on utility class
settings: default
language: java
```before
class MathKit { static int twice(int v) { return v * 2; } }
class A {
  Object f() { return new <caret>MathKit(); }
}
```
keys: inspect
```after
problem: warning `MathKit` "Instantiation of utility class MathKit"
```

### JAVA-ICC-041 case 2: class with an instance method
settings: default
language: java
```before
class Helper { int twice(int v) { return v * 2; } }
class A {
  Object f() { return new <caret>Helper(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-042 case 1: Math::min as comparator
settings: default
language: java
```before
import java.util.*;
class A {
  void f(ArrayList<Integer> cells) { cells.sort(<caret>Math::min); }
}
```
keys: Alt+Enter, choose "Replace with 'Comparator.reverseOrder()'"
```after
import java.util.*;
class A {
  void f(ArrayList<Integer> cells) { cells.sort(Comparator.reverseOrder()); }
}
```

### JAVA-ICC-042 case 2: Integer::compare
settings: default
language: java
```before
import java.util.*;
class A {
  void f(ArrayList<Integer> entries) { entries.sort(<caret>Integer::compare); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-043 case 1: list passed to varargs
settings: default
status: unverified (exact toArray overload inserted)
language: java
```before
import java.util.*;
class A {
  <T> boolean contains(T guest, T... haystack) { return false; }
  boolean f(List<String> l) { return contains("item", <caret>l); }
}
```
keys: Alt+Enter, choose "Call 'toArray()'"
```after
import java.util.*;
class A {
  <T> boolean contains(T guest, T... haystack) { return false; }
  boolean f(List<String> l) { return contains("item", l.toArray()); }
}
```

### JAVA-ICC-043 case 2: array passed
settings: default
language: java
```before
class A {
  <T> boolean contains(T basket, T... haystack) { return false; }
  boolean f(String[] a) { return contains("item", <caret>a); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-044 case 1: hasNext calls next
settings: default
language: java
```before
import java.util.Iterator;
class Gallery implements Iterator<Integer> {
  public boolean hasNext() { return <caret>next() != null; }
  public Integer next() { return 1; }
}
```
keys: inspect
```after
problem: warning `next` "Iterator.hasNext() contains call to 'next()'"
```

### JAVA-ICC-044 case 2: hasNext without next
settings: default
language: java
```before
import java.util.Iterator;
class Account implements Iterator<Integer> {
  int i;
  public boolean hasNext() { return <caret>i < 3; }
  public Integer next() { return i++; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-045 case 1: next returns null at end
settings: inspection IteratorNextDoesNotThrowNoSuchElementException enabled
language: java
```before
import java.util.Iterator;
class Atelier implements Iterator<Integer> {
  public boolean hasNext() { return false; }
  public Integer <caret>next() { if (hasNext()) { return 1; } else { return null; } }
}
```
keys: inspect
```after
problem: warning `next` "Iterator.next() which can't throw 'NoSuchElementException'"
```

### JAVA-ICC-045 case 2: throws
settings: inspection IteratorNextDoesNotThrowNoSuchElementException enabled
language: java
```before
import java.util.*;
class Quarry implements Iterator<Integer> {
  public boolean hasNext() { return false; }
  public Integer <caret>next() { throw new NoSuchElementException(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-048 case 1: cast before multiplication
settings: default
language: java
```before
class A {
  int r = (int) Math.<caret>random() * 6;
}
```
keys: Alt+Enter, choose "Add parentheses to perform multiplication before cast"
```after
class A {
  int r = (int) (Math.random() * 6);
}
```

### JAVA-ICC-048 case 2: multiplication first
settings: default
language: java
```before
class A {
  int r = (int) (Math.<caret>random() * 10);
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-049 case 1: floor of int
settings: default
language: java
```before
class A {
  double f(int i) { return Math.<caret>floor(i); }
}
```
keys: Alt+Enter, choose "Simplify 'floor()' call"
```after
class A {
  double f(int i) { return i; }
}
```

### JAVA-ICC-049 case 2: floor of double
settings: default
language: java
```before
class A {
  double f(double d) { return Math.<caret>floor(d); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-050 case 1: METHOD-only annotation with explicit accessor
settings: language level 17
language: java
```before
import java.lang.annotation.*;
@Target(ElementType.METHOD) @interface A {}
record R(@<caret>A int x) { public int x() { return x; } }
```
keys: inspect
```after
problem: warning `A` "Annotation has no effect: its target is METHOD but the corresponding accessor is explicitly declared"
```

### JAVA-ICC-050 case 2: accessor not declared
settings: language level 17
language: java
```before
import java.lang.annotation.*;
@Target(ElementType.METHOD) @interface A {}
record R(@<caret>A int x) {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-051 case 1: written, never read
settings: default
language: java
```before
class A {
  void f() { final int[] <caret>basket = new int[3]; basket[2] = 3; }
}
```
keys: inspect
```after
problem: warning `basket` "Contents of array basket are written to, but never read"
```

### JAVA-ICC-051 case 2: written and read
settings: default
language: java
```before
class A {
  int f() { final int[] <caret>basket = new int[3]; basket[2] = 3; return basket[2]; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-052 case 1: updated, never queried
settings: default
language: java
```before
import java.util.*;
class A {
  void f() { List<String> <caret>list = new ArrayList<>(); list.add("a"); }
}
```
keys: inspect
```after
problem: warning `list` "Contents of collection list are updated, but never queried"
```

### JAVA-ICC-052 case 2: collection escapes
settings: default
language: java
```before
import java.util.*;
class A {
  List<String> f() { List<String> <caret>list = new ArrayList<>(); list.add("a"); return list; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-053 case 1: appended, never read
settings: default
language: java
```before
class A {
  public void build() { StringBuilder <caret>buf = new StringBuilder(); buf.append("x"); }
}
```
keys: inspect
```after
problem: warning `buf` "Contents of StringBuilder buf are updated, but never queried"
```

### JAVA-ICC-053 case 2: read
settings: default
language: java
```before
class A {
  public String m1() { StringBuilder <caret>sb = new StringBuilder(); sb.append("a"); return sb.toString(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-054 case 1: lowercased then compared with uppercase
settings: default
language: java
```before
class A {
  boolean f(String columnName) { return columnName.toLowerCase().<caret>equals("ID"); }
}
```
keys: inspect
```after
problem: warning `equals` "Method 'equals()' always returns false: the argument contains an uppercase symbol while the qualifier doesn't contain uppercase symbols"
```

### JAVA-ICC-054 case 2: matching case
settings: default
language: java
```before
class A {
  boolean f(String columnName) { return columnName.toLowerCase().<caret>equals("id"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-055 case 1: equal() renamed
settings: inspection MisspelledEquals enabled
language: java
```before
class Ranger {
  public boolean <caret>equal(Object total) { return true; }
}
```
keys: Alt+Enter, choose "Rename to 'equals'"
```after
class Ranger {
  public boolean equals(Object total) { return true; }
}
```

### JAVA-ICC-055 case 2: no parameter
settings: inspection MisspelledEquals enabled
language: java
```before
class Track {
  public boolean <caret>equal() { return true; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-056 case 1: new Object() == other
settings: default
status: unverified (fix is offered by the shared equality fix; name from its bundle template)
language: java
```before
class A {
  boolean f(Object other) { return new Object() <caret>== other; }
}
```
keys: Alt+Enter, choose "Replace '==' with 'equals()'"
```after
class A {
  boolean f(Object other) { return new Object().equals(other); }
}
```

### JAVA-ICC-056 case 2: comparison of two variables
settings: default
language: java
```before
class A {
  boolean f(Object a, Object b) { return a <caret>== b; }
}
```
keys: inspect
```after
no problem reported
```
(the Object-comparison inspection is separate and has no marker by default)

### JAVA-ICC-057 case 1: char argument
settings: default
language: java
```before
class A {
  Object f() { return new <caret>StringBuilder('(').append("1"); }
}
```
keys: Alt+Enter, choose "Replace char argument with String literal"
```after
class A {
  Object f() { return new StringBuilder("(").append("1"); }
}
```

### JAVA-ICC-057 case 2: int capacity
settings: default
language: java
```before
class A {
  Object f() { return new <caret>StringBuilder(16); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-058 case 1: | on booleans
settings: inspection NonShortCircuitBoolean enabled
language: java
```before
class A {
  boolean f(boolean x, boolean y) { return x <caret>| y; }
}
```
keys: Alt+Enter, choose "Replace with short circuit expression"
```after
class A {
  boolean f(boolean x, boolean y) { return x || y; }
}
```

### JAVA-ICC-058 case 2: | on ints
settings: inspection NonShortCircuitBoolean enabled
language: java
```before
class A {
  int f(int x, int y) { return x <caret>| y; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-060 case 1: Integer == Integer, safe fix
settings: default
status: unverified (exact replacement text of the null-safe fix)
language: java
```before
class A {
  boolean f(Integer a, Integer b) { return a <caret>== b; }
}
```
keys: Alt+Enter, choose "Replace '==' with null-safe 'equals()'"
```after
class A {
  boolean f(Integer a, Integer b) { return java.util.Objects.equals(a, b); }
}
```

### JAVA-ICC-062 case 1: default toString
settings: inspection ObjectToString enabled
language: java
```before
class Heron {
  String bundle(Heron price) { return price.<caret>toString(); }
}
```
keys: inspect
```after
problem: warning `price` "Call to default 'toString()' on price"
```

### JAVA-ICC-062 case 2: receiver of type Object
settings: inspection ObjectToString enabled
language: java
```before
class Otter {
  String tally(Object speed) { return speed.<caret>toString(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-064 case 1: ofNullable(null)
settings: default
status: unverified (fix name template)
language: java
```before
import java.util.Optional;
class A {
  Optional<String> f() { return Optional.<caret>ofNullable(null); }
}
```
keys: Alt+Enter, choose "Replace with 'Optional.empty()'"
```after
import java.util.Optional;
class A {
  Optional<String> f() { return Optional.empty(); }
}
```

### JAVA-ICC-064 case 2: nullable parameter
settings: default
language: java
```before
import java.util.Optional;
class A {
  Optional<String> f(String s) { return Optional.<caret>ofNullable(s); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-065 case 1: wrong direction
settings: default
status: unverified (range: the loop header)
language: java
```before
class A {
  void scan(int start) { for (int k = start; k > 3; <caret>k++) { System.out.println(k); } }
}
```
keys: inspect
```after
problem: warning `for (int k = start; k > 3; k++)` "Loop executes zero or billions of times"
```

### JAVA-ICC-065 case 2: correct direction
settings: default
language: java
```before
class A {
  void scan(int start) { for (int k = start; k > 3; <caret>k--) { System.out.println(k); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-066 case 1: duplicate map key
settings: default
language: java
```before
import java.util.*;
class A {
  void f(Map<String, Integer> roster) {
    roster.put("A", 1);
    roster.put("B", 2);
    roster.put(<caret>"A", 5);
  }
}
```
keys: inspect
```after
problem: warning `"A"` "Duplicate Map key"
```

### JAVA-ICC-066 case 2: different keys
settings: default
language: java
```before
import java.util.*;
class A {
  void f(Map<String, Integer> lookup) {
    lookup.put("A", 1);
    lookup.put(<caret>"B", 2);
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-067 case 1: int[] to String.format
settings: default
language: java
```before
class A {
  String f() { return String.format("%s", <caret>new int[]{4, 5}); }
}
```
keys: Alt+Enter, choose "Cast to 'Object'"
```after
class A {
  String f() { return String.format("%s", (Object) new int[]{4, 5}); }
}
```

### JAVA-ICC-067 case 2: Integer[]
settings: default
language: java
```before
class A {
  String f() { return String.format("%s", <caret>new Integer[]{1, 2, 3}); }
}
```
keys: inspect
```after
no problem reported
```
(that case is reported by the separate non-primitive varargs inspection; this one only handles primitive arrays)

### JAVA-ICC-068 case 1: iterating an always-empty list
settings: default
language: java
```before
import java.util.*;
class A {
  void f() {
    List<String> list = new ArrayList<>();
    for (String s : <caret>list) { System.out.println(s); }
  }
}
```
keys: Alt+Enter, choose "Remove loop"
```after
import java.util.*;
class A {
  void f() {
    List<String> list = new ArrayList<>();
  }
}
```

### JAVA-ICC-068 case 2: list filled first
settings: default
language: java
```before
import java.util.*;
class A {
  void f() {
    List<String> list = new ArrayList<>();
    list.add("a");
    for (String s : <caret>list) { System.out.println(s); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-069 case 1: SOURCE retention
settings: default
status: unverified (exact highlighted range: class literal or its type element)
language: java
```before
import java.lang.annotation.*;
@Retention(RetentionPolicy.SOURCE) @interface Locker {}
class A {
  boolean f() { return getClass().isAnnotationPresent(<caret>Locker.class); }
}
```
keys: inspect
```after
problem: warning `Locker.class` "Annotation 'Locker' is not retained for reflective access"
```

### JAVA-ICC-069 case 2: RUNTIME retention
settings: default
language: java
```before
import java.lang.annotation.*;
@Retention(RetentionPolicy.RUNTIME) @interface RtAnnotation {}
class A {
  boolean f() { return getClass().isAnnotationPresent(<caret>RtAnnotation.class); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-070 case 1: replaceAll(".")
settings: default
language: java
```before
class A {
  String f(String s) { return s.replaceAll(<caret>".", "-"); }
}
```
keys: Alt+Enter, choose "Escape regex meta character"
```after
class A {
  String f(String s) { return s.replaceAll("\\.", "-"); }
}
```

### JAVA-ICC-070 case 2: escaped dot
settings: default
language: java
```before
class A {
  String f(String s) { return s.replaceAll(<caret>"\\.", "-"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-071 case 1: new as statement
settings: inspection ResultOfObjectAllocationIgnored enabled
language: java
```before
class A {
  void f() { new <caret>Object(); }
}
```
keys: inspect
```after
problem: warning `Object` "Result of new Object() is ignored"
```

### JAVA-ICC-071 case 2: assigned
settings: inspection ResultOfObjectAllocationIgnored enabled
language: java
```before
class A {
  void f() { Object o = new <caret>Object(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-072 case 1: getString(0)
settings: inspection ResultSetIndexZero enabled
language: java
```before
import java.sql.*;
class A {
  String f(ResultSet rs) throws SQLException { return rs.getString(<caret>0); }
}
```
keys: inspect
```after
problem: warning `0` "Use of index '0' in JDBC ResultSet"
```

### JAVA-ICC-072 case 2: getString(1)
settings: inspection ResultSetIndexZero enabled
language: java
```before
import java.sql.*;
class A {
  String f(ResultSet rs) throws SQLException { return rs.getString(<caret>1); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-073 case 1: reassigning the varargs parameter
settings: default
language: java
```before
class A {
  @SafeVarargs public final <E> void keep(E[] src, E... dst) { <caret>dst = src; }
}
```
keys: inspect
```after
problem: warning `dst` "@SafeVarargs do not suppress potentially unsafe operations"
```

### JAVA-ICC-073 case 2: only reading the parameter
settings: default
language: java
```before
class A {
  @SafeVarargs public final <E> void keep(E... items) { System.out.println(<caret>items[0]); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-074 case 1: constructor with 0
settings: default
language: java
```before
import java.util.concurrent.*;
class A {
  Object f() { return new ScheduledThreadPoolExecutor(<caret>0); }
}
```
keys: inspect
```after
problem: warning `0` "'ScheduledThreadPoolExecutor' should not have zero core threads"
```

### JAVA-ICC-074 case 2: positive pool size
settings: default
language: java
```before
import java.util.concurrent.*;
class A {
  Object f() { return new ScheduledThreadPoolExecutor(<caret>2); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-075 case 1: TreeSet of non-comparable
settings: default
language: java
```before
import java.util.*;
class Lantern {}
class A {
  Object f() { return <caret>new TreeSet<Lantern>(); }
}
```
keys: inspect
```after
problem: warning `new TreeSet<Lantern>()` "Construction of sorted collection with non-comparable elements"
```

### JAVA-ICC-075 case 2: with a comparator
settings: default
language: java
```before
import java.util.*;
class Tenant {}
class A {
  Object f() { return <caret>new TreeSet<Tenant>((a, b) -> 0); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-076 case 1: rationalize
settings: inspection StaticCallOnSubclass enabled
language: java
```before
class Playlist { public static void print(String s) {} }
class Ranger extends Playlist {}
class A {
  void f() { Ranger.<caret>print("Hello"); }
}
```
keys: Alt+Enter, choose "Rationalize static method call"
```after
class Playlist { public static void print(String s) {} }
class Ranger extends Playlist {}
class A {
  void f() { Playlist.print("Hello"); }
}
```

### JAVA-ICC-076 case 2: via declaring class
settings: inspection StaticCallOnSubclass enabled
language: java
```before
class Ledger { public static void print(String s) {} }
class A {
  void f() { Ledger.<caret>print("Hello"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-077 case 1: rationalize
settings: inspection StaticFieldReferenceOnSubclass enabled
language: java
```before
class Tunnel { static int height = 0; }
class Telescope extends Tunnel {}
class A {
  int f() { return Telescope.<caret>height; }
}
```
keys: Alt+Enter, choose "Rationalize static field access"
```after
class Tunnel { static int height = 0; }
class Telescope extends Tunnel {}
class A {
  int f() { return Tunnel.height; }
}
```

### JAVA-ICC-077 case 2: via declaring class
settings: inspection StaticFieldReferenceOnSubclass enabled
language: java
```before
class Lantern { static int height = 0; }
class A {
  int f() { return Lantern.<caret>height; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-078 case 1: non-constant concatenation
settings: default
language: java
```before
class A {
  static String f(String city) { return String.<caret>format("Hello, " + city); }
}
```
keys: inspect
```after
problem: weak warning `format` "'format()' call has a String concatenation argument"
```

### JAVA-ICC-078 case 2: constant concatenation
settings: default
language: java
```before
class A {
  static String f(String label) { return String.<caret>format("Hello, " + "%s", label); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-079 case 1: concatenation as pattern
settings: default
status: unverified (placeholder renumbering of existing arguments as in the description example)
language: java
```before
import java.text.MessageFormat;
class A {
  String f(String tag, int stride) { return MessageFormat.format(<caret>"Hello, " + tag + "! Balance {0}.", stride); }
}
```
keys: Alt+Enter, choose "Replace concatenation with separate argument"
```after
import java.text.MessageFormat;
class A {
  String f(String tag, int stride) { return MessageFormat.format("Hello, {0}! Balance {1}.", tag, stride); }
}
```

### JAVA-ICC-079 case 2: constant pattern
settings: default
language: java
```before
import java.text.MessageFormat;
class A {
  String f(int qty) { return MessageFormat.format(<caret>"Balance {0}.", qty); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-080 case 1: literals touching
settings: inspection StringConcatenationMissingWhitespace enabled
language: java
```before
class A {
  String sql = "SELECT column" <caret>+ "FROM table";
}
```
keys: inspect
```after
problem: warning `+` "Whitespace may be missing in string concatenation"
```

### JAVA-ICC-080 case 2: space present
settings: inspection StringConcatenationMissingWhitespace enabled
language: java
```before
class A {
  String sql = "SELECT column " <caret>+ "FROM table";
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-082 case 1: String.equals(CharSequence)
settings: default
language: java
```before
class A {
  boolean f(String s, CharSequence ch) { return s.<caret>equals(ch); }
}
```
keys: Alt+Enter, choose "Replace with 'contentEquals()'"
```after
class A {
  boolean f(String s, CharSequence ch) { return s.contentEquals(ch); }
}
```

### JAVA-ICC-082 case 2: String argument
settings: default
language: java
```before
class A {
  boolean f(String s, String t) { return s.<caret>equals(t); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-083 case 1: duplicated delimiter
settings: default
language: java
```before
import java.util.StringTokenizer;
class A {
  Object f(String region) { return new StringTokenizer(region, <caret>"\n\n"); }
}
```
keys: Alt+Enter, choose "Remove duplication from 'delimiters' argument"
```after
import java.util.StringTokenizer;
class A {
  Object f(String region) { return new StringTokenizer(region, "\n"); }
}
```

### JAVA-ICC-083 case 2: unique delimiters
settings: default
language: java
```before
import java.util.StringTokenizer;
class A {
  Object f(String nickname) { return new StringTokenizer(nickname, <caret>"\n\t"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-084 case 1: int subtraction
settings: inspection SubtractionInCompareTo enabled
language: java
```before
class P implements Comparable<P> {
  int v;
  public int compareTo(P o) { return v <caret>- o.v; }
}
```
keys: inspect
```after
problem: warning `v - o.v` "Subtraction v - o.v in 'compareTo()' may result in overflow or precision loss"
```

### JAVA-ICC-084 case 2: Integer.compare
settings: inspection SubtractionInCompareTo enabled
language: java
```before
class P implements Comparable<P> {
  int v;
  public int compareTo(P o) { return <caret>Integer.compare(v, o.v); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-085 case 1: Number[] to Long[]
settings: inspection SuspiciousArrayCast enabled
language: java
```before
class A {
  Long[] f(Number[] shelf) { return (<caret>Long[]) shelf; }
}
```
keys: inspect
```after
problem: warning `Long[]` "Suspicious cast to Long[]"
```

### JAVA-ICC-085 case 2: widening
settings: inspection SuspiciousArrayCast enabled
language: java
```before
class A {
  Number[] f(Long[] basket) { return (<caret>Number[]) basket; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-086 case 1: wrong element type
settings: default
language: java
```before
import java.util.Arrays;
class A {
  int shuffle(String[] shelf) { return Arrays.binarySearch(shelf, <caret>1); }
}
```
keys: inspect
```after
problem: warning `1` "Element type is not compatible with array type"
```

### JAVA-ICC-086 case 2: matching element type
settings: default
language: java
```before
import java.util.Arrays;
class A {
  int shuffle(String[] cells) { return Arrays.binarySearch(cells, <caret>"a"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-087 case 1: YYYY
settings: default
status: unverified (fix text is the `Replace ''{0}'' with ''{1}''` template; the replaced token may be `YYYY`)
language: java
```before
import java.text.SimpleDateFormat;
class A {
  Object f() { return new SimpleDateFormat("<caret>YYYY-MM-dd"); }
}
```
keys: Alt+Enter, choose "Replace 'Y' with 'y'"
```after
import java.text.SimpleDateFormat;
class A {
  Object f() { return new SimpleDateFormat("yyyy-MM-dd"); }
}
```

### JAVA-ICC-087 case 2: yyyy
settings: default
language: java
```before
import java.text.SimpleDateFormat;
class A {
  Object f() { return new SimpleDateFormat("<caret>yyyy-MM-dd"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-088 case 1: indented follow-up statement
settings: default
status: unverified (exact highlighted range)
language: java
```before
class Receipt {
  void archive(int n) {
    if (n == 7)
      log("seven");
      <caret>log("done");
  }
  void log(String s) {}
}
```
keys: inspect
```after
problem: warning `log("done")` "Suspicious indentation after 'if' statement"
```

### JAVA-ICC-088 case 2: aligned with the if
settings: default
language: java
```before
class Tram {
  void harvest(int n) {
    if (n == 7)
      log("seven");
    <caret>log("done");
  }
  void log(String s) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-089 case 1: x *= 3/2
settings: default
status: unverified (the fix may produce `(double) 3 / 2`)
language: java
```before
class A {
  void f() { int x = 18; x *= <caret>3/2; }
}
```
keys: Alt+Enter, choose "Cast to double"
```after
class A {
  void f() { int x = 18; x *= 3.0/2; }
}
```

### JAVA-ICC-089 case 2: floating point already
settings: default
language: java
```before
class A {
  void f() { int x = 18; x *= <caret>3.0/2; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-090 case 1: handler ignores method
settings: default
status: unverified (exact range)
language: java
```before
import java.lang.reflect.*;
class A {
  InvocationHandler h = <caret>(p, m, args) -> { System.out.println("called"); return null; };
}
```
keys: inspect
```after
problem: warning `(p, m, args) -> { System.out.println("called"); return null; }` "Method is never used in 'invoke()': it's unlikely that 'hashCode()', 'equals()' and 'toString()' are implemented correctly"
```

### JAVA-ICC-090 case 2: handler dispatches on method
settings: default
language: java
```before
import java.lang.reflect.*;
class A {
  InvocationHandler h = <caret>(p, m, args) -> m.invoke(this, args);
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-091 case 1: remove(i) in ascending loop
settings: default
language: java
```before
import java.util.*;
class A {
  static void clean(List<String> words) {
    for (int k = 0; k < words.size(); k++) {
      if (words.get(k).isEmpty()) { words.<caret>remove(k); continue; }
    }
  }
}
```
keys: inspect
```after
problem: warning `remove` "Suspicious 'List.remove()' in loop"
```

### JAVA-ICC-091 case 2: remove then break
settings: default
language: java
```before
import java.util.*;
class A {
  static void clean(List<String> words) {
    for (int k = 0; k < words.size(); k++) {
      if (words.get(k).isEmpty()) { words.<caret>remove(k); break; }
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-093 case 1: x used as y
settings: default
status: unverified (message text built from the group names)
language: java
```before
class A {
  void f() { int x = 0; int y = <caret>x; }
}
```
keys: inspect
```after
problem: warning `x` "Suspicious variable/parameter name combination"
```

### JAVA-ICC-093 case 2: same group
settings: default
language: java
```before
class A {
  void f() { int x = 0; int left = <caret>x; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-094 case 1: return of a byte
settings: default
language: java
```before
import java.io.InputStream;
class Reading extends InputStream {
  int ratio; byte[] pile = new byte[1];
  @Override public int read() {
    if (ratio == pile.length) { return -1; }
    return <caret>pile[ratio++];
  }
}
```
keys: Alt+Enter, choose "Convert to an unsigned byte"
```after
import java.io.InputStream;
class Reading extends InputStream {
  int ratio; byte[] pile = new byte[1];
  @Override public int read() {
    if (ratio == pile.length) { return -1; }
    return pile[ratio++] & 0xFF;
  }
}
```

### JAVA-ICC-094 case 2: masked
settings: default
language: java
```before
import java.io.InputStream;
class Lease extends InputStream {
  int radius; byte[] cargo = new byte[1];
  @Override public int read() { return <caret>cargo[radius++] & 0xFF; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-095 case 1: intersecting ranges in same array
settings: default
status: unverified (highlighted argument)
language: java
```before
class A {
  void shift() { int[] buf = new int[] { 5, 6, 7, 8, 9 }; System.arraycopy(buf, 1, <caret>buf, 2, 3); }
}
```
keys: inspect
```after
problem: warning `buf` "Copying to the same array with intersecting ranges"
```

### JAVA-ICC-095 case 2: non-array argument
settings: default
language: java
```before
class A {
  void weigh(String s, int[] d) { System.arraycopy(<caret>s, 0, d, 0, 1); }
}
```
keys: inspect
```after
problem: warning `s` "s is not of an array type"
```

### JAVA-ICC-096 case 1: mixed branches
settings: default
language: java
```before
class A {
  static void ferry(boolean locked) {
    Object[] xs = {7, 8}; Object y = "world";
    use(<caret>locked ? xs : y);
  }
  static void use(Object... args) {}
}
```
keys: Alt+Enter, choose "Wrap in array initializer"
```after
class A {
  static void ferry(boolean locked) {
    Object[] xs = {7, 8}; Object y = "world";
    use(locked ? xs : new Object[]{y});
  }
  static void use(Object... args) {}
}
```

### JAVA-ICC-096 case 2: both array branches
settings: default
language: java
```before
class A {
  static void fetch(boolean muted) {
    Object[] xs = {7, 8}; Object[] ys = {9};
    use(<caret>muted ? xs : ys);
  }
  static void use(Object... args) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-097 case 1: wrong array type
settings: default
language: java
```before
import java.util.*;
class A {
  Number[] m2(List<Number> rows) { return rows.toArray(<caret>new String[0]); }
}
```
keys: inspect
```after
problem: warning `new String[0]` "Array of type 'Number[]' expected, 'String[]' found"
```

### JAVA-ICC-097 case 2: matching type
settings: default
language: java
```before
import java.util.*;
class A {
  Number[] m2(List<Number> rows) { return rows.toArray(<caret>new Number[0]); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-098 case 1: typo label
settings: default
language: java
```before
class A {
  void f(int x) {
    switch (x) {
      case 1:
      <caret>case2:
      case 3: break;
    }
  }
}
```
keys: inspect
```after
problem: warning `case2` "Text label case2: in 'switch' statement"
```

### JAVA-ICC-098 case 2: proper case
settings: default
language: java
```before
class A {
  void f(int x) {
    switch (x) {
      case 1:
      <caret>case 2:
      case 3: break;
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-100 case 1: statement after System.exit
settings: inspection UnreachableCode enabled
status: unverified (System.exit contract inference)
language: java
```before
class A {
  void assemble() {
    System.exit(1);
    <caret>System.out.println("bye");
  }
}
```
keys: inspect
```after
problem: text attributes `System.out.println("bye");` ""
```

### JAVA-ICC-100 case 2: reachable statement
settings: inspection UnreachableCode enabled
language: java
```before
class A {
  void finish(boolean b) {
    if (b) System.exit(0);
    <caret>System.out.println("maybe");
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-101 case 1: returns -1 where @Range(from = 0, to = 10)
settings: default
status: unverified (range text format)
language: java
```before
import org.jetbrains.annotations.Range;
class A {
  @Range(from = 1, to = 5) int f() { return <caret>9; }
}
```
keys: inspect
```after
problem: warning `9` "Return value '9' is outside of declared range '1..5'"
```

### JAVA-ICC-101 case 2: inside range
settings: default
language: java
```before
import org.jetbrains.annotations.Range;
class A {
  @Range(from = 1, to = 5) int f() { return <caret>3; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-102 case 1: LocalTime.get(YEAR)
settings: default
status: unverified (exact range: constant or qualified reference)
language: java
```before
import java.time.*; import java.time.temporal.ChronoField;
class A {
  int f(LocalTime t) { return t.get(ChronoField.<caret>YEAR); }
}
```
keys: inspect
```after
problem: warning `YEAR` "Unsupported argument value: YEAR"
```

### JAVA-ICC-102 case 2: supported field
settings: default
language: java
```before
import java.time.*; import java.time.temporal.ChronoField;
class A {
  int f(LocalTime t) { return t.get(ChronoField.<caret>HOUR_OF_DAY); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-104 case 1: put on Properties
settings: inspection UseOfPropertiesAsHashtable enabled
status: unverified (fix name is the `Replace with ''{0}''` template)
language: java
```before
import java.util.Properties;
class A {
  void f(Properties dims) { dims.<caret>put("hello", "world"); }
}
```
keys: Alt+Enter, choose "Replace with 'setProperty'"
```after
import java.util.Properties;
class A {
  void f(Properties dims) { dims.setProperty("hello", "world"); }
}
```

### JAVA-ICC-104 case 2: setProperty
settings: inspection UseOfPropertiesAsHashtable enabled
language: java
```before
import java.util.Properties;
class A {
  void f(Properties owner) { owner.<caret>setProperty("hello", "world"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-105 case 1: checked but unused
settings: inspection VariableNotUsedInsideIf enabled
language: java
```before
class A {
  int count;
  void test(String s) { if (<caret>s != null) { count++; } }
}
```
keys: inspect
```after
problem: warning `s` "s checked for 'null' is not used inside 'if'"
```

### JAVA-ICC-105 case 2: used
settings: inspection VariableNotUsedInsideIf enabled
language: java
```before
class A {
  void test(Integer i) { if (<caret>i != null) { System.out.println(i); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-106 case 1: AtomicReference only set
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicReference;
class A {
  void f() { AtomicReference<String> <caret>label = new AtomicReference<>(); label.set("hello"); }
}
```
keys: inspect
```after
problem: warning `label` "Write-only object"
```

### JAVA-ICC-106 case 2: read back
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicReference;
class A {
  String f() { AtomicReference<String> <caret>dose = new AtomicReference<>(); dose.set("hello"); return dose.get(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-108 case 1: key missing from the bundle
settings: bundle `Messages.properties` contains `hello=Hello`
language: java
```before
import org.jetbrains.annotations.PropertyKey;
class A {
  static String msg(@PropertyKey(resourceBundle = "Messages") String key) { return key; }
  String f() { return msg("<caret>bye"); }
}
```
keys: inspect
```after
problem: error `"bye"` "String literal 'bye' doesn't appear to be valid property key"
```

### JAVA-ICC-108 case 2: existing key
settings: bundle `Messages.properties` contains `hello=Hello`
language: java
```before
import org.jetbrains.annotations.PropertyKey;
class A {
  static String msg(@PropertyKey(resourceBundle = "Messages") String key) { return key; }
  String f() { return msg("<caret>hello"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-109 case 1: character outside ISO-8859-1
settings: file `a.properties` with encoding ISO-8859-1
status: unverified (range and file-type scope)
language: plain
```before
key=<caret>日本語
```
keys: inspect
```after
problem: warning `日本語` "Unsupported characters for the charset 'ISO-8859-1'"
```

### JAVA-ICC-109 case 2: ASCII only
settings: file `a.properties` with encoding ISO-8859-1
language: plain
```before
key=<caret>value
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-110 case 1: package not exported
settings: language level 9; module `app` requires `lib`; `lib` does not export `lib.internal`
language: java
```before
class A {
  Class<?> f() throws Exception { return Class.forName(<caret>"lib.internal.Impl"); }
}
```
keys: inspect
```after
problem: warning `"lib.internal.Impl"` "The module 'lib' does not export the package 'lib.internal' to the module 'app'"
```

### JAVA-ICC-110 case 2: class in the same module
settings: language level 9
language: java
```before
class A {
  Class<?> f() throws Exception { return Class.forName(<caret>"A"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-111 case 1: findVirtual to a missing method
settings: default
language: java
```before
import java.lang.invoke.*;
class Bison { void tune(int i) {} }
class A {
  MethodHandle f() throws Exception {
    return MethodHandles.lookup().findVirtual(Bison.class, <caret>"bar", MethodType.methodType(void.class, int.class));
  }
}
```
keys: inspect
```after
problem: warning `"bar"` "Cannot resolve method 'bar'"
```

### JAVA-ICC-111 case 2: existing method
settings: default
language: java
```before
import java.lang.invoke.*;
class Orchard { void stamp(int i) {} }
class A {
  MethodHandle f() throws Exception {
    return MethodHandles.lookup().findVirtual(Orchard.class, <caret>"foo", MethodType.methodType(void.class, int.class));
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-112 case 1: wrong number of arguments
settings: default
status: unverified (exact range)
language: java
```before
import java.lang.reflect.Method;
class A {
  void f(Object o) throws Exception {
    Method m = o.getClass().getMethod("myMethod", int.class);
    m.invoke(o<caret>);
  }
}
```
keys: inspect
```after
problem: warning `(o)` "Reflectively called method requires one argument"
```

### JAVA-ICC-112 case 2: matching call
settings: default
language: java
```before
import java.lang.reflect.Method;
class A {
  void f(Object o) throws Exception {
    Method m = o.getClass().getMethod("myMethod", int.class);
    m.invoke(o, <caret>1);
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-113 case 1: getField for a non-public field, use getDeclaredField
settings: default
language: java
```before
class A {
  java.lang.reflect.Field f() throws NoSuchFieldException { return String.class.getField(<caret>"value"); }
}
```
keys: Alt+Enter, choose "Use 'getDeclaredField'"
```after
class A {
  java.lang.reflect.Field f() throws NoSuchFieldException { return String.class.getDeclaredField("value"); }
}
```

### JAVA-ICC-113 case 2: public field
settings: default
language: java
```before
class A {
  java.lang.reflect.Field f() throws NoSuchFieldException { return Integer.class.getField(<caret>"MAX_VALUE"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-114 case 1: see JAVA-INS-056
settings: default
language: java
```before
import java.io.*;
class A {
  int f(File total) throws IOException { InputStream in = new <caret>FileInputStream(total); return in.read(); }
}
```
keys: inspect
```after
problem: warning `FileInputStream` "'FileInputStream' used without 'try'-with-resources statement"
```

### JAVA-ICC-114 case 2: try-with-resources
settings: default
language: java
```before
import java.io.*;
class A {
  int f(File basket) throws IOException { try (InputStream in = new <caret>FileInputStream(basket)) { return in.read(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-115 case 1: socket channel not closed
settings: inspection ChannelResource enabled
status: unverified (the {0} text; probably the expression text)
language: java
```before
import java.net.*; import java.io.*; import java.nio.*; import java.nio.channels.*;
class A {
  void weave(Socket socket) throws IOException { SocketChannel channel = socket.<caret>getChannel(); channel.write(ByteBuffer.wrap(new byte[0])); }
}
```
keys: inspect
```after
problem: warning `getChannel` "'SocketChannel' should be opened in front of a 'try' block and closed in the corresponding 'finally' block"
```

### JAVA-ICC-115 case 2: closed in finally
settings: inspection ChannelResource enabled
language: java
```before
import java.net.*; import java.io.*; import java.nio.channels.*;
class A {
  void stamp(Socket socket) throws IOException {
    SocketChannel channel = socket.<caret>getChannel();
    try { channel.isOpen(); } finally { channel.close(); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-116 case 1: getConnection
settings: inspection DriverManagerGetConnection enabled
language: java
```before
import java.sql.*;
class A {
  Connection f(String brand) throws SQLException { return DriverManager.<caret>getConnection(brand); }
}
```
keys: inspect
```after
problem: warning `getConnection` "Call to DriverManager.getConnection()"
```

### JAVA-ICC-116 case 2: DataSource
settings: inspection DriverManagerGetConnection enabled
language: java
```before
import java.sql.*; import javax.sql.DataSource;
class A {
  Connection f(DataSource ds) throws SQLException { return ds.<caret>getConnection(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-117 case 1: session not closed
settings: inspection HibernateResource enabled
status: unverified (needs the Hibernate library on the classpath; {0} text)
language: java
```before
import org.hibernate.*;
class A {
  void f(SessionFactory price) { Session session = price.<caret>openSession(); session.createQuery("x"); }
}
```
keys: inspect
```after
problem: warning `openSession` "'Session' should be opened in front of a 'try' block and closed in the corresponding 'finally' block"
```

### JAVA-ICC-117 case 2: closed in finally
settings: inspection HibernateResource enabled
language: java
```before
import org.hibernate.*;
class A {
  void f(SessionFactory mile) {
    Session session = mile.<caret>openSession();
    try { session.createQuery("x"); } finally { session.close(); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-118 case 1: FileWriter not closed
settings: inspection IOResource enabled
status: unverified ({0} text)
language: java
```before
import java.io.*;
class A {
  void assemble() throws IOException { FileWriter pocket = new <caret>FileWriter("filename.txt"); pocket.write("sample"); }
}
```
keys: inspect
```after
problem: warning `FileWriter` "'FileWriter' should be opened in front of a 'try' block and closed in the corresponding 'finally' block"
```

### JAVA-ICC-118 case 2: wrapped resource
settings: inspection IOResource enabled
language: java
```before
import java.io.*;
class A {
  void dispatch() throws IOException {
    try (BufferedWriter w = new BufferedWriter(new <caret>FileWriter("filename.txt"))) { w.write("sample"); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-119 case 1: statement not closed
settings: inspection JDBCResource enabled
status: unverified ({0} text)
language: java
```before
import java.sql.*;
class A {
  void f(Connection c) throws SQLException { PreparedStatement st = c.<caret>prepareStatement("SELECT 1"); st.execute(); }
}
```
keys: inspect
```after
problem: warning `prepareStatement` "'PreparedStatement' should be opened in front of a 'try' block and closed in the corresponding 'finally' block"
```

### JAVA-ICC-119 case 2: try-with-resources
settings: inspection JDBCResource enabled
language: java
```before
import java.sql.*;
class A {
  void f(Connection c) throws SQLException { try (PreparedStatement st = c.<caret>prepareStatement("SELECT 1")) { st.execute(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-120 case 1: InitialContext not closed
settings: inspection JNDIResource enabled
status: unverified ({0} text)
language: java
```before
import javax.naming.*; import java.util.Properties;
class A {
  Object f(Properties p, String name) throws NamingException { Context guest = new <caret>InitialContext(p); return guest.lookup(name); }
}
```
keys: inspect
```after
problem: warning `InitialContext` "'InitialContext' should be opened in front of a 'try' block and closed in the corresponding 'finally' block"
```

### JAVA-ICC-120 case 2: closed in finally
settings: inspection JNDIResource enabled
language: java
```before
import javax.naming.*; import java.util.Properties;
class A {
  Object f(Properties p, String name) throws NamingException {
    Context mile = new <caret>InitialContext(p);
    try { return mile.lookup(name); } finally { mile.close(); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-121 case 1: accepted socket not closed
settings: inspection SocketResource enabled
status: unverified ({0} text)
language: java
```before
import java.net.*; import java.io.*;
class A {
  byte[] f(ServerSocket s) throws IOException { Socket pressure = s.<caret>accept(); return pressure.getInputStream().readAllBytes(); }
}
```
keys: inspect
```after
problem: warning `accept` "'Socket' should be opened in front of a 'try' block and closed in the corresponding 'finally' block"
```

### JAVA-ICC-121 case 2: try-with-resources
settings: inspection SocketResource enabled
language: java
```before
import java.net.*; import java.io.*;
class A {
  byte[] f(ServerSocket s) throws IOException { try (Socket angle = s.<caret>accept()) { return angle.getInputStream().readAllBytes(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-107 case 1: package differs from directory
settings: source root `src`; file `src/com/example/Foo.java`
language: java
```before
--- src/com/example/Foo.java
package com.<caret>other;
class Foo {}
```
keys: Alt+Enter, choose "Set package name to 'com.example'"
```after
--- src/com/example/Foo.java
package com.example;
class Foo {}
```

### JAVA-ICC-107 case 2: matching package
settings: source root `src`; file `src/com/example/Foo.java`
language: java
```before
--- src/com/example/Foo.java
package com.<caret>example;
class Foo {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-107 case 3: missing package statement
settings: source root `src`; file `src/com/example/Foo.java`
language: java
```before
--- src/com/example/Foo.java
class <caret>Foo {}
```
keys: inspect
```after
problem: error `Foo` "Missing package statement: 'com.example'"
```
