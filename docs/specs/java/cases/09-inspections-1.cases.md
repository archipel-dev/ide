# Java inspections catalog, part 1: acceptance cases

Format: `docs/specs/editor/cases/README.md`. Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-1.md`. Keys `Alt+Enter, choose "<fix>"` mean: press Alt+Enter at the caret and pick the quick fix with that exact name from the popup. Keys `inspect` run the inspection over the file (the highlight is what the editor shows); `problem:` lines list the expected highlight. `after: no problem reported` means the inspection must not highlight anything in the before text.

### JAVA-ICA-001 case 1: Narrowing int to byte is reported
settings: default
language: java
```before
class C {
  byte f(int a) {
    return (byte) a;
  }
}
```
keys: inspect
```after
problem: warning `byte` "Cast from 'int' to byte may result in loss of precision"
```

### JAVA-ICA-001 case 2: Ignore int to char option silences the report
settings: Ignore casts from int to char=on
language: java
```before
class C {
  char f(int a) {
    return (char) a;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-002 case 1: Cast of double literal becomes float literal
settings: default
language: java
```before
class C {
  float f = <caret>(float) 1.1;
}
```
keys: Alt+Enter, choose "Replace with '1.1f'"
```after
class C {
  float f = 1.1f;
}
```

### JAVA-ICA-003 case 1: Literal operand receives L suffix
settings: default
language: java
```before
class C {
  long f(int i) {
    long val = <caret>65536 * i;
    return val;
  }
}
```
keys: Alt+Enter, choose "Cast to long"
```after
class C {
  long f(int i) {
    long val = 65536L * i;
    return val;
  }
}
```

### JAVA-ICA-003 case 2: Shift gets a long cast
settings: default
language: java
```before
class C {
  long f(int i) {
    long value = <caret>i << 24;
    return value;
  }
}
```
keys: Alt+Enter, choose "Cast to long"
```after
class C {
  long f(int i) {
    long value = (long) i << 24;
    return value;
  }
}
```

### JAVA-ICA-003 case 3: Product used as int is not reported
settings: default
language: java
```before
class C {
  int f(int i, int j) {
    int val = <caret>i * j;
    return val;
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-004 case 1: Cast of int literal becomes long literal
settings: default
language: java
```before
class C {
  Long l = <caret>(long) 42;
}
```
keys: Alt+Enter, choose "Replace with '42L'"
```after
class C {
  Long l = 42L;
}
```

### JAVA-ICA-005 case 1: Cast to the declared type is removed
settings: default
language: java
```before
class C {
  int f() {
    int x = (<caret>int) (short) 5;
    return x;
  }
}
```
keys: Alt+Enter, choose "Remove cast"
```after
class C {
  int f() {
    int x = (short) 5;
    return x;
  }
}
```

### JAVA-ICA-006 case 1: Class named after its package
settings: default
language: java
```before
--- byteCode/ByteCodeAnalyzer.java
package byteCode;
class <caret>ByteCodeAnalyzer {}
```
keys: inspect
```after
problem: warning `ByteCodeAnalyzer` "Class name ByteCodeAnalyzer begins with its package name"
```

### JAVA-ICA-007 case 1: Class named like its interface
settings: default
language: java
```before
package util;
abstract class <caret>Iterable<T> implements java.lang.Iterable<T> {}
```
keys: inspect
```after
problem: warning `Iterable` "Class name Iterable is the same as one of its superclass' names"
```

### JAVA-ICA-008 case 1: Exception class with wrong suffix
settings: default
language: java
```before
class <caret>NotStartedEx extends Exception {}
```
keys: inspect
```after
problem: warning `NotStartedEx` "Exception class name NotStartedEx does not end with 'Exception'"
```

### JAVA-ICA-009 case 1: Test class with a non-matching name (default profile)
settings: library=junit4
language: java
```before
import org.junit.Test;
public class <caret>Checks {
  @Test public void works() {}
}
```
keys: inspect
```after
problem: warning `Checks` "Test class name Checks doesn't match regex '[A-Z][A-Za-z\d]*Test(s|Case)?|Test[A-Z][A-Za-z\d]*|IT(.*)|(.*)IT(Case)?'"
```

### JAVA-ICA-009 case 2: Plain class is not checked by default
settings: default
language: java
```before
class <caret>A {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-009 case 3: Class convention enabled reports short names
settings: Class naming convention: Class=on (minimum length 8)
language: java
```before
class <caret>Shorty {}
```
keys: inspect
```after
problem: warning `Shorty` "Class name Shorty is too short (6 < 8)"
```

### JAVA-ICA-010 case 1: Class named like an exception gets Exception as superclass
settings: default
language: java
```before
public class <caret>NotStartedException {}
```
keys: Alt+Enter, choose "Make 'NotStartedException' extend 'java.lang.Exception'"
```after
public class NotStartedException extends java.lang.Exception {}
```

### JAVA-ICA-086 case 1: Abstract class with one subclass
settings: default
language: java
```before
abstract class <caret>Base {}
class Inheritor extends Base {}
```
keys: inspect
```after
problem: warning `Base` "Abstract class Base has only one direct inheritor"
```

### JAVA-ICA-087 case 1: Array length in for condition
settings: default
language: java
```before
class C {
  void foo(Object[] x) {
    for (int i = 0; i < x.<caret>length; i++) { }
  }
}
```
keys: inspect
```after
problem: warning `length` "Check of array length in loop condition"
```

### JAVA-ICA-088 case 1: Large constant array allocation
settings: default
language: java
```before
class C {
  int[] a = <caret>new int[100];
}
```
keys: inspect
```after
problem: warning `new int[100]` "Large array allocation which is not checked for out-of-memory condition"
```

### JAVA-ICA-090 case 1: Interface with one implementation
settings: default
language: java
```before
interface <caret>Shape {}
class Circle implements Shape {}
```
keys: inspect
```after
problem: warning `Shape` "Interface Shape has only one direct inheritor"
```

### JAVA-ICA-091 case 1: Method call in for condition
settings: default
language: java
```before
class C {
  void f(String s) {
    for (int i = 0; i < s.<caret>length(); i++) { }
  }
}
```
keys: Alt+Enter, choose "Introduce variable (may change semantics)"
```after
class C {
  void f(String s) {
    int length = s.length();
    for (int i = 0; i < length; i++) { }
  }
}
```

### JAVA-ICA-092 case 1: Multiplication becomes shift
settings: default
language: java
```before
class C {
  int f(int x) {
    return <caret>x * 4;
  }
}
```
keys: Alt+Enter, choose "Replace with shift"
```after
class C {
  int f(int x) {
    return x << 2;
  }
}
```

### JAVA-ICA-093 case 1: Initializer with 65 elements
settings: Maximum number of elements=3
language: java
```before
class C {
  int[] a = <caret>{1, 2, 3, 4};
}
```
keys: inspect
```after
problem: warning `{1, 2, 3, 4}` "Primitive array initializer with too many elements (4)"
```

### JAVA-ICA-094 case 1: Inner class calls private outer method (language level 8)
settings: language level=8
language: java
```before
class Outer {
  private void x() {}
  class Inner {
    void y() {
      <caret>x();
    }
  }
}
```
keys: Alt+Enter, choose "Make 'x' package-private"
```after
class Outer {
  void x() {}
  class Inner {
    void y() {
      x();
    }
  }
}
```

### JAVA-ICA-096 case 1: startsWith with one char
settings: default
language: java
```before
class C {
  boolean f(String s) {
    return s.<caret>startsWith("x");
  }
}
```
keys: Alt+Enter, choose "Replace with 'charAt()' expression"
```after
class C {
  boolean f(String s) {
    return !s.isEmpty() && s.charAt(0) == 'x';
  }
}
```

### JAVA-ICA-109 case 1: assertEquals on arrays
settings: library=junit5
language: java
```before
import org.junit.jupiter.api.Assertions;
class MyTest {
  void myTest() {
    Object[] a = {};
    Object[] e = {""};
    Assertions.<caret>assertEquals(a, e, "message");
  }
}
```
keys: Alt+Enter, choose "Replace with 'assertArrayEquals()'"
```after
import org.junit.jupiter.api.Assertions;
class MyTest {
  void myTest() {
    Object[] a = {};
    Object[] e = {""};
    Assertions.assertArrayEquals(a, e, "message");
  }
}
```

### JAVA-ICA-110 case 1: assertEquals of enum constants
settings: library=junit4
language: java
```before
import org.junit.Assert;
enum A { a, b }
class Test {
  @org.junit.Test
  public void test() {
    Assert.<caret>assertEquals(A.a, A.b);
  }
}
```
keys: Alt+Enter, choose "Replace with 'assertSame()'"
```after
import org.junit.Assert;
enum A { a, b }
class Test {
  @org.junit.Test
  public void test() {
    Assert.assertSame(A.a, A.b);
  }
}
```

### JAVA-ICA-111 case 1: Checked exception expected but never thrown
settings: library=junit4
language: java
```before
class X {
  @org.junit.Test(expected = java.io.<caret>IOException.class)
  public void test() { }
}
```
keys: inspect
```after
problem: warning `java.io.IOException` "Expected 'java.io.IOException' never thrown in body of 'test()'"
```

### JAVA-ICA-112 case 1: Ignored method without a reason
settings: library=junit4
language: java
```before
import org.junit.*;
class IgnoredJUnitTest {
  @Ignore
  @Test
  public void <caret>foo1() { }
}
```
keys: inspect
```after
problem: warning `foo1` "Test method 'foo1()' is ignored/disabled without reason"
```

### JAVA-ICA-113 case 1: Old style method next to a JUnit 4 test
settings: library=junit4
status: unverified (exact fix text derived from the annotate-as template)
language: java
```before
import org.junit.Test;
public class JUnit3StyleTestMethodInJUnit4Class {
  @Test
  public void junit4Test() { }
  public void <caret>testJUnit3() { }
}
```
keys: Alt+Enter, choose "Annotate method 'testJUnit3' as '@Test'"
```after
import org.junit.Test;
public class JUnit3StyleTestMethodInJUnit4Class {
  @Test
  public void junit4Test() { }
  @Test
  public void testJUnit3() { }
}
```

### JAVA-ICA-113 case 2: Plain JUnit 5 class is fine
settings: library=junit4,junit5
language: java
```before
import org.junit.jupiter.api.Test;
public class SimpleJUnit5 {
  @Test
  public void <caret>testIt() { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-114 case 1: JUnit 3 class converted to JUnit 4
settings: library=junit3,junit4
language: java
```before
import junit.framework.TestCase;
class MainTestCase<caret> extends TestCase {
  public void test() {
    assertTrue(true);
  }
}
```
keys: Alt+Enter, choose "Convert to JUnit 4 test case"
```after
import org.junit.Test;
import static org.junit.Assert.assertTrue;
class MainTestCase {
  @Test
  public void test() {
    assertTrue(true);
  }
}
```

### JAVA-ICA-114 case 2: JUnit 4 class is not reported
settings: library=junit4
language: java
```before
class Plain<caret>Test {
  @org.junit.Test
  public void test() { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-115 case 1: Child repeats the runner
settings: library=junit4
language: java
```before
import org.junit.runner.RunWith;
import org.junit.runners.Suite;
import org.junit.runners.Parameterized;
@RunWith(Suite.class)
abstract class Parent {}
<caret>@RunWith(Parameterized.class)
class MyTest extends Parent {}
```
keys: inspect
```after
problem: warning `@RunWith(Parameterized.class)` "'@RunWith' annotation already exists in Parent class"
```

### JAVA-ICA-115 case 2: Runner declared once is fine
settings: library=junit4
language: java
```before
import org.junit.runner.RunWith;
import org.junit.runners.Parameterized;
abstract class Parent {}
<caret>@RunWith(Parameterized.class)
class MyTest extends Parent {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-116 case 1: assertArrayEquals with a message
settings: library=junit4,junit5
language: java
```before
import static org.junit.Assert.*;
import org.junit.jupiter.api.Test;
class Test1 {
  @Test
  public void testFirst() throws Exception {
    assert<caret>ArrayEquals("message", new Object[] {}, null);
  }
}
```
keys: Alt+Enter, choose "Replace with 'org.junit.jupiter.api.Assertions' method call"
```after
import static org.junit.Assert.*;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
class Test1 {
  @Test
  public void testFirst() throws Exception {
    Assertions.assertArrayEquals(new Object[] {}, null, "message");
  }
}
```

### JAVA-ICA-116 case 2: No equivalent method means no fix
settings: library=junit4,junit5
language: java
```before
import static org.junit.Assert.*;
import org.junit.jupiter.api.Test;
class Test1 {
  @Test
  public void testFirst() throws Exception {
    assert<caret>NotEquals(1, 1, 1);
  }
}
```
keys: Alt+Enter
```after
no quick fix "Replace with 'org.junit.jupiter.api.Assertions' method call" in the popup
```

### JAVA-ICA-117 case 1: JUnit 4 test migrated
settings: library=junit4,junit5
language: java
```before
import org.junit.Assert;
import org.junit.Test;
public class Relevant<caret>Test {
  @Test
  public void testIt() {
    Assert.assertEquals("expected", "actual");
  }
}
```
keys: Alt+Enter, choose "Migrate to JUnit 5"
```after
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Test;
public class RelevantTest {
  @Test
  public void testIt() {
    Assertions.assertEquals("expected", "actual");
  }
}
```

### JAVA-ICA-118 case 1: JUnit 5 lifecycle method made accessible
settings: library=junit5
language: java
```before
class MainTest {
  @org.junit.jupiter.api.BeforeEach
  private void bef<caret>oreEach() { }
}
```
keys: Alt+Enter, choose "Fix 'beforeEach' method signature"
```after
class MainTest {
  @org.junit.jupiter.api.BeforeEach
  public void beforeEach() { }
}
```

### JAVA-ICA-118 case 2: Wrong @Rule field reported with exact message
settings: library=junit4
language: java
```before
class RuleTest {
  @org.junit.Rule
  public int <caret>x;
}
```
keys: inspect
```after
problem: warning `x` "Field 'x' annotated with '@Rule' should be of type 'org.junit.rules.TestRule'"
```

### JAVA-ICA-118 case 3: @Before method with parameters
settings: library=junit4
language: java
```before
class MainTest {
  @org.junit.Before
  String <caret>before(int i) { return ""; }
}
```
keys: inspect
```after
problem: warning `before` "Method 'before' annotated with '@Before' should be public, of type 'void' and not declare parameter 'i'"
```

### JAVA-ICA-119 case 1: Remove the JUnit 4 annotation from a JUnit 3 case
settings: library=junit3,junit4
language: java
```before
public class MyTest extends junit.framework.TestCase {
  @org.junit.Test
  public void test<caret>Foo() { }
}
```
keys: Alt+Enter, choose "Remove '@Test' annotation"
```after
public class MyTest extends junit.framework.TestCase {
  public void testFoo() { }
}
```

### JAVA-ICA-119 case 2: Consistent JUnit 4 class is fine
settings: library=junit4
language: java
```before
public class JUnit4Test {
  @org.junit.Test
  public void <caret>testFoo() { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICA-120 case 1: assertNotNull becomes assertThat
settings: library=junit4,hamcrest
status: unverified (exact fix label not confirmed)
language: java
```before
import org.junit.Assert;
class T {
  void f(Object c) {
    Assert.<caret>assertNotNull(c);
  }
}
```
keys: inspect
```after
problem: warning `Assert.assertNotNull(c)` "Assert expression Assert.assertNotNull(c) can be replaced with 'assertThat()' call"
```

### JAVA-ICA-121 case 1: Several exceptions collapse
settings: library=junit4
language: java
```before
class T {
  @org.junit.Test
  public void testReflection() <caret>throws NoSuchMethodException, IllegalAccessException {
  }
}
```
keys: Alt+Enter, choose "Replace with 'throws Exception'"
```after
class T {
  @org.junit.Test
  public void testReflection() throws Exception {
  }
}
```

### JAVA-ICA-122 case 1: Missing data provider
settings: library=junit4
language: java
```before
import org.junit.runner.RunWith;
import org.junit.runners.Parameterized;
@RunWith(Parameterized.class)
public class <caret>ImportantTest {
}
```
keys: Alt+Enter, choose "Create data provider method '@Parameters public static Iterable parameters()'"
```after
import org.junit.runner.RunWith;
import org.junit.runners.Parameterized;
import org.junit.runners.Parameterized.Parameters;
@RunWith(Parameterized.class)
public class ImportantTest {
  @Parameters
  public static Iterable<Object[]> parameters() {
    return null;
  }
}
```

### JAVA-ICA-123 case 1: Call after other statements
settings: library=junit3
language: java
```before
class SuperTearDownInFinally extends junit.framework.TestCase {
  @Override
  public void tearDown() throws Exception {
    super.<caret>tearDown();
    System.out.println();
  }
}
```
keys: inspect
```after
problem: warning `tearDown` "'tearDown()' is not called from 'finally' block"
```

### JAVA-ICA-124 case 1: Obsolete assertion replaced
settings: library=junit3,junit4
language: java
```before
import org.junit.*;
public class NecessaryTest {
  @Test
  public void testIt() {
    junit.framework.Assert.<caret>assertEquals("expected", "actual");
  }
}
```
keys: Alt+Enter, choose "Replace with 'org.junit.Assert' method call"
```after
import org.junit.*;
public class NecessaryTest {
  @Test
  public void testIt() {
    Assert.assertEquals("expected", "actual");
  }
}
```
