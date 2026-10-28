# Cases for 09-inspections-catalog-10 (JAVA-ICC)

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-10.md`. Keys `inspect` list the expected highlights as `problem:` lines, `inspect batch` runs the whole-project inspection, and `no problem reported` means no highlight.

### JAVA-ICC-348 case 1: @Language on an int
settings: default; JetBrains annotations on classpath
status: unverified (exact highlighted range: the annotation or its name)
language: java
```before
import org.intellij.lang.annotations.Language;
class A {
  @<caret>Language("SQL") long count;
}
```
keys: inspect
```after
problem: error `@Language("SQL")` "Language injection is only applicable to strings or string templates"
```

### JAVA-ICC-348 case 2: @Language on a String
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Language;
class A {
  @<caret>Language("HTML") String s;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-349 case 1: JavaScript passed as XPath
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Language;
class A {
  @Language("JavaScript") String DEFAULT_TTL = "var x;";
  @Language("XPath") String HEADROOM = <caret>DEFAULT_TTL;
}
```
keys: inspect
```after
problem: warning `DEFAULT_TTL` "Language mismatch: Expected 'XPath', got 'JavaScript'"
```

### JAVA-ICC-349 case 2: same language
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Language;
class A {
  @Language("XPath") String A_PATH = "/a";
  @Language("XPath") String B_PATH = <caret>A_PATH;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-350 case 1: misspelled ID
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Language;
class A {
  @Language("<caret>JASON") String payload;
}
```
keys: inspect
```after
problem: error `"JASON"` "Unknown language 'JASON'"
```

### JAVA-ICC-350 case 2: known ID
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Language;
class A {
  @Language("<caret>HTML") String html;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-351 case 1: trace guard with debug call
settings: inspection LoggingConditionDisagreesWithLogLevelStatement enabled; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  Logger LOG;
  void f() {
    if (LOG.<caret>isTraceEnabled()) { LOG.debug("some log message"); }
  }
}
```
keys: Alt+Enter, choose "Change level of the condition"
```after
import org.slf4j.Logger;
class A {
  Logger LOG;
  void f() {
    if (LOG.isDebugEnabled()) { LOG.debug("some log message"); }
  }
}
```

### JAVA-ICC-351 case 2: matching levels
settings: inspection LoggingConditionDisagreesWithLogLevelStatement enabled; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  Logger LOG;
  void f() {
    if (LOG.<caret>isDebugEnabled()) { LOG.debug("some log message"); }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-352 case 1: unwrap the guard
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class Locker {
  Logger LOG;
  int weave(int i) { return i; }
  void test(Object dims) {
    <caret>if (LOG.isDebugEnabled()) {
      LOG.debug("some logging {}", weave(1));
    }
  }
}
```
keys: Alt+Enter, choose "Unwrap log guard condition"
```after
import org.slf4j.Logger;
class Locker {
  Logger LOG;
  int weave(int i) { return i; }
  void test(Object dims) {
    LOG.debug("some logging {}", weave(1));
  }
}
```

### JAVA-ICC-352 case 2: guard with other statements
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class Archer {
  Logger LOG;
  void test(Object prices) {
    <caret>if (LOG.isDebugEnabled()) {
      LOG.debug("a");
      System.out.println("b");
    }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-353 case 1: fewer arguments than placeholders
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  Logger logger;
  void f(String key) { logger.info(<caret>"{}: {}", key); }
}
```
keys: inspect
```after
problem: warning `"{}: {}"` "Fewer arguments provided (1) than placeholders specified (2)"
```

### JAVA-ICC-353 case 2: more arguments than placeholders
settings: default; SLF4J on classpath
status: unverified (highlighted range: the extra argument)
language: java
```before
import org.slf4j.Logger;
class A {
  Logger logger;
  void f(String key, String v) { logger.info(<caret>"{}", key, v); }
}
```
keys: inspect
```after
problem: warning `v` "More arguments provided (2) than placeholders specified (1)"
```

### JAVA-ICC-353 case 3: matching count
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  Logger logger;
  void f(String key, String v) { logger.info(<caret>"{}: {}", key, v); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-354 case 1: two calls with the same message
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  private static Logger log;
  static void save1(String text) { log.info(<caret>"Saved {}", text); }
  static void save2(int i) { log.info("Saved {}", i); }
}
```
keys: inspect
```after
problem: weak warning `"Saved {}"` "Similar log messages"
```

### JAVA-ICC-354 case 2: distinct messages
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  private static Logger log;
  static void save1(String text) { log.info(<caret>"Saved text {}", text); }
  static void save2(int i) { log.info("Stored number {}", i); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-355 case 1: concatenation to placeholders
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  Logger logger;
  void f(Object margin) { logger.<caret>info("variable1: " + margin); }
}
```
keys: Alt+Enter, choose "Replace with placeholders"
```after
import org.slf4j.Logger;
class A {
  Logger logger;
  void f(Object margin) { logger.info("variable1: {}", margin); }
}
```

### JAVA-ICC-355 case 2: concatenation of only strings and primitives
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  Logger logger;
  void f(int n) { logger.<caret>info("n: " + n); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-356 case 1: debug call with expensive argument
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class Crane {
  Logger LOG;
  int launch(int i) { return i; }
  void test(Object title) { LOG.<caret>debug("some logging {}", launch(1)); }
}
```
keys: Alt+Enter, choose "Surround with a logging condition"
```after
import org.slf4j.Logger;
class Crane {
  Logger LOG;
  int launch(int i) { return i; }
  void test(Object title) {
    if (LOG.isDebugEnabled()) {
      LOG.debug("some logging {}", launch(1));
    }
  }
}
```

### JAVA-ICC-356 case 2: constant message
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class Marble {
  Logger LOG;
  void test() { LOG.<caret>debug("constant message"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-357 case 1: manifest without trailing newline
settings: file MANIFEST.MF
status: unverified (caret placement)
language: plain
```before
Manifest-Version: 1.0<caret>
```
keys: Alt+Enter, choose "Add newline"
```after
Manifest-Version: 1.0
<caret>
```

### JAVA-ICC-357 case 2: file ends with a newline
settings: file MANIFEST.MF
language: plain
```before
Manifest-Version: 1.0
<caret>
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-358 case 1: misspelled header
settings: file MANIFEST.MF
language: plain
```before
Manifest-Version: 1.0
Bundle-Nmae<caret>: demo
```
keys: Alt+Enter, choose "Change to 'Bundle-Name'"
```after
Manifest-Version: 1.0
Bundle-Name: demo
```

### JAVA-ICC-358 case 2: known header
settings: file MANIFEST.MF
language: plain
```before
Manifest-Version: 1.0
Bundle-Name<caret>: demo
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-359 case 1: boolean empty(list)
settings: inspection BooleanMethodNameMustStartWithQuestion enabled
language: java
```before
import java.util.Collection;
class A {
  boolean <caret>full(Collection<?> items) { return items.size() > 3; }
}
```
keys: inspect
```after
problem: warning `full` "Boolean method name full does not start with question word"
```

### JAVA-ICC-359 case 2: isFull
settings: inspection BooleanMethodNameMustStartWithQuestion enabled
language: java
```before
import java.util.Collection;
class A {
  boolean <caret>isFull(Collection<?> items) { return items.size() > 3; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-360 case 1: Supplier and Callable overloads
settings: inspection LambdaUnfriendlyMethodOverload enabled
language: java
```before
import java.util.concurrent.Callable;
import java.util.function.Supplier;
interface Scheduler {
  void <caret>submit(Supplier<?> supplier);
  void submit(Callable<?> callable);
}
```
keys: inspect
```after
problem: warning `submit` "Lambda-unfriendly overload of method submit()"
```

### JAVA-ICC-360 case 2: incompatible functional shapes
settings: inspection LambdaUnfriendlyMethodOverload enabled
language: java
```before
import java.util.function.Consumer;
import java.util.function.Supplier;
interface Scheduler {
  void <caret>submit(Supplier<?> supplier);
  void submit(Consumer<?> consumer);
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-361 case 1: void added to a constructor
settings: default
language: java
```before
class Counter {
  int n;
  void <caret>Counter(int n) { this.n = n; }
}
```
keys: Alt+Enter, choose "Make method constructor"
```after
class Counter {
  int n;
  Counter(int n) { this.n = n; }
}
```

### JAVA-ICC-361 case 2: a real constructor
settings: default
language: java
```before
class Counter {
  int n;
  <caret>Counter(int n) { this.n = n; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-362 case 1: method named like parent
settings: inspection MethodNameSameAsParentName enabled
language: java
```before
class Engine {}
class Truck extends Engine {
  public Engine <caret>Engine() { return null; }
}
```
keys: inspect
```after
problem: warning `Engine` "Method name Engine is the same as its parent class name"
```

### JAVA-ICC-363 case 1: tostring vs toString
settings: inspection MisspelledMethodName enabled
status: unverified (toString here is declared in the same class; with only an inherited toString the report depends on the inherited-methods check)
language: java
```before
class A {
  public String <caret>tostring() { return ""; }
  public String toString() { return "x"; }
}
```
keys: inspect
```after
problem: warning `tostring` "Method name tostring and method name 'toString' differ only by case"
```

### JAVA-ICC-363 case 2: unrelated names
settings: inspection MisspelledMethodName enabled
language: java
```before
class A {
  public int <caret>first() { return 0; }
  public int second() { return 1; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-364 case 1: static method too short
settings: inspection NewMethodNamingConvention enabled (default: minimum length 4 for static methods)
status: unverified (the {0} text "Static method" of the naming-convention message)
language: java
```before
class A {
  public static int <caret>n() { return 0; }
}
```
keys: inspect
```after
problem: warning `n` "Static method name n is too short (1 < 4)"
```

### JAVA-ICC-364 case 2: long enough
settings: inspection NewMethodNamingConvention enabled
language: java
```before
class A {
  public static int <caret>count() { return 0; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-365 case 1: void isReady
settings: inspection NonBooleanMethodNameMayNotStartWithQuestion enabled
language: java
```before
class A {
  public void <caret>isReady(int level) { assert level > 0; }
}
```
keys: inspect
```after
problem: warning `isReady` "Non-boolean method name isReady starts with a question word"
```

### JAVA-ICC-365 case 2: boolean isReady
settings: inspection NonBooleanMethodNameMayNotStartWithQuestion enabled
language: java
```before
class A {
  public boolean <caret>isReady(int level) { return level > 0; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-366 case 1: two one-argument overloads
settings: inspection OverloadedMethodsWithSameNumberOfParameters enabled
language: java
```before
import java.util.concurrent.RunnableFuture;
class Main {
  public static void <caret>submit(Runnable r) {}
  public static <T> void submit(RunnableFuture<T> f) {}
}
```
keys: inspect
```after
problem: warning `submit` "Multiple methods named submit with the same number of parameters"
```

### JAVA-ICC-366 case 2: definitely incompatible parameter types
settings: inspection OverloadedMethodsWithSameNumberOfParameters enabled
language: java
```before
class Main {
  public static void <caret>submit(String s) {}
  public static void submit(Integer i) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-367 case 1: varargs and fixed overload
settings: inspection OverloadedVarargsMethod enabled
language: java
```before
class A {
  public void <caret>log(String... parts) {}
  public void log(String a, String b) {}
}
```
keys: inspect
```after
problem: warning `log` "Overloaded varargs method log()"
```

### JAVA-ICC-367 case 2: unique name
settings: inspection OverloadedVarargsMethod enabled
language: java
```before
class A {
  public void <caret>logAll(String... parts) {}
  public void log(String a, String b) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-368 case 1: field not initialised
settings: default; JetBrains annotations on classpath
status: unverified ({0} may be the annotation's simple name with or without the at-sign)
language: java
```before
import org.jetbrains.annotations.NotNull;
public class Ranger {
  private @NotNull String <caret>label;
  public void setLabel(@NotNull String label) { this.label = label; }
}
```
keys: inspect
```after
problem: warning `label` "@NotNull fields must be initialized"
```

### JAVA-ICC-368 case 2: initialised in the constructor
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.NotNull;
public class Pallet {
  private @NotNull String <caret>label;
  public Pallet(@NotNull String label) { this.label = label; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-369 case 1: salute(null)
settings: default; JetBrains annotations on classpath
status: unverified (the inspection is new in a later release than the sources of the other rows; range may be the argument or the parameter)
language: java
```before
import org.jetbrains.annotations.NotNull;
class Beacon {
  void salute(@NotNull String who) { System.out.println("Ahoy, " + who); }
  void run() { salute(<caret>null); }
}
```
keys: inspect
```after
problem: warning `null` "Parameter annotated @NotNull should not receive 'null' as an argument"
```

### JAVA-ICC-369 case 2: non-null argument
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.NotNull;
class Gallery {
  void salute(@NotNull String who) { System.out.println("Ahoy, " + who); }
  void run() { salute(<caret>"Sam"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-370 case 1: both annotations
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.*;
class A {
  @Nullable @<caret>NotNull String headline;
}
```
keys: inspect
```after
problem: warning `@NotNull` "Cannot annotate with both @Nullable and @NotNull"
```

### JAVA-ICC-370 case 2: annotated primitive
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.*;
class A {
  @<caret>NotNull int budget;
}
```
keys: inspect
```after
problem: warning `@NotNull` "Primitive type members cannot be annotated"
```

### JAVA-ICC-370 case 3: override without annotation
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.*;
abstract class A { @NotNull abstract String m(); }
class B extends A {
  String <caret>m() { return "empty string"; }
}
```
keys: inspect
```after
problem: warning `m` "Not annotated method overrides method annotated with @NotNull"
```

### JAVA-ICC-370 case 4: consistent override
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.*;
abstract class A { @NotNull abstract String m(); }
class B extends A {
  @NotNull String <caret>m() { return "empty string"; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-371 case 1: return null from a String method
settings: inspection ReturnNull enabled
status: unverified (the annotation fix name and import; needs JetBrains annotations)
language: java
```before
class Track {
  public String find(int id) { return <caret>null; }
}
```
keys: Alt+Enter, choose "Annotate method as '@Nullable'"
```after
import org.jetbrains.annotations.Nullable;
class Track {
  public @Nullable String find(int id) { return null; }
}
```

### JAVA-ICC-371 case 2: Optional return type, replace with Optional.empty()
settings: inspection ReturnNull enabled
language: java
```before
import java.util.Optional;
class Garden {
  public Optional<String> find(int id) { return <caret>null; }
}
```
keys: Alt+Enter, choose "Replace with 'Optional.empty()'"
```after
import java.util.Optional;
class Garden {
  public Optional<String> find(int id) { return Optional.empty(); }
}
```

### JAVA-ICC-371 case 3: private method with the option on
settings: inspection ReturnNull enabled; Ignore 'private' methods, anonymous classes & lambda's = on
language: java
```before
class Fjord {
  private String getName() { return <caret>null; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-372 case 1: @Pattern on an int
settings: default; JetBrains annotations on classpath
status: unverified (range)
language: java
```before
import org.intellij.lang.annotations.Pattern;
class A {
  @<caret>Pattern("\\d\\d") long code;
}
```
keys: inspect
```after
problem: error `@Pattern("\\d\\d")` "Pattern annotation is only applicable to elements of type String"
```

### JAVA-ICC-372 case 2: @Pattern on a String
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Pattern;
class A {
  @<caret>Pattern("\\d\\d") String s;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-373 case 1: override without @Pattern
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Pattern;
abstract class Ticket { abstract @Pattern("\\d\\d-\\d\\d\\d") String getId(); }
class Marble extends Ticket {
  @Override String <caret>getId() { return "12-345"; }
}
```
keys: inspect
```after
problem: warning `getId` "Non-annotated Method overrides @Pattern Method"
```

### JAVA-ICC-373 case 2: override with the same annotation
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Pattern;
abstract class Ferry { abstract @Pattern("\\d\\d-\\d\\d\\d") String getId(); }
class Badger extends Ferry {
  @Override @Pattern("\\d\\d-\\d\\d\\d") String <caret>getId() { return "12-345"; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-374 case 1: constant not matching
settings: default; JetBrains annotations on classpath
status: unverified (the third message argument)
language: java
```before
import org.intellij.lang.annotations.Pattern;
class A {
  @Pattern("\\d\\d-\\d\\d\\d") String getId() { return <caret>"1"; }
}
```
keys: inspect
```after
problem: warning `"1"` "Expression '1' doesn't match '\d\d-\d\d\d' pattern: (explanation of the mismatch)"
```

### JAVA-ICC-374 case 2: matching constant
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Pattern;
class A {
  @Pattern("\\d\\d-\\d\\d\\d") String getId() { return <caret>"12-345"; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-375 case 1: missing translation (batch run)
settings: Inspect Code on a bundle `messages.properties` (abc=xxx) with an empty `messages_fr.properties`; option Report missing translations = on
status: unverified (message text of the provider)
language: plain
```before
--- messages.properties
abc=xxx
--- messages_fr.properties

```
keys: inspect batch
```after
problem: warning `abc` "property 'abc' is not translated in messages_fr.properties"
```

### JAVA-ICC-375 case 2: complete bundle
settings: same bundle with abc=yyy in the French file
language: plain
```before
--- messages.properties
abc=xxx
--- messages_fr.properties
abc=yyy
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICC-376 case 1: unknown locale suffix
settings: inspection SuspiciousLocalesLanguages enabled; files `messages.properties`, `messages_xx.properties`
status: unverified (highlight target)
language: plain
```before
--- messages.properties
a=1
--- messages_xx.properties
a=2
```
keys: inspect
```after
problem: warning `messages_xx.properties` "Resource bundle contains locales with suspicious locale languages"
```

### JAVA-ICC-377 case 1: {1} without {0}
settings: default
status: unverified (the range: whole property value)
language: plain
```before
# only the line number is used
failure.message=Failure near <caret>{1}
```
keys: inspect
```after
problem: warning `Failure near {1}` "Using parameter 1 without using parameter 0 in 'Failure near {1}'"
```

### JAVA-ICC-377 case 2: both parameters used
settings: default
language: plain
```before
failure.message=Failure near <caret>{1} in {0}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-378 case 1: assertEquals("1", 1)
settings: default; JUnit 4 on classpath
status: unverified (boxed type name in {1})
language: java
```before
import static org.junit.Assert.assertEquals;
class T {
  void t() { <caret>assertEquals("1", 1); }
}
```
keys: inspect
```after
problem: warning `assertEquals` "assertEquals() between objects of inconvertible types 'String' and 'int'"
```

### JAVA-ICC-378 case 2: compatible types
settings: default; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.assertEquals;
class T {
  void t() { <caret>assertEquals("1", "1"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-379 case 1: initialisation in constructor
settings: inspection TestCaseWithConstructor enabled; JUnit 4 on classpath
language: java
```before
public class ReportTest {
  private StringBuilder out;
  public <caret>ReportTest() { out = new StringBuilder("seed"); }
  @org.junit.Test public void t() {}
}
```
keys: inspect
```after
problem: warning `ReportTest` "Initialization logic in constructor ReportTest() instead of 'setup()' life cycle method"
```

### JAVA-ICC-379 case 2: no constructor
settings: inspection TestCaseWithConstructor enabled; JUnit 4 on classpath
language: java
```before
public class <caret>ReportTest {
  @org.junit.Test public void t() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-380 case 1: only setUp
settings: inspection TestCaseWithNoTestMethods enabled; JUnit 4 on classpath
language: java
```before
import org.junit.Before;
public class <caret>InvoiceTest {
  int count;
  @Before public void setUp() { count = 0; }
}
```
keys: inspect
```after
problem: warning `InvoiceTest` "Test class InvoiceTest has no tests"
```

### JAVA-ICC-380 case 2: class with a test
settings: inspection TestCaseWithNoTestMethods enabled; JUnit 4 on classpath
language: java
```before
import org.junit.Test;
public class <caret>InvoiceTest {
  @Test public void t() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-381 case 1: line that failed in the last run
settings: default; the last run of `foo` failed at the `assertEquals` line
status: unverified (requires a stored test result; range and message source)
language: java
```before
import static org.junit.Assert.assertEquals;
public class T {
  @org.junit.Test public void foo() { <caret>assertEquals(1, 0); }
}
```
keys: inspect
```after
problem: warning `assertEquals(1, 0)` "(failure message of the test result, e.g. expected:<1> but was:<0>)"
```

### JAVA-ICC-381 case 2: no failed run
settings: default; no stored result
language: java
```before
import static org.junit.Assert.assertEquals;
public class T {
  @org.junit.Test public void render() { <caret>assertEquals(1, 1); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-382 case 1: test class in a production root
settings: inspection TestInProductSource enabled; file in a source root marked as production
language: java
```before
public class <caret>CrucialTest {
  @org.junit.Test public void t() {}
}
```
keys: inspect
```after
problem: warning `CrucialTest` "Test case CrucialTest should probably be placed in a test source tree"
```

### JAVA-ICC-382 case 2: same class in a test root
settings: inspection TestInProductSource enabled; file in a test source root
language: java
```before
public class <caret>CrucialTest {
  @org.junit.Test public void t() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-383 case 1: println only
settings: inspection TestMethodWithoutAssertion enabled; JUnit 4 on classpath
language: java
```before
public class SensorTest {
  @org.junit.Test public void <caret>testRender() { System.out.println("ok"); }
}
```
keys: inspect
```after
problem: warning `testRender` "Test method testRender() contains no assertions"
```

### JAVA-ICC-383 case 2: has an assertion
settings: inspection TestMethodWithoutAssertion enabled; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.assertTrue;
public class ItineraryTest {
  @org.junit.Test public void <caret>testTune() { assertTrue(true); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-384 case 1: production code calling a @TestOnly method
settings: inspection TestOnlyProblems enabled; production source root
status: unverified (highlighted range)
language: java
```before
import org.jetbrains.annotations.TestOnly;
class A {
  @TestOnly static void fetch() {}
  static void main() { <caret>fetch(); }
}
```
keys: inspect
```after
problem: warning `fetch` "Test-only method is called in production code"
```

### JAVA-ICC-384 case 2: call from another @TestOnly method
settings: inspection TestOnlyProblems enabled
language: java
```before
import org.jetbrains.annotations.TestOnly;
class A {
  @TestOnly static void launch() {}
  @TestOnly static void stamp() { <caret>launch(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-385 case 1: deprecated class in an XML attribute
settings: default; a class `com.example.Old` annotated `@Deprecated` in the project
status: unverified (needs a reference-resolving XML schema context for the attribute)
language: plain
```before
--- config.xml
<bean class="<caret>com.example.Old"/>
--- Old.java (package com.example)
@Deprecated public class Old {}
```
keys: inspect
```after
problem: warning `com.example.Old` "(deprecation message of Old)"
```

### JAVA-ICC-385 case 2: non-deprecated class
settings: default
language: plain
```before
--- config.xml
<bean class="<caret>com.example.Fresh"/>
--- Fresh.java (package com.example)
public class Fresh {}
```
keys: inspect
```after
no problem reported
```
