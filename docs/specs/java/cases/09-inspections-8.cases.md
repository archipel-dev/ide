# Cases for 09-inspections-catalog-8 (JAVA-ICC)

Format: see `docs/specs/editor/cases/README.md` and the header of `09-inspections-7.cases.md` (`inspect`, `problem:` lines,
`no problem reported`). Double-brace tokens carry the short name and are replaced by the requirement ID of that inspection.

### JAVA-ICC-122 case 1: new URLClassLoader
settings: inspection ClassLoaderInstantiation enabled
language: java
```before
import java.net.*;
class A {
  Object f() { return new <caret>URLClassLoader(new URL[0]); }
}
```
keys: inspect
```after
problem: WARNING URLClassLoader "Instantiation of URLClassLoader may pose security concerns"
```

### JAVA-ICC-123 case 1: Cloneable class
settings: inspection CloneableClassInSecureContext enabled
language: java
```before
class <caret>SecureBean implements Cloneable {}
```
keys: Alt+Enter, choose "Generate 'clone()' method which always throws exception"
```after
class SecureBean {}
```
(the interface is removed when the class implements `Cloneable` directly, as in the description)

### JAVA-ICC-124 case 1: subclass of ClassLoader
settings: inspection CustomClassloader enabled
language: java
```before
class <caret>MyLoader extends ClassLoader {}
```
keys: inspect
```after
problem: WARNING MyLoader "Custom ClassLoader class MyLoader"
```

### JAVA-ICC-125 case 1: subclass of SecurityManager
settings: inspection CustomSecurityManager enabled
language: java
```before
class <caret>CustomSecurityManager extends SecurityManager {}
```
keys: inspect
```after
problem: WARNING CustomSecurityManager "Custom SecurityManager class CustomSecurityManager"
```

### JAVA-ICC-126 case 1: overridable method gets final
settings: inspection DesignForExtension enabled
language: java
```before
class Foo {
  public boolean <caret>equals(Object o) { return true; }
}
```
keys: Alt+Enter, choose "Make method 'equals()' 'final'"
```after
class Foo {
  public final boolean equals(Object o) { return true; }
}
```

### JAVA-ICC-126 case 2: private and empty methods
settings: inspection DesignForExtension enabled
language: java
```before
class Foo {
  private void <caret>a() { System.out.println(); }
  public void b() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-127 case 1: concatenated query
settings: inspection JDBCExecuteWithNonConstantString enabled
language: java
```before
import java.sql.*;
class A {
  Object f(Statement s, String name) throws SQLException { return s.<caret>executeQuery("select * from " + name); }
}
```
keys: inspect
```after
problem: WARNING executeQuery "Call to Statement.executeQuery() with non-constant argument"
```

### JAVA-ICC-127 case 2: constant query
settings: inspection JDBCExecuteWithNonConstantString enabled
language: java
```before
import java.sql.*;
class A {
  Object f(Statement s) throws SQLException { return s.<caret>executeQuery("select * from t"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-128 case 1: concatenated statement
settings: inspection JDBCPrepareStatementWithNonConstantString enabled
language: java
```before
import java.sql.*;
class A {
  Object f(Connection c, String n) throws SQLException { return c.<caret>prepareStatement("SELECT * FROM user WHERE name='" + n + "'"); }
}
```
keys: inspect
```after
problem: WARNING prepareStatement "Call to Connection.prepareStatement() with non-constant argument"
```

### JAVA-ICC-128 case 2: static final field with option on
settings: inspection JDBCPrepareStatementWithNonConstantString enabled, Consider 'static final' fields constant = on
language: java
```before
import java.sql.*;
class A {
  static final String SQL = "SELECT " + System.getenv("X");
  Object f(Connection c) throws SQLException { return c.<caret>prepareStatement(SQL); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-129 case 1: System.loadLibrary with concatenation
settings: inspection LoadLibraryWithNonConstantString enabled
language: java
```before
class A {
  void test(int i) { System.<caret>loadLibrary("foo" + i); }
}
```
keys: inspect
```after
problem: WARNING loadLibrary "Call to System.loadLibrary() with non-constant argument"
```

### JAVA-ICC-129 case 2: constant
settings: inspection LoadLibraryWithNonConstantString enabled
language: java
```before
class A {
  void test() { System.<caret>loadLibrary("foo"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-130 case 1: protected clone
settings: inspection NonFinalClone enabled
language: java
```before
class Main implements Cloneable {
  @Override protected Object <caret>clone() throws CloneNotSupportedException { return super.clone(); }
}
```
keys: inspect
```after
problem: WARNING clone "Non-final clone() method, compromising security"
```

### JAVA-ICC-130 case 2: final class
settings: inspection NonFinalClone enabled
language: java
```before
final class Main implements Cloneable {
  @Override protected Object <caret>clone() throws CloneNotSupportedException { return super.clone(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-131 case 1: public static array
settings: inspection PublicStaticArrayField enabled
language: java
```before
class A {
  public static String[] <caret>allowedPasswords = {"foo", "bar"};
}
```
keys: inspect
```after
problem: WARNING allowedPasswords "'public static' array field allowedPasswords, compromising security"
```

### JAVA-ICC-131 case 2: private static array
settings: inspection PublicStaticArrayField enabled
language: java
```before
class A {
  private static String[] <caret>names = {"foo"};
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-132 case 1: public static final list
settings: inspection PublicStaticCollectionField enabled
language: java
```before
import java.util.*;
class A {
  public static final List<String> <caret>EVENTS = new ArrayList<>();
}
```
keys: inspect
```after
problem: WARNING EVENTS "'public static' collection field EVENTS, compromising security"
```

### JAVA-ICC-132 case 2: unmodifiable initializer
settings: inspection PublicStaticCollectionField enabled
language: java
```before
import java.util.*;
class A {
  public static final List<String> <caret>EVENTS = Collections.unmodifiableList(new ArrayList<>());
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-133 case 1: exec with concatenation
settings: inspection RuntimeExecWithNonConstantString enabled
language: java
```before
class A {
  void f(String i) throws Exception { Runtime.getRuntime().<caret>exec("foo" + i); }
}
```
keys: inspect
```after
problem: WARNING exec "Call to Runtime.exec() with non-constant argument"
```

### JAVA-ICC-133 case 2: constant
settings: inspection RuntimeExecWithNonConstantString enabled
language: java
```before
class A {
  void f() throws Exception { Runtime.getRuntime().<caret>exec("foo"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-134 case 1: missing writeObject
settings: inspection SerializableDeserializableClassInSecureContext enabled
language: java
```before
import java.io.*;
class <caret>DeserializableClass implements Serializable {
  private int sensitive = 736326;
  private void readObject(ObjectInputStream in) { throw new Error(); }
}
```
keys: Alt+Enter, choose "Add 'writeObject()' method which always throws an exception"
```after
import java.io.*;
class DeserializableClass implements Serializable {
  private int sensitive = 736326;
  private void readObject(ObjectInputStream in) { throw new Error(); }
  private void writeObject(java.io.ObjectOutputStream out) throws java.io.IOException {
    throw new java.io.NotSerializableException("DeserializableClass");
  }
}
```

### JAVA-ICC-134 case 2: both methods throw
settings: inspection SerializableDeserializableClassInSecureContext enabled
language: java
```before
import java.io.*;
class <caret>C implements Serializable {
  private void readObject(ObjectInputStream in) { throw new Error(); }
  private void writeObject(ObjectOutputStream out) { throw new Error(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-135 case 1: System.getProperties
settings: inspection SystemProperties enabled
language: java
```before
class A {
  Object f() { return System.<caret>getProperties(); }
}
```
keys: inspect
```after
problem: WARNING getProperties "Call to System.getProperties() may pose security concerns"
```

### JAVA-ICC-136 case 1: setSecurityManager
settings: inspection SystemSetSecurityManager enabled
language: java
```before
class A {
  void f() { System.<caret>setSecurityManager(null); }
}
```
keys: inspect
```after
problem: WARNING setSecurityManager "Call to System.setSecurityManager() may pose security concerns"
```

### JAVA-ICC-137 case 1: new Random
settings: inspection UnsecureRandomNumberGeneration enabled
language: java
```before
import java.util.Random;
class A {
  long f() { return new <caret>Random().nextLong(); }
}
```
keys: inspect
```after
problem: WARNING Random "For security purposes, use 'java.security.SecureRandom' instead of java.util.Random"
```

### JAVA-ICC-137 case 2: SecureRandom
settings: inspection UnsecureRandomNumberGeneration enabled
language: java
```before
import java.security.SecureRandom;
class A {
  long f() { return new <caret>SecureRandom().nextLong(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-138 case 1: comparator without Serializable
settings: inspection ComparatorNotSerializable enabled
language: java
```before
import java.util.Comparator;
class <caret>Foo implements Comparator<Object> {
  public int compare(Object a, Object b) { return 0; }
}
```
keys: inspect
```after
problem: WARNING Foo "Comparator class Foo is not declared as Serializable"
```

### JAVA-ICC-138 case 2: already Serializable
settings: inspection ComparatorNotSerializable enabled
language: java
```before
import java.util.Comparator;
class <caret>Foo implements Comparator<Object>, java.io.Serializable {
  public int compare(Object a, Object b) { return 0; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-139 case 1: only a constructor with parameters
settings: default
language: java
```before
import java.io.*;
class <caret>E implements Externalizable {
  E(int x) {}
  public void writeExternal(ObjectOutput o) {}
  public void readExternal(ObjectInput i) {}
}
```
keys: inspect
```after
problem: WARNING E "Externalizable class E has no 'public' no-arg constructor"
```

### JAVA-ICC-139 case 2: package-private no-arg constructor, make it public
settings: default
language: java
```before
import java.io.*;
class <caret>E implements Externalizable {
  E() {}
  public void writeExternal(ObjectOutput o) {}
  public void readExternal(ObjectInput i) {}
}
```
keys: Alt+Enter, choose "Make constructor 'public'"
```after
import java.io.*;
class E implements Externalizable {
  public E() {}
  public void writeExternal(ObjectOutput o) {}
  public void readExternal(ObjectInput i) {}
}
```

### JAVA-ICC-140 case 1: readObject in Externalizable
settings: inspection ExternalizableWithSerializationMethods enabled
language: java
```before
import java.io.*;
abstract class <caret>Crucial implements Externalizable {
  int value;
  private void readObject(ObjectInputStream in) throws IOException { value = in.readInt(); }
}
```
keys: inspect
```after
problem: WARNING Crucial "Externalizable class Crucial defines 'readObject()'"
```

### JAVA-ICC-140 case 2: no serialization methods
settings: inspection ExternalizableWithSerializationMethods enabled
language: java
```before
import java.io.*;
abstract class <caret>Crucial implements Externalizable { int value; }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-141 case 1: writeObject and serialVersionUID
settings: language level 17
status: unverified (fix text and annotation placement)
language: java
```before
import java.io.*;
class Main implements Serializable {
  private static final long <caret>serialVersionUID = 7874493593505141603L;
}
```
keys: Alt+Enter, choose "Annotate as '@Serial'"
```after
import java.io.*;
class Main implements Serializable {
  @Serial
  private static final long serialVersionUID = 7874493593505141603L;
}
```

### JAVA-ICC-141 case 2: already annotated
settings: language level 17
language: java
```before
import java.io.*;
class Main implements Serializable {
  @Serial private static final long <caret>serialVersionUID = 7874493593505141603L;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-142 case 1: non-serializable field
settings: inspection NonSerializableFieldInSerializableClass enabled
language: java
```before
import java.io.Serializable;
class NonSerializableClass {}
public class SerializableClass implements Serializable {
  NonSerializableClass <caret>clazz;
}
```
keys: inspect
```after
problem: WARNING clazz "Non-serializable field 'clazz' in Serializable class"
```

### JAVA-ICC-142 case 2: static field
settings: inspection NonSerializableFieldInSerializableClass enabled
language: java
```before
import java.io.Serializable;
class NonSerializableClass {}
public class SerializableClass implements Serializable {
  static NonSerializableClass <caret>staticClazz;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-143 case 1: setAttribute with a non-serializable value
settings: inspection NonSerializableObjectBoundToHttpSession enabled (servlet API on classpath)
language: java
```before
import javax.servlet.http.HttpSession;
class A {
  static class NonSerializable {}
  void foo(HttpSession session) { session.setAttribute("foo", <caret>new NonSerializable()); }
}
```
keys: inspect
```after
problem: WARNING new NonSerializable() "Non-serializable object bound to HttpSession"
```

### JAVA-ICC-144 case 1: writeObject of non-serializable
settings: inspection NonSerializableObjectPassedToObjectStream enabled
language: java
```before
import java.io.*;
public class IWantToSerializeThis {
  void f(ObjectOutputStream stream) throws IOException { stream.writeObject(<caret>new IWantToSerializeThis()); }
}
```
keys: inspect
```after
problem: WARNING new IWantToSerializeThis() "Non-serializable object passed to ObjectOutputStream"
```

### JAVA-ICC-144 case 2: String
settings: inspection NonSerializableObjectPassedToObjectStream enabled
language: java
```before
import java.io.*;
class A {
  void f(ObjectOutputStream stream) throws IOException { stream.writeObject(<caret>"s"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-145 case 1: class with readObject and writeObject
settings: inspection NonSerializableWithSerializationMethods enabled
language: java
```before
import java.io.*;
public class <caret>SampleClass {
  private void readObject(ObjectInputStream str) {}
  private void writeObject(ObjectOutputStream str) {}
}
```
keys: inspect
```after
problem: WARNING SampleClass "Non-serializable class SampleClass defines 'readObject()' and 'writeObject()'"
```

### JAVA-ICC-145 case 2: Serializable class
settings: inspection NonSerializableWithSerializationMethods enabled
language: java
```before
import java.io.*;
public class <caret>SampleClass implements Serializable {
  private void readObject(ObjectInputStream str) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-146 case 1: serialVersionUID in a non-serializable class
settings: default
language: java
```before
public class <caret>IWantToSerializeThis {
  private static final long serialVersionUID = 2669293150219020249L;
}
```
keys: inspect
```after
problem: WARNING IWantToSerializeThis "Non-serializable class IWantToSerializeThis defines a 'serialVersionUID' field"
```

### JAVA-ICC-146 case 2: Serializable class
settings: default
language: java
```before
public class <caret>C implements java.io.Serializable {
  private static final long serialVersionUID = 1L;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-147 case 1: public readObject
settings: inspection ReadObjectAndWriteObjectPrivate enabled
language: java
```before
import java.io.*;
public class Test implements Serializable {
  public void <caret>readObject(ObjectInputStream stream) { }
}
```
keys: Alt+Enter, choose "Make 'private'"
```after
import java.io.*;
public class Test implements Serializable {
  private void readObject(ObjectInputStream stream) { }
}
```

### JAVA-ICC-147 case 2: private
settings: inspection ReadObjectAndWriteObjectPrivate enabled
language: java
```before
import java.io.*;
public class Test implements Serializable {
  private void <caret>readObject(ObjectInputStream stream) { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-148 case 1: field not initialized in readObject
settings: inspection ReadObjectInitialization enabled
language: java
```before
import java.io.*;
class DataObject implements Serializable {
  String <caret>s;
  int i;
  private void readObject(ObjectInputStream stream) throws IOException { i = stream.readInt(); }
}
```
keys: inspect
```after
problem: WARNING s "Instance field s may not be initialized during 'readObject()' call"
```

### JAVA-ICC-148 case 2: transient field
settings: inspection ReadObjectInitialization enabled
language: java
```before
import java.io.*;
class DataObject implements Serializable {
  transient String <caret>s;
  int i;
  private void readObject(ObjectInputStream stream) throws IOException { i = stream.readInt(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-149 case 1: public writeReplace
settings: inspection ReadResolveAndWriteReplaceProtected enabled
language: java
```before
import java.io.*;
class ClassWithSerialization implements Serializable {
  public Object <caret>writeReplace() { return this; }
}
```
keys: Alt+Enter, choose "Make 'protected'"
```after
import java.io.*;
class ClassWithSerialization implements Serializable {
  protected Object writeReplace() { return this; }
}
```

### JAVA-ICC-149 case 2: protected
settings: inspection ReadResolveAndWriteReplaceProtected enabled
language: java
```before
import java.io.*;
class ClassWithSerialization implements Serializable {
  protected Object <caret>writeReplace() { return this; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-150 case 1: non-final serialVersionUID
settings: language level 17
status: unverified (exact range: annotation or member)
language: java
```before
import java.io.*;
class Test implements Serializable {
  @<caret>Serial
  private static long serialVersionUID = 7874493593505141603L;
}
```
keys: inspect
```after
problem: WARNING @Serial "Annotated member is not a part of the serialization mechanism"
```

### JAVA-ICC-150 case 2: valid use
settings: language level 17
language: java
```before
import java.io.*;
class Test implements Serializable {
  @<caret>Serial
  private static final long serialVersionUID = 7874493593505141603L;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-151 case 1: no methods
settings: inspection SerializableHasSerializationMethods enabled
language: java
```before
import java.io.*;
class <caret>A implements Serializable { int x; }
```
keys: inspect
```after
problem: WARNING A "Serializable class A does not define 'readObject()' or 'writeObject()'"
```

### JAVA-ICC-151 case 2: class without instance fields with option on
settings: inspection SerializableHasSerializationMethods enabled, Ignore classes that do not define instance fields = on
language: java
```before
import java.io.*;
class <caret>A implements Serializable { static int x; }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-152 case 1: add serialVersionUID
settings: inspection SerializableInnerClassHasSerialVersionUIDField enabled
status: unverified (value is generated from the class shape)
language: java
```before
import java.io.Serializable;
class Outer implements Serializable {
  class <caret>Inner implements Serializable {}
}
```
keys: Alt+Enter, choose "Add 'serialVersionUID' field"
```after
import java.io.Serializable;
class Outer implements Serializable {
  class Inner implements Serializable {
    private static final long serialVersionUID = <generated long>L;
  }
}
```

### JAVA-ICC-152 case 2: static nested class
settings: inspection SerializableInnerClassHasSerialVersionUIDField enabled
language: java
```before
import java.io.Serializable;
class Outer {
  static class <caret>Inner implements Serializable {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-153 case 1: serializable inner in non-serializable outer
settings: inspection SerializableInnerClassWithNonSerializableOuterClass enabled
language: java
```before
import java.io.Serializable;
class A {
  class <caret>Main implements Serializable { }
}
```
keys: inspect
```after
problem: WARNING Main "Inner class Main is serializable while its outer class is not"
```

### JAVA-ICC-153 case 2: static nested
settings: inspection SerializableInnerClassWithNonSerializableOuterClass enabled
language: java
```before
import java.io.Serializable;
class A {
  static class <caret>Main implements Serializable { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-154 case 1: writeObject in a record
settings: language level 17
language: java
```before
import java.io.*;
record R1() implements Serializable {
  private void <caret>writeObject(ObjectOutputStream out) throws IOException { }
}
```
keys: inspect
```after
problem: WARNING writeObject "writeObject() will be ignored during record serialization"
```

### JAVA-ICC-154 case 2: record without such members
settings: language level 17
language: java
```before
import java.io.*;
record R1(int <caret>x) implements Serializable { }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-155 case 1: anonymous class capturing a local
settings: inspection SerializableStoresNonSerializable enabled
language: java
```before
import java.io.Serializable;
interface A extends Serializable { void foo(); }
class B {}
class C {
  void foo() {
    B b = new B();
    A a = new A() { public void foo() { System.out.println(<caret>b); } };
  }
}
```
keys: inspect
```after
problem: WARNING b "Serializable anonymous class implicitly stores non-Serializable object of type 'B'"
```

### JAVA-ICC-155 case 2: captured String
settings: inspection SerializableStoresNonSerializable enabled
language: java
```before
import java.io.Serializable;
interface A extends Serializable { void foo(); }
class C {
  void foo() {
    String b = "x";
    A a = new A() { public void foo() { System.out.println(<caret>b); } };
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-156 case 1: ancestor without no-arg constructor
settings: inspection SerializableWithUnconstructableAncestor enabled
language: java
```before
import java.io.Serializable;
class Ancestor { Ancestor(String name) {} }
class <caret>Descendant extends Ancestor implements Serializable { Descendant() { super("Bob"); } }
```
keys: inspect
```after
problem: WARNING Descendant "Descendant has a non-serializable ancestor 'Ancestor' without no-arg constructor"
```

### JAVA-ICC-156 case 2: ancestor with no-arg constructor
settings: inspection SerializableWithUnconstructableAncestor enabled
language: java
```before
import java.io.Serializable;
class Ancestor { Ancestor() {} }
class <caret>Descendant extends Ancestor implements Serializable { }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-157 case 1: missing modifiers
settings: inspection SerialPersistentFieldsWithWrongSignature enabled
language: java
```before
import java.io.*;
class List implements Serializable {
  private List next;
  ObjectStreamField[] <caret>serialPersistentFields = {new ObjectStreamField("next", List.class)};
}
```
keys: inspect
```after
problem: WARNING serialPersistentFields "serialPersistentFields field of a Serializable class is not declared 'private static final ObjectStreamField[]'"
```

### JAVA-ICC-157 case 2: correct declaration
settings: inspection SerialPersistentFieldsWithWrongSignature enabled
language: java
```before
import java.io.*;
class List implements Serializable {
  private List next;
  private static final ObjectStreamField[] <caret>serialPersistentFields = {new ObjectStreamField("next", List.class)};
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-158 case 1: non-static field
settings: default
language: java
```before
import java.io.Serializable;
class SampleClass implements Serializable {
  private long <caret>serialVersionUID = 1;
}
```
keys: Alt+Enter, choose "Make serialVersionUID 'private static final'"
```after
import java.io.Serializable;
class SampleClass implements Serializable {
  private static final long serialVersionUID = 1;
}
```

### JAVA-ICC-158 case 2: correct field
settings: default
language: java
```before
import java.io.Serializable;
class SampleClass implements Serializable {
  private static final long <caret>serialVersionUID = 1;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-159 case 1: transient in non-serializable class
settings: inspection TransientFieldInNonSerializableClass enabled
language: java
```before
public class NonSerializableClass {
  private <caret>transient String password;
}
```
keys: Alt+Enter, choose "Remove 'transient' modifier"
```after
public class NonSerializableClass {
  private String password;
}
```

### JAVA-ICC-159 case 2: Serializable class
settings: inspection TransientFieldInNonSerializableClass enabled
language: java
```before
public class C implements java.io.Serializable {
  private <caret>transient String password;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-160 case 1: initialized transient without readObject
settings: inspection TransientFieldNotInitialized enabled
language: java
```before
import java.io.Serializable;
class Person implements Serializable {
  transient String <caret>name = "Default";
}
```
keys: inspect
```after
problem: WARNING name "Transient field name not initialized on deserialization"
```

### JAVA-ICC-160 case 2: default-null transient
settings: inspection TransientFieldNotInitialized enabled
language: java
```before
import java.io.Serializable;
class Person implements Serializable {
  transient String <caret>surname;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-161 case 1: assertTrue without message
settings: inspection AssertWithoutMessage enabled; JUnit 5 on classpath
language: java
```before
import static org.junit.jupiter.api.Assertions.*;
class T {
  boolean checkValid() { return true; }
  void t() { <caret>assertTrue(checkValid()); }
}
```
keys: Alt+Enter, choose "Add error message"
```after
import static org.junit.jupiter.api.Assertions.*;
class T {
  boolean checkValid() { return true; }
  void t() { assertTrue(checkValid(), "<caret>"); }
}
```

### JAVA-ICC-161 case 2: message present
settings: inspection AssertWithoutMessage enabled; JUnit 5 on classpath
language: java
```before
import static org.junit.jupiter.api.Assertions.*;
class T {
  void t() { <caret>assertTrue(true, "msg"); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-162 case 1: assertNotNull("foo")
settings: inspection ConstantAssertArgument enabled; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.*;
class T {
  void t() { assertNotNull(<caret>"foo"); }
}
```
keys: inspect
```after
problem: WARNING "foo" "Argument "foo" is constant"
```

### JAVA-ICC-162 case 2: variable argument
settings: inspection ConstantAssertArgument enabled; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.*;
class T {
  void t(Object o) { assertNotNull(<caret>o); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-163 case 1: assert in try/catch AssertionError
settings: default
status: unverified (the {0} text and range)
language: java
```before
class A {
  void javaAssertion() {
    try { <caret>assert 1 == 2; } catch (AssertionError e) { }
  }
}
```
keys: inspect
```after
problem: WARNING assert 1 == 2; "'assert' cannot fail as it's suppressed by a surrounding 'catch'"
```

### JAVA-ICC-163 case 2: catch of another exception
settings: default
language: java
```before
class A {
  void javaAssertion() {
    try { <caret>assert 1 == 2; } catch (IllegalStateException e) { }
  }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-164 case 1: actual first
settings: default; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.assertEquals;
class T {
  int actual() { return 1; }
  void t() { <caret>assertEquals(actual(), 1); }
}
```
keys: Alt+Enter, choose "Flip compared arguments"
```after
import static org.junit.Assert.assertEquals;
class T {
  int actual() { return 1; }
  void t() { assertEquals(1, actual()); }
}
```

### JAVA-ICC-164 case 2: expected first
settings: default; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.assertEquals;
class T {
  int actual() { return 1; }
  void t() { <caret>assertEquals(1, actual()); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-165 case 1: assertEquals(true, x)
settings: default; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.*;
class T {
  boolean x() { return true; }
  void t() { <caret>assertEquals(true, x()); }
}
```
keys: Alt+Enter, choose "Simplify assertion"
```after
import static org.junit.Assert.*;
class T {
  boolean x() { return true; }
  void t() { assertTrue(x()); }
}
```

### JAVA-ICC-165 case 2: assertEquals on objects
settings: default; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.*;
class T {
  void t(Object a, Object b) { <caret>assertEquals(a, b); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-166 case 1: SimpleDateFormat static field
settings: inspection AccessToNonThreadSafeStaticFieldFromInstance enabled
status: unverified (default non-thread-safe list; {0} text may be qualified)
language: java
```before
import java.text.SimpleDateFormat;
class Sample {
  private static final SimpleDateFormat df = new SimpleDateFormat("yyyy-MM-dd");
  String method() { return <caret>df.format(new java.util.Date()); }
}
```
keys: inspect
```after
problem: WARNING df "Access to non-thread-safe static field df of type 'SimpleDateFormat'"
```

### JAVA-ICC-166 case 2: thread-safe type
settings: inspection AccessToNonThreadSafeStaticFieldFromInstance enabled
language: java
```before
class Sample {
  private static final StringBuffer sb = new StringBuffer();
  String method() { return <caret>sb.toString(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-167 case 1: static field under synchronized(this)
settings: inspection AccessToStaticFieldLockedOnInstance enabled
language: java
```before
class A {
  static String test;
  public void foo() { synchronized (this) { System.out.println(<caret>test); } }
}
```
keys: inspect
```after
problem: WARNING test "Access to static field test locked on instance data"
```

### JAVA-ICC-167 case 2: lock on a static lock
settings: inspection AccessToStaticFieldLockedOnInstance enabled
language: java
```before
class A {
  static String test;
  static final Object LOCK = new Object();
  public void foo() { synchronized (LOCK) { System.out.println(<caret>test); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-168 case 1: static volatile field
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicIntegerFieldUpdater;
class A {
  private static volatile int value = 0;
  private static final AtomicIntegerFieldUpdater<A> updater = AtomicIntegerFieldUpdater.newUpdater(A.class, <caret>"value");
}
```
keys: inspect
```after
problem: WARNING "value" "Field 'value' has 'static' modifier"
```

### JAVA-ICC-168 case 2: missing field
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicIntegerFieldUpdater;
class B {
  private static final AtomicIntegerFieldUpdater<B> updater = AtomicIntegerFieldUpdater.newUpdater(B.class, <caret>"value");
}
```
keys: inspect
```after
problem: WARNING "value" "No field named 'value' found in class 'B'"
```

### JAVA-ICC-168 case 3: valid declaration
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicIntegerFieldUpdater;
class C {
  volatile int value;
  private static final AtomicIntegerFieldUpdater<C> updater = AtomicIntegerFieldUpdater.newUpdater(C.class, <caret>"value");
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-169 case 1: instance field
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicIntegerFieldUpdater;
class Main {
  private volatile int id;
  private AtomicIntegerFieldUpdater<Main> <caret>idFieldUpdater = AtomicIntegerFieldUpdater.newUpdater(Main.class, "id");
}
```
keys: Alt+Enter, choose "Make 'idFieldUpdater' static final"
```after
import java.util.concurrent.atomic.AtomicIntegerFieldUpdater;
class Main {
  private volatile int id;
  private static final AtomicIntegerFieldUpdater<Main> idFieldUpdater = AtomicIntegerFieldUpdater.newUpdater(Main.class, "id");
}
```

### JAVA-ICC-169 case 2: already static final
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicIntegerFieldUpdater;
class Main {
  private volatile int id;
  private static final AtomicIntegerFieldUpdater<Main> <caret>idFieldUpdater = AtomicIntegerFieldUpdater.newUpdater(Main.class, "id");
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-170 case 1: await outside a loop
settings: inspection AwaitNotInLoop enabled
language: java
```before
import java.util.concurrent.locks.Condition;
class A {
  void acquire(Condition released) throws InterruptedException { released.<caret>await(); }
}
```
keys: inspect
```after
problem: WARNING await "Call to await() is not in loop"
```

### JAVA-ICC-170 case 2: await in a while loop
settings: inspection AwaitNotInLoop enabled
language: java
```before
import java.util.concurrent.locks.Condition;
class A {
  boolean acquired;
  void acquire(Condition released) throws InterruptedException { while (acquired) { released.<caret>await(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-171 case 1: field condition never signalled
settings: inspection AwaitWithoutCorrespondingSignal enabled
language: java
```before
import java.util.concurrent.locks.*;
class Queue {
  private final Condition isEmpty = new ReentrantLock().newCondition();
  void remove() throws InterruptedException { isEmpty.<caret>await(); }
}
```
keys: inspect
```after
problem: WARNING await "Call to await() without corresponding signal() or signalAll()"
```

### JAVA-ICC-171 case 2: signalled elsewhere
settings: inspection AwaitWithoutCorrespondingSignal enabled
language: java
```before
import java.util.concurrent.locks.*;
class Queue {
  private final Condition isEmpty = new ReentrantLock().newCondition();
  void add() { isEmpty.signal(); }
  void remove() throws InterruptedException { isEmpty.<caret>await(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-172 case 1: sleep in a while loop
settings: default
language: java
```before
class X {
  volatile int x;
  public void waitX() throws Exception { while (x > 0) { Thread.<caret>sleep(10); } }
}
```
keys: inspect
```after
problem: WARNING sleep "Call to Thread.sleep() in a loop, probably busy-waiting"
```

### JAVA-ICC-172 case 2: sleep outside a loop
settings: default
language: java
```before
class X {
  public void pause() throws Exception { Thread.<caret>sleep(10); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-173 case 1: native call in a synchronized block
settings: inspection CallToNativeMethodWhileLocked enabled
language: java
```before
class A {
  final Object lock = new Object();
  native void nativeMethod();
  void example() { synchronized (lock) { <caret>nativeMethod(); } }
}
```
keys: inspect
```after
problem: WARNING nativeMethod "Call to native method nativeMethod() in a synchronized context"
```

### JAVA-ICC-173 case 2: no lock
settings: inspection CallToNativeMethodWhileLocked enabled
language: java
```before
class A {
  native void nativeMethod();
  void example() { <caret>nativeMethod(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-174 case 1: signal to signalAll
settings: inspection ConditionSignal enabled
language: java
```before
import java.util.concurrent.locks.Condition;
class A {
  void f(Condition c) { c.<caret>signal(); }
}
```
keys: Alt+Enter, choose "Replace method call with 'signalAll()'"
```after
import java.util.concurrent.locks.Condition;
class A {
  void f(Condition c) { c.signalAll(); }
}
```

### JAVA-ICC-174 case 2: signalAll
settings: inspection ConditionSignal enabled
language: java
```before
import java.util.concurrent.locks.Condition;
class A {
  void f(Condition c) { c.<caret>signalAll(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-175 case 1: non-volatile field, make volatile
settings: default
language: java
```before
class Foo {
  private Helper helper = null;
  public Helper getHelper() {
    <caret>if (helper == null) synchronized (this) { if (helper == null) helper = new Helper(); }
    return helper;
  }
  static class Helper {}
}
```
keys: Alt+Enter, choose "Make 'helper' volatile"
```after
class Foo {
  private volatile Helper helper = null;
  public Helper getHelper() {
    if (helper == null) synchronized (this) { if (helper == null) helper = new Helper(); }
    return helper;
  }
  static class Helper {}
}
```

### JAVA-ICC-175 case 2: single check
settings: default
language: java
```before
class Foo {
  private Helper helper = null;
  public synchronized Helper getHelper() {
    <caret>if (helper == null) helper = new Helper();
    return helper;
  }
  static class Helper {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-176 case 1: empty block
settings: default
language: java
```before
class A {
  final Object lock = new Object();
  void f() { <caret>synchronized (lock) {} }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'synchronized'"
```after
class A {
  final Object lock = new Object();
  void f() { }
}
```

### JAVA-ICC-176 case 2: non-empty block
settings: default
language: java
```before
class A {
  final Object lock = new Object();
  void f() { <caret>synchronized (lock) { System.out.println(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-177 case 1: class extends Thread
settings: inspection ExtendsThread enabled
language: java
```before
class <caret>MainThread extends Thread { }
```
keys: inspect
```after
problem: WARNING MainThread "Class MainThread directly extends 'java.lang.Thread'"
```

### JAVA-ICC-177 case 2: implements Runnable
settings: inspection ExtendsThread enabled
language: java
```before
class <caret>Main implements Runnable { public void run() {} }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-178 case 1: mixed access
settings: inspection FieldAccessedSynchronizedAndUnsynchronized enabled
language: java
```before
public class Program {
  Console <caret>console;
  public synchronized void execute() { console.print("running"); }
  public void check() { console.check(); }
  static class Console { void print(String s) {} void check() {} }
}
```
keys: inspect
```after
problem: WARNING console "Field console is accessed in both synchronized and unsynchronized contexts"
```

### JAVA-ICC-178 case 2: volatile field
settings: inspection FieldAccessedSynchronizedAndUnsynchronized enabled
language: java
```before
public class Program {
  volatile Console <caret>console;
  public synchronized void execute() { console.print("running"); }
  public void check() { console.check(); }
  static class Console { void print(String s) {} void check() {} }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-179 case 1: whole body in synchronized(this)
settings: inspection MethodMayBeSynchronized enabled
language: java
```before
class A {
  public int <caret>generateInt(int x) { synchronized (this) { return 1; } }
}
```
keys: Alt+Enter, choose "Make method synchronized and remove synchronized block"
```after
class A {
  public synchronized int generateInt(int x) { return 1; }
}
```

### JAVA-ICC-179 case 2: other statements in the body
settings: inspection MethodMayBeSynchronized enabled
language: java
```before
class A {
  public int <caret>generateInt(int x) { int y = x; synchronized (this) { return y; } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-180 case 1: notify without state change
settings: inspection NakedNotify enabled
language: java
```before
class A {
  void f() { synchronized (this) { <caret>notify(); } }
}
```
keys: inspect
```after
problem: WARNING notify "Call to notify() without corresponding state change"
```

### JAVA-ICC-180 case 2: state change before
settings: inspection NakedNotify enabled
language: java
```before
class A {
  boolean ready;
  void f() { synchronized (this) { ready = true; <caret>notify(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-181 case 1: nested blocks
settings: inspection NestedSynchronizedStatement enabled
language: java
```before
class A {
  final Object a = new Object(), b = new Object();
  void f() { synchronized (a) { <caret>synchronized (b) { } } }
}
```
keys: inspect
```after
problem: WARNING synchronized "Nested synchronized statement"
```

### JAVA-ICC-181 case 2: sequential blocks
settings: inspection NestedSynchronizedStatement enabled
language: java
```before
class A {
  final Object a = new Object(), b = new Object();
  void f() { synchronized (a) { } <caret>synchronized (b) { } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-182 case 1: v = 2 * v
settings: default
language: java
```before
class A {
  private volatile int v = 1;
  void foo() { <caret>v = 2 * v; }
}
```
keys: inspect
```after
problem: WARNING v "Non-atomic operation on volatile field v"
```

### JAVA-ICC-182 case 2: inside synchronized
settings: default
language: java
```before
class A {
  private volatile int v = 1;
  synchronized void foo() { <caret>v = 2 * v; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-183 case 1: unsynchronized override
settings: inspection NonSynchronizedMethodOverridesSynchronizedMethod enabled
language: java
```before
class Super { synchronized void process() {} }
class Sub extends Super {
  void <caret>process() {}
}
```
keys: inspect
```after
problem: WARNING process "Unsynchronized method process() overrides synchronized method"
```

### JAVA-ICC-183 case 2: synchronized override
settings: inspection NonSynchronizedMethodOverridesSynchronizedMethod enabled
language: java
```before
class Super { synchronized void process() {} }
class Sub extends Super {
  synchronized void <caret>process() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-184 case 1: notifyAll on Condition
settings: inspection NotifyCalledOnCondition enabled
language: java
```before
import java.util.concurrent.locks.*;
class C {
  final Lock l = new ReentrantLock();
  final Condition c = l.newCondition();
  void release() { c.<caret>notifyAll(); }
}
```
keys: inspect
```after
problem: WARNING notifyAll "Call to notifyAll() on Condition object"
```

### JAVA-ICC-184 case 2: signalAll
settings: inspection NotifyCalledOnCondition enabled
language: java
```before
import java.util.concurrent.locks.*;
class C {
  final Lock l = new ReentrantLock();
  final Condition c = l.newCondition();
  void release() { c.<caret>signalAll(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-185 case 1: notify on a field never waited on
settings: inspection NotifyWithoutCorrespondingWait enabled
language: java
```before
class A {
  final Object synList = new Object();
  void f() { synchronized (synList) { synList.<caret>notify(); } }
}
```
keys: inspect
```after
problem: WARNING notify "Call to notify() without corresponding wait()"
```

### JAVA-ICC-185 case 2: wait exists
settings: inspection NotifyWithoutCorrespondingWait enabled
language: java
```before
class A {
  final Object synList = new Object();
  void f() { synchronized (synList) { synList.<caret>notify(); } }
  void g() throws InterruptedException { synchronized (synList) { synList.wait(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-186 case 1: notify to notifyAll
settings: inspection ObjectNotify enabled
language: java
```before
class A {
  synchronized void f() { <caret>notify(); }
}
```
keys: Alt+Enter, choose "Replace method call with 'notifyAll()'"
```after
class A {
  synchronized void f() { notifyAll(); }
}
```

### JAVA-ICC-186 case 2: notifyAll
settings: inspection ObjectNotify enabled
language: java
```before
class A {
  synchronized void f() { <caret>notifyAll(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-187 case 1: public field in synchronized method
settings: inspection PublicFieldAccessedInSynchronizedContext enabled
language: java
```before
class Bar { public String field1; }
class A {
  public Bar myBar;
  synchronized public void sample() { myBar.<caret>field1 = "bar"; }
}
```
keys: inspect
```after
problem: WARNING field1 "Non-private field field1 accessed in synchronized context"
```

### JAVA-ICC-187 case 2: private field
settings: inspection PublicFieldAccessedInSynchronizedContext enabled
language: java
```before
class A {
  private String <caret>f;
  synchronized public void sample() { f = "bar"; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-188 case 1: lock without try/finally
settings: inspection SafeLock enabled
status: unverified ({0} text and range)
language: java
```before
import java.util.concurrent.locks.Lock;
class A {
  Lock lock;
  void f() { lock.<caret>lock(); System.out.println(); lock.unlock(); }
}
```
keys: inspect
```after
problem: WARNING lock "'Lock' should be locked in front of a 'try' block and unlocked in the corresponding 'finally' block"
```

### JAVA-ICC-188 case 2: lock before try with finally unlock
settings: inspection SafeLock enabled
language: java
```before
import java.util.concurrent.locks.Lock;
class A {
  Lock lock;
  void f() { lock.<caret>lock(); try { System.out.println(); } finally { lock.unlock(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-189 case 1: random captured by a lambda
settings: inspection SharedThreadLocalRandom enabled
status: unverified (exact range)
language: java
```before
import java.util.concurrent.*;
class Main {
  void f() {
    ThreadLocalRandom random = ThreadLocalRandom.<caret>current();
    CompletableFuture.supplyAsync(() -> random.nextInt());
  }
}
```
keys: inspect
```after
problem: WARNING current "'ThreadLocalRandom' instance might be shared between threads"
```

### JAVA-ICC-189 case 2: direct use
settings: inspection SharedThreadLocalRandom enabled
language: java
```before
import java.util.concurrent.*;
class Main {
  int f() { return ThreadLocalRandom.<caret>current().nextInt(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-190 case 1: signal with no await
settings: inspection SignalWithoutCorrespondingAwait enabled
language: java
```before
import java.util.concurrent.locks.*;
class Queue {
  private final Condition isEmpty = new ReentrantLock().newCondition();
  void add() { isEmpty.<caret>signal(); }
}
```
keys: inspect
```after
problem: WARNING signal "Call to signal() without corresponding await()"
```

### JAVA-ICC-190 case 2: await exists
settings: inspection SignalWithoutCorrespondingAwait enabled
language: java
```before
import java.util.concurrent.locks.*;
class Queue {
  private final Condition isEmpty = new ReentrantLock().newCondition();
  void add() { isEmpty.<caret>signal(); }
  void remove() throws InterruptedException { isEmpty.await(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-191 case 1: sleep in synchronized block
settings: inspection SleepWhileHoldingLock enabled
language: java
```before
class A {
  final Object lock = new Object();
  void f() throws Exception { synchronized (lock) { Thread.<caret>sleep(100); } }
}
```
keys: inspect
```after
problem: WARNING sleep "Call to Thread.sleep() while synchronized"
```

### JAVA-ICC-191 case 2: sleep outside
settings: inspection SleepWhileHoldingLock enabled
language: java
```before
class A {
  void f() throws Exception { Thread.<caret>sleep(100); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-192 case 1: parent references child
settings: default
language: java
```before
class Parent { static final Child field = new <caret>Child(); }
class Child extends Parent { }
```
keys: inspect
```after
problem: WARNING Child "Referencing subclass Child from superclass Parent initializer might lead to class loading deadlock"
```

### JAVA-ICC-192 case 2: no subclass reference
settings: default
language: java
```before
class Parent { static final Object field = new <caret>Object(); }
class Child extends Parent { }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-193 case 1: synchronized(getClass())
settings: default
language: java
```before
class A {
  void f() { synchronized (<caret>getClass()) {} }
}
```
keys: inspect
```after
problem: WARNING getClass "Synchronization on getClass()"
```

### JAVA-ICC-193 case 2: class literal
settings: default
language: java
```before
class A {
  void f() { synchronized (<caret>A.class) {} }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-194 case 1: local variable
settings: default
language: java
```before
class A {
  void bar() { final Object lock = new Object(); synchronized (<caret>lock) { } }
}
```
keys: inspect
```after
problem: WARNING lock "Synchronization on local variable lock"
```

### JAVA-ICC-194 case 2: field
settings: default
language: java
```before
class A {
  final Object lock = new Object();
  void bar() { synchronized (<caret>lock) { } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-195 case 1: static lock
settings: inspection SynchronizationOnStaticField enabled
language: java
```before
class A {
  static final Object LOCK = new Object();
  void f() { synchronized (<caret>LOCK) { } }
}
```
keys: inspect
```after
problem: WARNING LOCK "Synchronization on 'static' field LOCK"
```

### JAVA-ICC-195 case 2: instance field
settings: inspection SynchronizationOnStaticField enabled
language: java
```before
class A {
  final Object lock = new Object();
  void f() { synchronized (<caret>lock) { } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-196 case 1: move synchronization into method
settings: inspection SynchronizedMethod enabled
language: java
```before
class Main {
  public <caret>synchronized void fooBar() { System.out.println(); }
}
```
keys: Alt+Enter, choose "Move synchronization into method"
```after
class Main {
  public void fooBar() {
    synchronized (this) {
      System.out.println();
    }
  }
}
```

### JAVA-ICC-196 case 2: unsynchronized method
settings: inspection SynchronizedMethod enabled
language: java
```before
class Main {
  public void <caret>fooBar() { }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-197 case 1: field initialised with a string literal
settings: inspection SynchronizedOnLiteralObject enabled
language: java
```before
class Main {
  final String mutex = "Mutex";
  void method() { synchronized (<caret>mutex) { } }
}
```
keys: inspect
```after
problem: WARNING mutex "Synchronization on String mutex which is initialized by a literal"
```

### JAVA-ICC-197 case 2: plain Object lock
settings: inspection SynchronizedOnLiteralObject enabled
language: java
```before
class Main {
  final Object mutex = new Object();
  void method() { synchronized (<caret>mutex) { } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-198 case 1: synchronized on a ReentrantLock
settings: inspection SynchronizeOnLock enabled
language: java
```before
import java.util.concurrent.locks.ReentrantLock;
class A {
  final ReentrantLock lock = new ReentrantLock();
  public void foo() { synchronized (<caret>lock) {} }
}
```
keys: inspect
```after
problem: WARNING lock "Synchronization on a 'ReentrantLock' object is unlikely to be intentional"
```

### JAVA-ICC-198 case 2: plain object
settings: inspection SynchronizeOnLock enabled
language: java
```before
class A {
  final Object lock = new Object();
  public void foo() { synchronized (<caret>lock) {} }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-199 case 1: non-final lock field
settings: default
language: java
```before
class A {
  private Object o;
  public void foo() { synchronized (<caret>o) { } }
}
```
keys: inspect
```after
problem: WARNING o "Synchronization on a non-final field o"
```

### JAVA-ICC-199 case 2: final lock field
settings: default
language: java
```before
class A {
  private final Object o = new Object();
  public void foo() { synchronized (<caret>o) { } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-200 case 1: synchronized(this)
settings: inspection SynchronizeOnThis enabled
language: java
```before
class A {
  public void print() { synchronized (<caret>this) { System.out.println("synchronized"); } }
}
```
keys: inspect
```after
problem: WARNING this "Lock operations on 'this' may have unforeseen side-effects"
```

### JAVA-ICC-200 case 2: private lock object
settings: inspection SynchronizeOnThis enabled
language: java
```before
class A {
  private final Object lock = new Object();
  public void print() { synchronized (<caret>lock) { } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-201 case 1: call at language level 8
settings: language level 8
status: unverified (the method does not resolve in a JDK 11+ SDK; needs a JDK 8 class library)
language: java
```before
class A {
  void f() { System.<caret>runFinalizersOnExit(true); }
}
```
keys: inspect
```after
problem: WARNING runFinalizersOnExit "Call to System.runFinalizersOnExit()"
```

### JAVA-ICC-202 case 1: instance ThreadLocal
settings: inspection ThreadLocalNotStaticFinal enabled
language: java
```before
class A {
  private ThreadLocal<Boolean> <caret>tl = ThreadLocal.withInitial(() -> Boolean.TRUE);
}
```
keys: Alt+Enter, choose "Make 'tl' static final"
```after
class A {
  private static final ThreadLocal<Boolean> tl = ThreadLocal.withInitial(() -> Boolean.TRUE);
}
```

### JAVA-ICC-202 case 2: static final
settings: inspection ThreadLocalNotStaticFinal enabled
language: java
```before
class A {
  private static final ThreadLocal<Boolean> <caret>tl = ThreadLocal.withInitial(() -> Boolean.TRUE);
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-203 case 1: set(null)
settings: default
language: java
```before
class A {
  void f(ThreadLocal<Integer> threadLocal) { threadLocal.<caret>set(null); }
}
```
keys: Alt+Enter, choose "Replace with 'ThreadLocal.remove()' call"
```after
class A {
  void f(ThreadLocal<Integer> threadLocal) { threadLocal.remove(); }
}
```

### JAVA-ICC-203 case 2: set with a value
settings: default
language: java
```before
class A {
  void f(ThreadLocal<Integer> threadLocal) { threadLocal.<caret>set(1); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-204 case 1: setPriority
settings: inspection ThreadPriority enabled
language: java
```before
class A {
  void f(Thread t) { t.<caret>setPriority(Thread.MAX_PRIORITY); }
}
```
keys: inspect
```after
problem: WARNING setPriority "Call to Thread.setPriority()"
```

### JAVA-ICC-205 case 1: start() in constructor
settings: inspection ThreadStartInConstruction enabled
language: java
```before
class MyThread extends Thread {
  MyThread() { <caret>start(); }
}
```
keys: inspect
```after
problem: WARNING start "Call to start() during object construction"
```

### JAVA-ICC-205 case 2: final class
settings: inspection ThreadStartInConstruction enabled
language: java
```before
final class MyThread extends Thread {
  MyThread() { <caret>start(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-206 case 1: Thread.stop
settings: inspection ThreadStopSuspendResume enabled; language level 8
language: java
```before
class A {
  void f(Thread t) { t.<caret>stop(); }
}
```
keys: inspect
```after
problem: WARNING stop "Call to Thread.stop()"
```

### JAVA-ICC-207 case 1: new Thread().start()
settings: default
language: java
```before
class A {
  void f() { new <caret>Thread().start(); }
}
```
keys: inspect
```after
problem: WARNING Thread "Instantiating a Thread with default 'run()' method"
```

### JAVA-ICC-207 case 2: Runnable argument
settings: default
language: java
```before
class A {
  void f(Runnable r) { new <caret>Thread(r).start(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-208 case 1: Thread.yield
settings: inspection ThreadYield enabled
language: java
```before
class A {
  void f() { Thread.<caret>yield(); }
}
```
keys: inspect
```after
problem: WARNING yield "Call to Thread.yield()"
```

### JAVA-ICC-209 case 1: wait() first in synchronized block
settings: inspection UnconditionalWait enabled
language: java
```before
class Bar {
  void foo() throws InterruptedException { synchronized (this) { <caret>wait(); } }
}
```
keys: inspect
```after
problem: WARNING wait "Unconditional call to wait()"
```

### JAVA-ICC-209 case 2: wait guarded by a condition
settings: inspection UnconditionalWait enabled
language: java
```before
class Bar {
  boolean ready;
  void foo() throws InterruptedException { synchronized (this) { while (!ready) { <caret>wait(); } } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-210 case 1: volatile int[]
settings: inspection VolatileArrayField enabled
language: java
```before
class Data {
  private volatile int[] <caret>idx = new int[0];
}
```
keys: inspect
```after
problem: WARNING idx "Volatile field idx of type 'int[]'"
```

### JAVA-ICC-210 case 2: volatile int
settings: inspection VolatileArrayField enabled
language: java
```before
class Data {
  private volatile int <caret>idx;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-211 case 1: wait on Condition
settings: inspection WaitCalledOnCondition enabled
language: java
```before
import java.util.concurrent.locks.Condition;
class A {
  boolean acquired;
  void acquire(Condition released) throws InterruptedException { while (acquired) { released.<caret>wait(); } }
}
```
keys: inspect
```after
problem: WARNING wait "Call to wait() on Condition object"
```

### JAVA-ICC-211 case 2: await
settings: inspection WaitCalledOnCondition enabled
language: java
```before
import java.util.concurrent.locks.Condition;
class A {
  boolean acquired;
  void acquire(Condition released) throws InterruptedException { while (acquired) { released.<caret>await(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-212 case 1: wait on lock while holding this
settings: inspection WaitNotifyNotInSynchronizedContext enabled
status: unverified (#ref text for qualified calls)
language: java
```before
class Sync {
  private final Object lock = new Object();
  void test() throws InterruptedException { synchronized (this) { lock.<caret>wait(); } }
}
```
keys: inspect
```after
problem: WARNING wait "Call to lock.wait while not synchronized on 'lock'"
```

### JAVA-ICC-212 case 2: synchronized on the lock
settings: inspection WaitNotifyNotInSynchronizedContext enabled
language: java
```before
class Sync {
  private final Object lock = new Object();
  void test() throws InterruptedException { synchronized (lock) { lock.<caret>wait(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-213 case 1: wait under if
settings: inspection WaitNotInLoop enabled
language: java
```before
class BoundedCounter {
  private int count;
  synchronized void inc() throws InterruptedException { if (count >= 10) <caret>wait(); ++count; }
}
```
keys: inspect
```after
problem: WARNING wait "Call to wait() is not in loop"
```

### JAVA-ICC-213 case 2: wait in while
settings: inspection WaitNotInLoop enabled
language: java
```before
class BoundedCounter {
  private int count;
  synchronized void inc() throws InterruptedException { while (count >= 10) <caret>wait(); ++count; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-214 case 1: wait() without timeout
settings: inspection WaitOrAwaitWithoutTimeout enabled
language: java
```before
class A {
  void foo(Object bar) throws InterruptedException { bar.<caret>wait(); }
}
```
keys: inspect
```after
problem: WARNING wait "wait without timeout"
```

### JAVA-ICC-214 case 2: wait(timeout)
settings: inspection WaitOrAwaitWithoutTimeout enabled
language: java
```before
class A {
  void foo(Object bar) throws InterruptedException { bar.<caret>wait(100); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-215 case 1: wait inside nested synchronized
settings: default
language: java
```before
class A {
  final Object lockA = new Object(), lockB = new Object();
  void f() throws InterruptedException { synchronized (lockA) { synchronized (lockB) { lockB.<caret>wait(); } } }
}
```
keys: inspect
```after
problem: WARNING wait "Call to wait() is made while holding two locks"
```

### JAVA-ICC-215 case 2: single lock
settings: default
language: java
```before
class A {
  final Object lockB = new Object();
  void f() throws InterruptedException { synchronized (lockB) { lockB.<caret>wait(); } }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-216 case 1: wait on field without notify
settings: inspection WaitWithoutCorrespondingNotify enabled
language: java
```before
public class Foo {
  public Object foo = new Object();
  void bar() throws InterruptedException { this.foo.<caret>wait(); }
}
```
keys: inspect
```after
problem: WARNING wait "Call to wait() without corresponding notify() or notifyAll()"
```

### JAVA-ICC-216 case 2: notify exists
settings: inspection WaitWithoutCorrespondingNotify enabled
language: java
```before
public class Foo {
  public Object foo = new Object();
  void bar() throws InterruptedException { this.foo.<caret>wait(); }
  void baz() { this.foo.notify(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICC-217 case 1: spin loop on a non-volatile field
settings: default
language: java
```before
class SpinsOnField {
  boolean ready = false;
  void run() { <caret>while (!ready) { } }
  void markAsReady() { ready = true; }
}
```
keys: Alt+Enter, choose "Make 'ready' volatile and add Thread.onSpinWait()"
```after
class SpinsOnField {
  volatile boolean ready = false;
  void run() { while (!ready) { Thread.onSpinWait(); } }
  void markAsReady() { ready = true; }
}
```

### JAVA-ICC-217 case 2: loop with a body doing work
settings: default
language: java
```before
class SpinsOnField {
  boolean ready = false;
  void run() { <caret>while (!ready) { System.out.println("x"); } }
}
```
keys: inspect
```after
no problem reported
```
