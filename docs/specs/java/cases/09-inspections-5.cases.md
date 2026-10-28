# Cases for 09-inspections-catalog-5.md

Format: see docs/specs/editor/cases/README.md. Inspection cases: `keys: Alt+Enter, choose "<fix>"` applies the named quick fix at the caret; `keys: inspect` runs the inspection and `after` then lists `problem: <level> <range text> "<message>"` lines or `no problem reported`.

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
<caret>import java.util.List;
import java.util.ArrayList;
public class Example {
    List list = new ArrayList();
}
```
keys: Alt+Enter, choose "Optimize imports"
```after
import java.util.ArrayList;
import java.util.List;
public class Example {
    List list = new ArrayList();
}
```

### JAVA-ICB-131 case 2: Sorted imports
settings: default
language: java
```before
<caret>import java.util.ArrayList;
import java.util.List;
public class Example {
    List list = new ArrayList();
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-132 case 1: Package import
settings: enable inspection OnDemandImport
language: java
status: unverified (text substituted for #ref on an import statement)
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
language: java
status: unverified (text substituted for #ref on an import statement)
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
language: java
status: unverified (whether the now redundant java.lang import is kept)
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
language: java
status: unverified (whether the fix keeps the qualified import and the precise import form)
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
public class Example {
    public static void main(String[] args) {
        System.out.println("Hello World!");
    }
}
```
keys: Alt+Enter, choose "Remove unused imports"
```after
public class Example {
    public static void main(String[] args) {
        System.out.println("Hello World!");
    }
}
```

### JAVA-ICB-137 case 2: Used import
settings: default
language: java
```before
<caret>import java.util.ArrayList;
public class Example {
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
abstract class <caret>Example {
    public String getName() {
        return "IntelliJ IDEA";
    }
}
```
keys: inspect
```after
problem: warning `Example` "Class Example is declared 'abstract', and has no 'abstract' methods"
```

### JAVA-ICB-141 case 1: Redundant abstract redeclaration
settings: enable inspection AbstractMethodOverridesAbstractMethod
language: java
```before
abstract class A {
    abstract void foo();
}
abstract class B extends A {
    abstract void <caret>foo();
}
```
keys: Alt+Enter, choose "Remove redundant abstract method declaration"
```after
abstract class A {
    abstract void foo();
}
abstract class B extends A {
}
```

### JAVA-ICB-142 case 1: Abstract overrides concrete
settings: enable inspection AbstractMethodOverridesConcreteMethod
language: java
```before
class A {
    void foo() {}
}
abstract class B extends A {
    abstract void <caret>foo();
}
```
keys: inspect
```after
problem: warning `foo` "Abstract method foo() overrides concrete method"
```

### JAVA-ICB-143 case 1: Subclass misses the implementation
settings: enable inspection AbstractMethodWithMissingImplementations
language: java
```before
abstract class Base {
    abstract void <caret>foo();
}
class Impl extends Base {}
```
keys: inspect
```after
problem: warning `foo` "Abstract method foo() is not implemented in every subclass"
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
language: java
status: unverified (member chooser may appear; the fix name for a class implementing an interface)
```before
interface A {}
abstract class B implements A {}
abstract class C1 extends B {}
abstract class C2 extends B {}
abstract class C3 extends B {}
abstract class C4 extends B {}
abstract class C5 extends B {}
class MyInheritor implements <caret>A {}
```
keys: Alt+Enter, choose "Make extends 'B'"
```after
interface A {}
abstract class B implements A {}
abstract class C1 extends B {}
abstract class C2 extends B {}
abstract class C3 extends B {}
abstract class C4 extends B {}
abstract class C5 extends B {}
class MyInheritor extends B {}
```

### JAVA-ICB-147 case 2: Fewer than five inheritors
settings: default
language: java
```before
interface A {}
abstract class B implements A {}
abstract class C1 extends B {}
class MyInheritor implements <caret>A {}
```
keys: Alt+Enter
```after
the intention list does not contain "Make extends 'B'"; no problem reported
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
public abstract class Foo {
    public <caret>Foo() {
        /* ... */
    }
}
```
keys: Alt+Enter, choose "Make 'protected'"
```after
public abstract class Foo {
    protected Foo() {
        /* ... */
    }
}
```

### JAVA-ICB-152 case 1: Shadowed Object
settings: enable inspection ParameterTypePreventsOverriding
language: java
```before
public class A {
    public void method(Object o) {}
}
public class B extends A {
    public void method(<caret>Object o) {}
    class Object {}
}
```
keys: Alt+Enter, choose "Change type of parameter to 'java.lang.Object'"
```after
public class A {
    public void method(Object o) {}
}
public class B extends A {
    public void method(java.lang.Object o) {}
    class Object {}
}
```

### JAVA-ICB-153 case 1: Array overriding varargs
settings: enable inspection ProblematicVarargsMethodOverride
language: java
```before
class A {
    void foo(String... args) {}
}
class B extends A {
    @Override
    void <caret>foo(String[] args) {}
}
```
keys: Alt+Enter, choose "Convert to varargs method"
```after
class A {
    void foo(String... args) {}
}
class B extends A {
    @Override
    void foo(String... args) {}
}
```

### JAVA-ICB-154 case 1: Redundant interface
settings: enable inspection RedundantImplements
language: java
```before
interface One {}
interface Two extends One {}
class X implements <caret>One, Two {}
```
keys: Alt+Enter, choose "Remove redundant interface declaration"
```after
interface One {}
interface Two extends One {}
class X implements Two {}
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
language: java
status: unverified (generated form of the inserted call for a method that throws)
```before
class A implements Cloneable {
    @Override
    public Object <caret>clone() {
        return new A();
    }
}
```
keys: Alt+Enter, choose "Insert call to super method"
```after
class A implements Cloneable {
    @Override
    public Object clone() {
        super.clone();
        return new A();
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
    void foo() {
        List<<caret>? extends Integer> list;
    }
}
```
keys: Alt+Enter, choose "Replace type parameter with actual class"
```after
import java.util.List;
class A {
    void foo() {
        List<Integer> list;
    }
}
```

### JAVA-ICB-159 case 1: Abstract call in constructor
settings: enable inspection AbstractMethodCallInConstructor
language: java
```before
abstract class Parent {
    abstract void abstractMethod();
    Parent() {
        <caret>abstractMethod();
    }
}
```
keys: inspect
```after
problem: warning `abstractMethod` "Call to 'abstract' method abstractMethod() during object construction"
```

### JAVA-ICB-160 case 1: Replace with regular initialization
settings: default
language: java
```before
import java.util.*;
class A {
    void m() {
        List<Integer> list = new ArrayList<>() <caret>{{
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
        List<Integer> list = new ArrayList<>();
        list.add(1);
        list.add(2);
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
        List<Integer> list = new ArrayList<>() <caret>{
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
class Foo {
    public int <caret>bar;
    static { }
}
```
keys: Alt+Enter, choose "Make initialization explicit"
```after
class Foo {
    public int bar = 0;
    static { }
}
```

### JAVA-ICB-162 case 1: Read in constructor
settings: enable inspection InstanceVariableUninitializedUse
language: java
```before
class Foo {
    int bar;
    Foo() {
        System.out.println(<caret>bar);
    }
}
```
keys: inspect
```after
problem: warning `bar` "Instance field bar used before initialized"
```

### JAVA-ICB-163 case 1: Static read in static init
settings: enable inspection NonFinalStaticVariableUsedInClassInitialization
language: java
```before
class Foo {
    public static int bar = 0;
    static {
        System.out.println(<caret>bar);
    }
}
```
keys: inspect
```after
problem: warning `bar` "Non-final static field bar used during class initialization"
```

### JAVA-ICB-164 case 1: Lazy static init
settings: enable inspection NonThreadSafeLazyInitialization
language: java
```before
import java.util.List;
class X {
    private static List<String> list;
    public List<String> getList() {
        if (list == null) {
            <caret>list = List.of("one", "two", "tree");
        }
        return list;
    }
}
```
keys: Alt+Enter, choose "Introduce holder class"
```after
import java.util.List;
class X {
    private static final class ListHolder {
        static final List<String> list = List.of("one", "two", "tree");
    }
    public List<String> getList() {
        return ListHolder.list;
    }
}
```

### JAVA-ICB-165 case 1: Overridable call in constructor
settings: enable inspection OverridableMethodCallDuringObjectConstruction
language: java
status: unverified (package-local methods are treated as safe; this case uses a method of the same package and relies on the inherited method being overridable by subclasses)
```before
class Child extends Parent {
    Child() {
        <caret>someMethod();
    }
}
class Parent {
    void someMethod() {}
}
```
keys: inspect
```after
problem: warning `someMethod` "Call to overridable method someMethod() during object construction"
```

### JAVA-ICB-166 case 1: Overridden method called
settings: enable inspection OverriddenMethodCallDuringObjectConstruction
language: java
```before
abstract class Parent {
    void someMethod() {}
}
class Child extends Parent {
    Child() {
        <caret>someMethod();
    }
    @Override
    void someMethod() {}
}
```
keys: inspect
```after
problem: warning `someMethod` "Call to overridden method someMethod() during object construction"
```

### JAVA-ICB-167 case 1: Uninitialized static field
settings: enable inspection StaticVariableInitialization
language: java
```before
class Foo {
    public static int <caret>bar;
    static { }
}
```
keys: Alt+Enter, choose "Make initialization explicit"
```after
class Foo {
    public static int bar = 0;
    static { }
}
```

### JAVA-ICB-168 case 1: Read of uninitialized static
settings: enable inspection StaticVariableUninitializedUse
language: java
```before
class Foo {
    public static int bar;
    public static void main(String[] args) {
        System.out.println(<caret>bar);
    }
}
```
keys: inspect
```after
problem: warning `bar` "Static field bar used before initialization"
```

### JAVA-ICB-169 case 1: Escape in initializer
settings: enable inspection ThisEscapedInConstructor
language: java
```before
class Foo {
    {
        System.out.println(<caret>this);
    }
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
        JPanel panel = new JPanel(new BorderLayout(2, 2));
        JLabel label = new JLabel("Hello World");
        panel.add(label, BorderLayout.<caret>NORTH);
    }
}
```
keys: Alt+Enter, choose "Replace with 'BorderLayout.PAGE_START'"
```after
import javax.swing.*;
import java.awt.*;
class A {
    void m() {
        JPanel panel = new JPanel(new BorderLayout(2, 2));
        JLabel label = new JLabel("Hello World");
        panel.add(label, BorderLayout.PAGE_START);
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
    char c = '©';
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
the intention list does not contain "Convert to Basic Latin"; no problem reported
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
    void setTitle(@Nls(capitalization = Nls.Capitalization.Title) String title) {}
    void m() {
        setTitle(<caret>"This is sentence capitalization but should be title");
    }
}
```
keys: Alt+Enter, choose "Properly capitalize 'This is sentence capitalization but should be title'"
```after
import org.jetbrains.annotations.Nls;
class A {
    void setTitle(@Nls(capitalization = Nls.Capitalization.Title) String title) {}
    void m() {
        setTitle("This Is Sentence Capitalization but Should Be Title");
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
language: java
status: unverified (tooltip layout of the file list)
```before
--- C1.java
class C1 { String CONST1 = <caret>"duplicate string"; }
--- C2.java
class C2 { String CONST2 = "duplicate string"; }
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
    void foo(byte[] bytes) {
        String s = new <caret>String(bytes);
    }
}
```
keys: Alt+Enter, choose "Add 'StandardCharsets.UTF_8' argument"
```after
import java.nio.charset.StandardCharsets;
class A {
    void foo(byte[] bytes) {
        String s = new String(bytes, StandardCharsets.UTF_8);
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
    void print(Double d) {
        System.out.println(d.<caret>toString());
    }
}
```
keys: inspect
```after
problem: warning `toString` "Number.toString() called in an internationalized context"
```

### JAVA-ICB-181 case 1: SimpleDateFormat
settings: enable inspection SimpleDateFormatWithoutLocale
language: java
status: unverified (text of #ref for a new expression)
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
    String getMessage(String string, int number) {
        return string <caret>+ number;
    }
}
```
keys: inspect
```after
problem: warning `+` "String concatenation string + number in an internationalized context"
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
language: java
status: unverified (the offered argument choices)
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
    String s = "<caret>x";
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
public @interface <caret>UnitTest {}
```
keys: inspect
```after
problem: warning `UnitTest` "UnitTest should have @Retention(RetentionPolicy.RUNTIME)"
```

### JAVA-ICB-187 case 2: Runtime retention present
settings: default
language: java
```before
import java.lang.annotation.*;
import org.junit.platform.commons.annotation.Testable;
@Retention(RetentionPolicy.RUNTIME)
@Testable
public @interface <caret>UnitTest {}
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
    void m() {
        <caret>for (int x : Arrays.asList(1, 2, 3)) {
            System.out.println(x);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with old-style 'for' statement"
```after
import java.util.*;
class A {
    void m() {
        for (Iterator<Integer> iterator = Arrays.asList(1, 2, 3).iterator(); iterator.hasNext(); ) {
            final int x = iterator.next();
            System.out.println(x);
        }
    }
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
public class <caret>Example {}
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
    void <caret>foo(String... ss) {}
    void bar() {
        foo("a", "b");
    }
}
```
keys: Alt+Enter, choose "Convert varargs parameter to array"
```after
class A {
    void foo(String[] ss) {}
    void bar() {
        foo(new String[]{"a", "b"});
    }
}
```

### JAVA-ICB-194 case 2: Array parameter
settings: default
language: java
```before
class A {
    void <caret>foo(String[] ss) {}
}
```
keys: Alt+Enter
```after
the intention list does not contain "Convert varargs parameter to array"; no problem reported
```

### JAVA-ICB-195 case 1: Hashtable keys
settings: enable inspection EnumerationCanBeIteration
language: java
```before
import java.util.*;
class A {
    void m(Hashtable<String, String> map) {
        Enumeration<String> keys = map.<caret>keys();
        while (keys.hasMoreElements()) {
            String name = keys.nextElement();
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'Iterator' construct"
```after
import java.util.*;
class A {
    void m(Hashtable<String, String> map) {
        Iterator<String> iterator = map.keySet().iterator();
        while (iterator.hasNext()) {
            String name = iterator.next();
        }
    }
}
```

### JAVA-ICB-196 case 1: Replace with record pattern
settings: default
language: java
status: unverified (Java 21 for-each record patterns are removed in later language levels; language level of the test project must allow them)
```before
import java.util.List;
record Rec(Integer x, String y) {}
class A {
    static void test(List<Rec> records) {
        for (Rec <caret>record : records) {
            System.out.println(record.y());
            Integer x = record.x;
            System.out.println(x);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with a record pattern"
```after
import java.util.List;
record Rec(Integer x, String y) {}
class A {
    static void test(List<Rec> records) {
        for (Rec(Integer x, String y) : records) {
            System.out.println(y);
            System.out.println(x);
        }
    }
}
```

### JAVA-ICB-196 case 2: Component unused
settings: default
language: java
```before
import java.util.List;
record Rec(Integer x, String y) {}
class A {
    static void test(List<Rec> records) {
        for (Rec <caret>record : records) {
            System.out.println(record.y());
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
    void test(String str) {
        <caret>if (str.equals("1")) {
            System.out.println(1);
        } else if (str.equals("2")) {
            System.out.println(2);
        } else if (str.equals("3")) {
            System.out.println(3);
        } else {
            System.out.println(4);
        }
    }
}
```
keys: Alt+Enter, choose "Replace 'if' with 'switch'"
```after
class A {
    void test(String str) {
        switch (str) {
            case "1" -> System.out.println(1);
            case "2" -> System.out.println(2);
            case "3" -> System.out.println(3);
            default -> System.out.println(4);
        }
    }
}
```

### JAVA-ICB-197 case 2: Two branches only
settings: default
language: java
```before
class A {
    void test(String str) {
        <caret>if (str.equals("1")) {
            System.out.println(1);
        } else if (str.equals("2")) {
            System.out.println(2);
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-198 case 1: Loop to repeat
settings: default
language: java
status: unverified (language level 11; fix name for the StringBuilder.append form)
```before
class A {
    void append(StringBuilder sb, int count, Object obj) {
        <caret>for (int i = 0; i < count; i++) {
            sb.append(obj);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'String.repeat()'"
```after
class A {
    void append(StringBuilder sb, int count, Object obj) {
        sb.append(String.valueOf(obj).repeat(Math.max(0, count)));
    }
}
```

### JAVA-ICB-198 case 2: Counter used in the body
settings: default
language: java
```before
class A {
    void append(StringBuilder sb, int count) {
        <caret>for (int i = 0; i < count; i++) {
            sb.append(i);
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
language: java
status: unverified (caret location on the reported element)
```before
class A {
    public int compare(int x, int y) {
        return <caret>x > y ? 1 : x < y ? -1 : 0;
    }
}
```
keys: Alt+Enter, choose "Replace with 'Integer.compare()'"
```after
class A {
    public int compare(int x, int y) {
        return Integer.compare(x, y);
    }
}
```

### JAVA-ICB-199 case 2: Not a three-way comparison
settings: default
language: java
```before
class A {
    public int compare(int x, int y) {
        return <caret>x > y ? 1 : 0;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-200 case 1: Generate constructor
settings: enable inspection ClassWithoutConstructor
language: java
status: unverified (visibility modifier of the generated constructor follows the class)
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
public class <caret>Bean {
    private String name;
    public Bean(String name) {
        this.name = name;
    }
}
```
keys: inspect
```after
problem: warning `Bean` "Class Bean is missing a no-arg constructor"
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
    int getPayerId() { return id; }
    void setPayerId(int i) { id = i; }
    void m(Bean bean) {
        bean.<caret>setPayerId(bean.getPayerId());
    }
}
```
keys: inspect
```after
problem: warning `setPayerId` "Property value set to itself"
```

### JAVA-ICB-204 case 1: Getter returns another field
settings: enable inspection SuspiciousGetterSetter
language: java
```before
class Point {
    private int x;
    private int y;
    public int <caret>getY() {
        return x;
    }
}
```
keys: inspect
```after
problem: warning `getY` "Getter getY() returns field 'x'"
```

### JAVA-ICB-205 case 1: Replace with block comment
settings: default
language: java
```before
class A {
    <caret>/**
     * Dangling comment
     */
    /**
     * Method javadoc
     */
    public void m() {}
}
```
keys: Alt+Enter, choose "Replace with block comment"
```after
class A {
    /*
      Dangling comment
    */
    /**
     * Method javadoc
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
     * Method javadoc
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
language: java
status: unverified (exact text of the fix name)
```before
import java.util.*;
class A {
    /**
     * @return empty <caret><code>ArrayList&lt;Integer&gt;</code>
     */
    List<Integer> getList() { return null; }
}
```
keys: Alt+Enter, choose "Replace with '{@code ...}'"
```after
import java.util.*;
class A {
    /**
     * @return empty {@code ArrayList<Integer>}
     */
    List<Integer> getList() { return null; }
}
```

### JAVA-ICB-207 case 1: Blank line
settings: default
language: java
```before
class Main {
    /**
     * Doesn't do anything.
    <caret>
     * Does absolutely nothing
     */
    void foo() {}
}
```
keys: Alt+Enter, choose "Insert <p>"
```after
class Main {
    /**
     * Doesn't do anything.
     * <p>
     * Does absolutely nothing
     */
    void foo() {}
}
```

### JAVA-ICB-207 case 2: Paragraph tag already present
settings: default
language: java
```before
class Main {
    /**
     * Doesn't do anything.
     * <p>
     * Does absolutely nothing
     */
    <caret>void foo() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-208 case 1: Unknown tag
settings: default
language: java
status: unverified (exact message text for an unknown tag)
```before
class A {
    /**
     * Invalid tag name
     * @<caret>poram param description
     */
    public void sample(int param) {}
}
```
keys: inspect
```after
problem: warning `poram` "Wrong tag @poram"
```

### JAVA-ICB-208 case 2: Known tag
settings: default
language: java
```before
class A {
    /**
     * Valid tag name
     * @<caret>param param description
     */
    public void sample(int param) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-209 case 1: Self-closed tag
settings: enable inspection JavadocHtmlLint
language: java
status: unverified (message comes from the JDK's DocLint, its text depends on the JDK version)
```before
class A {
    /**
     * Self-closed tag: <caret><br/>
     */
    void sample() {}
}
```
keys: inspect
```after
problem: error `<br/>` "self-closing element not allowed"
```

### JAVA-ICB-210 case 1: Plain URL
settings: default
language: java
```before
class Main {
    /**
     * <caret>https://en.wikipedia.org/
     */
    void foo() {}
}
```
keys: Alt+Enter, choose "Replace URL with link"
```after
class Main {
    /**
     * <a href="https://en.wikipedia.org/">https://en.wikipedia.org/</a>
     */
    void foo() {}
}
```

### JAVA-ICB-210 case 2: URL inside a tag
settings: default
language: java
```before
class Main {
    /**
     * <a href="https://en.wikipedia.org/"><caret>https://en.wikipedia.org/</a>
     */
    void foo() {}
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
     * @param <caret>someParam description
     **/
    void foo() {
    }
}
```
keys: inspect
```after
problem: error `someParam` "Cannot resolve symbol someParam"
```

### JAVA-ICB-211 case 2: Existing parameter
settings: default
language: java
```before
class A {
    /**
     * @param <caret>p description
     **/
    void foo(int p) {
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-212 case 1: Boolean wording, non-boolean return
settings: default
language: java
status: unverified (exact highlighted words)
```before
class User {}
interface Repo {
    /**
     * @return <caret>true if user is found, false otherwise
     */
    User findUser(String name);
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
     * @return <caret>true if user is found, false otherwise
     */
    boolean hasUser(String name);
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
     * @deprecated use {@code example()} instead
     */
    void <caret>sample() {}
}
```
keys: Alt+Enter, choose "Add '@Deprecated' annotation"
```after
class A {
    /**
     * @deprecated use {@code example()} instead
     */
    @Deprecated
    void sample() {}
}
```

### JAVA-ICB-213 case 2: Already annotated
settings: default
language: java
```before
class A {
    /**
     * @deprecated use {@code example()} instead
     */
    @Deprecated
    void <caret>sample() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-214 case 1: Add Javadoc
settings: enable inspection MissingJavadoc
language: java
status: unverified (generated skeleton content depends on required tags)
```before
public class A {
    public void <caret>sample(int param) {
    }
}
```
keys: Alt+Enter, choose "Add Javadoc"
```after
public class A {
    /**
     *
     */
    public void sample(int param) {
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
keys: inspect (batch, whole project)
```after
problem: warning `com.sample` "Package 'com.sample' is missing a package-info.java file"
```

### JAVA-ICB-216 case 1: Convert package.html
settings: enable inspection PackageDotHtmlMayBePackageInfo
language: plain
status: unverified (inspection result view vs editor)
```before
--- com/sample/package.html
<html>
  <body>
    Documentation example.
  </body>
</html>
```
keys: Alt+Enter on the file, choose "Convert to 'package-info.java'"
```after
--- com/sample/package-info.java
/**
 * Documentation example.
 */
package com.sample;
```

### JAVA-ICB-217 case 1: Missing package statement
settings: default
language: java
status: unverified (position of the inserted statement)
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
public class Main {
    /*
    * Hello,
    */
    // World!<caret>
    void f() {
    }
}
```
keys: Alt+Enter, choose "Replace with Javadoc comment"
```after
public class Main {
    /**
     * Hello,
     * World!
     */
    void f() {
    }
}
```

### JAVA-ICB-218 case 2: Comment inside a method body
settings: default
language: java
```before
public class Main {
    void f() {
        // not attached to a declaration<caret>
        int x = 0;
    }
}
```
keys: Alt+Enter
```after
the intention list does not contain "Replace with Javadoc comment"; no problem reported
```

### JAVA-ICB-219 case 1: Only inheritDoc
settings: enable inspection UnnecessaryInheritDoc
language: java
```before
class Example implements Comparable<Example> {
    /**
     * <caret>{@inheritDoc}
     */
    @Override
    public int compareTo(Example o) {
        return 0;
    }
}
```
keys: Alt+Enter, choose "Remove unnecessary {@inheritDoc}"
```after
class Example implements Comparable<Example> {
    @Override
    public int compareTo(Example o) {
        return 0;
    }
}
```

### JAVA-ICB-220 case 1: Self link
settings: enable inspection UnnecessaryJavaDocLink
language: java
```before
class Example {
    /**
     * <caret>@see Example#method
     */
    public void method() { }
}
```
keys: Alt+Enter, choose "Remove unnecessary '@see'"
```after
class Example {
    public void method() { }
}
```

### JAVA-ICB-221 case 1: Two loggers
settings: enable inspection ClassWithMultipleLoggers
language: java
```before
import java.util.logging.Logger;
public class <caret>Critical {
    protected static final Logger LOG = Logger.getLogger("a");
    protected static final Logger myLogger = Logger.getLogger("b");
}
```
keys: inspect
```after
problem: warning `Critical` "Class Critical declares multiple loggers"
```

### JAVA-ICB-222 case 1: No logger
settings: enable inspection ClassWithoutLogger
language: java
```before
public class <caret>NoLoggerDeclared {
    int calculateNthDigitOfPi(int n) {
        return 1;
    }
}
```
keys: inspect
```after
problem: warning `NoLoggerDeclared` "Class NoLoggerDeclared does not declare a logger"
```

### JAVA-ICB-223 case 1: Replace foreign class
settings: default
language: java
status: unverified (requires slf4j on the classpath)
```before
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Paramount {
    protected static final Logger LOG = LoggerFactory.getLogger(<caret>Critical.class);
}
class Critical {}
```
keys: Alt+Enter, choose "Replace with 'Paramount.class'"
```after
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Paramount {
    protected static final Logger LOG = LoggerFactory.getLogger(Paramount.class);
}
class Critical {}
```

### JAVA-ICB-223 case 2: Own class literal
settings: default
language: java
```before
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Paramount {
    protected static final Logger LOG = LoggerFactory.getLogger(<caret>Paramount.class);
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
public class Significant {
    private Logger <caret>LOG = Logger.getLogger("x");
}
```
keys: Alt+Enter, choose "Make 'LOG' static final"
```after
import java.util.logging.Logger;
public class Significant {
    private static final Logger LOG = Logger.getLogger("x");
}
```

### JAVA-ICB-225 case 1: Method without logging
settings: enable inspection PublicMethodWithoutLogging
language: java
```before
import java.util.logging.Logger;
public class Crucial {
    private static final Logger LOG = Logger.getLogger("x");
    public void <caret>doImportantStuff() {
    }
    public void doOtherStuff() {
        LOG.info("do other stuff");
    }
}
```
keys: inspect
```after
problem: warning `doImportantStuff` "'public' method doImportantStuff() has no logging call"
```

### JAVA-ICB-226 case 1: Concatenation to placeholders
settings: default
language: java
status: unverified (requires slf4j on the classpath)
```before
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Vital {
    private static final Logger LOG = LoggerFactory.getLogger(Vital.class);
    public void saveTheWorld(int i, String s, boolean b) {
        LOG.<caret>info("saveTheWorld(" + i + ", " + s + ", " + b + ")");
    }
}
```
keys: Alt+Enter, choose "Replace concatenation with parameterized log message"
```after
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Vital {
    private static final Logger LOG = LoggerFactory.getLogger(Vital.class);
    public void saveTheWorld(int i, String s, boolean b) {
        LOG.info("saveTheWorld({}, {}, {})", i, s, b);
    }
}
```

### JAVA-ICB-226 case 2: Parameterized message
settings: default
language: java
```before
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public class Vital {
    private static final Logger LOG = LoggerFactory.getLogger(Vital.class);
    public void saveTheWorld(int i) {
        LOG.<caret>info("saveTheWorld({})", i);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-227 case 1: Deprecated annotation
settings: default
language: java
status: unverified (highlighted element and exact quoting)
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
language: java
status: unverified (exact message text of the processor warning; needs lombok on the classpath)
```before
import lombok.ToString;
<caret>@ToString
class Person {
    final String name = "";
    @Override
    public String toString() { return name; }
}
```
keys: Alt+Enter, choose "Remove annotation"
```after
class Person {
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
language: java
status: unverified (text of {0})
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
language: java
status: unverified (resulting import form)
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
language: java
status: unverified (name chosen for the new class: a dialog may ask)
```before
class A {
    void sample() {
        Thread thread = new Thread(new <caret>Runnable() {
            @Override
            public void run() {
            }
        });
    }
}
```
keys: Alt+Enter, choose "Convert to named 'static' inner class"
```after
class A {
    void sample() {
        Thread thread = new Thread(new Task());
    }

    private static class Task implements Runnable {
        @Override
        public void run() {
        }
    }
}
```

### JAVA-ICB-233 case 1: Make static
settings: default
language: java
```before
public class Outer {
    class <caret>Inner {
        public void foo() {
            bar("x");
        }
        private void bar(String string) {}
    }
}
```
keys: Alt+Enter, choose "Make 'static'"
```after
public class Outer {
    static class Inner {
        public void foo() {
            bar("x");
        }
        private void bar(String string) {}
    }
}
```

### JAVA-ICB-233 case 2: Inner class uses the outer instance
settings: default
language: java
```before
public class Outer {
    int field;
    class <caret>Inner {
        public int foo() {
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
public class Example {
    static List<String> <caret>list = new ArrayList<>();
}
```
keys: inspect
```after
problem: warning `list` "Static collection list"
```

### JAVA-ICB-236 case 1: StringBuilder field
settings: enable inspection StringBufferField
language: java
```before
public class Example {
    private StringBuilder <caret>builder = new StringBuilder();
}
```
keys: inspect
```after
problem: warning `builder` "'StringBuilder' field builder"
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
class Item {
    public static final Item[] EMPTY_ARRAY = new Item[0];
}
class EmptyNode {
    Item[] getChildren() {
        return <caret>new Item[0];
    }
}
```
keys: Alt+Enter, choose "Replace with 'Item.EMPTY_ARRAY'"
```after
class Item {
    public static final Item[] EMPTY_ARRAY = new Item[0];
}
class EmptyNode {
    Item[] getChildren() {
        return Item.EMPTY_ARRAY;
    }
}
```

### JAVA-ICB-238 case 2: No constant available
settings: default
language: java
```before
class Item {}
class EmptyNode {
    Item[] getChildren() {
        return <caret>new Item[0];
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
    void <caret>methodWithTwoLoops(int n1, int n2) {
        for (int i = 0; i < n1; i++) {
            System.out.println(i);
        }
        int j = 0;
        while (j < n2) {
            System.out.println(j);
            j++;
        }
    }
}
```
keys: inspect
```after
problem: warning `methodWithTwoLoops` "methodWithTwoLoops contains 2 loops"
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
        System.out.println(1);
        System.out.println(2);
        System.out.println(3);
    }
}
```
keys: inspect
```after
problem: warning `m` "m is too long (# Non-comment source statements = 3)"
```

### JAVA-ICB-246 case 1: Four statements
settings: enable inspection OverlyLongLambda
language: java
status: unverified (highlighted element of the lambda)
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
    void <caret>doSmth(int a, int b, boolean flag1, boolean flag2) {
        if (!flag1 && !flag2) {
            if (a != b) {
                System.out.println();
            }
        }
    }
}
```
keys: inspect
```after
problem: warning `doSmth` "doSmth contains 3 negations"
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
keys: inspect (batch, whole project)
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
keys: inspect (batch, whole project)
```after
problem: warning `Helper` "Class Helper has only dependencies on and/or dependents in module 'lib'"
```

### JAVA-ICB-253 case 1: Same class in two jars
settings: default
language: plain
status: unverified (needs the module dependency model; the inspection exists only for the IDE project model)
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
keys: inspect (batch, whole project)
```after
problem: warning module app "Module app with language level 8 depends on module lib with language level 17"
```

### JAVA-ICB-255 case 1: Module with one class
settings: enable inspection ModuleWithTooFewClasses
language: plain
```before
modules: app
--- app/A.java
class A {}
```
keys: inspect (batch, whole project)
```after
problem: warning module app "Module 'app' contains too few classes (1 < 10)"
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
keys: inspect (batch, whole project)
```after
problem: warning module app "Module 'app' contains too many classes (3 > 2)"
```

