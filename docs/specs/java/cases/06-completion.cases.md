# Java completion: acceptance cases

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to
the requirement of the same ID in `docs/specs/java/06-completion.md`. Cases of this file that need
a library put it on the module with `settings: library=<name>`; JUnit and Hamcrest libraries are on the *test*
classpath of the module (production sources do not see them, JAVA-RES-029).

Keys of a completion case (the executable runner, `crates/archipel-ide-java/tests/java_cases.rs`):

* `keys: complete` presses Ctrl+Space once at the caret (invocation count 1); `complete [count N]` presses it N
  times. The `after` block then does not repeat the code but states the popup, one line per statement:
  `item: <name> (<package or outer class>)` (the class is offered; the tail is left out for classes of the file
  and of its package), `not item: <name or name (package)>` (it is not offered), `rank: <a> < <b>` (both are
  offered, `a` above `b`) and `no items`.
* `keys: complete, choose "<row>"` additionally picks the row with exactly that text; `after` is then the
  resulting file with `<caret>` behind the inserted name (inside the parentheses of a call that takes
  arguments, behind them for a call that takes none).
* Rows of members, variables and keywords are named by their bare name (`size`, `names`, `return`): a method is one
  row per name, its overloads merged (JAVA-CMP-016). Four more statements describe a row: `once: <name>` (exactly
  one row has that name), `tail: <name> = <parameter list>` (the grey text after a method name), `type:
  <name> = <type>` (the right-aligned type of a method, field or variable) and `struck: <name>` / `not struck:
  <name>` (deprecated rows are struck through, JAVA-CMP-017). `item: Class.member (package)` names a static member
  row of the second press (JAVA-CMP-022).

### JAVA-CMP-009 case 1: Annotation position offers annotation types of the test classpath
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>
  void greets() {}
}
```
keys: complete
```after
item: Test (org.junit.jupiter.api)
not item: TestInstance (org.junit.jupiter.api)
not item: String
```

### JAVA-CMP-019 case 1: Choosing an annotation type adds its import
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>
  void greets() {}
}
```
keys: complete, choose "Test (org.junit.jupiter.api)"
```after
package p;

import org.junit.jupiter.api.Test;

class AppTest {
  @Test<caret>
  void greets() {}
}
```

### JAVA-CMP-009 case 2: Production sources do not see test libraries
settings: library=junit5
language: java
```before
--- src/main/java/p/App.java
package p;

class App {
  @Te<caret>
  void greets() {}
}
```
keys: complete
```after
not item: Test (org.junit.jupiter.api)
```

### JAVA-CMP-009 case 3: A JUnit 4 project offers its own annotation type
settings: library=junit4
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>
  void greets() {}
}
```
keys: complete
```after
item: Test (org.junit)
not item: Test (org.junit.jupiter.api)
```

### JAVA-CMP-029 case 1: Annotation types are filtered by the element they annotate
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>
  private int counter;
}
```
keys: complete
```after
not item: Test (org.junit.jupiter.api)
```

### JAVA-CMP-018 case 1: Class names after new come from the JDK index with a package hint
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Object list = new ArrayLi<caret>
  }
}
```
keys: complete
```after
item: ArrayList (java.util)
not item: ArrayDeque (java.util)
not item: List (java.util)
```

### JAVA-CMP-019 case 2: Choosing ArrayList after new adds the import
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Object list = new ArrayLi<caret>
  }
}
```
keys: complete, choose "ArrayList (java.util)"
```after
package p;

import java.util.ArrayList;

class Main {
  void run() {
    Object list = new ArrayList<caret>
  }
}
```

### JAVA-CMP-018 case 2: Camel-hump prefixes match class names
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Object list = new ArLi<caret>
  }
}
```
keys: complete
```after
item: ArrayList (java.util)
```

### JAVA-CMP-009 case 4: Type position offers classes and interfaces, the discouraged ones last
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  private Lis<caret>
}
```
keys: complete
```after
item: List (java.util)
item: List (java.awt)
rank: List (java.util) < List (java.awt)
not item: Test (org.junit.jupiter.api)
```

### JAVA-CMP-009 case 5: Only interfaces after implements
settings: default
language: java
```before
--- src/main/java/p/Job.java
package p;

class Job implements Ru<caret> {
}
```
keys: complete
```after
item: Runnable (java.lang)
not item: Runtime (java.lang)
not item: RuntimeException (java.lang)
```

### JAVA-CMP-009 case 6: Only Throwable subtypes after throws
settings: default
language: java
```before
--- src/main/java/p/Reader.java
package p;

class Reader {
  void read() throws IOEx<caret> {
  }
}
```
keys: complete
```after
item: IOException (java.io)
not item: IOExceptionList
not item: String
```

### JAVA-CMP-009 case 7: Only Throwable subtypes in a catch parameter
settings: default
language: java
```before
--- src/main/java/p/Reader.java
package p;

class Reader {
  void read() {
    try {
    } catch (FileNotF<caret> e) {
    }
  }
}
```
keys: complete
```after
item: FileNotFoundException (java.io)
not item: FileNotFoundExceptionX
```

### JAVA-CMP-019 case 3: Choosing a Throwable type in a catch parameter adds the import
settings: default
language: java
```before
--- src/main/java/p/Reader.java
package p;

class Reader {
  void read() {
    try {
    } catch (FileNotF<caret> e) {
    }
  }
}
```
keys: complete, choose "FileNotFoundException (java.io)"
```after
package p;

import java.io.FileNotFoundException;

class Reader {
  void read() {
    try {
    } catch (FileNotFoundException<caret> e) {
    }
  }
}
```

### JAVA-CMP-020 case 1: A name clash inserts the fully qualified name
settings: default
language: java
```before
--- src/main/java/p/Clash.java
package p;

import java.util.Date;

class Clash {
  private Dat<caret>
}
```
keys: complete, choose "Date (java.sql)"
```after
package p;

import java.util.Date;

class Clash {
  private java.sql.Date<caret>
}
```

### JAVA-CMP-019 case 4: A class that is already imported inserts no second import
settings: default
language: java
```before
--- src/main/java/p/Reuse.java
package p;

import java.util.List;

class Reuse {
  private Lis<caret>
}
```
keys: complete, choose "List (java.util)"
```after
package p;

import java.util.List;

class Reuse {
  private List<caret>
}
```

### JAVA-CMP-018 case 3: Classes of the same package are offered without a package hint
settings: default
language: java
```before
--- src/main/java/p/Widget.java
package p;

public class Widget {
}
--- src/main/java/p/Use.java
package p;

class Use {
  private Wid<caret>
}
```
keys: complete, choose "Widget"
```after
package p;

class Use {
  private Widget<caret>
}
```

### JAVA-CMP-025 case 1: Platform-internal packages are not offered
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  private Unsa<caret>
}
```
keys: complete
```after
not item: Unsafe (sun.misc)
```

### JAVA-CMP-025 case 2: The third press includes library-internal classes
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  private Unsa<caret>
}
```
keys: complete [count 3]
```after
item: Unsafe (sun.misc)
```

### JAVA-CMP-008 case 1: A list receiver lists its members, the inherited ones and those of Object
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.ArrayList;
import java.util.List;

class Main {
  void run() {
    List<String> names = new ArrayList<>();
    names.<caret>
  }
}
```
keys: complete
```after
item: add
item: get
item: size
item: stream
item: forEach
item: toString
not item: emptyList
not item: of
```

### JAVA-CMP-016 case 1: Overloads of a method are one row
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.ArrayList;
import java.util.List;

class Main {
  void run() {
    List<String> names = new ArrayList<>();
    names.<caret>
  }
}
```
keys: complete
```after
once: add
once: remove
once: toArray
tail: size = ()
type: size = int
type: get = String
```

### JAVA-CMP-008 case 2: A call result is typed through the receiver's type arguments
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.ArrayList;
import java.util.List;

class Main {
  void run() {
    List<String> names = new ArrayList<>();
    names.get(0).<caret>
  }
}
```
keys: complete
```after
item: length
item: charAt
item: substring
item: isBlank
type: length = int
not item: add
not item: size
```

### JAVA-CMP-008 case 3: Calls chain
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  StringBuilder builder() {
    return new StringBuilder();
  }

  void run() {
    builder().append("a").append(1).<caret>
  }
}
```
keys: complete
```after
item: append
item: reverse
item: length
item: toString
not item: builder
not item: run
```

### JAVA-CMP-008 case 4: A type qualifier lists static members and member types only
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.Collections;
import java.util.Map;

class Main {
  void run() {
    Collections.<caret>
  }
}
```
keys: complete
```after
item: emptyList
item: sort
item: unmodifiableList
not item: toString
not item: getClass
```

### JAVA-CMP-008 case 5: A member type is offered after its outer type
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.Map;

class Main {
  void run() {
    Map.<caret>
  }
}
```
keys: complete
```after
item: Entry
item: of
not item: size
not item: put
```

### JAVA-CMP-008 case 6: this lists the members of the class with those it inherits
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Base {
  protected int shared;

  void greet() {}
}

class Main extends Base {
  String name = "n";

  void run() {
    this.<caret>
  }
}
```
keys: complete
```after
item: name
item: run
item: greet
item: shared
item: toString
```

### JAVA-CMP-008 case 7: super lists the superclass members only
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Base {
  protected int shared;

  void greet() {}
}

class Main extends Base {
  String name = "n";

  void run() {
    super.<caret>
  }
}
```
keys: complete
```after
item: greet
item: shared
item: toString
not item: name
not item: run
```

### JAVA-CMP-008 case 8: Members come from supertypes in the classpath
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.ArrayList;

class Main {
  void run() {
    ArrayList<String> xs = new ArrayList<>();
    xs.<caret>
  }
}
```
keys: complete
```after
item: add
item: forEach
item: stream
item: removeIf
item: hashCode
not item: removeRange
not item: elementData
```

### JAVA-CMP-008 case 9: Members of a source class and of its source supertype
settings: default
language: java
```before
--- src/main/java/p/Animal.java
package p;

public class Animal {
  public String name() {
    return "";
  }
}
--- src/main/java/p/Dog.java
package p;

public class Dog extends Animal {
  public void bark() {}
}
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Dog dog = new Dog();
    dog.<caret>
  }
}
```
keys: complete
```after
item: bark
item: name
item: toString
```

### JAVA-CMP-008 case 10: Private members of another class are hidden on the first press
settings: default
language: java
```before
--- src/main/java/p/Vault.java
package p;

public class Vault {
  private int secret;
  int shared;

  public void open() {}

  private void lock() {}
}
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Vault vault = new Vault();
    vault.<caret>
  }
}
```
keys: complete
```after
item: open
item: shared
not item: secret
not item: lock
```

### JAVA-CMP-008 case 11: The second press offers the inaccessible members after the others
settings: default
language: java
```before
--- src/main/java/p/Vault.java
package p;

public class Vault {
  private int secret;

  public void open() {}

  private void lock() {}
}
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Vault vault = new Vault();
    vault.<caret>
  }
}
```
keys: complete [count 2]
```after
item: secret
item: lock
rank: open < lock
rank: open < secret
```

### JAVA-CMP-008 case 12: An unknown receiver offers nothing natively
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Mystery mystery = null;
    mystery.<caret>
  }
}
```
keys: complete
```after
no items
```

### JAVA-CMP-003 case 1: Nothing is completed after a number literal
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  int one = 1.<caret>
}
```
keys: complete
```after
no items
```

### JAVA-CMP-003 case 2: A primitive receiver has no members
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    int count = 3;
    count.<caret>
  }
}
```
keys: complete
```after
no items
```

### JAVA-CMP-017 case 1: Deprecated members are struck through
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.Date;

class Main {
  void run() {
    Date date = new Date();
    date.<caret>
  }
}
```
keys: complete
```after
item: getYear
struck: getYear
item: getTime
not struck: getTime
```

### JAVA-CMP-035 case 1: Choosing a method that takes arguments puts the caret inside the call
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.ArrayList;
import java.util.List;

class Main {
  void run() {
    List<String> names = new ArrayList<>();
    names.ad<caret>
  }
}
```
keys: complete, choose "add"
```after
package p;

import java.util.ArrayList;
import java.util.List;

class Main {
  void run() {
    List<String> names = new ArrayList<>();
    names.add(<caret>)
  }
}
```

### JAVA-CMP-035 case 2: Choosing a method without arguments closes the call
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.ArrayList;
import java.util.List;

class Main {
  void run() {
    List<String> names = new ArrayList<>();
    names.siz<caret>
  }
}
```
keys: complete, choose "size"
```after
package p;

import java.util.ArrayList;
import java.util.List;

class Main {
  void run() {
    List<String> names = new ArrayList<>();
    names.size()<caret>
  }
}
```

### JAVA-CMP-008 case 13: Locals, parameters, fields and methods of the class are offered
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  String title = "t";

  String helper() {
    return title;
  }

  void run(int count) {
    String name = "n";
    <caret>
  }
}
```
keys: complete
```after
item: name
item: count
item: title
item: helper
item: run
item: toString
type: name = String
type: count = int
type: title = String
```

### JAVA-CMP-008 case 14: A local that is declared later is not in scope
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    String early = "e";
    ear<caret>
    String late = "l";
  }
}
```
keys: complete
```after
item: early
not item: late
```

### JAVA-CMP-008 case 15: Statically imported members are offered unqualified
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

import static org.junit.jupiter.api.Assertions.assertTrue;

class AppTest {
  void checks() {
    assertTr<caret>
  }
}
```
keys: complete
```after
item: assertTrue
tail: assertTrue = (boolean)
```

### JAVA-CMP-008 case 16: Statically imported members of a class on demand are offered unqualified
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

import static org.junit.jupiter.api.Assertions.*;

class AppTest {
  void checks() {
    assertEq<caret>
  }
}
```
keys: complete
```after
item: assertEquals
tail: assertEquals = (Object, Object)
type: assertEquals = void
```

### JAVA-CMP-028 case 1: Variables of the expected type come first, then other variables, then methods
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  String title = "t";

  String helper() {
    return title;
  }

  void run() {
    String name = "n";
    int n = 1;
    String s = <caret>
  }
}
```
keys: complete
```after
rank: name < title
rank: title < n
rank: n < helper
rank: helper < toString
```

### JAVA-CMP-027 case 1: The expected type ranks its inheritors first after new
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

import java.util.List;

class Main {
  void run() {
    List<String> names = new Arr<caret>
  }
}
```
keys: complete
```after
item: ArrayList (java.util)
item: ArrayStoreException (java.lang)
rank: ArrayList (java.util) < ArrayStoreException (java.lang)
```

### JAVA-CMP-011 case 1: Modifiers that are legal after those already written
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  public st<caret>
}
```
keys: complete
```after
item: static
item: strictfp
not item: public
not item: private
not item: protected
```

### JAVA-CMP-011 case 2: A statement keyword where a statement starts
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  int run(int n) {
    ret<caret>
  }
}
```
keys: complete
```after
item: return
not item: break
not item: continue
not item: else
```

### JAVA-CMP-011 case 3: break and continue inside a loop, else after an if
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run(int n) {
    while (n > 0) {
      if (n == 3) {
        n--;
      }
      <caret>
    }
  }
}
```
keys: complete
```after
item: break
item: continue
item: else
item: return
```

### JAVA-CMP-011 case 4: The clauses of a class header
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

public class Main ext<caret> {
}
```
keys: complete
```after
item: extends
not item: class
```

### JAVA-CMP-011 case 5: Declaration keywords and primitive types in a class body
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  <caret>
}
```
keys: complete
```after
item: public
item: private
item: static
item: final
item: class
item: void
item: int
```

### JAVA-CMP-022 case 1: The first press offers no static members of other classes
settings: library=hamcrest,junit4
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  void checks() {
    assertTh<caret>
  }
}
```
keys: complete
```after
no items
```

### JAVA-CMP-022 case 2: The second press offers static members of every class, once per class and name
settings: library=hamcrest,junit4
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  void checks() {
    assertTh<caret>
  }
}
```
keys: complete [count 2]
```after
item: MatcherAssert.assertThat (org.hamcrest)
item: Assert.assertThat (org.junit)
once: MatcherAssert.assertThat (org.hamcrest)
struck: Assert.assertThat (org.junit)
not struck: MatcherAssert.assertThat (org.hamcrest)
rank: MatcherAssert.assertThat (org.hamcrest) < Assert.assertThat (org.junit)
```

### JAVA-CMP-022 case 3: Choosing a static member writes the qualified call and imports the class
settings: library=hamcrest,junit4
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  void checks() {
    assertTh<caret>
  }
}
```
keys: complete [count 2], choose "MatcherAssert.assertThat (org.hamcrest)"
```after
package p;

import org.hamcrest.MatcherAssert;

class AppTest {
  void checks() {
    MatcherAssert.assertThat(<caret>)
  }
}
```

### JAVA-CMP-022 case 4: A statically imported class needs no qualifier
settings: library=hamcrest,junit4
language: java
```before
--- src/test/java/p/AppTest.java
package p;

import static org.hamcrest.MatcherAssert.*;

class AppTest {
  void checks() {
    assertTh<caret>
  }
}
```
keys: complete [count 2]
```after
item: assertThat
not item: MatcherAssert.assertThat (org.hamcrest)
```

### JAVA-CMP-008 case 17: After new only classes with an accessible constructor are offered
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Object o = new Arr<caret>
  }
}
```
keys: complete
```after
item: ArrayList (java.util)
item: ArrayDeque (java.util)
not item: Arrays (java.util)
not item: Array (java.lang.reflect)
```
