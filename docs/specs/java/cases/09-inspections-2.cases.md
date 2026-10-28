# Java inspections catalog, part 2: acceptance cases

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-2.md`. Keys `inspect` list the expected highlights as `problem:` lines, `inspect batch` runs the whole-project inspection, and `no problem reported` means no highlight.

### JAVA-ICA-125 case 1: Blocking call from a non-blocking method
settings: library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.Blocking;
import org.jetbrains.annotations.NonBlocking;
public class Poller {
  @Blocking
  private static void fetchAll() {}
  @NonBlocking
  private static void tick() {
    <caret>fetchAll();
  }
  private static void direct() {
    fetchAll();
  }
}
```
keys: inspect
```after
problem: warning `fetchAll` "Possibly blocking call in non-blocking context could lead to thread starvation"
```

### JAVA-ICA-125 case 2: Thread.sleep in a non-blocking method
settings: library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.NonBlocking;
public class Poller {
  @NonBlocking
  private static void tick() {
    try {
      Thread.<caret>sleep(1L);
    } catch (InterruptedException ignored) {}
  }
}
```
keys: inspect
```after
problem: warning `sleep` "Possibly blocking call in non-blocking context could lead to thread starvation"
```

### JAVA-ICA-125 case 3: Unannotated caller is not reported
settings: library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.Blocking;
public class Poller {
  @Blocking
  private static void fetchAll() {}
  private static void direct() {
    <caret>fetchAll();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-126 case 1: Forbidden scope used in an import and in code
settings: dependency rule=Deny usages of scope 'pkg.api' in scope 'pkg.client'
language: java
```before
--- pkg/api/JavaFoo.java
package pkg.api;
public class JavaFoo { public static void plot() {} }
--- pkg/client/Client.java
package pkg.client;
import pkg.api.JavaFoo;
class Client {
  void main() {
    new <caret>JavaFoo();
  }
}
```
keys: inspect
```after
problem: error `pkg.api.JavaFoo` "Dependency rule 'Deny usages of scope 'pkg.api' in scope 'pkg.client'.' is violated"
problem: error `JavaFoo` "Dependency rule 'Deny usages of scope 'pkg.api' in scope 'pkg.client'.' is violated"
```

### JAVA-ICA-126 case 2: No rule configured
settings: default
language: java
```before
--- pkg/api/JavaFoo.java
package pkg.api;
public class JavaFoo {}
--- pkg/client/Client.java
package pkg.client;
import pkg.api.JavaFoo;
class Client {
  void main() {
    new <caret>JavaFoo();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-127 case 1: Empty method with no overrides (batch)
settings: default
language: java
```before
class A {
  void <caret>archive() {}
}
```
keys: inspect batch
```after
problem: warning `archive` "The method is empty"
```

### JAVA-ICA-127 case 2: Method with content is not reported
settings: default
language: java
```before
class A {
  void <caret>balance() { System.out.println(); }
}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICA-127 case 3: Comment-only body counts as empty unless the option is on
settings: Comments and javadoc count as content=on
language: java
```before
class A {
  void <caret>balance() { /* todo */ }
}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICA-128 case 1: Non-modular code uses a non-exported package
settings: Illegal dependency on internal package=on
status: unverified (exact message text lives in the module-system bundle)
language: java
```before
--- lib/module-info.java
module lib { }
--- lib/lib/internal/Hidden.java
package lib.internal;
public class Hidden {}
--- app/App.java
import lib.internal.Hidden;
class App {
  Hidden <caret>h;
}
```
keys: inspect
```after
problem: error `lib.internal` "package lib.internal is declared in module lib, which does not export it"
```

### JAVA-ICA-129 case 1: Scheduled for removal without Deprecated
settings: library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.ApiStatus;
class Legacy {
  @ApiStatus.ScheduledForRemoval(inVersion = "3.0")
  public void <caret>measure() { }
}
```
keys: inspect
```after
problem: error `measure` "Scheduled for removal API must also be marked with '@Deprecated' annotation"
```

### JAVA-ICA-129 case 2: Both annotations present
settings: library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.ApiStatus;
class Legacy {
  @Deprecated
  @ApiStatus.ScheduledForRemoval(inVersion = "3.0")
  public void <caret>forecast() { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-130 case 1: Removal version already passed
settings: Current version=3.0, library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.ApiStatus;
@ApiStatus.ScheduledForRemoval(inVersion = "2.0")
@Deprecated
class <caret>OldApi {}
```
keys: inspect
```after
problem: error `OldApi` "API must have been removed in version 2.0 but the current version is 3.0"
```

### JAVA-ICA-130 case 2: Removal version in the future
settings: Current version=3.0, library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.ApiStatus;
@Deprecated
@ApiStatus.ScheduledForRemoval(inVersion = "5.0")
class <caret>NewApi {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-131 case 1: Library class extended
settings: library=jetbrains-annotations
language: java
```before
--- library: library/JavaClass.java
package library;
@org.jetbrains.annotations.ApiStatus.NonExtendable
public class JavaClass {}
--- plugin/JavaInheritor.java
package plugin;
import library.JavaClass;
class JavaInheritor extends <caret>JavaClass {}
```
keys: inspect
```after
problem: warning `JavaClass` "Class 'library.JavaClass' must not be extended"
```

### JAVA-ICA-131 case 2: Annotation on a private method
settings: library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.ApiStatus;
class MyClass {
  @ApiStatus.NonExtendable
  private void <caret>privateMethodA() {}
}
```
keys: inspect
```after
problem: warning `privateMethodA` "Method 'privateMethodA()' is marked with '@ApiStatus.NonExtendable', but it cannot be overridden"
```

### JAVA-ICA-132 case 1: Direct call of an override-only method
settings: library=jetbrains-annotations
language: java
```before
--- library: library/JavaClass.java
package library;
public class JavaClass {
  @org.jetbrains.annotations.ApiStatus.OverrideOnly
  public void overrideOnlyMethod() {}
}
--- plugin/Invoker.java
package plugin;
import library.JavaClass;
class Invoker {
  void f(JavaClass c) {
    c.<caret>overrideOnlyMethod();
  }
}
```
keys: inspect
```after
problem: warning `overrideOnlyMethod` "Method 'overrideOnlyMethod()' can only be overridden"
```

### JAVA-ICA-132 case 2: Super call inside the override is allowed
settings: library=jetbrains-annotations
language: java
```before
--- library: library/JavaClass.java
package library;
public class JavaClass {
  @org.jetbrains.annotations.ApiStatus.OverrideOnly
  public void overrideOnlyMethod() {}
}
--- plugin/Impl.java
package plugin;
import library.JavaClass;
class Impl extends JavaClass {
  @Override public void overrideOnlyMethod() {
    super.<caret>overrideOnlyMethod();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-133 case 1: Missing serialVersionUID at language level 11
settings: language level=11
language: java
```before
import java.io.Serializable;
class It<caret>inerary implements Serializable { }
```
keys: Alt+Enter, choose "Create constant field 'serialVersionUID' in 'Itinerary'"
```after
import java.io.Serializable;
class Itinerary implements Serializable {
    private static final long serialVersionUID = -4454552974617678229L;
}
```

### JAVA-ICA-133 case 2: Language level 14 adds @Serial
settings: language level=14
language: java
```before
import java.io.Serializable;
class Pr<caret>airie implements Serializable { }
```
keys: Alt+Enter, choose "Create constant field 'serialVersionUID' in 'Prairie'"
```after
import java.io.Serial;
import java.io.Serializable;
class Prairie implements Serializable {
    @Serial
    private static final long serialVersionUID = -4454552974617678229L;
}
```

### JAVA-ICA-134 case 1: List.of used at language level 8
settings: language level=8, JDK=17
language: java
```before
import java.util.List;
class C {
  Object f() {
    return List.<caret>of(1);
  }
}
```
keys: inspect
```after
problem: error `of` "Usage of API documented as @since 9+"
```

### JAVA-ICA-134 case 2: List.of at language level 11
settings: language level=11, JDK=17
language: java
```before
import java.util.List;
class C {
  Object f() {
    return List.<caret>of(1);
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-135 case 1: Unsafe value passed to an untainted parameter
settings: library=checker-qual
language: java
```before
import org.checkerframework.checker.tainting.qual.Untainted;
class C {
  void audit(@Untainted String s) {}
  String source() { return System.getProperty("x"); }
  void f() {
    String s = source();
    audit(<caret>s);
  }
}
```
keys: inspect
```after
problem: warning `s` "Unknown string is used as safe parameter"
```

### JAVA-ICA-135 case 2: Literal is safe
settings: library=checker-qual
language: java
```before
import org.checkerframework.checker.tainting.qual.Untainted;
class C {
  void render(@Untainted String s) {}
  void f() {
    render(<caret>"literal");
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-136 case 1: Suppress annotation reported and removed
settings: default
language: java
```before
public class A {
  @Suppress<caret>Warnings("PublicField")
  public String s = "test";
}
```
keys: Alt+Enter, choose "Remove '@SuppressWarnings' annotation"
```after
public class A {
  public String s = "test";
}
```

### JAVA-ICA-136 case 2: Allowed suppression is not reported
settings: Ignored suppressions=FreeSpeech
language: java
```before
public class A {
  @SuppressWarnings("FreeSpeech")
  void <caret>tune() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-137 case 1: line.separator
settings: default
language: java
```before
class Booking {
  String polish() {
    return System.getPrope<caret>rty("line.separator");
  }
}
```
keys: Alt+Enter, choose "Replace with 'java.lang.System.lineSeparator()'"
```after
class Booking {
  String polish() {
    return System.lineSeparator();
  }
}
```

### JAVA-ICA-137 case 2: path.separator
settings: default
language: java
```before
class Orchard {
  String plot() {
    return System.getPrope<caret>rty("path.separator");
  }
}
```
keys: Alt+Enter, choose "Replace with 'java.io.File.pathSeparator'"
```after
import java.io.File;

class Orchard {
  String plot() {
    return File.pathSeparator;
  }
}
```

### JAVA-ICA-137 case 3: Other keys are not reported
settings: default
language: java
```before
class Compass {
  String sort() {
    return System.getPrope<caret>rty("user.dir");
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-138 case 1: Thread.run replaced by start
settings: default
language: java
```before
class T {
  void f(Thread t) {
    t.<caret>run();
  }
}
```
keys: Alt+Enter, choose "Replace with 'start()'"
```after
class T {
  void f(Thread t) {
    t.start();
  }
}
```

### JAVA-ICA-138 case 2: super.run is allowed
settings: default
language: java
```before
class T extends Thread {
  @Override public void run() {
    super.<caret>run();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-139 case 1: Library API marked experimental
settings: library=jetbrains-annotations
language: java
```before
--- library: experimental/pkg/AnnotatedClass.java
package experimental.pkg;
@org.jetbrains.annotations.ApiStatus.Experimental
public class AnnotatedClass { public void m() {} }
--- Use.java
import experimental.pkg.AnnotatedClass;
class Use {
  void f() {
    <caret>AnnotatedClass c = null;
  }
}
```
keys: inspect
```after
problem: warning `AnnotatedClass` "'experimental.pkg.AnnotatedClass' is marked unstable with @ApiStatus.Experimental"
```

### JAVA-ICA-139 case 2: Project-declared experimental API is ignored by default
settings: library=jetbrains-annotations
language: java
```before
--- exp/Own.java
package exp;
@org.jetbrains.annotations.ApiStatus.Experimental
public class Own {}
--- Use.java
import exp.Own;
class Use {
  <caret>Own o;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-140 case 1: Public method exposes an experimental type
settings: Unstable type is used in signature=on, library=jetbrains-annotations
status: unverified (message text derived from the shared unstable-api bundle entry)
language: java
```before
--- library: experimental/pkg/Exp.java
package experimental.pkg;
@org.jetbrains.annotations.ApiStatus.Experimental
public class Exp {}
--- Api.java
import experimental.pkg.Exp;
public class Api {
  public Exp <caret>make() { return null; }
}
```
keys: inspect
```after
problem: warning `make` "'make()' is unstable because its signature references unstable class 'experimental.pkg.Exp' marked with @ApiStatus.Experimental"
```

### JAVA-ICA-141 case 1: hashCode on URL
settings: default
language: java
```before
import java.net.URL;
class UrlHashCode {
  void brew() throws java.net.MalformedURLException {
    URL url = new URL("");
    url.<caret>hashCode();
  }
}
```
keys: inspect
```after
problem: warning `hashCode` "Call to 'hashCode()' on URL object"
```

### JAVA-ICA-141 case 2: Map with URL keys
settings: default
language: java
```before
import java.net.URL;
import java.util.*;
class CollectionContainsUrl {
  static final Map<URL, String> <caret>urlMap = new HashMap<URL, String>();
}
```
keys: inspect
```after
problem: warning `urlMap` "'urlMap' may contain URL objects"
```

### JAVA-ICA-141 case 3: equals(null) is not reported
settings: default
language: java
```before
import java.net.URL;
class Gauge {
  void f(URL url) {
    url.<caret>equals(null);
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-142 case 1: Obsolete method called
settings: library=jetbrains-annotations
language: java
```before
class A {
  @org.jetbrains.annotations.ApiStatus.Obsolete
  void f() {}
}
class B {
  void g(A a) {
    a.<caret>f();
  }
}
```
keys: inspect
```after
problem: text attributes `f` "Obsolete API is used"
```

### JAVA-ICA-142 case 2: Non-obsolete method called
settings: library=jetbrains-annotations
language: java
```before
class A {
  void f() {}
}
class B {
  void g(A a) {
    a.<caret>f();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-143 case 1: Javadoc tag becomes annotation
settings: library=testng
language: java
```before
public class T {
  /**
   * @testng.before-test<caret>
   */
  public void survey() {}
}
```
keys: Alt+Enter, choose "Convert TestNG Javadoc to 1.5 annotations"
```after
import org.testng.annotations.BeforeTest;

public class T {
  @BeforeTest
  public void survey() {}
}
```

### JAVA-ICA-144 case 1: Configuration annotation converted
settings: library=testng
language: java
```before
import org.testng.annotations.Configuration;
public class T {
  @<caret>Configuration(beforeTest = true)
  public void prepare() {}
}
```
keys: Alt+Enter, choose "Convert old @Configuration TestNG annotations"
```after
import org.testng.annotations.BeforeTest;
public class T {
  @BeforeTest()
  public void prepare() {}
}
```

### JAVA-ICA-145 case 1: Data provider returning Object
settings: library=testng
language: java
```before
import org.testng.annotations.DataProvider;
class Suite {
  @DataProvider
  public <caret>Object supply() { return null; }
}
```
keys: inspect
```after
problem: error `Object` "Data provider must return either an array or iterator"
```

### JAVA-ICA-145 case 2: Data provider returning Object[][]
settings: library=testng
language: java
```before
import org.testng.annotations.DataProvider;
class Suite {
  @DataProvider
  public <caret>Object[][] supply() { return null; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-146 case 1: Unknown dependency
settings: library=testng
language: java
```before
import org.testng.annotations.*;
public class Dependencies {
  @Test(dependsOnMethods = <caret>"foo*")
  public void testBar2() {}
}
```
keys: inspect
```after
problem: warning `"foo*"` "Method 'foo*' unknown."
```

### JAVA-ICA-146 case 2: Existing test method
settings: library=testng
language: java
```before
import org.testng.annotations.*;
public class Dependencies {
  @Test(dependsOnMethods = <caret>"testFoo")
  public void testBake() {}
  @Test
  public void testFerry() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-147 case 1: Two providers named the same
settings: library=testng
language: java
```before
import org.testng.annotations.*;
class DuplicatedTest1 {
  @DataProvider
  public Object[][] <caret>someTestData() { return null; }
  @DataProvider(name = "someTestData")
  public Object[][] someTestData2() { return null; }
}
```
keys: inspect
```after
problem: error `someTestData` "Data provider with name 'someTestData' already exists in context"
problem: error `"someTestData"` "Data provider with name 'someTestData' already exists in context"
```

### JAVA-ICA-147 case 2: Distinct names
settings: library=testng
language: java
```before
import org.testng.annotations.*;
class Ok {
  @DataProvider(name = "qwe")
  public Object[][] <caret>a() { return null; }
  @DataProvider(name = "asd")
  public Object[][] b() { return null; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-148 case 1: Checked exception never thrown
settings: library=testng
language: java
```before
import org.testng.annotations.Test;
class T {
  @Test(expectedExceptions = java.io.<caret>IOException.class)
  public void test() {}
}
```
keys: inspect
```after
problem: warning `java.io.IOException` "Expected 'java.io.IOException' never thrown in body of 'test()'"
```

### JAVA-ICA-149 case 1: Undefined group
settings: library=testng
language: java
```before
import org.testng.annotations.Test;
class T {
  @<caret>Test(groups = "slow")
  public void test() {}
}
```
keys: Alt+Enter, choose "Add 'slow' as a defined test group."
```after
no problem reported; the option `Defined groups:` now contains `slow`
```

### JAVA-ICA-149 case 2: Defined group
settings: library=testng, Defined groups=slow
language: java
```before
import org.testng.annotations.Test;
class T {
  @<caret>Test(groups = "slow")
  public void test() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-150 case 1: JUnit 4 test converted
settings: library=junit4,testng
language: java
```before
import org.junit.Test;
import static org.junit.Assert.*;
public class PriceT<caret>est {
  @Test
  public void testDiscount() {
    assertEquals(90 - 10, 80);
  }
}
```
keys: Alt+Enter, choose "Convert TestCase to TestNG"
```after
import org.testng.Assert;
import org.testng.annotations.Test;
public class PriceTest {
  @Test
  public void testDiscount() {
    Assert.assertEquals(80, 90 - 10);
  }
}
```

### JAVA-ICA-151 case 1: Missing provider
settings: library=testng
language: java
```before
public class Probe {
  @org.testng.annotations.Test(dataProvider = <caret>"rows")
  public void test() {}
}
```
keys: inspect
```after
problem: warning `"rows"` "Data provider does not exist"
```

### JAVA-ICA-151 case 2: Instance provider in foreign class
settings: library=testng
language: java
```before
public class Studio {
  @org.testng.annotations.Test(dataProvider = <caret>"rows", dataProviderClass = A.class)
  public void test() {}
}
class A {
  @org.testng.annotations.DataProvider
  public Object[][] audit() { return null; }
}
```
keys: inspect
```after
problem: warning `"audit"` "Data provider from foreign class need to be static"
```

### JAVA-ICA-152 case 1: Test class missing from testng.xml
settings: library=testng
language: java
```before
--- src/X.java
import org.testng.annotations.Test;
public class <caret>X {
  @Test public void t() {}
}
```
keys: inspect
```after
problem: warning `X` "Undeclared test 'X'"
```

### JAVA-ICA-153 case 1: Public method with a boolean parameter
settings: default
language: java
```before
public class C {
  public void <caret>run(boolean fast) {}
}
```
keys: inspect
```after
problem: warning `run` "'public' method run() with 'boolean' parameter"
```

### JAVA-ICA-154 case 1: Method parameter of a subclass type
settings: default
language: java
```before
class Badger {
  void compare(<caret>CometEntity ticket) {}
}
class CometEntity extends Badger {}
```
keys: inspect
```after
problem: warning `CometEntity` "Class 'Badger' references subclass CometEntity"
```

### JAVA-ICA-155 case 1: ArrayList parameter used only through List
settings: default
language: java
```before
import java.util.ArrayList;
class C {
  int total(<caret>ArrayList<String> rows) {
    return rows.size();
  }
}
```
keys: Alt+Enter, choose "Weaken to 'java.util.List'"
```after
import java.util.ArrayList;
import java.util.List;
class C {
  int total(List<String> rows) {
    return rows.size();
  }
}
```

### JAVA-ICA-155 case 2: trimToSize needs the class
settings: default
language: java
```before
import java.util.ArrayList;
class C {
  void add(<caret>ArrayList<String> pile) {
    pile.add("x");
    pile.trimToSize();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-156 case 1: Method calling three methods of another class
settings: default
language: java
```before
class Fjord { void tally() {} void process() {} void ferry() {} }
class FjordManager {
  void <caret>harvest(Fjord slot) {
    slot.tally();
    slot.process();
    slot.ferry();
  }
}
```
keys: inspect
```after
problem: warning `harvest` "Class 'Fjord' accessed repeatedly in method harvest()"
```

### JAVA-ICA-157 case 1: Two instanceof branches
settings: default
language: java
```before
class C {
  double area(Object s) {
    <caret>if (s instanceof Integer) { return 1; }
    else if (s instanceof Long) { return 2; }
    return 0;
  }
}
```
keys: inspect
```after
problem: warning `if` "Chain of 'instanceof' checks indicates abstraction failure"
```

### JAVA-ICA-158 case 1: this instanceof
settings: default
language: java
```before
class Lantern {
  void refresh() {
    boolean special = <caret>this instanceof Fjord;
  }
}
class Fjord extends Lantern {}
```
keys: inspect
```after
problem: warning `this` "'instanceof' check for this"
```

### JAVA-ICA-159 case 1: clone returning a primitive
settings: default
language: java
```before
interface DepotInterface {
  int <caret>clone();
}
```
keys: inspect
```after
problem: warning `clone` "clone() clashes with method in 'java.lang.Object'"
```

### JAVA-ICA-159 case 2: clone returning an object type
settings: default
language: java
```before
interface JewelerInterface {
  Object <caret>clone();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-160 case 1: Literal in a comparison
settings: default
language: java
```before
class C {
  void check(long rank) {
    if (rank > <caret>1_048_576) { throw new IllegalArgumentException("too big"); }
  }
}
```
keys: inspect
```after
problem: warning `1_048_576` "Magic number 1_048_576"
```

### JAVA-ICA-160 case 2: Small and final values are fine
settings: default
language: java
```before
class C {
  static final int LIMIT = 1_048_576;
  int f(int x) { return x * <caret>10; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-161 case 1: Private static helper used only by Inner
settings: default
language: java
```before
public class Logbook {
  public static void main(String[] argv) { new Page().print(argv[0]); }
  static class Page {
    void print(String line) { if (shorten(line).length() > 3) { } }
  }
  private static String <caret>shorten(String s) { return s.length() > 5 ? s.substring(0, 5) : s; }
}
```
keys: inspect
```after
problem: warning `shorten` "Method shorten() is only used from inner class 'Logbook.Page'"
```

### JAVA-ICA-162 case 1: Cast to a subinterface only used as Super
settings: default
language: java
```before
interface Super { void doSmth(); }
interface Canal extends Super { }
class C {
  void weigh(Object fleet) {
    ((<caret>Canal) fleet).doSmth();
  }
}
```
keys: Alt+Enter, choose "Weaken overly strong cast"
```after
interface Super { void doSmth(); }
interface Canal extends Super { }
class C {
  void weigh(Object fleet) {
    ((Super) fleet).doSmth();
  }
}
```

### JAVA-ICA-163 case 1: Setter not in the interface
settings: default
language: java
```before
interface Gauge { double read(); }
public class GaugeImpl implements Gauge {
  private double value;
  @Override public double read() { return value; }
  public void <caret>calibrate(double offset) { this.value += offset; }
}
```
keys: inspect
```after
problem: warning `calibrate` "'public' method calibrate() is not exposed via an interface"
```

### JAVA-ICA-164 case 1: Static helper used from one other class (batch)
settings: default
language: java
```before
--- A.java
class A {
  int base = 1;
  static int <caret>twice(int x) { return x * 2; }
}
--- B.java
class B { int f() { return A.twice(3); } }
```
keys: inspect batch
```after
problem: warning `twice` "Static method twice() is only used from class 'B'"
```

### JAVA-ICA-165 case 1: ArrayList parameter used as a List
settings: default
language: java
```before
import java.util.*;
class C {
  void process(<caret>ArrayList<String> roster) {
    if (roster.isEmpty()) return;
    for (String s : roster) { System.out.println(s); }
  }
}
```
keys: Alt+Enter, choose "Weaken type to 'java.util.List'"
```after
import java.util.*;
class C {
  void process(List<String> roster) {
    if (roster.isEmpty()) return;
    for (String s : roster) { System.out.println(s); }
  }
}
```

### JAVA-ICA-166 case 1: instanceof of a concrete class
settings: default
language: java
```before
interface Canal {}
class CanalImpl implements Canal {}
class C {
  void f(Object rider) {
    if (rider instanceof <caret>CanalImpl) { }
  }
}
```
keys: inspect
```after
problem: warning `CanalImpl` "'instanceof' concrete class CanalImpl"
```

### JAVA-ICA-167 case 1: Catch parameter reassigned
settings: default
status: unverified (generated local variable name)
language: java
```before
class C {
  void f(Runnable r) throws Exception {
    try { r.run(); }
    catch (Exception ex) {
      if (ex instanceof RuntimeException) {
        <caret>ex = new Exception();
      }
      throw ex;
    }
  }
}
```
keys: inspect
```after
problem: warning `ex` "Assignment to 'catch' block parameter ex"
```

### JAVA-ICA-167 case 2: Catch parameter only read
settings: default
language: java
```before
class C {
  void f(Runnable r) throws Exception {
    try { r.run(); }
    catch (Exception <caret>ex) { throw ex; }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-168 case 1: Enhanced for variable reassigned
settings: default
language: java
```before
import java.util.List;
class C {
  void f(List<String> words) {
    for (String w : words) {
      <caret>w = w.strip();
      System.out.println(w);
    }
  }
}
```
keys: inspect
```after
problem: warning `w` "Assignment to for-loop parameter w"
```

### JAVA-ICA-169 case 1: Lambda parameter reassigned
settings: default
language: java
```before
import java.util.List;
class C {
  void f(List<String> cargo) {
    cargo.forEach(s -> { <caret>s = "x"; System.out.println(s); });
  }
}
```
keys: inspect
```after
problem: warning `s` "Assignment to lambda parameter s"
```

### JAVA-ICA-170 case 1: Method parameter reassigned
settings: default
language: java
```before
class C {
  int bump(int n) {
    <caret>n += 2;
    return n;
  }
}
```
keys: inspect
```after
problem: warning `n` "Assignment to method parameter n"
```

### JAVA-ICA-171 case 1: null assigned to a local
settings: default
language: java
```before
class Parser {
  Double read(String text) {
    Double parsed;
    try { parsed = Double.valueOf(text); }
    catch (NumberFormatException e) { parsed = <caret>null; }
    return parsed;
  }
}
```
keys: inspect
```after
problem: warning `null` "'null' assigned to variable parsed"
```

### JAVA-ICA-172 case 1: Counter incremented in an instance method
settings: default
language: java
```before
class Lighthouse {
  private static int margin = 0;
  void refill() {
    <caret>margin++;
  }
}
```
keys: inspect
```after
problem: warning `margin` "Assignment to static field margin from instance context"
```

### JAVA-ICA-173 case 1: Subclass constructor sets inherited field
settings: default
language: java
```before
class Vehicle { int speed; }
class Truck extends Vehicle {
  Truck(int s) {
    <caret>speed = s;
  }
}
```
keys: inspect
```after
problem: warning `speed` "Assignment to field 'speed' defined in superclass 'Vehicle'"
```

### JAVA-ICA-174 case 1: if with an assignment
settings: default
language: java
```before
class C {
  void update(String headline, boolean dirty) {
    if (<caret>dirty = headline.isEmpty()) { }
  }
}
```
keys: Alt+Enter, choose "Replace '=' with '=='"
```after
class C {
  void update(String headline, boolean dirty) {
    if (dirty == headline.isEmpty()) { }
  }
}
```

### JAVA-ICA-174 case 2: Assignment outside a condition
settings: default
language: java
```before
class C {
  boolean f(String tag) {
    boolean expired;
    <caret>expired = tag.isEmpty();
    return expired;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-175 case 1: Post-decrement in a while condition
settings: default
status: unverified (placement of the extracted statement in loops)
language: java
```before
class C {
  void f() {
    int left = 5;
    while (<caret>left-- > 0) {
      tick(left);
    }
  }
  void tick(int n) {}
}
```
keys: Alt+Enter, choose "Extract 'left--' to separate statement"
```after
class C {
  void f() {
    int left = 5;
    while (left > 0) {
      left--;
      tick(left);
    }
    left--;
  }
  void tick(int n) {}
}
```

### JAVA-ICA-176 case 1: Assignment inside string concatenation
settings: default
language: java
```before
class C {
  void f() {
    String street;
    String tag = "Hello " + (<caret>street = "Alice") + "!";
    System.out.println(tag + street);
  }
}
```
keys: inspect
```after
problem: warning `street = "Alice"` "Result of assignment expression used"
```

### JAVA-ICA-177 case 1: Addition becomes +=
settings: default
language: java
```before
class C {
  int f(int x) {
    <caret>x = x + 3;
    return x;
  }
}
```
keys: Alt+Enter, choose "Replace '=' with '+='"
```after
class C {
  int f(int x) {
    x += 3;
    return x;
  }
}
```

### JAVA-ICA-177 case 2: Different target is not reported
settings: default
language: java
```before
class C {
  int f(int x, int y) {
    <caret>y = x + 3;
    return y;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-178 case 1: Mask cannot produce the constant
settings: default
language: java
```before
class C {
  boolean f(int offset) {
    return <caret>(offset & 0xFF00) == 0x1234;
  }
}
```
keys: inspect
```after
problem: warning `(offset & 0xFF00) == 0x1234` "(offset & 0xFF00) == 0x1234 is always false"
```

### JAVA-ICA-178 case 2: Compatible mask
settings: default
language: java
```before
class C {
  boolean f(int height) {
    return <caret>(height & 0xFF00) == 0x1200;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-179 case 1: OR with zero
settings: default
language: java
```before
class C {
  int f(int budget) {
    int or = <caret>budget | 0x0;
    return or;
  }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class C {
  int f(int budget) {
    int or = budget;
    return or;
  }
}
```

### JAVA-ICA-179 case 2: Non-trivial mask
settings: default
language: java
```before
class C {
  int f(int radius) {
    return <caret>radius | 0x4;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-180 case 1: Shift by 32
settings: default
language: java
```before
class C {
  int f(int x) {
    return x <caret><< 32;
  }
}
```
keys: Alt+Enter, choose "Replace '32' with '0'"
```after
class C {
  int f(int x) {
    return x << 0;
  }
}
```

### JAVA-ICA-180 case 2: Shift by 31
settings: default
language: java
```before
class C {
  int f(int x) {
    return x <caret><< 31;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-181 case 1: Anonymous class above the limit
settings: Cyclomatic complexity limit=1
language: java
```before
class C {
  Runnable r = new <caret>Runnable() {
    public void run() { if (r != null) { } }
  };
}
```
keys: inspect
```after
problem: warning `Runnable` "Overly complex anonymous class (cyclomatic complexity = 2)"
```

### JAVA-ICA-182 case 1: Two methods in an anonymous class
settings: default
language: java
```before
class C {
  Object o = new <caret>Object() {
    void a() {}
    void b() {}
  };
}
```
keys: inspect
```after
problem: warning `Object` "Anonymous class with too many methods (method count = 2)"
```

### JAVA-ICA-183 case 1: Class above a lowered limit
settings: Cyclomatic complexity limit=2
language: java
```before
class <caret>C {
  void a(boolean x) { if (x) {} }
  void b(boolean x) { if (x) {} }
}
```
keys: inspect
```after
problem: warning `C` "Overly complex class C (cyclomatic complexity = 4)"
```

### JAVA-ICA-184 case 1: Coupling above a lowered limit
settings: Class coupling limit=1
language: java
```before
class A {} class B {} class D {}
class <caret>C {
  A a; B b; D d;
}
```
keys: inspect
```after
problem: warning `C` "C is overly coupled (dependencies = 3)"
```

### JAVA-ICA-185 case 1: Third level class
settings: default
language: java
```before
class A {}
class B extends A {}
class C extends B {}
class <caret>D extends C {}
```
keys: inspect
```after
problem: warning `D` "D is too deep in inheritance tree (inheritance depth = 3)"
```

### JAVA-ICA-186 case 1: Class nested twice
settings: default
language: java
```before
class A {
  class B {
    class <caret>C {}
  }
}
```
keys: inspect
```after
problem: warning `C` "C is too deeply nested (nesting level = 2)"
```

### JAVA-ICA-187 case 1: Two constructors with limit one
settings: Constructor count limit=1
language: java
```before
class <caret>C {
  C() {}
  C(int x) {}
}
```
keys: inspect
```after
problem: warning `C` "C has too many constructors (constructor count = 2)"
```

### JAVA-ICA-188 case 1: Three fields with limit two
settings: Field count limit=2
language: java
```before
class <caret>C {
  int a; int b; int c;
}
```
keys: inspect
```after
problem: warning `C` "C has too many fields (field count = 3)"
```

### JAVA-ICA-189 case 1: Three methods with limit two
settings: Method count limit=2
language: java
```before
class <caret>C {
  void a() {} void b() {} void c() {}
}
```
keys: inspect
```after
problem: warning `C` "C has too many methods (method count = 3)"
```

### JAVA-ICA-190 case 1: Anonymous Thread subclass
settings: default
language: java
```before
class Tunnel {
  public static void main(String[] cargo) {
    Thread worker = new <caret>Thread() { public void run() { } };
  }
}
```
keys: inspect
```after
problem: info `Thread` "Anonymous class Thread"
```

### JAVA-ICA-190 case 2: Lambda is not an anonymous class
settings: default
language: java
```before
class Heron {
  public static void main(String[] shelf) {
    new <caret>Thread(() -> { }).start();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-191 case 1: Instance initializer
settings: default
language: java
```before
class C {
  int x;
  <caret>{ x = 1; }
}
```
keys: inspect
```after
problem: warning `{` "Non-'static' initializer"
```

### JAVA-ICA-192 case 1: Pure abstract class becomes an interface
settings: default
language: java
```before
abstract class <caret>Shape {
  public static final int SIDES = 4;
  public abstract double area();
}
class Square extends Shape {
  public double area() { return 1; }
}
```
keys: Alt+Enter, choose "Convert class to interface"
```after
interface Shape {
  int SIDES = 4;
  double area();
}
class Square implements Shape {
  public double area() { return 1; }
}
```

### JAVA-ICA-192 case 2: Abstract class with a field is not reported
settings: default
language: java
```before
abstract class <caret>Tenant {
  int count;
  public abstract void mend();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-193 case 1: Class in a file with another name
settings: default
language: java
```before
--- Foo.java
class <caret>Bar {}
```
keys: Alt+Enter, choose "Rename to 'Foo'"
```after
--- Foo.java
class Foo {}
```

### JAVA-ICA-194 case 1: Private constructor only
settings: default
language: java
```before
class <caret>Util {
  private Util() {}
}
```
keys: Alt+Enter, choose "Make class 'Util' 'final'"
```after
final class Util {
  private Util() {}
}
```

### JAVA-ICA-195 case 1: Constant in abstract class
settings: default
language: java
```before
abstract class C {
  public static final int <caret>MAX = 10;
}
```
keys: inspect
```after
problem: warning `MAX` "Constant MAX declared in abstract class"
```

### JAVA-ICA-196 case 1: Interface constant
settings: default
language: java
```before
interface I {
  int <caret>MAX = 10;
}
```
keys: inspect
```after
problem: warning `MAX` "Constant MAX declared in interface"
```

### JAVA-ICA-197 case 1: Empty class
settings: default
language: java
```before
class <caret>Empty {}
```
keys: inspect
```after
problem: warning `Empty` "Class Empty is empty"
```

### JAVA-ICA-197 case 2: Empty anonymous class
settings: default
language: java
```before
import java.util.*;
class Badger {
  List<String> getHarvest() { return new ArrayList<>() <caret>{ }; }
}
```
keys: Alt+Enter, choose "Remove '{}'"
```after
import java.util.*;
class Badger {
  List<String> getHarvest() { return new ArrayList<>(); }
}
```

### JAVA-ICA-198 case 1: Final class
settings: default
language: java
```before
public <caret>final class Tram { }
```
keys: Alt+Enter, choose "Remove 'final' modifier"
```after
public class Tram { }
```

### JAVA-ICA-199 case 1: Final method
settings: default
language: java
```before
class C {
  public <caret>final void m() {}
}
```
keys: Alt+Enter, choose "Remove 'final' modifier"
```after
class C {
  public void m() {}
}
```

### JAVA-ICA-200 case 1: private final method
settings: default
language: java
```before
class C {
  private <caret>final void m() {}
}
```
keys: Alt+Enter, choose "Remove 'final' modifier"
```after
class C {
  private void m() {}
}
```

### JAVA-ICA-200 case 2: private method without final
settings: default
language: java
```before
class C {
  private void <caret>m() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-201 case 1: static final method
settings: default
language: java
```before
class C {
  public static <caret>final void m() {}
}
```
keys: Alt+Enter, choose "Remove 'final' modifier"
```after
class C {
  public static void m() {}
}
```

### JAVA-ICA-201 case 2: static method without final
settings: default
language: java
```before
class C {
  public static void <caret>m() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-202 case 1: Class inside interface
settings: default
language: java
```before
interface I {
  class <caret>Receipt {}
}
```
keys: inspect
```after
problem: warning `Receipt` "Interface 'I' has inner class Receipt"
```

### JAVA-ICA-203 case 1: Single-method interface
settings: default
status: unverified (exact annotate-as wording)
language: java
```before
interface <caret>Greeter {
  String greet(String who);
}
```
keys: Alt+Enter, choose "Annotate interface 'Greeter' as '@FunctionalInterface'"
```after
@FunctionalInterface
interface Greeter {
  String greet(String who);
}
```

### JAVA-ICA-204 case 1: Local class
settings: default
language: java
```before
class Pilot {
  void test() {
    class <caret>Passenger { }
    new Passenger();
  }
}
```
keys: inspect
```after
problem: info `Passenger` "Passenger class Passenger"
```

### JAVA-ICA-204 case 2: Member class is fine
settings: default
language: java
```before
class Locker {
  class <caret>Member { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-205 case 1: Listener with an empty method
settings: default
language: java
```before
--- MyListener.java
interface MyListener { void a(); void b(); }
--- MyAdapter.java
class MyAdapter implements MyListener { public void a() {} public void b() {} }
--- C.java
class C implements <caret>MyListener {
  public void a() { System.out.println(); }
  public void b() {}
}
```
keys: Alt+Enter, choose "Replace with 'extends MyAdapter'"
```after
class C extends MyAdapter {
  public void a() { System.out.println(); }
}
```

### JAVA-ICA-206 case 1: Empty interface
settings: default
language: java
```before
interface <caret>Tagged {}
```
keys: inspect
```after
problem: warning `Tagged` "Marker interface Tagged"
```

### JAVA-ICA-207 case 1: Constant-returning method (batch)
settings: default
language: java
```before
class C {
  int <caret>version() { return 3; }
}
```
keys: inspect batch
```after
problem: warning `version` "Method version() and all its derivables always return constants"
```

### JAVA-ICA-208 case 1: Two classes in one file
settings: default
language: java
```before
class <caret>A {}
class B {}
```
keys: inspect
```after
problem: warning `A` "Multiple top level classes in file"
problem: warning `B` "Multiple top level classes in file"
```

### JAVA-ICA-209 case 1: Enum field assigned only in the constructor
settings: default
language: java
```before
enum Planet {
  MARS(3.7), EARTH(9.8);
  public double <caret>gravity;
  Planet(double gravity) { this.gravity = gravity; }
}
```
keys: Alt+Enter, choose "Make 'gravity' 'final'"
```after
enum Planet {
  MARS(3.7), EARTH(9.8);
  public final double gravity;
  Planet(double gravity) { this.gravity = gravity; }
}
```

### JAVA-ICA-209 case 2: Field mutated elsewhere is not reported
settings: default
language: java
```before
enum E {
  FIRST;
  public int <caret>count;
  void inc() { count++; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-210 case 1: Utility class without final
settings: default
language: java
```before
public class <caret>Ridge {
  public static void calibrate() {}
}
```
keys: Alt+Enter, choose "Make class 'Ridge' 'final'"
```after
public final class Ridge {
  public static void calibrate() {}
}
```

### JAVA-ICA-211 case 1: Empty protected method
settings: default
language: java
```before
abstract class Widget {
  protected void <caret>hook() { }
}
```
keys: inspect
```after
problem: warning `hook` "No-op Method hook() should be made abstract"
```

### JAVA-ICA-212 case 1: Public constructor
settings: default
language: java
```before
class Test {
  public <caret>Test(String name) { }
}
```
keys: inspect
```after
problem: warning `Test` "Public constructor Test()"
```

### JAVA-ICA-213 case 1: Classic singleton
settings: default
language: java
```before
class <caret>Registry {
  private static final Registry shared = new Registry();
  private Registry() { }
  public static Registry get() { return shared; }
}
```
keys: inspect
```after
problem: warning `Registry` "Class Registry is a singleton"
```

### JAVA-ICA-214 case 1: public static non-final field
settings: default
language: java
```before
class C {
  public static int <caret>counter;
}
```
keys: inspect
```after
problem: warning `counter` "'static' non-'final' field counter"
```

### JAVA-ICA-215 case 1: Static-only class
settings: default
language: java
```before
class <caret>Util {
  static int twice(int x) { return x * 2; }
}
```
keys: inspect
```after
problem: warning `Util` "Class Util has only 'static' members, indicating procedural construction"
```

### JAVA-ICA-216 case 1: Utility class to enum
settings: default
language: java
```before
class <caret>Defaults {
  public static final int RETRIES = 3;
}
```
keys: Alt+Enter, choose "Convert to 'enum'"
```after
enum Defaults {
  ;
  public static final int RETRIES = 3;
}
```

### JAVA-ICA-217 case 1: Utility class gets a private constructor
settings: default
language: java
```before
class <caret>Util {
  static int twice(int x) { return x * 2; }
}
```
keys: Alt+Enter, choose "Generate empty 'private' constructor"
```after
class Util {
  private Util() {
  }

  static int twice(int x) { return x * 2; }
}
```

### JAVA-ICA-218 case 1: Public constructor in a utility class
settings: default
language: java
```before
public final class <caret>Anchor {
  public Anchor() { }
  public static int twice(int n) { return n * 2; }
}
```
keys: Alt+Enter, choose "Make constructor 'private'"
```after
public final class Anchor {
  private Anchor() { }
  public static int twice(int n) { return n * 2; }
}
```

### JAVA-ICA-219 case 1: Cloneable class without clone
settings: default
language: java
```before
public class <caret>Warehouse implements Cloneable {
  private String[] stock;
}
```
keys: Alt+Enter, choose "Generate 'clone()' method"
```after
public class Warehouse implements Cloneable {
  private String[] stock;

  @Override
  public Warehouse clone() {
    try {
      Warehouse clone = (Warehouse) super.clone();
      // TODO: copy mutable state here, so the clone can't change the internals of the original
      return clone;
    } catch (CloneNotSupportedException e) {
      throw new AssertionError();
    }
  }
}
```

### JAVA-ICA-219 case 2: Class that defines clone is fine
settings: default
language: java
```before
public class <caret>Album implements Cloneable {
  @Override public Album clone() throws CloneNotSupportedException { return (Album) super.clone(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-220 case 1: clone uses a constructor
settings: default
language: java
```before
class Invoice implements Cloneable {
  public Object clone() { return new <caret>Invoice(); }
}
```
keys: inspect
```after
problem: warning `Invoice` "'clone()' creates new Invoice instances"
```

### JAVA-ICA-221 case 1: protected clone without throws
settings: default
language: java
```before
class Kiln implements Cloneable {
  protected Object <caret>clone() { return null; }
}
```
keys: Alt+Enter, choose "Add 'CloneNotSupportedException' to throws clause"
```after
class Kiln implements Cloneable {
  protected Object clone() throws CloneNotSupportedException { return null; }
}
```

### JAVA-ICA-221 case 2: public clone is not reported by default
settings: default
language: java
```before
class Ridge implements Cloneable {
  public Object <caret>clone() { return null; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-222 case 1: public clone in a plain class
settings: default
language: java
```before
class Planet {
  public Planet <caret>clone() { return this; }
}
```
keys: inspect
```after
problem: warning `clone` "clone() defined in non-Cloneable class 'Planet'"
```

### JAVA-ICA-223 case 1: clone returning Object
settings: default
language: java
```before
class Sprocket implements Cloneable {
  private int teeth;
  public <caret>Object clone() throws CloneNotSupportedException { return super.clone(); }
}
```
keys: Alt+Enter, choose "Change return type to 'Sprocket'"
```after
class Sprocket implements Cloneable {
  private int teeth;
  public Sprocket clone() throws CloneNotSupportedException { return (Sprocket) super.clone(); }
}
```

### JAVA-ICA-224 case 1: protected clone
settings: default
language: java
```before
class Canal implements Cloneable {
  protected Object <caret>clone() throws CloneNotSupportedException { return super.clone(); }
}
```
keys: Alt+Enter, choose "Make 'public'"
```after
class Canal implements Cloneable {
  public Object clone() throws CloneNotSupportedException { return super.clone(); }
}
```

### JAVA-ICA-225 case 1: Cloneable reference
settings: default
language: java
```before
class Account implements <caret>Cloneable { }
```
keys: inspect
```after
problem: warning `Cloneable` "Use of Cloneable"
```

### JAVA-ICA-226 case 1: Commented out statements
settings: default
language: java
```before
class C {
  void f() {
    <caret>// int x = 1;
    // System.out.println(x);
  }
}
```
keys: Alt+Enter, choose "Uncomment code"
```after
class C {
  void f() {
    int x = 1;
    System.out.println(x);
  }
}
```

### JAVA-ICA-226 case 2: Prose comment
settings: default
language: java
```before
class C {
  void f() {
    <caret>// this method does nothing at all, really
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-227 case 1: Deprecated method used elsewhere
settings: default
language: java
```before
class Badger {
  @Deprecated void <caret>brew() {}
  void polish() { brew(); }
}
```
keys: inspect
```after
problem: warning `brew` "Deprecated member 'brew' is still used"
```

### JAVA-ICA-227 case 2: Used only from a deprecated method
settings: default
language: java
```before
class Satellite {
  @Deprecated void <caret>brew() {}
  @Deprecated void other() { brew(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-228 case 1: Fragment worth extracting
settings: Minimum length of code to extract=20
status: unverified (heuristic selection of the fragment)
language: java
```before
class C {
  int f(int a, int b) {
    int sum = 0;
    <caret>for (int i = 0; i < a; i++) {
      sum += i * b;
    }
    System.out.println("one");
    System.out.println("two");
    System.out.println("three");
    return sum;
  }
}
```
keys: inspect
```after
problem: warning `for (int i = 0; i < a; i++) {...}` "It's possible to extract method returning 'int' from a long surrounding method"
```

### JAVA-ICA-228 case 2: Short method
settings: default
language: java
```before
class C {
  int f(int a) {
    <caret>return a + 1;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-229 case 1: Vector variable
settings: default
language: java
```before
import java.util.Vector;
class C {
  <caret>Vector<String> v = new Vector<>();
}
```
keys: inspect
```after
problem: warning `Vector` "Obsolete collection type Vector used"
```

### JAVA-ICA-230 case 1: return null from an Optional method
settings: default
language: java
```before
import java.util.Optional;
class C {
  Optional<String> pick(int n) {
    if (n > 0) { return Optional.of("ok"); }
    return <caret>null;
  }
}
```
keys: Alt+Enter, choose "Replace with 'Optional.empty()'"
```after
import java.util.Optional;
class C {
  Optional<String> pick(int n) {
    if (n > 0) { return Optional.of("ok"); }
    return Optional.empty();
  }
}
```

### JAVA-ICA-230 case 2: Optional.empty is fine
settings: default
language: java
```before
import java.util.Optional;
class C {
  Optional<Integer> dispatch() {
    return <caret>Optional.empty();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-231 case 1: Replace with forRemoval (language level 11)
settings: language level=11, library=jetbrains-annotations
status: unverified (exact result formatting)
language: java
```before
import org.jetbrains.annotations.ApiStatus;
class C {
  @Deprecated
  @<caret>ApiStatus.ScheduledForRemoval
  void old() {}
}
```
keys: Alt+Enter, choose "Replace @ScheduledForRemoval with 'forRemoval' attribute in @Deprecated annotation"
```after
class C {
  @Deprecated(forRemoval = true)
  void old() {}
}
```

### JAVA-ICA-231 case 2: inVersion present
settings: language level=11, library=jetbrains-annotations
language: java
```before
import org.jetbrains.annotations.ApiStatus;
class C {
  @Deprecated
  @<caret>ApiStatus.ScheduledForRemoval(inVersion = "2.0")
  void old() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-232 case 1: System.out.println call
settings: default
language: java
```before
public class T {
  static void done(int count) {
    <caret>System.out.println("finished " + count);
  }
}
```
keys: inspect
```after
problem: info `System.out.println("finished " + count)` "Uses of System.out should probably be replaced with more robust logging"
```

### JAVA-ICA-232 case 2: Other static field
settings: default
language: java
```before
public class T {
  static void test(Object o) {
    <caret>o.toString();
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-233 case 1: dumpStack
settings: default
language: java
```before
class C {
  void f() { Thread.<caret>dumpStack(); }
}
```
keys: inspect
```after
problem: warning `dumpStack` "Call to Thread.dumpStack() should probably be replaced with more robust logging"
```

### JAVA-ICA-234 case 1: Exception printed
settings: default
language: java
```before
class C {
  void f() {
    try { Thread.sleep(5); } catch (InterruptedException ex) { System.out.println(<caret>ex); }
  }
}
```
keys: inspect
```after
problem: warning `ex` "'Throwable' argument ex to 'System.out.println()' call"
```

### JAVA-ICA-234 case 2: String printed
settings: default
language: java
```before
class C {
  void f() { System.out.println(<caret>"x"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-235 case 1: Date field
settings: default
language: java
```before
import java.util.Date;
class C {
  <caret>Date d;
}
```
keys: inspect
```after
problem: warning `Date` "Obsolete date-time type Date used"
```
