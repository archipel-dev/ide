# Cases for 09-inspections-catalog-5.md

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-5.md`. Keys `inspect` list the expected highlights as `problem:` lines, `inspect batch` runs the whole-project inspection, and `no problem reported` means no highlight.

### JAVA-ICB-130 case 1: Delete java.lang import
settings: enable inspection JavaLangImport
language: java
```before
<caret>import java.lang.String;
class A {
    String s;
}
```
keys: Alt+Enter, choose "Delete unnecessary import"
```after
class A {
    String s;
}
```

### JAVA-ICB-131 case 1: Missorted imports
settings: default
language: java
```before
<caret>import java.util.Map;
import java.util.HashMap;
public class Index {
    Map table = new HashMap();
}
```
keys: Alt+Enter, choose "Optimize imports"
```after
import java.util.HashMap;
import java.util.Map;
public class Index {
    Map table = new HashMap();
}
```

### JAVA-ICB-131 case 2: Sorted imports
settings: default
language: java
```before
<caret>import java.util.HashMap;
import java.util.Map;
public class Index {
    Map table = new HashMap();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-132 case 1: Package import
settings: enable inspection OnDemandImport
status: unverified (text substituted for #ref on an import statement)
language: java
```before
<caret>import java.util.*;
class A {
    List<String> l;
}
```
keys: inspect
```after
problem: warning `import java.util.*;` "Package import import java.util.*;"
```

### JAVA-ICB-133 case 1: Same package
settings: enable inspection SamePackageImport
language: java
```before
--- p/B.java
package p;
public class B {}
--- p/A.java
package p;
<caret>import p.B;
class A {
    B b;
}
```
keys: Alt+Enter, choose "Delete unnecessary import"
```after
--- p/A.java
package p;
class A {
    B b;
}
```

### JAVA-ICB-134 case 1: Single class import
settings: enable inspection SingleClassImport
status: unverified (text substituted for #ref on an import statement)
language: java
```before
<caret>import java.util.List;
class A {
    List<String> l;
}
```
keys: inspect
```after
problem: warning `import java.util.List;` "Single class import import java.util.List;"
```

### JAVA-ICB-135 case 1: Replace with non-static import
settings: enable inspection StaticImport
status: unverified (whether the now redundant java.lang import is kept)
language: java
```before
<caret>import static java.lang.Math.max;
class A {
    int m(int a, int b) {
        return max(a, b);
    }
}
```
keys: Alt+Enter, choose "Replace with non-static import"
```after
import java.lang.Math;
class A {
    int m(int a, int b) {
        return Math.max(a, b);
    }
}
```

### JAVA-ICB-136 case 1: Qualifier of a configured class
settings: auto-import table contains java.util.stream.Collectors
status: unverified (whether the fix keeps the qualified import and the precise import form)
language: java
```before
import java.util.List;
import java.util.stream.Collectors;
class A {
    List<String> m(java.util.stream.Stream<String> s) {
        return s.collect(<caret>Collectors.toList());
    }
}
```
keys: Alt+Enter, choose "Add static import"
```after
import java.util.List;
import static java.util.stream.Collectors.*;
class A {
    List<String> m(java.util.stream.Stream<String> s) {
        return s.collect(toList());
    }
}
```

### JAVA-ICB-136 case 2: Class not in the table
settings: default
language: java
```before
class A {
    int m() {
        return <caret>Math.max(1, 2);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-137 case 1: Remove unused imports
settings: default
language: java
```before
<caret>import java.util.ArrayList;
public class Depot {
    public static int twice(int n) {
        return n * 2;
    }
}
```
keys: Alt+Enter, choose "Remove unused imports"
```after
public class Depot {
    public static int twice(int n) {
        return n * 2;
    }
}
```

### JAVA-ICB-137 case 2: Used import
settings: default
language: java
```before
<caret>import java.util.ArrayList;
public class Voyage {
    ArrayList<String> list;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-138 case 1: Abstract extends concrete
settings: enable inspection AbstractClassExtendsConcreteClass
language: java
```before
class Base {}
abstract class <caret>Derived extends Base {}
```
keys: inspect
```after
problem: warning `Derived` "Class Derived is declared 'abstract', and extends a concrete class"
```

### JAVA-ICB-139 case 1: No concrete subclass
settings: enable inspection AbstractClassNeverImplemented
language: java
```before
abstract class <caret>Shape {}
```
keys: inspect
```after
problem: warning `Shape` "Abstract class Shape has no concrete subclass"
```

### JAVA-ICB-140 case 1: Abstract class with only concrete methods
settings: enable inspection AbstractClassWithoutAbstractMethods
language: java
```before
abstract class <caret>Tunnel {
    public int length() {
        return 42;
    }
}
```
keys: inspect
```after
problem: warning `Tunnel` "Class Tunnel is declared 'abstract', and has no 'abstract' methods"
```

### JAVA-ICB-141 case 1: Redundant abstract redeclaration
settings: enable inspection AbstractMethodOverridesAbstractMethod
language: java
```before
abstract class A {
    abstract void harvest();
}
abstract class B extends A {
    abstract void <caret>harvest();
}
```
keys: Alt+Enter, choose "Remove redundant abstract method declaration"
```after
abstract class A {
    abstract void harvest();
}
abstract class B extends A {
}
```

### JAVA-ICB-142 case 1: Abstract overrides concrete
settings: enable inspection AbstractMethodOverridesConcreteMethod
language: java
```before
class A {
    void stamp() {}
}
abstract class B extends A {
    abstract void <caret>stamp();
}
```
keys: inspect
```after
problem: warning `stamp` "Abstract method stamp() overrides concrete method"
```

### JAVA-ICB-143 case 1: Subclass misses the implementation
settings: enable inspection AbstractMethodWithMissingImplementations
language: java
```before
abstract class Base {
    abstract void <caret>assemble();
}
class Impl extends Base {}
```
keys: inspect
```after
problem: warning `assemble` "Abstract method assemble() is not implemented in every subclass"
```

### JAVA-ICB-144 case 1: Class implements annotation
settings: default
language: java
```before
@interface Ann {}
class <caret>C implements Ann {
    public Class<? extends java.lang.annotation.Annotation> annotationType() { return Ann.class; }
}
```
keys: inspect
```after
problem: warning `Ann` "Class 'C' implements annotation interface Ann"
```

### JAVA-ICB-144 case 2: Class implements a regular interface
settings: default
language: java
```before
interface I {}
class C implements <caret>I {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-145 case 1: Extends ArrayList
settings: enable inspection ExtendsConcreteCollection
language: java
```before
class <caret>MyList extends java.util.ArrayList<String> {}
```
keys: inspect
```after
problem: warning `MyList` "Class MyList explicitly extends 'java.util.ArrayList'"
```

### JAVA-ICB-146 case 1: Extends utility class
settings: enable inspection ExtendsUtilityClass
language: java
```before
class Util { static void help() {} }
class <caret>Sub extends Util {}
```
keys: inspect
```after
problem: warning `Sub` "Class Sub extends utility class 'Util'"
```

### JAVA-ICB-147 case 1: Suggest common base
settings: default
status: unverified (member chooser may appear; the fix name for a class implementing an interface)
language: java
```before
interface Shape {}
abstract class Polygon implements Shape {}
abstract class Triangle extends Polygon {}
abstract class Quad extends Polygon {}
abstract class Pentagon extends Polygon {}
abstract class Hexagon extends Polygon {}
abstract class Heptagon extends Polygon {}
class Ferry implements <caret>Shape {}
```
keys: Alt+Enter, choose "Make extends 'Polygon'"
```after
interface Shape {}
abstract class Polygon implements Shape {}
abstract class Triangle extends Polygon {}
abstract class Quad extends Polygon {}
abstract class Pentagon extends Polygon {}
abstract class Hexagon extends Polygon {}
abstract class Heptagon extends Polygon {}
class Ferry extends Polygon {}
```

### JAVA-ICB-147 case 2: Fewer than five inheritors
settings: default
language: java
```before
interface Shape {}
abstract class Polygon implements Shape {}
abstract class Triangle extends Polygon {}
class Ferry implements <caret>Shape {}
```
keys: Alt+Enter
```after
no quick fix "Make extends 'Polygon'" in the popup
```

### JAVA-ICB-148 case 1: Final configuration class
settings: framework Spring present
language: java
```before
import org.springframework.context.annotation.Configuration;
@Configuration
final class <caret>Config {}
```
keys: Alt+Enter, choose "Make class 'Config' extendable"
```after
import org.springframework.context.annotation.Configuration;
@Configuration
class Config {}
```

### JAVA-ICB-148 case 2: Plain final class
settings: default
language: java
```before
final class <caret>Plain {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-149 case 1: No implementation
settings: enable inspection InterfaceNeverImplemented
language: java
```before
interface <caret>Service {}
```
keys: inspect
```after
problem: warning `Service` "Interface Service has no concrete subclass"
```

### JAVA-ICB-150 case 1: Add @Override
settings: default
language: java
```before
class Base {
    void run() {}
}
class X extends Base {
    void <caret>run() {}
}
```
keys: Alt+Enter, choose "Annotate method with '@Override'"
```after
class Base {
    void run() {}
}
class X extends Base {
    @Override
    void run() {}
}
```

### JAVA-ICB-150 case 2: Object methods ignored by default
settings: default
language: java
```before
class X {
    public String <caret>toString() {
        return "x";
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-151 case 1: Public constructor
settings: enable inspection NonProtectedConstructorInAbstractClass
language: java
```before
public abstract class Crane {
    public <caret>Crane() {
        init();
    }
    void init() {}
}
```
keys: Alt+Enter, choose "Make 'protected'"
```after
public abstract class Crane {
    protected Crane() {
        init();
    }
    void init() {}
}
```

### JAVA-ICB-152 case 1: Shadowed Object
settings: enable inspection ParameterTypePreventsOverriding
language: java
```before
public class Base {
    public void apply(String s) {}
}
public class Derived extends Base {
    public void apply(<caret>String s) {}
    class String {}
}
```
keys: Alt+Enter, choose "Change type of parameter to 'java.lang.String'"
```after
public class Base {
    public void apply(String s) {}
}
public class Derived extends Base {
    public void apply(java.lang.String s) {}
    class String {}
}
```

### JAVA-ICB-153 case 1: Array overriding varargs
settings: enable inspection ProblematicVarargsMethodOverride
language: java
```before
class A {
    void balance(String... args) {}
}
class B extends A {
    @Override
    void <caret>balance(String[] args) {}
}
```
keys: Alt+Enter, choose "Convert to varargs method"
```after
class A {
    void balance(String... args) {}
}
class B extends A {
    @Override
    void balance(String... args) {}
}
```

### JAVA-ICB-154 case 1: Redundant interface
settings: enable inspection RedundantImplements
language: java
```before
interface Lease {}
interface Ledger extends Lease {}
class X implements <caret>Lease, Ledger {}
```
keys: Alt+Enter, choose "Remove redundant interface declaration"
```after
interface Lease {}
interface Ledger extends Lease {}
class X implements Ledger {}
```

### JAVA-ICB-155 case 1: Identical body
settings: default
language: java
```before
class A {
    int f() { return 1; }
}
class B extends A {
    int <caret>f() { return 1; }
}
```
keys: Alt+Enter, choose "Remove redundant method"
```after
class A {
    int f() { return 1; }
}
class B extends A {
}
```

### JAVA-ICB-155 case 2: Different body
settings: default
language: java
```before
class A {
    int f() { return 1; }
}
class B extends A {
    int <caret>f() { return 2; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-156 case 1: clone without super
settings: default
status: unverified (generated form of the inserted call for a method that throws)
language: java
```before
class Box implements Cloneable {
    @Override
    public Object <caret>clone() {
        return new Box();
    }
}
```
keys: Alt+Enter, choose "Insert call to super method"
```after
class Box implements Cloneable {
    @Override
    public Object clone() {
        super.clone();
        return new Box();
    }
}
```

### JAVA-ICB-156 case 2: Calls super
settings: default
language: java
```before
class A {
    void f() {}
}
class B extends A {
    @Override
    void <caret>f() { super.f(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-157 case 1: Constants interface
settings: enable inspection StaticInheritance
language: java
```before
interface Consts { int MAX = 10; }
class A implements <caret>Consts {
    int m() { return MAX; }
}
```
keys: Alt+Enter, choose "Replace inheritance with qualified references in the whole project"
```after
interface Consts { int MAX = 10; }
class A {
    int m() { return Consts.MAX; }
}
```

### JAVA-ICB-158 case 1: Wildcard bound
settings: enable inspection TypeParameterExtendsFinalClass
language: java
```before
import java.util.List;
class A {
    void bundle() {
        List<<caret>? extends Integer> pile;
    }
}
```
keys: Alt+Enter, choose "Replace type parameter with actual class"
```after
import java.util.List;
class A {
    void bundle() {
        List<Integer> pile;
    }
}
```

### JAVA-ICB-159 case 1: Abstract call in constructor
settings: enable inspection AbstractMethodCallInConstructor
language: java
```before
abstract class Bison {
    abstract void plot();
    Bison() {
        <caret>plot();
    }
}
```
keys: inspect
```after
problem: warning `plot` "Call to 'abstract' method plot() during object construction"
```

### JAVA-ICB-160 case 1: Replace with regular initialization
settings: default
language: java
```before
import java.util.*;
class A {
    void m() {
        List<Integer> cells = new ArrayList<>() <caret>{{
            add(1);
            add(2);
        }};
    }
}
```
keys: Alt+Enter, choose "Replace with regular initialization"
```after
import java.util.*;
class A {
    void m() {
        List<Integer> cells = new ArrayList<>();
        cells.add(1);
        cells.add(2);
    }
}
```

### JAVA-ICB-160 case 2: Regular anonymous class with a method
settings: default
language: java
```before
import java.util.*;
class A {
    void m() {
        List<Integer> rows = new ArrayList<>() <caret>{
            @Override public boolean add(Integer i) { return super.add(i); }
        };
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-161 case 1: Uninitialized field
settings: enable inspection InstanceVariableInitialization
language: java
```before
class Lease {
    public int <caret>budget;
    static { }
}
```
keys: Alt+Enter, choose "Make initialization explicit"
```after
class Lease {
    public int budget = 0;
    static { }
}
```

### JAVA-ICB-162 case 1: Read in constructor
settings: enable inspection InstanceVariableUninitializedUse
language: java
```before
class Turbine {
    int width;
    Turbine() {
        show(<caret>width);
    }
    void show(int v) {}
}
```
keys: inspect
```after
problem: warning `width` "Instance field width used before initialized"
```

### JAVA-ICB-163 case 1: Static read in static init
settings: enable inspection NonFinalStaticVariableUsedInClassInitialization
language: java
```before
class Mason {
    public static int weight = 0;
    static {
        show(<caret>weight);
    }
    static void show(int v) {}
}
```
keys: inspect
```after
problem: warning `weight` "Non-final static field weight used during class initialization"
```

### JAVA-ICB-164 case 1: Lazy static init
settings: enable inspection NonThreadSafeLazyInitialization
status: unverified (name of the generated holder class)
language: java
```before
import java.util.*;
class X {
    private static Map<String, Integer> table;
    public Map<String, Integer> lookup() {
        if (table == null) {
            <caret>table = new HashMap<>();
        }
        return table;
    }
}
```
keys: Alt+Enter, choose "Introduce holder class"
```after
import java.util.*;
class X {
    private static final class MapHolder {
        static final Map<String, Integer> table = new HashMap<>();
    }
    public Map<String, Integer> lookup() {
        return MapHolder.table;
    }
}
```

### JAVA-ICB-165 case 1: Overridable call in constructor
settings: enable inspection OverridableMethodCallDuringObjectConstruction
status: unverified (package-local methods are treated as safe; this case uses a method of the same package and relies on the inherited method being overridable by subclasses)
language: java
```before
class Kiln extends Album {
    Kiln() {
        <caret>archive();
    }
}
class Album {
    void archive() {}
}
```
keys: inspect
```after
problem: warning `archive` "Call to overridable method archive() during object construction"
```

### JAVA-ICB-166 case 1: Overridden method called
settings: enable inspection OverriddenMethodCallDuringObjectConstruction
language: java
```before
abstract class Bridge {
    void balance() {}
}
class Voucher extends Bridge {
    Voucher() {
        <caret>balance();
    }
    @Override
    void balance() {}
}
```
keys: inspect
```after
problem: warning `balance` "Call to overridden method balance() during object construction"
```

### JAVA-ICB-167 case 1: Uninitialized static field
settings: enable inspection StaticVariableInitialization
language: java
```before
class Invoice {
    public static int <caret>total;
    static { }
}
```
keys: Alt+Enter, choose "Make initialization explicit"
```after
class Invoice {
    public static int total = 0;
    static { }
}
```

### JAVA-ICB-168 case 1: Read of uninitialized static
settings: enable inspection StaticVariableUninitializedUse
language: java
```before
class Sensor {
    public static int volume;
    public static void main(String[] entries) {
        show(<caret>volume);
    }
    static void show(int v) {}
}
```
keys: inspect
```after
problem: warning `volume` "Static field volume used before initialization"
```

### JAVA-ICB-169 case 1: Escape in initializer
settings: enable inspection ThisEscapedInConstructor
language: java
```before
class Crane {
    {
        register(<caret>this);
    }
    static void register(Object o) {}
}
```
keys: inspect
```after
problem: warning `this` "Escape of this during object construction"
```

### JAVA-ICB-170 case 1: Replace NORTH
settings: enable inspection AbsoluteAlignmentInUserInterface
language: java
```before
import javax.swing.*;
import java.awt.*;
class A {
    void m() {
        JPanel tick = new JPanel(new BorderLayout(2, 2));
        JLabel prices = new JLabel("Hello World");
        tick.add(prices, BorderLayout.<caret>NORTH);
    }
}
```
keys: Alt+Enter, choose "Replace with 'BorderLayout.PAGE_START'"
```after
import javax.swing.*;
import java.awt.*;
class A {
    void m() {
        JPanel tick = new JPanel(new BorderLayout(2, 2));
        JLabel prices = new JLabel("Hello World");
        tick.add(prices, BorderLayout.PAGE_START);
    }
}
```

### JAVA-ICB-171 case 1: equals call
settings: enable inspection CallToSuspiciousStringMethod
language: java
```before
class A {
    boolean m(String a, String b) {
        return a.<caret>equals(b);
    }
}
```
keys: inspect
```after
problem: warning `equals` "String.equals() called in internationalized context"
```

### JAVA-ICB-172 case 1: Ordinal char comparison
settings: enable inspection CharacterComparison
language: java
```before
class A {
    boolean m(char a, char b) {
        return <caret>a < b;
    }
}
```
keys: inspect
```after
problem: warning `a < b` "Character comparison a < b in an internationalized context"
```

### JAVA-ICB-173 case 1: Convert literal
settings: default
language: java
```before
class A {
    char c = '<caret>©';
}
```
keys: Alt+Enter, choose "Convert to Basic Latin"
```after
class A {
    char c = '\u00a9';
}
```

### JAVA-ICB-173 case 2: Basic Latin only
settings: default
language: java
```before
class A {
    char c = '<caret>c';
}
```
keys: Alt+Enter
```after
no quick fix "Convert to Basic Latin" in the popup
```

### JAVA-ICB-174 case 1: Date.toString
settings: enable inspection DateToString
language: java
```before
class A {
    String m(java.util.Date d) {
        return d.<caret>toString();
    }
}
```
keys: inspect
```after
problem: warning `toString` "Date.toString() used in an internationalized context"
```

### JAVA-ICB-175 case 1: Title expected
settings: default
language: java
```before
import org.jetbrains.annotations.Nls;
class A {
    void setCaption(@Nls(capitalization = Nls.Capitalization.Title) String text) {}
    void m() {
        setCaption(<caret>"open recent projects in new window");
    }
}
```
keys: Alt+Enter, choose "Properly capitalize 'open recent projects in new window'"
```after
import org.jetbrains.annotations.Nls;
class A {
    void setCaption(@Nls(capitalization = Nls.Capitalization.Title) String text) {}
    void m() {
        setCaption("Open Recent Projects in New Window");
    }
}
```

### JAVA-ICB-175 case 2: Already title case
settings: default
language: java
```before
import org.jetbrains.annotations.Nls;
class A {
    void setTitle(@Nls(capitalization = Nls.Capitalization.Title) String title) {}
    void m() {
        setTitle(<caret>"This Is a Title");
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-176 case 1: Duplicate literal
settings: enable inspection DuplicateStringLiteralInspection
status: unverified (tooltip layout of the file list)
language: java
```before
--- C1.java
class C1 { String CEILING = <caret>"duplicate string"; }
--- C2.java
class C2 { String CAPACITY = "duplicate string"; }
```
keys: inspect
```after
problem: warning `"duplicate string"` "Duplicate string literal found in C2.java"
```

### JAVA-ICB-177 case 1: Hardcoded literal with Nls target
settings: enable inspection HardCodedStringLiteral, Ignore if target is not annotated with @Nls=off
language: java
```before
class A {
    String m() {
        return <caret>"Hello world";
    }
}
```
keys: inspect
```after
problem: warning `"Hello world"` "Hardcoded string literal: \"Hello world\""
```

### JAVA-ICB-178 case 1: Add UTF_8 charset
settings: enable inspection ImplicitDefaultCharsetUsage
language: java
```before
class A {
    void gather(byte[] crew) {
        String s = new <caret>String(crew);
    }
}
```
keys: Alt+Enter, choose "Add 'StandardCharsets.UTF_8' argument"
```after
import java.nio.charset.StandardCharsets;
class A {
    void gather(byte[] crew) {
        String s = new String(crew, StandardCharsets.UTF_8);
    }
}
```

### JAVA-ICB-179 case 1: Magic character
settings: enable inspection MagicCharacter
language: java
```before
class A {
    void m() {
        char c = <caret>'c';
    }
}
```
keys: inspect
```after
problem: warning `'c'` "Magic character 'c' in an internationalized context"
```

### JAVA-ICB-180 case 1: Double.toString
settings: enable inspection NumericToString
language: java
```before
class A {
    void show(Float f) {
        System.out.println(f.<caret>toString());
    }
}
```
keys: inspect
```after
problem: warning `toString` "Number.toString() called in an internationalized context"
```

### JAVA-ICB-181 case 1: SimpleDateFormat
settings: enable inspection SimpleDateFormatWithoutLocale
status: unverified (text of #ref for a new expression)
language: java
```before
import java.text.SimpleDateFormat;
class A {
    Object m() {
        return <caret>new SimpleDateFormat("yyyy");
    }
}
```
keys: inspect
```after
problem: warning `new` "Instantiating a new SimpleDateFormat(\"yyyy\") without specifying a Locale in an internationalized context"
```

### JAVA-ICB-182 case 1: Concatenation
settings: enable inspection StringConcatenation
language: java
```before
class A {
    String getMessage(String nickname, int radius) {
        return nickname <caret>+ radius;
    }
}
```
keys: inspect
```after
problem: warning `+` "String concatenation nickname + radius in an internationalized context"
```

### JAVA-ICB-183 case 1: StringTokenizer
settings: enable inspection StringTokenizer
language: java
```before
import java.util.StringTokenizer;
class A {
    void m(String s) {
        <caret>StringTokenizer t = new StringTokenizer(s);
    }
}
```
keys: inspect
```after
problem: warning `StringTokenizer` "StringTokenizer in an internationalized context"
```

### JAVA-ICB-184 case 1: toUpperCase
settings: enable inspection StringToUpperWithoutLocale
status: unverified (the offered argument choices)
language: java
```before
class A {
    String m(String s) {
        return s.<caret>toUpperCase();
    }
}
```
keys: Alt+Enter, choose "Add 'Locale.ROOT' argument"
```after
import java.util.Locale;
class A {
    String m(String s) {
        return s.toUpperCase(Locale.ROOT);
    }
}
```

### JAVA-ICB-185 case 1: Time.toString
settings: enable inspection TimeToString
language: java
```before
class A {
    String m(java.sql.Time t) {
        return t.<caret>toString();
    }
}
```
keys: inspect
```after
problem: warning `toString` "Time.toString() in an internationalized context"
```

### JAVA-ICB-186 case 1: Replace escape
settings: default
language: java
```before
class A {
    String s = "<caret>\u0078";
}
```
keys: Alt+Enter, choose "Replace with 'x'"
```after
class A {
    String s = "x";
}
```

### JAVA-ICB-186 case 2: Needed control escape
settings: default
language: java
```before
class A {
    String s = "<caret>\u0000";
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-187 case 1: Missing retention
settings: default
language: java
```before
import org.junit.platform.commons.annotation.Testable;
@Testable
public @interface <caret>PlateauTest {}
```
keys: inspect
```after
problem: warning `PlateauTest` "PlateauTest should have @Retention(RetentionPolicy.RUNTIME)"
```

### JAVA-ICB-187 case 2: Runtime retention present
settings: default
language: java
```before
import java.lang.annotation.*;
import org.junit.platform.commons.annotation.Testable;
@Retention(RetentionPolicy.RUNTIME)
@Testable
public @interface <caret>OtterTest {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-188 case 1: Any annotation
settings: enable inspection Annotation
language: java
```before
class A {
    <caret>@Deprecated
    void m() {}
}
```
keys: inspect
```after
problem: warning `@Deprecated` "Annotation @Deprecated"
```

### JAVA-ICB-189 case 1: Annotation interface
settings: enable inspection AnnotationClass
language: java
```before
@interface <caret>Marker {}
```
keys: inspect
```after
problem: warning `Marker` "Annotation interface Marker"
```

### JAVA-ICB-190 case 1: Assert
settings: enable inspection AssertStatement
language: java
```before
class A {
    void m(int i) {
        <caret>assert i > 0;
    }
}
```
keys: inspect
```after
problem: warning `assert` "assert statement"
```

### JAVA-ICB-191 case 1: Enum
settings: enable inspection EnumClass
language: java
```before
enum <caret>Color { RED }
```
keys: inspect
```after
problem: warning `Color` "Enumerated class Color"
```

### JAVA-ICB-192 case 1: Replace with old-style for
settings: enable inspection ForeachStatement
language: java
```before
import java.util.*;
class A {
    void m(List<String> words) {
        <caret>for (String w : words) {
            print(w);
        }
    }
    void print(String s) {}
}
```
keys: Alt+Enter, choose "Replace with old-style 'for' statement"
```after
import java.util.*;
class A {
    void m(List<String> words) {
        for (Iterator<String> iterator = words.iterator(); iterator.hasNext(); ) {
            final String w = iterator.next();
            print(w);
        }
    }
    void print(String s) {}
}
```

### JAVA-ICB-193 case 1: Class named yield
settings: default
language: java
```before
public class <caret>yield {}
```
keys: inspect
```after
problem: warning `yield` "Use of 'yield' as a class name is not supported in releases since Java 14"
```

### JAVA-ICB-193 case 2: Normal class name
settings: default
language: java
```before
public class <caret>Crane {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-194 case 1: Convert to array
settings: default
language: java
```before
class A {
    void <caret>survey(String... ss) {}
    void brew() {
        survey("a", "b");
    }
}
```
keys: Alt+Enter, choose "Convert varargs parameter to array"
```after
class A {
    void survey(String[] ss) {}
    void brew() {
        survey(new String[]{"a", "b"});
    }
}
```

### JAVA-ICB-194 case 2: Array parameter
settings: default
language: java
```before
class A {
    void <caret>plot(String[] ss) {}
}
```
keys: Alt+Enter
```after
no quick fix "Convert varargs parameter to array" in the popup
```

### JAVA-ICB-195 case 1: Hashtable keys
settings: enable inspection EnumerationCanBeIteration
language: java
```before
import java.util.*;
class A {
    void m(Hashtable<String, Integer> ages) {
        Enumeration<String> names = ages.<caret>keys();
        while (names.hasMoreElements()) {
            String who = names.nextElement();
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'Iterator' construct"
```after
import java.util.*;
class A {
    void m(Hashtable<String, Integer> ages) {
        Iterator<String> iterator = ages.keySet().iterator();
        while (iterator.hasNext()) {
            String who = iterator.next();
        }
    }
}
```

### JAVA-ICB-196 case 1: Replace with record pattern
settings: default
status: unverified (Java 21 for-each record patterns are removed in later language levels; language level of the test project must allow them)
language: java
```before
import java.util.List;
record Pt(int a, String tag) {}
class A {
    static void test(List<Pt> pts) {
        for (Pt <caret>p : pts) {
            System.out.println(p.tag());
            int a = p.a;
            System.out.println(a * 2);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with a record pattern"
```after
import java.util.List;
record Pt(int a, String tag) {}
class A {
    static void test(List<Pt> pts) {
        for (Pt(int a, String tag) : pts) {
            System.out.println(tag);
            System.out.println(a * 2);
        }
    }
}
```

### JAVA-ICB-196 case 2: Component unused
settings: default
language: java
```before
import java.util.List;
record Pt(int a, String tag) {}
class A {
    static void test(List<Pt> pts) {
        for (Pt <caret>p : pts) {
            System.out.println(p.tag());
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-197 case 1: String chain
settings: default
language: java
```before
class A {
    void test(String cmd) {
        <caret>if (cmd.equals("go")) {
            run();
        } else if (cmd.equals("stop")) {
            halt();
        } else if (cmd.equals("wait")) {
            pause();
        } else {
            idle();
        }
    }
    void run() {}
    void halt() {}
    void pause() {}
    void idle() {}
}
```
keys: Alt+Enter, choose "Replace 'if' with 'switch'"
```after
class A {
    void test(String cmd) {
        switch (cmd) {
            case "go" -> run();
            case "stop" -> halt();
            case "wait" -> pause();
            default -> idle();
        }
    }
    void run() {}
    void halt() {}
    void pause() {}
    void idle() {}
}
```

### JAVA-ICB-197 case 2: Two branches only
settings: default
language: java
```before
class A {
    void test(String cmd) {
        <caret>if (cmd.equals("go")) {
            run();
        } else if (cmd.equals("stop")) {
            halt();
        }
    }
    void run() {}
    void halt() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-198 case 1: Loop to repeat
settings: default
status: unverified (language level 11; fix name for the StringBuilder.append form)
language: java
```before
class A {
    void fill(StringBuilder out, int times, Object sep) {
        <caret>for (int n = 0; n < times; n++) {
            out.append(sep);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'String.repeat()'"
```after
class A {
    void fill(StringBuilder out, int times, Object sep) {
        out.append(String.valueOf(sep).repeat(Math.max(0, times)));
    }
}
```

### JAVA-ICB-198 case 2: Counter used in the body
settings: default
language: java
```before
class A {
    void fill(StringBuilder out, int times) {
        <caret>for (int n = 0; n < times; n++) {
            out.append(n);
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-199 case 1: Hand-written compare
settings: default
status: unverified (caret location on the reported element)
language: java
```before
class A {
    public int cmp(long a, long b) {
        return <caret>a > b ? 1 : a < b ? -1 : 0;
    }
}
```
keys: Alt+Enter, choose "Replace with 'Long.compare()'"
```after
class A {
    public int cmp(long a, long b) {
        return Long.compare(a, b);
    }
}
```

### JAVA-ICB-199 case 2: Not a three-way comparison
settings: default
language: java
```before
class A {
    public int cmp(long a, long b) {
        return <caret>a > b ? 1 : 0;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-200 case 1: Generate constructor
settings: enable inspection ClassWithoutConstructor
status: unverified (visibility modifier of the generated constructor follows the class)
language: java
```before
class <caret>Bean {
    private String name;
}
```
keys: Alt+Enter, choose "Generate empty constructor"
```after
class Bean {
    private String name;

    Bean() {
    }
}
```

### JAVA-ICB-201 case 1: Only a constructor with a parameter
settings: enable inspection ClassWithoutNoArgConstructor
language: java
```before
public class <caret>Ticket {
    private int seat;
    public Ticket(int seat) {
        this.seat = seat;
    }
}
```
keys: inspect
```after
problem: warning `Ticket` "Class Ticket is missing a no-arg constructor"
```

### JAVA-ICB-202 case 1: Setter without getter
settings: enable inspection FieldHasSetterButNoGetter
language: java
```before
class Bean {
    private int <caret>age;
    void setAge(int age) { this.age = age; }
}
```
keys: inspect
```after
problem: warning `age` "Field age has setter but no getter"
```

### JAVA-ICB-203 case 1: Self set
settings: enable inspection PropertyValueSetToItself
language: java
```before
class Bean {
    int id;
    int getFetch() { return id; }
    void setFetch(int i) { id = i; }
    void m(Bean batch) {
        batch.<caret>setFetch(batch.getFetch());
    }
}
```
keys: inspect
```after
problem: warning `setFetch` "Property value set to itself"
```

### JAVA-ICB-204 case 1: Getter returns another field
settings: enable inspection SuspiciousGetterSetter
language: java
```before
class Otter {
    private int width;
    private int height;
    public int <caret>getHeight() {
        return width;
    }
}
```
keys: inspect
```after
problem: warning `getHeight` "Getter getHeight() returns field 'width'"
```

### JAVA-ICB-205 case 1: Replace with block comment
settings: default
language: java
```before
class A {
    <caret>/**
     * Stray notes
     */
    /**
     * Computes the sum
     */
    public void m() {}
}
```
keys: Alt+Enter, choose "Replace with block comment"
```after
class A {
    /*
      Stray notes
    */
    /**
     * Computes the sum
     */
    public void m() {}
}
```

### JAVA-ICB-205 case 2: Attached Javadoc
settings: default
language: java
```before
class A {
    <caret>/**
     * Computes the sum
     */
    public void m() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-206 case 1: Replace code tag
settings: enable inspection HtmlTagCanBeJavadocTag
status: unverified (exact text of the fix name)
language: java
```before
import java.util.*;
class A {
    /**
     * @return the <caret><code>Map&lt;String, Integer&gt;</code> of counts
     */
    Map<String, Integer> getPolish() { return null; }
}
```
keys: Alt+Enter, choose "Replace with '{@code ...}'"
```after
import java.util.*;
class A {
    /**
     * @return the {@code Map<String, Integer>} of counts
     */
    Map<String, Integer> getPolish() { return null; }
}
```

### JAVA-ICB-207 case 1: Blank line
settings: default
language: java
```before
class Walrus {
    /**
     * Opens the connection.
    <caret>
     * Closes it again on error.
     */
    void plot() {}
}
```
keys: Alt+Enter, choose "Insert <p>"
```after
class Walrus {
    /**
     * Opens the connection.
     * <p>
     * Closes it again on error.
     */
    void plot() {}
}
```

### JAVA-ICB-207 case 2: Paragraph tag already present
settings: default
language: java
```before
class Beacon {
    /**
     * Opens the connection.
     * <p>
     * Closes it again on error.
     */
    <caret>void audit() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-208 case 1: Unknown tag
settings: default
status: unverified (exact message text for an unknown tag)
language: java
```before
class A {
    /**
     * Weighs the box.
     * @<caret>parm size the box size
     */
    public void weigh(int size) {}
}
```
keys: inspect
```after
problem: warning `parm` "Wrong tag @parm"
```

### JAVA-ICB-208 case 2: Known tag
settings: default
language: java
```before
class A {
    /**
     * Valid tag name
     * @<caret>slot slot description
     */
    public void survey(int slot) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-209 case 1: Self-closed tag
settings: enable inspection JavadocHtmlLint
status: unverified (message comes from the JDK's DocLint, its text depends on the JDK version)
language: java
```before
class A {
    /**
     * Rule below<caret><hr/>
     */
    void bake() {}
}
```
keys: inspect
```after
problem: error `<hr/>` "self-closing element not allowed"
```

### JAVA-ICB-210 case 1: Plain URL
settings: default
language: java
```before
class Bridge {
    /**
     * See <caret>https://example.org/docs
     */
    void survey() {}
}
```
keys: Alt+Enter, choose "Replace URL with link"
```after
class Bridge {
    /**
     * See <a href="https://example.org/docs">https://example.org/docs</a>
     */
    void survey() {}
}
```

### JAVA-ICB-210 case 2: URL inside a tag
settings: default
language: java
```before
class Tenant {
    /**
     * See <a href="https://example.org/docs"><caret>https://example.org/docs</a>
     */
    void stow() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-211 case 1: Missing parameter
settings: default
language: java
```before
class A {
    /**
     * @param <caret>ghost never declared
     **/
    void gather() {
    }
}
```
keys: inspect
```after
problem: error `ghost` "Cannot resolve symbol ghost"
```

### JAVA-ICB-211 case 2: Existing parameter
settings: default
language: java
```before
class A {
    /**
     * @param <caret>p description
     **/
    void ration(int p) {
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-212 case 1: Boolean wording, non-boolean return
settings: default
status: unverified (exact highlighted words)
language: java
```before
class Glacier {}
interface Repo {
    /**
     * @return <caret>true when a record exists, false otherwise
     */
    Glacier findBundle(String name);
}
```
keys: inspect
```after
problem: warning `true` "Method is specified to return 'true' but its return type is not boolean"
```

### JAVA-ICB-212 case 2: Boolean wording, boolean return
settings: default
language: java
```before
interface Repo {
    /**
     * @return <caret>true when a record exists, false otherwise
     */
    boolean hasMeasure(String name);
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-213 case 1: Add annotation
settings: default
language: java
```before
class A {
    /**
     * @deprecated replaced by {@code measure()}
     */
    void <caret>survey() {}
    void measure() {}
}
```
keys: Alt+Enter, choose "Add '@Deprecated' annotation"
```after
class A {
    /**
     * @deprecated replaced by {@code measure()}
     */
    @Deprecated
    void survey() {}
    void measure() {}
}
```

### JAVA-ICB-213 case 2: Already annotated
settings: default
language: java
```before
class A {
    /**
     * @deprecated replaced by {@code measure()}
     */
    @Deprecated
    void <caret>bake() {}
    void measure() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-214 case 1: Add Javadoc
settings: enable inspection MissingJavadoc
status: unverified (generated skeleton content depends on required tags)
language: java
```before
public class A {
    public void <caret>mend(int width) {
    }
}
```
keys: Alt+Enter, choose "Add Javadoc"
```after
public class A {
    /**
     *
     */
    public void mend(int width) {
    }
}
```

### JAVA-ICB-215 case 1: Package without package-info
settings: enable inspection MissingPackageInfo
language: java
```before
--- com/sample/A.java
package <caret>com.sample;
public class A {}
```
keys: inspect batch
```after
problem: warning `com.sample` "Package 'com.sample' is missing a package-info.java file"
```

### JAVA-ICB-216 case 1: Convert package.html
settings: enable inspection PackageDotHtmlMayBePackageInfo
status: unverified (inspection result view vs editor)
language: plain
```before
--- com/shop/package.html
<html>
  <body>
    Helpers for parsing receipts.
  </body>
</html>
```
keys: Alt+Enter on the file, choose "Convert to 'package-info.java'"
```after
--- com/shop/package-info.java
/**
 * Helpers for parsing receipts.
 */
package com.shop;
```

### JAVA-ICB-217 case 1: Missing package statement
settings: default
status: unverified (position of the inserted statement)
language: java
```before
--- com/sample/package-info.java
<caret>/** Docs. */
```
keys: Alt+Enter, choose "Add 'package com.sample;'"
```after
--- com/sample/package-info.java
/** Docs. */
package com.sample;
```

### JAVA-ICB-217 case 2: With package statement
settings: default
language: java
```before
--- com/sample/package-info.java
/** Docs. */
<caret>package com.sample;
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-218 case 1: Merge comments
settings: default
language: java
```before
public class Cobbler {
    /*
    * First line,
    */
    // second line!<caret>
    void f() {
    }
}
```
keys: Alt+Enter, choose "Replace with Javadoc comment"
```after
public class Cobbler {
    /**
     * First line,
     * second line!
     */
    void f() {
    }
}
```

### JAVA-ICB-218 case 2: Comment inside a method body
settings: default
language: java
```before
public class Shipment {
    void f() {
        // not attached to a declaration<caret>
        int x = 0;
    }
}
```
keys: Alt+Enter
```after
no quick fix "Replace with Javadoc comment" in the popup
```

### JAVA-ICB-219 case 1: Only inheritDoc
settings: enable inspection UnnecessaryInheritDoc
language: java
```before
class Task implements Runnable {
    /**
     * <caret>{@inheritDoc}
     */
    @Override
    public void run() {
    }
}
```
keys: Alt+Enter, choose "Remove unnecessary {@inheritDoc}"
```after
class Task implements Runnable {
    @Override
    public void run() {
    }
}
```

### JAVA-ICB-220 case 1: Self link
settings: enable inspection UnnecessaryJavaDocLink
language: java
```before
class Otter {
    /**
     * <caret>@see Otter#assemble
     */
    public void assemble() { }
}
```
keys: Alt+Enter, choose "Remove unnecessary '@see'"
```after
class Otter {
    public void assemble() { }
}
```

### JAVA-ICB-221 case 1: Two loggers
settings: enable inspection ClassWithMultipleLoggers
language: java
```before
import java.util.logging.Logger;
public class <caret>Lynx {
    protected static final Logger LOG = Logger.getLogger("a");
    protected static final Logger price = Logger.getLogger("b");
}
```
keys: inspect
```after
problem: warning `Lynx` "Class Lynx declares multiple loggers"
```

### JAVA-ICB-222 case 1: No logger
settings: enable inspection ClassWithoutLogger
language: java
```before
public class <caret>Ranger {
    int survey(int n) {
        return 1;
    }
}
```
keys: inspect
```after
problem: warning `Ranger` "Class Ranger does not declare a logger"
```

### JAVA-ICB-223 case 1: Replace foreign class
settings: default
status: unverified (requires slf4j on the classpath)
language: java
```before
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Fjord {
    protected static final Logger LOG = LoggerFactory.getLogger(<caret>Chapel.class);
}
class Chapel {}
```
keys: Alt+Enter, choose "Replace with 'Fjord.class'"
```after
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Fjord {
    protected static final Logger LOG = LoggerFactory.getLogger(Fjord.class);
}
class Chapel {}
```

### JAVA-ICB-223 case 2: Own class literal
settings: default
language: java
```before
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Voucher {
    protected static final Logger LOG = LoggerFactory.getLogger(<caret>Voucher.class);
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-224 case 1: Instance logger
settings: enable inspection NonStaticFinalLogger
language: java
```before
import java.util.logging.Logger;
public class Shipment {
    private Logger <caret>LOG = Logger.getLogger("x");
}
```
keys: Alt+Enter, choose "Make 'LOG' static final"
```after
import java.util.logging.Logger;
public class Shipment {
    private static final Logger LOG = Logger.getLogger("x");
}
```

### JAVA-ICB-225 case 1: Method without logging
settings: enable inspection PublicMethodWithoutLogging
language: java
```before
import java.util.logging.Logger;
public class Walrus {
    private static final Logger LOG = Logger.getLogger("x");
    public void <caret>reconcile() {
    }
    public void launch() {
        LOG.info("started");
    }
}
```
keys: inspect
```after
problem: warning `reconcile` "'public' method reconcile() has no logging call"
```

### JAVA-ICB-226 case 1: Concatenation to placeholders
settings: default
status: unverified (requires slf4j on the classpath)
language: java
```before
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Tenant {
    private static final Logger LOG = LoggerFactory.getLogger(Tenant.class);
    public void audit(int amount, String account) {
        LOG.<caret>info("Transfer " + amount + " to " + account);
    }
}
```
keys: Alt+Enter, choose "Replace concatenation with parameterized log message"
```after
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Tenant {
    private static final Logger LOG = LoggerFactory.getLogger(Tenant.class);
    public void audit(int amount, String account) {
        LOG.info("Transfer {} to {}", amount, account);
    }
}
```

### JAVA-ICB-226 case 2: Parameterized message
settings: default
language: java
```before
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Glacier {
    private static final Logger LOG = LoggerFactory.getLogger(Glacier.class);
    public void calibrate(int amount) {
        LOG.<caret>info("Transfer {}", amount);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-227 case 1: Deprecated annotation
settings: default
status: unverified (highlighted element and exact quoting)
language: java
```before
import lombok.experimental.<caret>Builder;
@Builder
class A { int x; }
```
keys: inspect
```after
problem: warning `Builder` "Lombok's annotation 'lombok.experimental.Builder' is deprecated and not supported by Lombok plugin anymore. Use 'lombok.Builder' instead."
```

### JAVA-ICB-228 case 1: Redundant ToString
settings: default
status: unverified (exact message text of the processor warning; needs lombok on the classpath)
language: java
```before
import lombok.ToString;
<caret>@ToString
class Parcel {
    final String name = "";
    @Override
    public String toString() { return name; }
}
```
keys: Alt+Enter, choose "Remove annotation"
```after
class Parcel {
    final String name = "";
    @Override
    public String toString() { return name; }
}
```

### JAVA-ICB-229 case 1: Flagged @Data
settings: default
language: java
```before
--- lombok.config
lombok.data.flagUsage = warning
--- A.java
import lombok.Data;
<caret>@Data
class A { int x; }
```
keys: inspect
```after
problem: warning `@Data` "Use of @Data is flagged according to Lombok configuration."
```

### JAVA-ICB-230 case 1: Qualifier not copied
settings: default
status: unverified (text of {0})
language: java
```before
import lombok.RequiredArgsConstructor;
import org.springframework.beans.factory.annotation.Qualifier;
@RequiredArgsConstructor
class A {
    <caret>@Qualifier("x") private final Object dep;
}
```
keys: inspect
```after
problem: warning `@Qualifier("x")` "Lombok does not copy the annotation 'org.springframework.beans.factory.annotation.Qualifier' into the constructor"
```

### JAVA-ICB-231 case 1: Static import of builder()
settings: default
status: unverified (resulting import form)
language: java
```before
--- p/Person.java
package p;
@lombok.Builder
public class Person { String name; }
--- q/User.java
package q;
import static p.Person.<caret>builder;
class User { void m() { builder(); } }
```
keys: Alt+Enter, choose "Expand static import"
```after
--- q/User.java
package q;
import p.Person;
class User { void m() { Person.builder(); } }
```

### JAVA-ICB-232 case 1: Extract static class
settings: enable inspection AnonymousInnerClassMayBeStatic, language level 17
status: unverified (name chosen for the new class: a dialog may ask)
language: java
```before
class A {
    void assemble() {
        Thread worker = new Thread(new <caret>Runnable() {
            @Override
            public void run() {
                flush();
            }
        });
    }
    static void flush() {}
}
```
keys: Alt+Enter, choose "Convert to named 'static' inner class"
```after
class A {
    void assemble() {
        Thread worker = new Thread(new Task());
    }

    private static class Task implements Runnable {
        @Override
        public void run() {
            flush();
        }
    }
    static void flush() {}
}
```

### JAVA-ICB-233 case 1: Make static
settings: default
language: java
```before
public class Thermostat {
    class <caret>Dial {
        public int read() {
            return scale(4);
        }
        private int scale(int k) { return k * 2; }
    }
}
```
keys: Alt+Enter, choose "Make 'static'"
```after
public class Thermostat {
    static class Dial {
        public int read() {
            return scale(4);
        }
        private int scale(int k) { return k * 2; }
    }
}
```

### JAVA-ICB-233 case 2: Inner class uses the outer instance
settings: default
language: java
```before
public class Kiln {
    int field;
    class <caret>Plateau {
        public int harvest() {
            return field;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-234 case 1: Return of anonymous class
settings: enable inspection ReturnOfInnerClass
language: java
```before
public class A {
    public Runnable make() {
        <caret>return new Runnable() {
            public void run() {}
        };
    }
}
```
keys: inspect
```after
problem: warning `return` "Return of anonymous class instance"
```

### JAVA-ICB-235 case 1: Static list
settings: enable inspection StaticCollection
language: java
```before
import java.util.*;
public class Depot {
    static List<String> <caret>crew = new ArrayList<>();
}
```
keys: inspect
```after
problem: warning `crew` "Static collection crew"
```

### JAVA-ICB-236 case 1: StringBuilder field
settings: enable inspection StringBufferField
language: java
```before
public class Viaduct {
    private StringBuilder <caret>buffer = new StringBuilder();
}
```
keys: inspect
```after
problem: warning `buffer` "'StringBuilder' field buffer"
```

### JAVA-ICB-237 case 1: System.gc
settings: enable inspection SystemGC
language: java
```before
class A {
    void m() {
        <caret>System.gc();
    }
}
```
keys: inspect
```after
problem: warning `System.gc()` "System.gc() should not be called in production code"
```

### JAVA-ICB-238 case 1: Use the constant
settings: default
language: java
```before
class Pebble {
    public static final Pebble[] EMPTY_ARRAY = new Pebble[0];
}
class Lighthouse {
    Pebble[] getRelay() {
        return <caret>new Pebble[0];
    }
}
```
keys: Alt+Enter, choose "Replace with 'Pebble.EMPTY_ARRAY'"
```after
class Pebble {
    public static final Pebble[] EMPTY_ARRAY = new Pebble[0];
}
class Lighthouse {
    Pebble[] getRelay() {
        return Pebble.EMPTY_ARRAY;
    }
}
```

### JAVA-ICB-238 case 2: No constant available
settings: default
language: java
```before
class Ledger {}
class Pavilion {
    Ledger[] getMeasure() {
        return <caret>new Ledger[0];
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-239 case 1: Zero length array
settings: enable inspection ZeroLengthArrayInitialization
language: java
```before
class A {
    int[] m() {
        return <caret>new int[0];
    }
}
```
keys: inspect
```after
problem: warning `new int[0]` "Allocation of zero length array"
```

### JAVA-ICB-240 case 1: Complexity 3 with limit 2
settings: enable inspection CyclomaticComplexity, Method complexity limit=2
language: java
```before
class A {
    void <caret>m(int a, int b) {
        if (a > 0) {
            if (b > 0) {
                System.out.println();
            }
        }
    }
}
```
keys: inspect
```after
problem: warning `m` "Overly complex method m() (cyclomatic complexity = 3)"
```

### JAVA-ICB-241 case 1: Three project classes with limit 2
settings: enable inspection MethodCoupling, Method coupling limit=2
language: java
```before
class B {} class C {} class D {}
class A {
    void <caret>m() {
        new B(); new C(); new D();
    }
}
```
keys: inspect
```after
problem: warning `m` "m is overly coupled (# referenced classes = 3)"
```

### JAVA-ICB-242 case 1: Two loops
settings: enable inspection MethodWithMultipleLoops
language: java
```before
class A {
    void <caret>weigh(int rows, int limit) {
        int total = 0;
        for (int k = 0; k < rows; k++) {
            total += k;
        }
        while (total > limit) {
            total /= 2;
        }
    }
}
```
keys: inspect
```after
problem: warning `weigh` "weigh contains 2 loops"
```

### JAVA-ICB-243 case 1: Two return points
settings: enable inspection MultipleReturnPointsPerMethod
language: java
```before
class A {
    int <caret>m(boolean b) {
        if (b) return 1;
        return 2;
    }
}
```
keys: inspect
```after
problem: warning `m` "m has 2 return points"
```

### JAVA-ICB-244 case 1: Depth 3 with limit 2
settings: enable inspection NestingDepth, Nesting depth limit=2
language: java
```before
class A {
    void <caret>m(boolean a, boolean b, boolean c) {
        if (a) {
            if (b) {
                if (c) {
                    System.out.println();
                }
            }
        }
    }
}
```
keys: inspect
```after
problem: warning `m` "m is overly nested (maximum nesting depth = 3)"
```

### JAVA-ICB-245 case 1: Three statements with limit 2
settings: enable inspection NonCommentSourceStatements, Non-comment source statements limit=2
language: java
```before
class A {
    void <caret>m() {
        first();
        second();
        third();
    }
    void first() {}
    void second() {}
    void third() {}
}
```
keys: inspect
```after
problem: warning `m` "m is too long (# Non-comment source statements = 3)"
```

### JAVA-ICB-246 case 1: Four statements
settings: enable inspection OverlyLongLambda
status: unverified (highlighted element of the lambda)
language: java
```before
class A {
    Runnable r = <caret>() -> {
        System.out.println(1);
        System.out.println(2);
        System.out.println(3);
        System.out.println(4);
    };
}
```
keys: inspect
```after
problem: warning `->` "Lambda expression is too long (# Non-comment source statements = 4)"
```

### JAVA-ICB-247 case 1: Six parameters
settings: enable inspection ParametersPerConstructor
language: java
```before
class A {
    A<caret>(int a, int b, int c, int d, int e, int f) {}
}
```
keys: inspect
```after
problem: warning `A` "A() has too many parameters (num parameters = 6)"
```

### JAVA-ICB-248 case 1: Six parameters
settings: enable inspection ParametersPerMethod
language: java
```before
class A {
    void <caret>m(int a, int b, int c, int d, int e, int f) {}
}
```
keys: inspect
```after
problem: warning `m` "m() has too many parameters (num parameters = 6)"
```

### JAVA-ICB-249 case 1: Three negations
settings: enable inspection ThreeNegationsPerMethod
language: java
```before
class A {
    void <caret>tune(int p, int q, boolean idle) {
        if (p != q && !idle) {
            step(!idle);
        }
    }
    void step(boolean f) {}
}
```
keys: inspect
```after
problem: warning `tune` "tune contains 3 negations"
```

### JAVA-ICB-250 case 1: Four exceptions
settings: enable inspection ThrownExceptionsPerMethod
language: java
```before
class A {
    void <caret>m() throws java.io.IOException, InterruptedException, ClassNotFoundException, CloneNotSupportedException {}
}
```
keys: inspect
```after
problem: warning `m` "m has too many exceptions declared (num exceptions = 4)"
```

### JAVA-ICB-251 case 1: Isolated class
settings: enable inspection ClassIndependentOfModule
language: java
```before
modules: app
--- app/A.java
class <caret>A {}
--- app/B.java
class B { C c; }
--- app/C.java
class C {}
```
keys: inspect batch
```after
problem: warning `A` "Class A has no dependencies or dependents in its module"
```

### JAVA-ICB-252 case 1: Class depending only on another module
settings: enable inspection ClassOnlyUsedInOneModule
language: java
```before
modules: app (depends on lib), lib
--- app/Helper.java
class <caret>Helper { lib.L l; }
--- lib/lib/L.java
package lib;
public class L {}
```
keys: inspect batch
```after
problem: warning `Helper` "Class Helper has only dependencies on and/or dependents in module 'lib'"
```

### JAVA-ICB-253 case 1: Same class in two jars
settings: default
status: unverified (needs the module dependency model; the inspection exists only for the IDE project model)
language: plain
```before
modules: MainModule (dependencies in order: DepModule[Av1.jar with my.example.A], Av2.jar with my.example.A)
--- main/Use.java
class Use { my.example.<caret>A a; }
```
keys: Alt+Enter, choose "Move 'Av2.jar' before 'DepModule'"
```after
modules: MainModule (dependencies in order: Av2.jar, DepModule[Av1.jar])
```

### JAVA-ICB-253 case 2: Single provider
settings: default
language: plain
```before
modules: MainModule (dependencies: Av1.jar with my.example.A)
--- main/Use.java
class Use { my.example.<caret>A a; }
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-254 case 1: Level 8 module depends on level 17 module
settings: enable inspection InconsistentLanguageLevel
language: plain
```before
modules: app (language level 8, depends on lib), lib (language level 17)
```
keys: inspect batch
```after
problem: warning module `app` "Module app with language level 8 depends on module lib with language level 17"
```

### JAVA-ICB-255 case 1: Module with one class
settings: enable inspection ModuleWithTooFewClasses
language: plain
```before
modules: app
--- app/A.java
class A {}
```
keys: inspect batch
```after
problem: warning module `app` "Module 'app' contains too few classes (1 < 10)"
```

### JAVA-ICB-256 case 1: Module above a limit of 2
settings: enable inspection ModuleWithTooManyClasses, Maximum number of classes=2
language: plain
```before
modules: app
--- app/A.java
class A {}
--- app/B.java
class B {}
--- app/C.java
class C {}
```
keys: inspect batch
```after
problem: warning module `app` "Module 'app' contains too many classes (3 > 2)"
```
