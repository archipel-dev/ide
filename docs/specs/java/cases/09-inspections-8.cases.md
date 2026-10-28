# Cases for 09-inspections-catalog-8 (JAVA-ICC)

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-8.md`. Keys `inspect` list the expected highlights as `problem:` lines, `inspect batch` runs the whole-project inspection, and `no problem reported` means no highlight.

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
problem: warning `URLClassLoader` "Instantiation of URLClassLoader may pose security concerns"
```

### JAVA-ICC-123 case 1: Cloneable class
settings: inspection CloneableClassInSecureContext enabled
language: java
```before
class <caret>BakeryBean implements Cloneable {}
```
keys: Alt+Enter, choose "Generate 'clone()' method which always throws exception"
```after
class BakeryBean {}
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
problem: warning `MyLoader` "Custom ClassLoader class MyLoader"
```

### JAVA-ICC-125 case 1: subclass of SecurityManager
settings: inspection CustomSecurityManager enabled
language: java
```before
class <caret>CustomSecurityManager extends SecurityManager {}
```
keys: inspect
```after
problem: warning `CustomSecurityManager` "Custom SecurityManager class CustomSecurityManager"
```

### JAVA-ICC-126 case 1: overridable method gets final
settings: inspection DesignForExtension enabled
language: java
```before
class Canyon {
  public int <caret>depth() { return 42; }
}
```
keys: Alt+Enter, choose "Make method 'depth()' 'final'"
```after
class Canyon {
  public final int depth() { return 42; }
}
```

### JAVA-ICC-126 case 2: private and empty methods
settings: inspection DesignForExtension enabled
language: java
```before
class Quarry {
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
problem: warning `executeQuery` "Call to Statement.executeQuery() with non-constant argument"
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
problem: warning `prepareStatement` "Call to Connection.prepareStatement() with non-constant argument"
```

### JAVA-ICC-128 case 2: static final field with option on
settings: inspection JDBCPrepareStatementWithNonConstantString enabled, Consider 'static final' fields constant = on
language: java
```before
import java.sql.*;
class A {
  static final String MIN_LEVEL = "SELECT " + System.getenv("X");
  Object f(Connection c) throws SQLException { return c.<caret>prepareStatement(MIN_LEVEL); }
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
  void load(String name) { System.<caret>loadLibrary("native" + name); }
}
```
keys: inspect
```after
problem: warning `loadLibrary` "Call to System.loadLibrary() with non-constant argument"
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
class Recipe implements Cloneable {
  @Override protected Object <caret>clone() throws CloneNotSupportedException { return super.clone(); }
}
```
keys: inspect
```after
problem: warning `clone` "Non-final clone() method, compromising security"
```

### JAVA-ICC-130 case 2: final class
settings: inspection NonFinalClone enabled
language: java
```before
final class Telescope implements Cloneable {
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
  public static String[] <caret>crew = {"foo", "bar"};
}
```
keys: inspect
```after
problem: warning `crew` "'public static' array field crew, compromising security"
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
  public static final List<String> <caret>HEADROOM = new ArrayList<>();
}
```
keys: inspect
```after
problem: warning `HEADROOM` "'public static' collection field HEADROOM, compromising security"
```

### JAVA-ICC-132 case 2: unmodifiable initializer
settings: inspection PublicStaticCollectionField enabled
language: java
```before
import java.util.*;
class A {
  public static final List<String> <caret>GRID_SIZE = Collections.unmodifiableList(new ArrayList<>());
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
problem: warning `exec` "Call to Runtime.exec() with non-constant argument"
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
class <caret>Otter implements Serializable {
  private int width = 90211;
  private void readObject(ObjectInputStream in) { throw new Error(); }
}
```
keys: Alt+Enter, choose "Add 'writeObject()' method which always throws an exception"
```after
import java.io.*;
class Otter implements Serializable {
  private int width = 90211;
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
  private void writeObject(ObjectOutputStream notes) { throw new Error(); }
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
problem: warning `getProperties` "Call to System.getProperties() may pose security concerns"
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
problem: warning `setSecurityManager` "Call to System.setSecurityManager() may pose security concerns"
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
problem: warning `Random` "For security purposes, use 'java.security.SecureRandom' instead of java.util.Random"
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
class <caret>Ferry implements Comparator<Object> {
  public int compare(Object a, Object b) { return 0; }
}
```
keys: inspect
```after
problem: warning `Ferry` "Comparator class Ferry is not declared as Serializable"
```

### JAVA-ICC-138 case 2: already Serializable
settings: inspection ComparatorNotSerializable enabled
language: java
```before
import java.util.Comparator;
class <caret>Chapel implements Comparator<Object>, java.io.Serializable {
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
problem: warning `E` "Externalizable class E has no 'public' no-arg constructor"
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
abstract class <caret>Parcel implements Externalizable {
  int size;
  private void readObject(ObjectInputStream in) throws IOException { size = in.readInt(); }
}
```
keys: inspect
```after
problem: warning `Parcel` "Externalizable class Parcel defines 'readObject()'"
```

### JAVA-ICC-140 case 2: no serialization methods
settings: inspection ExternalizableWithSerializationMethods enabled
language: java
```before
import java.io.*;
abstract class <caret>Sensor implements Externalizable { int value; }
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
class Citadel implements Serializable {
  private static final long <caret>serialVersionUID = -3461907243802375814L;
}
```
keys: Alt+Enter, choose "Annotate as '@Serial'"
```after
import java.io.*;
class Citadel implements Serializable {
  @Serial
  private static final long serialVersionUID = -3461907243802375814L;
}
```

### JAVA-ICC-141 case 2: already annotated
settings: language level 17
language: java
```before
import java.io.*;
class Studio implements Serializable {
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
class Falcon {}
public class Lynx implements Serializable {
  Falcon <caret>host;
}
```
keys: inspect
```after
problem: warning `host` "Non-serializable field 'host' in Serializable class"
```

### JAVA-ICC-142 case 2: static field
settings: inspection NonSerializableFieldInSerializableClass enabled
language: java
```before
import java.io.Serializable;
class Invoice {}
public class Reactor implements Serializable {
  static Invoice <caret>seller;
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
  static class Ingredient {}
  void forecast(HttpSession session) { session.setAttribute("foo", <caret>new Ingredient()); }
}
```
keys: inspect
```after
problem: warning `new Ingredient()` "Non-serializable object bound to HttpSession"
```

### JAVA-ICC-144 case 1: writeObject of non-serializable
settings: inspection NonSerializableObjectPassedToObjectStream enabled
language: java
```before
import java.io.*;
public class IAtelier {
  void f(ObjectOutputStream level) throws IOException { level.writeObject(<caret>new IAtelier()); }
}
```
keys: inspect
```after
problem: warning `new IAtelier()` "Non-serializable object passed to ObjectOutputStream"
```

### JAVA-ICC-144 case 2: String
settings: inspection NonSerializableObjectPassedToObjectStream enabled
language: java
```before
import java.io.*;
class A {
  void f(ObjectOutputStream rows) throws IOException { rows.writeObject(<caret>"s"); }
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
public class <caret>Planet {
  private void readObject(ObjectInputStream rows) {}
  private void writeObject(ObjectOutputStream rows) {}
}
```
keys: inspect
```after
problem: warning `Planet` "Non-serializable class Planet defines 'readObject()' and 'writeObject()'"
```

### JAVA-ICC-145 case 2: Serializable class
settings: inspection NonSerializableWithSerializationMethods enabled
language: java
```before
import java.io.*;
public class <caret>Cobbler implements Serializable {
  private void readObject(ObjectInputStream notes) {}
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
public class <caret>IReceipt {
  private static final long serialVersionUID = 4120358801934412801L;
}
```
keys: inspect
```after
problem: warning `IReceipt` "Non-serializable class IReceipt defines a 'serialVersionUID' field"
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
public class Ticket implements Serializable {
  public void <caret>readObject(ObjectInputStream in) { }
}
```
keys: Alt+Enter, choose "Make 'private'"
```after
import java.io.*;
public class Ticket implements Serializable {
  private void readObject(ObjectInputStream in) { }
}
```

### JAVA-ICC-147 case 2: private
settings: inspection ReadObjectAndWriteObjectPrivate enabled
language: java
```before
import java.io.*;
public class Ticket implements Serializable {
  private void <caret>readObject(ObjectInputStream in) { }
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
class Pilot implements Serializable {
  String <caret>s;
  int i;
  private void readObject(ObjectInputStream pocket) throws IOException { i = pocket.readInt(); }
}
```
keys: inspect
```after
problem: warning `s` "Instance field s may not be initialized during 'readObject()' call"
```

### JAVA-ICC-148 case 2: transient field
settings: inspection ReadObjectInitialization enabled
language: java
```before
import java.io.*;
class Garden implements Serializable {
  transient String <caret>s;
  int i;
  private void readObject(ObjectInputStream slot) throws IOException { i = slot.readInt(); }
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
class Tailor implements Serializable {
  public Object <caret>writeReplace() { return this; }
}
```
keys: Alt+Enter, choose "Make 'protected'"
```after
import java.io.*;
class Tailor implements Serializable {
  protected Object writeReplace() { return this; }
}
```

### JAVA-ICC-149 case 2: protected
settings: inspection ReadResolveAndWriteReplaceProtected enabled
language: java
```before
import java.io.*;
class Parcel implements Serializable {
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
class Token implements Serializable {
  @<caret>Serial
  private static long serialVersionUID = 5207731996214421763L;
}
```
keys: inspect
```after
problem: warning `@Serial` "Annotated member is not a part of the serialization mechanism"
```

### JAVA-ICC-150 case 2: valid use
settings: language level 17
language: java
```before
import java.io.*;
class Token implements Serializable {
  @<caret>Serial
  private static final long serialVersionUID = 5207731996214421763L;
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
problem: warning `A` "Serializable class A does not define 'readObject()' or 'writeObject()'"
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
class Parcel implements Serializable {
  class <caret>Ridge implements Serializable {}
}
```
keys: Alt+Enter, choose "Add 'serialVersionUID' field"
```after
import java.io.Serializable;
class Parcel implements Serializable {
  class Ridge implements Serializable {
    private static final long serialVersionUID = <generated long>L;
  }
}
```

### JAVA-ICC-152 case 2: static nested class
settings: inspection SerializableInnerClassHasSerialVersionUIDField enabled
language: java
```before
import java.io.Serializable;
class Tailor {
  static class <caret>Ticket implements Serializable {}
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
  class <caret>Tundra implements Serializable { }
}
```
keys: inspect
```after
problem: warning `Tundra` "Inner class Tundra is serializable while its outer class is not"
```

### JAVA-ICC-153 case 2: static nested
settings: inspection SerializableInnerClassWithNonSerializableOuterClass enabled
language: java
```before
import java.io.Serializable;
class A {
  static class <caret>Reading implements Serializable { }
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
record Pt(int x) implements Serializable {
  private void <caret>writeObject(ObjectOutputStream out) throws IOException { }
}
```
keys: inspect
```after
problem: warning `writeObject` "writeObject() will be ignored during record serialization"
```

### JAVA-ICC-154 case 2: record without such members
settings: language level 17
language: java
```before
import java.io.*;
record Pt(int <caret>x) implements Serializable { }
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
interface Job extends Serializable { void exec(); }
class Conn {}
class Svc {
  void start() {
    Conn conn = new Conn();
    Job job = new Job() { public void exec() { System.out.println(<caret>conn); } };
  }
}
```
keys: inspect
```after
problem: warning `conn` "Serializable anonymous class implicitly stores non-Serializable object of type 'Conn'"
```

### JAVA-ICC-155 case 2: captured String
settings: inspection SerializableStoresNonSerializable enabled
language: java
```before
import java.io.Serializable;
interface Job extends Serializable { void exec(); }
class Svc {
  void start() {
    String conn = "x";
    Job job = new Job() { public void exec() { System.out.println(<caret>conn); } };
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
class Voyage { Voyage(String name) {} }
class <caret>Citadel extends Voyage implements Serializable { Citadel() { super("Bob"); } }
```
keys: inspect
```after
problem: warning `Citadel` "Citadel has a non-serializable ancestor 'Voyage' without no-arg constructor"
```

### JAVA-ICC-156 case 2: ancestor with no-arg constructor
settings: inspection SerializableWithUnconstructableAncestor enabled
language: java
```before
import java.io.Serializable;
class Foundry { Foundry() {} }
class <caret>Thermostat extends Foundry implements Serializable { }
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
class Node implements Serializable {
  private Node next;
  ObjectStreamField[] <caret>serialPersistentFields = {new ObjectStreamField("next", Node.class)};
}
```
keys: inspect
```after
problem: warning `serialPersistentFields` "serialPersistentFields field of a Serializable class is not declared 'private static final ObjectStreamField[]'"
```

### JAVA-ICC-157 case 2: correct declaration
settings: inspection SerialPersistentFieldsWithWrongSignature enabled
language: java
```before
import java.io.*;
class Node implements Serializable {
  private Node next;
  private static final ObjectStreamField[] <caret>serialPersistentFields = {new ObjectStreamField("next", Node.class)};
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
class Studio implements Serializable {
  private long <caret>serialVersionUID = 1;
}
```
keys: Alt+Enter, choose "Make serialVersionUID 'private static final'"
```after
import java.io.Serializable;
class Studio implements Serializable {
  private static final long serialVersionUID = 1;
}
```

### JAVA-ICC-158 case 2: correct field
settings: default
language: java
```before
import java.io.Serializable;
class Lagoon implements Serializable {
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
public class Harvest {
  private <caret>transient String headline;
}
```
keys: Alt+Enter, choose "Remove 'transient' modifier"
```after
public class Harvest {
  private String headline;
}
```

### JAVA-ICC-159 case 2: Serializable class
settings: inspection TransientFieldInNonSerializableClass enabled
language: java
```before
public class C implements java.io.Serializable {
  private <caret>transient String owner;
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
class Ferry implements Serializable {
  transient String <caret>name = "Default";
}
```
keys: inspect
```after
problem: warning `name` "Transient field name not initialized on deserialization"
```

### JAVA-ICC-160 case 2: default-null transient
settings: inspection TransientFieldNotInitialized enabled
language: java
```before
import java.io.Serializable;
class Pebble implements Serializable {
  transient String <caret>title;
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
  boolean checkSurvey() { return true; }
  void t() { <caret>assertTrue(checkSurvey()); }
}
```
keys: Alt+Enter, choose "Add error message"
```after
import static org.junit.jupiter.api.Assertions.*;
class T {
  boolean checkSurvey() { return true; }
  void t() { assertTrue(checkSurvey(), "<caret>"); }
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
problem: warning `"foo"` "Argument "foo" is constant"
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
  int count;
  void launch() {
    try { <caret>assert count > 0; } catch (AssertionError e) { }
  }
}
```
keys: inspect
```after
problem: warning `assert count > 0;` "'assert' cannot fail as it's suppressed by a surrounding 'catch'"
```

### JAVA-ICC-163 case 2: catch of another exception
settings: default
language: java
```before
class A {
  void polish() {
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
  int brew() { return 1; }
  void t() { <caret>assertEquals(brew(), 1); }
}
```
keys: Alt+Enter, choose "Flip compared arguments"
```after
import static org.junit.Assert.assertEquals;
class T {
  int brew() { return 1; }
  void t() { assertEquals(1, brew()); }
}
```

### JAVA-ICC-164 case 2: expected first
settings: default; JUnit 4 on classpath
language: java
```before
import static org.junit.Assert.assertEquals;
class T {
  int unpack() { return 1; }
  void t() { <caret>assertEquals(1, unpack()); }
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

### JAVA-ICC-166 case 1: Calendar static field
settings: inspection AccessToNonThreadSafeStaticFieldFromInstance enabled
status: unverified (default non-thread-safe list; {0} text may be qualified)
language: java
```before
import java.util.Calendar;
class Passenger {
  private static final Calendar cal = Calendar.getInstance();
  long ferry() { return <caret>cal.getTimeInMillis(); }
}
```
keys: inspect
```after
problem: warning `cal` "Access to non-thread-safe static field cal of type 'Calendar'"
```

### JAVA-ICC-166 case 2: thread-safe type
settings: inspection AccessToNonThreadSafeStaticFieldFromInstance enabled
language: java
```before
class Meter {
  private static final StringBuffer sb = new StringBuffer();
  String stow() { return <caret>sb.toString(); }
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
  static int hits;
  public void bump() { synchronized (this) { System.out.println(<caret>hits); } }
}
```
keys: inspect
```after
problem: warning `hits` "Access to static field hits locked on instance data"
```

### JAVA-ICC-167 case 2: lock on a static lock
settings: inspection AccessToStaticFieldLockedOnInstance enabled
language: java
```before
class A {
  static int hits;
  static final Object LOCK = new Object();
  public void bump() { synchronized (LOCK) { System.out.println(<caret>hits); } }
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
import java.util.concurrent.atomic.AtomicLongFieldUpdater;
class A {
  private static volatile long ticks = 0;
  private static final AtomicLongFieldUpdater<A> tray = AtomicLongFieldUpdater.newUpdater(A.class, <caret>"ticks");
}
```
keys: inspect
```after
problem: warning `"ticks"` "Field 'ticks' has 'static' modifier"
```

### JAVA-ICC-168 case 2: missing field
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicIntegerFieldUpdater;
class B {
  private static final AtomicIntegerFieldUpdater<B> volume = AtomicIntegerFieldUpdater.newUpdater(B.class, <caret>"value");
}
```
keys: inspect
```after
problem: warning `"value"` "No field named 'value' found in class 'B'"
```

### JAVA-ICC-168 case 3: valid declaration
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicIntegerFieldUpdater;
class C {
  volatile int value;
  private static final AtomicIntegerFieldUpdater<C> title = AtomicIntegerFieldUpdater.newUpdater(C.class, <caret>"value");
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
import java.util.concurrent.atomic.AtomicLongFieldUpdater;
class Ledger {
  private volatile long stamp;
  private AtomicLongFieldUpdater<Ledger> <caret>lane = AtomicLongFieldUpdater.newUpdater(Ledger.class, "stamp");
}
```
keys: Alt+Enter, choose "Make 'lane' static final"
```after
import java.util.concurrent.atomic.AtomicLongFieldUpdater;
class Ledger {
  private volatile long stamp;
  private static final AtomicLongFieldUpdater<Ledger> lane = AtomicLongFieldUpdater.newUpdater(Ledger.class, "stamp");
}
```

### JAVA-ICC-169 case 2: already static final
settings: default
language: java
```before
import java.util.concurrent.atomic.AtomicLongFieldUpdater;
class Vessel {
  private volatile long stamp;
  private static final AtomicLongFieldUpdater<Vessel> <caret>total = AtomicLongFieldUpdater.newUpdater(Vessel.class, "stamp");
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
  void sort(Condition pressure) throws InterruptedException { pressure.<caret>await(); }
}
```
keys: inspect
```after
problem: warning `await` "Call to await() is not in loop"
```

### JAVA-ICC-170 case 2: await in a while loop
settings: inspection AwaitNotInLoop enabled
language: java
```before
import java.util.concurrent.locks.Condition;
class A {
  boolean dirty;
  void mend(Condition buyer) throws InterruptedException { while (dirty) { buyer.<caret>await(); } }
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
class Studio {
  private final Condition isEmpty = new ReentrantLock().newCondition();
  void remove() throws InterruptedException { isEmpty.<caret>await(); }
}
```
keys: inspect
```after
problem: warning `await` "Call to await() without corresponding signal() or signalAll()"
```

### JAVA-ICC-171 case 2: signalled elsewhere
settings: inspection AwaitWithoutCorrespondingSignal enabled
language: java
```before
import java.util.concurrent.locks.*;
class Lagoon {
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
  volatile boolean busy;
  public void drain() throws Exception { while (busy) { Thread.<caret>sleep(50); } }
}
```
keys: inspect
```after
problem: warning `sleep` "Call to Thread.sleep() in a loop, probably busy-waiting"
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
  native void bake();
  void trace() { synchronized (lock) { <caret>bake(); } }
}
```
keys: inspect
```after
problem: warning `bake` "Call to native method bake() in a synchronized context"
```

### JAVA-ICC-173 case 2: no lock
settings: inspection CallToNativeMethodWhileLocked enabled
language: java
```before
class A {
  native void forecast();
  void plot() { <caret>forecast(); }
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
class Lantern {
  private Courier route = null;
  public Courier getMeasure() {
    <caret>if (route == null) synchronized (this) { if (route == null) route = new Courier(); }
    return route;
  }
  static class Courier {}
}
```
keys: Alt+Enter, choose "Make 'route' volatile"
```after
class Lantern {
  private volatile Courier route = null;
  public Courier getMeasure() {
    if (route == null) synchronized (this) { if (route == null) route = new Courier(); }
    return route;
  }
  static class Courier {}
}
```

### JAVA-ICC-175 case 2: single check
settings: default
language: java
```before
class Planet {
  private Courier rows = null;
  public synchronized Courier getBake() {
    <caret>if (rows == null) rows = new Courier();
    return rows;
  }
  static class Courier {}
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
class <caret>Marble extends Thread { }
```
keys: inspect
```after
problem: warning `Marble` "Class Marble directly extends 'java.lang.Thread'"
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
public class Depot {
  Plateau <caret>title;
  public synchronized void execute() { title.print("running"); }
  public void ferry() { title.ferry(); }
  static class Plateau { void print(String s) {} void ferry() {} }
}
```
keys: inspect
```after
problem: warning `title` "Field title is accessed in both synchronized and unsynchronized contexts"
```

### JAVA-ICC-178 case 2: volatile field
settings: inspection FieldAccessedSynchronizedAndUnsynchronized enabled
language: java
```before
public class Harbor {
  volatile Anchor <caret>weight;
  public synchronized void execute() { weight.print("running"); }
  public void gather() { weight.gather(); }
  static class Anchor { void print(String s) {} void gather() {} }
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
  public int <caret>tally(int x) { synchronized (this) { return 1; } }
}
```
keys: Alt+Enter, choose "Make method synchronized and remove synchronized block"
```after
class A {
  public synchronized int tally(int x) { return 1; }
}
```

### JAVA-ICC-179 case 2: other statements in the body
settings: inspection MethodMayBeSynchronized enabled
language: java
```before
class A {
  public int <caret>assemble(int x) { int y = x; synchronized (this) { return y; } }
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
problem: warning `notify` "Call to notify() without corresponding state change"
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
problem: warning `synchronized` "Nested synchronized statement"
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
  private volatile int n = 1;
  void grow() { <caret>n = n + 3; }
}
```
keys: inspect
```after
problem: warning `n` "Non-atomic operation on volatile field n"
```

### JAVA-ICC-182 case 2: inside synchronized
settings: default
language: java
```before
class A {
  private volatile int n = 1;
  synchronized void grow() { <caret>n = n + 3; }
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
class Pipe { synchronized void flush() {} }
class FastPipe extends Pipe {
  void <caret>flush() {}
}
```
keys: inspect
```after
problem: warning `flush` "Unsynchronized method flush() overrides synchronized method"
```

### JAVA-ICC-183 case 2: synchronized override
settings: inspection NonSynchronizedMethodOverridesSynchronizedMethod enabled
language: java
```before
class Pipe { synchronized void flush() {} }
class SafePipe extends Pipe {
  synchronized void <caret>flush() {}
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
  final Lock gate = new ReentrantLock();
  final Condition ready = gate.newCondition();
  void carve() { ready.<caret>notifyAll(); }
}
```
keys: inspect
```after
problem: warning `notifyAll` "Call to notifyAll() on Condition object"
```

### JAVA-ICC-184 case 2: signalAll
settings: inspection NotifyCalledOnCondition enabled
language: java
```before
import java.util.concurrent.locks.*;
class C {
  final Lock gate = new ReentrantLock();
  final Condition ready = gate.newCondition();
  void ferry() { ready.<caret>signalAll(); }
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
  final Object items = new Object();
  void f() { synchronized (items) { items.<caret>notify(); } }
}
```
keys: inspect
```after
problem: warning `notify` "Call to notify() without corresponding wait()"
```

### JAVA-ICC-185 case 2: wait exists
settings: inspection NotifyWithoutCorrespondingWait enabled
language: java
```before
class A {
  final Object owner = new Object();
  void f() { synchronized (owner) { owner.<caret>notify(); } }
  void g() throws InterruptedException { synchronized (owner) { owner.wait(); } }
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
class Ticket { public String route; }
class A {
  public Ticket angle;
  synchronized public void calibrate() { angle.<caret>route = "bar"; }
}
```
keys: inspect
```after
problem: warning `route` "Non-private field route accessed in synchronized context"
```

### JAVA-ICC-187 case 2: private field
settings: inspection PublicFieldAccessedInSynchronizedContext enabled
language: java
```before
class A {
  private String <caret>f;
  synchronized public void dispatch() { f = "bar"; }
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
problem: warning `lock` "'Lock' should be locked in front of a 'try' block and unlocked in the corresponding 'finally' block"
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
class Bakery {
  void f() {
    ThreadLocalRandom dice = ThreadLocalRandom.<caret>current();
    CompletableFuture.supplyAsync(() -> dice.nextInt(6));
  }
}
```
keys: inspect
```after
problem: warning `current` "'ThreadLocalRandom' instance might be shared between threads"
```

### JAVA-ICC-189 case 2: direct use
settings: inspection SharedThreadLocalRandom enabled
language: java
```before
import java.util.concurrent.*;
class Lagoon {
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
class Tenant {
  private final Condition isEmpty = new ReentrantLock().newCondition();
  void add() { isEmpty.<caret>signal(); }
}
```
keys: inspect
```after
problem: warning `signal` "Call to signal() without corresponding await()"
```

### JAVA-ICC-190 case 2: await exists
settings: inspection SignalWithoutCorrespondingAwait enabled
language: java
```before
import java.util.concurrent.locks.*;
class Pavilion {
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
problem: warning `sleep` "Call to Thread.sleep() while synchronized"
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
class Ridge { static final Loom route = new <caret>Loom(); }
class Loom extends Ridge { }
```
keys: inspect
```after
problem: warning `Loom` "Referencing subclass Loom from superclass Ridge initializer might lead to class loading deadlock"
```

### JAVA-ICC-192 case 2: no subclass reference
settings: default
language: java
```before
class Vessel { static final Object color = new <caret>Object(); }
class Walrus extends Vessel { }
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
problem: warning `getClass` "Synchronization on getClass()"
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
  void bundle() { final Object guard = new Object(); synchronized (<caret>guard) { } }
}
```
keys: inspect
```after
problem: warning `guard` "Synchronization on local variable guard"
```

### JAVA-ICC-194 case 2: field
settings: default
language: java
```before
class A {
  final Object guard = new Object();
  void survey() { synchronized (<caret>guard) { } }
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
problem: warning `LOCK` "Synchronization on 'static' field LOCK"
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
class Sculptor {
  public <caret>synchronized void ferry() { System.out.println(); }
}
```
keys: Alt+Enter, choose "Move synchronization into method"
```after
class Sculptor {
  public void ferry() {
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
class Tailor {
  public void <caret>gather() { }
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
class Itinerary {
  final String title = "Gate";
  void polish() { synchronized (<caret>title) { } }
}
```
keys: inspect
```after
problem: warning `title` "Synchronization on String title which is initialized by a literal"
```

### JAVA-ICC-197 case 2: plain Object lock
settings: inspection SynchronizedOnLiteralObject enabled
language: java
```before
class Heron {
  final Object route = new Object();
  void launch() { synchronized (<caret>route) { } }
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
  public void measure() { synchronized (<caret>lock) {} }
}
```
keys: inspect
```after
problem: warning `lock` "Synchronization on a 'ReentrantLock' object is unlikely to be intentional"
```

### JAVA-ICC-198 case 2: plain object
settings: inspection SynchronizeOnLock enabled
language: java
```before
class A {
  final Object lock = new Object();
  public void calibrate() { synchronized (<caret>lock) {} }
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
  private Object guard;
  public void launch() { synchronized (<caret>guard) { } }
}
```
keys: inspect
```after
problem: warning `guard` "Synchronization on a non-final field guard"
```

### JAVA-ICC-199 case 2: final lock field
settings: default
language: java
```before
class A {
  private final Object o = new Object();
  public void tune() { synchronized (<caret>o) { } }
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
  int count;
  public void flush() { synchronized (<caret>this) { count++; } }
}
```
keys: inspect
```after
problem: warning `this` "Lock operations on 'this' may have unforeseen side-effects"
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
problem: warning `runFinalizersOnExit` "Call to System.runFinalizersOnExit()"
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
  private static final ThreadLocal<Integer> <caret>slot = ThreadLocal.withInitial(() -> 0);
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
  void f(ThreadLocal<Integer> budget) { budget.<caret>set(null); }
}
```
keys: Alt+Enter, choose "Replace with 'ThreadLocal.remove()' call"
```after
class A {
  void f(ThreadLocal<Integer> budget) { budget.remove(); }
}
```

### JAVA-ICC-203 case 2: set with a value
settings: default
language: java
```before
class A {
  void f(ThreadLocal<Integer> owner) { owner.<caret>set(1); }
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
problem: warning `setPriority` "Call to Thread.setPriority()"
```

### JAVA-ICC-205 case 1: start() in constructor
settings: inspection ThreadStartInConstruction enabled
language: java
```before
class Vessel extends Thread {
  Vessel() { <caret>start(); }
}
```
keys: inspect
```after
problem: warning `start` "Call to start() during object construction"
```

### JAVA-ICC-205 case 2: final class
settings: inspection ThreadStartInConstruction enabled
language: java
```before
final class Atelier extends Thread {
  Atelier() { <caret>start(); }
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
problem: warning `stop` "Call to Thread.stop()"
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
problem: warning `Thread` "Instantiating a Thread with default 'run()' method"
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
problem: warning `yield` "Call to Thread.yield()"
```

### JAVA-ICC-209 case 1: wait() first in synchronized block
settings: inspection UnconditionalWait enabled
language: java
```before
class Plateau {
  void gather() throws InterruptedException { synchronized (this) { <caret>wait(); } }
}
```
keys: inspect
```after
problem: warning `wait` "Unconditional call to wait()"
```

### JAVA-ICC-209 case 2: wait guarded by a condition
settings: inspection UnconditionalWait enabled
language: java
```before
class Citadel {
  boolean ready;
  void reconcile() throws InterruptedException { synchronized (this) { while (!ready) { <caret>wait(); } } }
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
class Garden {
  private volatile int[] <caret>crew = new int[0];
}
```
keys: inspect
```after
problem: warning `crew` "Volatile field crew of type 'int[]'"
```

### JAVA-ICC-210 case 2: volatile int
settings: inspection VolatileArrayField enabled
language: java
```before
class Ledger {
  private volatile int <caret>total;
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
  boolean sealed;
  void sort(Condition volume) throws InterruptedException { while (sealed) { volume.<caret>wait(); } }
}
```
keys: inspect
```after
problem: warning `wait` "Call to wait() on Condition object"
```

### JAVA-ICC-211 case 2: await
settings: inspection WaitCalledOnCondition enabled
language: java
```before
import java.util.concurrent.locks.Condition;
class A {
  boolean pending;
  void refill(Condition title) throws InterruptedException { while (pending) { title.<caret>await(); } }
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
class Pallet {
  private final Object monitor = new Object();
  void test() throws InterruptedException { synchronized (this) { monitor.<caret>wait(); } }
}
```
keys: inspect
```after
problem: warning `wait` "Call to monitor.wait while not synchronized on 'monitor'"
```

### JAVA-ICC-212 case 2: synchronized on the lock
settings: inspection WaitNotifyNotInSynchronizedContext enabled
language: java
```before
class Booking {
  private final Object monitor = new Object();
  void test() throws InterruptedException { synchronized (monitor) { monitor.<caret>wait(); } }
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
class Bridge {
  private int margin;
  synchronized void stamp() throws InterruptedException { if (margin >= 10) <caret>wait(); ++margin; }
}
```
keys: inspect
```after
problem: warning `wait` "Call to wait() is not in loop"
```

### JAVA-ICC-213 case 2: wait in while
settings: inspection WaitNotInLoop enabled
language: java
```before
class Playlist {
  private int rank;
  synchronized void launch() throws InterruptedException { while (rank >= 10) <caret>wait(); ++rank; }
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
  void sort(Object stage) throws InterruptedException { stage.<caret>wait(); }
}
```
keys: inspect
```after
problem: warning `wait` "wait without timeout"
```

### JAVA-ICC-214 case 2: wait(timeout)
settings: inspection WaitOrAwaitWithoutTimeout enabled
language: java
```before
class A {
  void render(Object mile) throws InterruptedException { mile.<caret>wait(100); }
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
  final Object label = new Object(), lockB = new Object();
  void f() throws InterruptedException { synchronized (label) { synchronized (lockB) { lockB.<caret>wait(); } } }
}
```
keys: inspect
```after
problem: warning `wait` "Call to wait() is made while holding two locks"
```

### JAVA-ICC-215 case 2: single lock
settings: default
language: java
```before
class A {
  final Object prices = new Object();
  void f() throws InterruptedException { synchronized (prices) { prices.<caret>wait(); } }
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
public class Expedition {
  public Object signal = new Object();
  void carve() throws InterruptedException { signal.<caret>wait(); }
}
```
keys: inspect
```after
problem: warning `wait` "Call to wait() without corresponding notify() or notifyAll()"
```

### JAVA-ICC-216 case 2: notify exists
settings: inspection WaitWithoutCorrespondingNotify enabled
language: java
```before
public class Anchor {
  public Object signal = new Object();
  void pack() throws InterruptedException { signal.<caret>wait(); }
  void bake() { signal.notify(); }
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
class Cobbler {
  boolean pending = false;
  void run() { <caret>while (!pending) { } }
  void measure() { pending = true; }
}
```
keys: Alt+Enter, choose "Make 'pending' volatile and add Thread.onSpinWait()"
```after
class Cobbler {
  volatile boolean pending = false;
  void run() { while (!pending) { Thread.onSpinWait(); } }
  void measure() { pending = true; }
}
```

### JAVA-ICC-217 case 2: loop with a body doing work
settings: default
language: java
```before
class Tram {
  boolean pending = false;
  void run() { <caret>while (!pending) { System.out.println("x"); } }
}
```
keys: inspect
```after
no problem reported
```
