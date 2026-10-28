# Cases for 09-inspections-catalog-10 (JAVA-ICC)

Format: see `docs/specs/editor/cases/README.md` and the header of `09-inspections-7.cases.md` (`inspect`, `problem:` lines,
`no problem reported`). Double-brace tokens carry the short name and are replaced by the requirement ID of that inspection.
Logging cases assume `org.slf4j` on the classpath; test cases assume JUnit 4 (`org.junit`) on the classpath.

### JAVA-ICC-348 case 1: @Language on an int
settings: default; JetBrains annotations on classpath
status: unverified (exact highlighted range: the annotation or its name)
language: java
```before
import org.intellij.lang.annotations.Language;
class A {
  @<caret>Language("HTML") int i;
}
```
keys: inspect
```after
problem: ERROR @Language("HTML") "Language injection is only applicable to strings or string templates"
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
  @Language("JavaScript") String JS_CODE = "var x;";
  @Language("XPath") String XPATH_CODE = <caret>JS_CODE;
}
```
keys: inspect
```after
problem: WARNING JS_CODE "Language mismatch: Expected 'XPath', got 'JavaScript'"
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
  @Language("<caret>HMTL") String html;
}
```
keys: inspect
```after
problem: ERROR "HMTL" "Unknown language 'HMTL'"
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
class TestObject {
  Logger LOG;
  int expensiveCalculation(int i) { return i; }
  void test(Object object) {
    <caret>if (LOG.isDebugEnabled()) {
      LOG.debug("some logging {}", expensiveCalculation(1));
    }
  }
}
```
keys: Alt+Enter, choose "Unwrap log guard condition"
```after
import org.slf4j.Logger;
class TestObject {
  Logger LOG;
  int expensiveCalculation(int i) { return i; }
  void test(Object object) {
    LOG.debug("some logging {}", expensiveCalculation(1));
  }
}
```

### JAVA-ICC-352 case 2: guard with other statements
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class TestObject {
  Logger LOG;
  void test(Object object) {
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
problem: WARNING "{}: {}" "Fewer arguments provided (1) than placeholders specified (2)"
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
problem: WARNING v "More arguments provided (2) than placeholders specified (1)"
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
  static void request1(String text) { log.info(<caret>"Message: {}", text); }
  static void request2(int i) { log.info("Message: {}", i); }
}
```
keys: inspect
```after
problem: WEAK WARNING "Message: {}" "Similar log messages"
```

### JAVA-ICC-354 case 2: distinct messages
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class A {
  private static Logger log;
  static void request1(String text) { log.info(<caret>"First request: {}", text); }
  static void request2(int i) { log.info("Second call: {}", i); }
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
  void f(Object variable1) { logger.<caret>info("variable1: " + variable1); }
}
```
keys: Alt+Enter, choose "Replace with placeholders"
```after
import org.slf4j.Logger;
class A {
  Logger logger;
  void f(Object variable1) { logger.info("variable1: {}", variable1); }
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
class TestObject {
  Logger LOG;
  int expensiveCalculation(int i) { return i; }
  void test(Object object) { LOG.<caret>debug("some logging {}", expensiveCalculation(1)); }
}
```
keys: Alt+Enter, choose "Surround with a logging condition"
```after
import org.slf4j.Logger;
class TestObject {
  Logger LOG;
  int expensiveCalculation(int i) { return i; }
  void test(Object object) {
    if (LOG.isDebugEnabled()) {
      LOG.debug("some logging {}", expensiveCalculation(1));
    }
  }
}
```

### JAVA-ICC-356 case 2: constant message
settings: default; SLF4J on classpath
language: java
```before
import org.slf4j.Logger;
class TestObject {
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
import java.util.List;
class A {
  boolean <caret>empty(List<String> list) { return list.isEmpty(); }
}
```
keys: inspect
```after
problem: WARNING empty "Boolean method name empty does not start with question word"
```

### JAVA-ICC-359 case 2: isEmpty
settings: inspection BooleanMethodNameMustStartWithQuestion enabled
language: java
```before
import java.util.List;
class A {
  boolean <caret>isEmpty(List<String> list) { return list.isEmpty(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-360 case 1: Supplier and Callable overloads
settings: inspection LambdaUnfriendlyMethodOverload enabled
status: unverified (message text not in the bundle excerpt read)
language: java
```before
import java.util.concurrent.Callable;
import java.util.function.Supplier;
interface MyExecutor {
  void <caret>execute(Supplier<?> supplier);
  void execute(Callable<?> callable);
}
```
keys: inspect
```after
problem: WARNING execute (the overload is lambda-unfriendly)
```

### JAVA-ICC-360 case 2: incompatible functional shapes
settings: inspection LambdaUnfriendlyMethodOverload enabled
language: java
```before
import java.util.function.Consumer;
import java.util.function.Supplier;
interface MyExecutor {
  void <caret>execute(Supplier<?> supplier);
  void execute(Consumer<?> consumer);
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
class MyClass {
  int val;
  void <caret>MyClass(int val) { this.val = val; }
}
```
keys: Alt+Enter, choose "Make method constructor"
```after
class MyClass {
  int val;
  MyClass(int val) { this.val = val; }
}
```

### JAVA-ICC-361 case 2: a real constructor
settings: default
language: java
```before
class MyClass {
  int val;
  <caret>MyClass(int val) { this.val = val; }
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
class Parent {}
class Child extends Parent {
  public Parent <caret>Parent() { return null; }
}
```
keys: inspect
```after
problem: WARNING Parent "Method name Parent is the same as its parent class name"
```

### JAVA-ICC-363 case 1: hashcode vs hashCode
settings: inspection MisspelledMethodName enabled
status: unverified (hashCode here is declared in the same class; with only an inherited hashCode the report depends on the inherited-methods check)
language: java
```before
class A {
  public int <caret>hashcode() { return 0; }
  public int hashCode() { return 1; }
}
```
keys: inspect
```after
problem: WARNING hashcode "Method name hashcode and method name 'hashCode' differ only by case"
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
status: unverified (message text of the naming-convention message not in the bundle excerpt read)
language: java
```before
class A {
  public static int <caret>n() { return 0; }
}
```
keys: inspect
```after
problem: WARNING n (the method name is shorter than the minimum length)
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

### JAVA-ICC-365 case 1: void hasName
settings: inspection NonBooleanMethodNameMayNotStartWithQuestion enabled
language: java
```before
import java.util.Set;
class A {
  Set<String> names;
  public void <caret>hasName(String name) { assert names.contains(name); }
}
```
keys: inspect
```after
problem: WARNING hasName "Non-boolean method name hasName starts with a question word"
```

### JAVA-ICC-365 case 2: boolean hasName
settings: inspection NonBooleanMethodNameMayNotStartWithQuestion enabled
language: java
```before
import java.util.Set;
class A {
  Set<String> names;
  public boolean <caret>hasName(String name) { return names.contains(name); }
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
  public static void <caret>execute(Runnable r) {}
  public static <T> void execute(RunnableFuture<T> c) {}
}
```
keys: inspect
```after
problem: WARNING execute "Multiple methods named execute with the same number of parameters"
```

### JAVA-ICC-366 case 2: definitely incompatible parameter types
settings: inspection OverloadedMethodsWithSameNumberOfParameters enabled
language: java
```before
class Main {
  public static void <caret>execute(String s) {}
  public static void execute(Integer i) {}
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
  public void <caret>execute(Runnable... r) {}
  public void execute(Runnable r1, Runnable r2) {}
}
```
keys: inspect
```after
problem: WARNING execute "Overloaded varargs method execute()"
```

### JAVA-ICC-367 case 2: unique name
settings: inspection OverloadedVarargsMethod enabled
language: java
```before
class A {
  public void <caret>executeAll(Runnable... r) {}
  public void execute(Runnable r1, Runnable r2) {}
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
public class MyClass {
  private @NotNull String <caret>value;
  public void setValue(@NotNull String value) { this.value = value; }
}
```
keys: inspect
```after
problem: WARNING value "@NotNull fields must be initialized"
```

### JAVA-ICC-368 case 2: initialised in the constructor
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.NotNull;
public class MyClass {
  private @NotNull String <caret>value;
  public MyClass(@NotNull String value) { this.value = value; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-369 case 1: greet(null)
settings: default; JetBrains annotations on classpath
status: unverified (the inspection is new in a later release than the sources of the other rows; range may be the argument or the parameter)
language: java
```before
import org.jetbrains.annotations.NotNull;
class Greeter {
  void greet(@NotNull String name) { System.out.println("Hello, " + name); }
  void run() { greet(<caret>null); }
}
```
keys: inspect
```after
problem: WARNING null "Parameter annotated @NotNull should not receive 'null' as an argument"
```

### JAVA-ICC-369 case 2: non-null argument
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.NotNull;
class Greeter {
  void greet(@NotNull String name) { System.out.println("Hello, " + name); }
  void run() { greet(<caret>"Bob"); }
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
  @Nullable @<caret>NotNull String myFooString;
}
```
keys: inspect
```after
problem: WARNING @NotNull "Cannot annotate with both @Nullable and @NotNull"
```

### JAVA-ICC-370 case 2: annotated primitive
settings: default; JetBrains annotations on classpath
language: java
```before
import org.jetbrains.annotations.*;
class A {
  @<caret>NotNull int myFoo;
}
```
keys: inspect
```after
problem: WARNING @NotNull "Primitive type members cannot be annotated"
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
problem: WARNING m "Not annotated method overrides method annotated with @NotNull"
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
class Person {
  public String getName() { return <caret>null; }
}
```
keys: Alt+Enter, choose "Annotate method as '@Nullable'"
```after
import org.jetbrains.annotations.Nullable;
class Person {
  public @Nullable String getName() { return null; }
}
```

### JAVA-ICC-371 case 2: Optional return type, replace with Optional.empty()
settings: inspection ReturnNull enabled
language: java
```before
import java.util.Optional;
class Person {
  public Optional<String> getName() { return <caret>null; }
}
```
keys: Alt+Enter, choose "Replace with 'Optional.empty()'"
```after
import java.util.Optional;
class Person {
  public Optional<String> getName() { return Optional.empty(); }
}
```

### JAVA-ICC-371 case 3: private method with the option on
settings: inspection ReturnNull enabled; Ignore 'private' methods, anonymous classes & lambda's = on
language: java
```before
class Person {
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
  @<caret>Pattern("\\d\\d") int i;
}
```
keys: inspect
```after
problem: ERROR @Pattern("\\d\\d") "Pattern annotation is only applicable to elements of type String"
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
abstract class Parent { abstract @Pattern("\\d\\d-\\d\\d\\d") String getId(); }
class Child extends Parent {
  @Override String <caret>getId() { return "12-345"; }
}
```
keys: inspect
```after
problem: WARNING getId "Non-annotated Method overrides @Pattern Method"
```

### JAVA-ICC-373 case 2: override with the same annotation
settings: default; JetBrains annotations on classpath
language: java
```before
import org.intellij.lang.annotations.Pattern;
abstract class Parent { abstract @Pattern("\\d\\d-\\d\\d\\d") String getId(); }
class Child extends Parent {
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
problem: WARNING "1" "Expression '1' doesn't match '\d\d-\d\d\d' pattern: (explanation of the mismatch)"
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
keys: action InspectCode (scope: whole project)
```after
inspection results: Properties files | Inconsistent resource bundle | property 'abc' is not translated in messages_fr.properties
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
keys: action InspectCode (scope: whole project)
```after
no problem reported by Inconsistent resource bundle
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
problem: WARNING messages_xx.properties "Resource bundle contains locales with suspicious locale languages"
```

### JAVA-ICC-377 case 1: {1} without {0}
settings: default
status: unverified (the #ref text in the message and the range)
language: plain
```before
# parameter {0} is not used
error.message=Something happened in line <caret>{1}
```
keys: inspect
```after
problem: WARNING the value "Using parameter {1} without using parameter {0} in ..."
```

### JAVA-ICC-377 case 2: both parameters used
settings: default
language: plain
```before
error.message=Something happened in line <caret>{1} of {0}
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
problem: WARNING assertEquals "assertEquals() between objects of inconvertible types 'String' and 'int'"
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
import java.io.*;
public class ImportantTest {
  private File file;
  public <caret>ImportantTest() throws IOException { file = File.createTempFile("xyz", ".tmp"); }
  @org.junit.Test public void t() {}
}
```
keys: inspect
```after
problem: WARNING ImportantTest "Initialization logic in constructor ImportantTest() instead of 'setup()' life cycle method"
```

### JAVA-ICC-379 case 2: no constructor
settings: inspection TestCaseWithConstructor enabled; JUnit 4 on classpath
language: java
```before
public class <caret>ImportantTest {
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
public class <caret>CrucialTest {
  @Before public void setUp() { System.out.println("setting up"); }
}
```
keys: inspect
```after
problem: WARNING CrucialTest "Test class CrucialTest has no tests"
```

### JAVA-ICC-380 case 2: class with a test
settings: inspection TestCaseWithNoTestMethods enabled; JUnit 4 on classpath
language: java
```before
import org.junit.Test;
public class <caret>CrucialTest {
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
problem: WARNING assertEquals(1, 0) "(failure message of the test result, e.g. expected:<1> but was:<0>)"
```

### JAVA-ICC-381 case 2: no failed run
settings: default; no stored result
language: java
```before
import static org.junit.Assert.assertEquals;
public class T {
  @org.junit.Test public void foo() { <caret>assertEquals(1, 1); }
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
problem: WARNING CrucialTest "Test case CrucialTest should probably be placed in a test source tree"
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
public class ExtensiveTest {
  @org.junit.Test public void <caret>testAlive() { System.out.println("nothing"); }
}
```
keys: inspect
```after
problem: WARNING testAlive "Test method testAlive() contains no assertions"
```

### JAVA-ICC-383 case 2: has an assertion
settings: inspection TestMethodWithoutAssertion enabled; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.assertTrue;
public class ExtensiveTest {
  @org.junit.Test public void <caret>testAlive() { assertTrue(true); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-384 case 1: production code calling a @TestOnly method
settings: inspection TestOnlyProblems enabled; production source root
status: unverified (message text not read)
language: java
```before
import org.jetbrains.annotations.TestOnly;
class A {
  @TestOnly static void foo() {}
  static void main() { <caret>foo(); }
}
```
keys: inspect
```after
problem: WARNING foo (the call of a test-only method from production code)
```

### JAVA-ICC-384 case 2: call from another @TestOnly method
settings: inspection TestOnlyProblems enabled
language: java
```before
import org.jetbrains.annotations.TestOnly;
class A {
  @TestOnly static void foo() {}
  @TestOnly static void bar() { <caret>foo(); }
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
problem: WARNING com.example.Old "(deprecation message of Old)"
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
