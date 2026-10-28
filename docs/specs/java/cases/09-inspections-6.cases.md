# Cases for 09-inspections-catalog-6.md

Format: see docs/specs/editor/cases/README.md. Inspection cases: `keys: Alt+Enter, choose "<fix>"` applies the named quick fix at the caret; `keys: inspect` runs the inspection and `after` then lists `problem: <level> <range text> "<message>"` lines or `no problem reported`.

### JAVA-ICB-257 case 1: Class with dollar sign
settings: enable inspection DollarSignInName
language: java
```before
class <caret>SalaryIn$ {}
```
keys: inspect
```after
problem: warning `SalaryIn$` "Identifier SalaryIn$ contains '$'"
```

### JAVA-ICB-258 case 1: Constant too short
settings: enable inspection FieldNamingConvention (kind Constant)
language: java
```before
public class A {
    public static final int <caret>MAX = 42;
}
```
keys: inspect
```after
problem: warning `MAX` "Constant name MAX is too short (3 < 5)"
```

### JAVA-ICB-259 case 1: Terminal digits
settings: default
language: java
status: unverified (exact wording of the rename action)
```before
--- module-info.java
module foo.<caret>bar2 {}
```
keys: Alt+Enter, choose "Rename module 'foo.bar2' to 'foo.bar'"
```after
--- module-info.java
module foo.bar {}
```

### JAVA-ICB-259 case 2: No digits
settings: default
language: java
```before
--- module-info.java
module foo.<caret>bar {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-260 case 1: Upper-case lambda parameter
settings: enable inspection LambdaParameterNamingConvention
language: java
```before
import java.util.function.Function;
class A {
    Function<String, String> id = <caret>X -> X;
}
```
keys: inspect
```after
problem: warning `X` "Lambda parameter name X doesn't match regex '[a-z][A-Za-z\d]*'"
```

### JAVA-ICB-261 case 1: Upper-case variable
settings: enable inspection LocalVariableNamingConvention
language: java
```before
class A {
    void m() {
        int <caret>X = 42;
    }
}
```
keys: inspect
```after
problem: warning `X` "Local variable name X doesn't match regex '[a-z][A-Za-z\d]*'"
```

### JAVA-ICB-262 case 1: Short package name
settings: enable inspection PackageNamingConvention
language: java
```before
package <caret>io;
class A {}
```
keys: inspect
```after
problem: warning `io` "Package name io is too short"
```

### JAVA-ICB-263 case 1: Super constructor parameter
settings: enable inspection ParameterNameDiffersFromOverriddenParameter
language: java
```before
class Person {
    Person(String fullName) {}
}
class Child extends Person {
    Child(String <caret>name) { super(name); }
}
```
keys: Alt+Enter, choose "Rename to 'fullName'"
```after
class Person {
    Person(String fullName) {}
}
class Child extends Person {
    Child(String fullName) { super(fullName); }
}
```

### JAVA-ICB-264 case 1: Upper-case parameter
settings: enable inspection ParameterNamingConvention
language: java
```before
class A {
    void fooBar(int <caret>X) {}
}
```
keys: inspect
```after
problem: warning `X` "Parameter name X doesn't match regex '[a-z][A-Za-z\d]*'"
```

### JAVA-ICB-265 case 1: Questionable variable
settings: enable inspection QuestionableName
language: java
status: unverified (exact default name list; `aa` is the documented example)
```before
class A {
    void m() {
        int <caret>aa = 42;
    }
}
```
keys: inspect
```after
problem: warning `aa` "Questionable name aa"
```

### JAVA-ICB-266 case 1: i that is a String
settings: enable inspection StandardVariableNames
language: java
```before
class A {
    void m() {
        String <caret>i = "x";
    }
}
```
keys: inspect
```after
problem: warning `i` "Variable named i doesn't have type 'int'"
```

### JAVA-ICB-267 case 1: Static not final
settings: enable inspection UpperCaseFieldNameNotConstant
language: java
```before
public class A {
    public static int <caret>THE_ANSWER = 42;
}
```
keys: inspect
```after
problem: warning `THE_ANSWER` "Non-constant field THE_ANSWER with constant-style name"
```

### JAVA-ICB-268 case 1: Odd check
settings: enable inspection BadOddness
language: java
```before
class A {
    boolean m(int x) {
        return <caret>x % 2 == 1;
    }
}
```
keys: inspect
```after
problem: warning `x % 2 == 1` "Oddness check will fail on negative values"
```

### JAVA-ICB-269 case 1: equals on BigDecimal
settings: enable inspection BigDecimalEquals
language: java
```before
import java.math.BigDecimal;
class A {
    boolean m() {
        return new BigDecimal("2.0").<caret>equals(new BigDecimal("2.00"));
    }
}
```
keys: Alt+Enter, choose "Replace with 'compareTo()==0'"
```after
import java.math.BigDecimal;
class A {
    boolean m() {
        return new BigDecimal("2.0").compareTo(new BigDecimal("2.00")) == 0;
    }
}
```

### JAVA-ICB-270 case 1: divide without rounding
settings: default
language: java
```before
import java.math.BigDecimal;
class A {
    BigDecimal m() {
        return BigDecimal.valueOf(1).<caret>divide(BigDecimal.valueOf(3));
    }
}
```
keys: inspect
```after
problem: warning `divide` "'BigDecimal.divide()' called without a rounding mode argument"
```

### JAVA-ICB-270 case 2: divide with rounding
settings: default
language: java
```before
import java.math.*;
class A {
    BigDecimal m() {
        return BigDecimal.valueOf(1).<caret>divide(BigDecimal.valueOf(3), RoundingMode.HALF_UP);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-271 case 1: Integer constructor
settings: language level 17
language: java
```before
class A {
    Integer i = <caret>new Integer(1);
}
```
keys: Alt+Enter, choose "Replace with 'Integer.valueOf()'"
```after
class A {
    Integer i = Integer.valueOf(1);
}
```

### JAVA-ICB-271 case 2: Language level 8 with default option
settings: language level 8
language: java
```before
class A {
    Integer i = <caret>new Integer(1);
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-272 case 1: Cast to int
settings: enable inspection CharUsedInArithmeticContext
language: java
```before
class A {
    int a = <caret>'a' + 42;
}
```
keys: Alt+Enter, choose "Insert cast to int"
```after
class A {
    int a = (int) 'a' + 42;
}
```

### JAVA-ICB-273 case 1: short vs char
settings: default
language: java
```before
class A {
    short shortValue() { return 1; }
    boolean m() {
        return <caret>Character.MAX_VALUE == shortValue();
    }
}
```
keys: inspect
```after
problem: warning `Character.MAX_VALUE == shortValue()` "Equality comparison Character.MAX_VALUE == shortValue() of short and char values"
```

### JAVA-ICB-273 case 2: char vs char
settings: default
language: java
```before
class A {
    boolean m(char c) {
        return <caret>Character.MAX_VALUE == c;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-274 case 1: Replace comparison
settings: default
language: java
```before
class A {
    boolean m(double x) {
        return x == Double.<caret>NaN;
    }
}
```
keys: Alt+Enter, choose "Replace with 'isNaN()'"
```after
class A {
    boolean m(double x) {
        return Double.isNaN(x);
    }
}
```

### JAVA-ICB-274 case 2: Comparison with a number
settings: default
language: java
```before
class A {
    boolean m(double x) {
        return x == <caret>1.0;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-275 case 1: Leading dot
settings: enable inspection ConfusingFloatingPointLiteral
language: java
```before
class A {
    double d = <caret>.03;
}
```
keys: Alt+Enter, choose "Change to canonical form"
```after
class A {
    double d = 0.03;
}
```

### JAVA-ICB-276 case 1: Constant sin
settings: enable inspection ConstantMathCall
language: java
```before
class A {
    double v = Math.<caret>sin(0.0);
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    double v = 0.0;
}
```

### JAVA-ICB-277 case 1: Integer division by zero
settings: default
language: java
```before
class A {
    int m(int x) {
        return x / <caret>0;
    }
}
```
keys: inspect
```after
problem: warning `0` "Division by zero"
```

### JAVA-ICB-277 case 2: Division by a constant
settings: default
language: java
```before
class A {
    int m(int x) {
        return x / <caret>2;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-278 case 1: d1 == d2
settings: enable inspection FloatingPointEquality
language: java
```before
class A {
    boolean m(double d1, double d2) {
        return <caret>d1 == d2;
    }
}
```
keys: inspect
```after
problem: warning `d1 == d2` "d1 == d2: floating-point values compared for exact equality"
```

### JAVA-ICB-279 case 1: Make conversion explicit
settings: enable inspection ImplicitNumericConversion
language: java
```before
class A {
    double m(int i) {
        return <caret>i * 10;
    }
}
```
keys: Alt+Enter, choose "Convert to 'double'"
```after
class A {
    double m(int i) {
        return (double) (i * 10);
    }
}
```

### JAVA-ICB-280 case 1: Insert underscores
settings: default
language: java
```before
class A {
    int million = <caret>1000000;
}
```
keys: Alt+Enter, choose "Replace '1000000' with '1_000_000'"
```after
class A {
    int million = 1_000_000;
}
```

### JAVA-ICB-280 case 2: Short literal
settings: default
language: java
```before
class A {
    int n = <caret>1000;
}
```
keys: Alt+Enter
```after
the intention list does not contain "Insert underscores into numeric literal"; no problem reported
```

### JAVA-ICB-281 case 1: Cast numerator
settings: default
language: java
```before
class A {
    float x = 3.0F + <caret>3 * 2 / 5;
}
```
keys: Alt+Enter, choose "Cast the numerator to 'float'"
```after
class A {
    float x = 3.0F + ((float) (3 * 2)) / 5;
}
```

### JAVA-ICB-281 case 2: Floating division
settings: default
language: java
```before
class A {
    float x = 3.0F + <caret>3.0F * 2 / 5;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-282 case 1: Lowercase l
settings: default
language: java
```before
class A {
    long nights = <caret>100l;
}
```
keys: Alt+Enter, choose "Replace '100l' with '100L'"
```after
class A {
    long nights = 100L;
}
```

### JAVA-ICB-282 case 2: Uppercase L
settings: default
language: java
```before
class A {
    long nights = <caret>100L;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-283 case 1: Cast after expanding
settings: default
language: java
status: unverified (the shape of the produced expression)
```before
class A {
    void m() {
        long c = 1;
        c += <caret>1.2;
    }
}
```
keys: Alt+Enter, choose "Cast to 'long' after expanding compound assignment"
```after
class A {
    void m() {
        long c = 1;
        c += (long) 1.2;
    }
}
```

### JAVA-ICB-283 case 2: Compatible types
settings: default
language: java
```before
class A {
    void m() {
        long c = 1;
        c += <caret>2;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-284 case 1: Add L suffix
settings: default
language: java
```before
class A {
    long mask = <caret>0xFFFF_FFFF;
}
```
keys: Alt+Enter, choose "Add 'L' suffix (changes semantics)"
```after
class A {
    long mask = 0xFFFF_FFFFL;
}
```

### JAVA-ICB-284 case 2: Positive constant
settings: default
language: java
```before
class A {
    long mask = <caret>0x7FFF_FFFF;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-285 case 1: Math.sin
settings: enable inspection NonReproducibleMathCall
language: java
```before
class A {
    double m(double x) {
        return Math.<caret>sin(x);
    }
}
```
keys: Alt+Enter, choose "Replace with 'StrictMath' call"
```after
class A {
    double m(double x) {
        return StrictMath.sin(x);
    }
}
```

### JAVA-ICB-286 case 1: int overflow
settings: default
language: java
```before
class A {
    long b = <caret>30 * 24 * 60 * 60 * 1000;
}
```
keys: inspect
```after
problem: warning `30 * 24 * 60 * 60 * 1000` "Numeric overflow in expression"
```

### JAVA-ICB-286 case 2: No overflow with long
settings: default
language: java
```before
class A {
    long b = <caret>30L * 24 * 60 * 60 * 1000;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-287 case 1: Remove leading zeroes
settings: enable inspection OctalAndDecimalIntegersMixed
language: java
```before
class A {
    int[] elapsed = <caret>{1, 13, 052};
}
```
keys: Alt+Enter, choose "Remove leading zeroes to make decimals"
```after
class A {
    int[] elapsed = {1, 13, 52};
}
```

### JAVA-ICB-288 case 1: Convert to decimal
settings: default
language: java
```before
class A {
    int i = <caret>015;
}
```
keys: Alt+Enter, choose "Convert octal literal to decimal literal"
```after
class A {
    int i = 13;
}
```

### JAVA-ICB-288 case 2: Zero
settings: default
language: java
```before
class A {
    int i = <caret>0;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-289 case 1: Seven terms
settings: enable inspection OverlyComplexArithmeticExpression
language: java
```before
class A {
    int calc(int a, int b) {
        return <caret>a + a + a + b + b + b + b;
    }
}
```
keys: inspect
```after
problem: warning `a + a + a + b + b + b + b` "Overly complex arithmetic expression"
```

### JAVA-ICB-290 case 1: Add zero
settings: default
language: java
```before
class A {
    int f(int a) {
        int y = <caret>a + 0;
        return y;
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    int f(int a) {
        int y = a;
        return y;
    }
}
```

### JAVA-ICB-290 case 2: Multiply by two
settings: default
language: java
```before
class A {
    int f(int a) {
        int y = <caret>a * 2;
        return y;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-291 case 1: Remove underscores
settings: default
language: java
```before
class A {
    int million = <caret>1_000_000;
}
```
keys: Alt+Enter, choose "Replace '1_000_000' with '1000000'"
```after
class A {
    int million = 1000000;
}
```

### JAVA-ICB-291 case 2: Literal without underscores
settings: default
language: java
```before
class A {
    int n = <caret>1000000;
}
```
keys: Alt+Enter
```after
the intention list does not contain "Remove underscores from numeric literal"; no problem reported
```

### JAVA-ICB-292 case 1: 1_000_0000
settings: enable inspection SuspiciousLiteralUnderscore
language: java
```before
class A {
    int oneMillion = <caret>1_000_0000;
}
```
keys: inspect
```after
problem: warning `1_000_0000` "Group in number literal with underscores does not have length 3"
```

### JAVA-ICB-293 case 1: Double plus
settings: default
language: java
```before
class A {
    int unaryPlus(int i) {
        int x = <caret>+ +i;
        return x;
    }
}
```
keys: Alt+Enter, choose "Remove unary '+'"
```after
class A {
    int unaryPlus(int i) {
        int x = i;
        return x;
    }
}
```

### JAVA-ICB-293 case 2: Plain unary plus in initializer
settings: default
language: java
```before
class A {
    int m() {
        int x = <caret>+1;
        return x;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-294 case 1: Fold into parent operator
settings: default
language: java
```before
class A {
    void unaryMinus(int i) {
        i += <caret>- 8;
    }
}
```
keys: Alt+Enter, choose "Remove unary minus and invert parent operation sign"
```after
class A {
    void unaryMinus(int i) {
        i -= 8;
    }
}
```

### JAVA-ICB-294 case 2: Single negation
settings: default
language: java
```before
class A {
    int m(int i) {
        return <caret>-i;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-295 case 1: Replace with string constructor
settings: default
language: java
status: unverified (exact text of the first fix name; the second fix is "Replace with 'BigDecimal.valueOf()'")
```before
import java.math.BigDecimal;
class Constructor {
    void foo() {
        <caret>new BigDecimal(0.1);
    }
}
```
keys: Alt+Enter, choose "Replace with '"0.1"'"
```after
import java.math.BigDecimal;
class Constructor {
    void foo() {
        new BigDecimal("0.1");
    }
}
```

### JAVA-ICB-295 case 2: Variable argument ignored by default
settings: default
language: java
```before
import java.math.BigDecimal;
class Constructor {
    void foo(double d) {
        <caret>new BigDecimal(d);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-296 case 1: Class that belongs to another package
settings: enable inspection ClassOnlyUsedInOnePackage
language: java
```before
--- p/A.java
package p;
class <caret>A { q.B b; }
--- q/B.java
package q;
public class B {}
```
keys: inspect (batch, whole project)
```after
problem: warning `A` "Class A has only dependencies on and/or dependents in package 'q'"
```

### JAVA-ICB-297 case 1: Unconnected class
settings: enable inspection ClassUnconnectedToPackage
language: java
```before
--- p/A.java
package p;
class <caret>A {}
--- p/B.java
package p;
class B { C c; }
--- p/C.java
package p;
class C {}
```
keys: inspect (batch, whole project)
```after
problem: warning `A` "Class A has no dependencies or dependents in its package"
```

### JAVA-ICB-298 case 1: Two independent groups
settings: enable inspection DisjointPackage
language: java
```before
--- p/A.java
package p;
class A { B b; }
--- p/B.java
package p;
class B {}
--- p/C.java
package p;
class C { D d; }
--- p/D.java
package p;
class D {}
```
keys: inspect (batch, whole project)
```after
problem: warning package `p` "Package p can be decomposed into 2 independent packages"
```

### JAVA-ICB-299 case 1: Exception-only package
settings: enable inspection ExceptionPackage
language: java
```before
--- errors/MyException.java
package errors;
public class MyException extends Exception {}
--- errors/OtherException.java
package errors;
public class OtherException extends RuntimeException {}
```
keys: inspect (batch, whole project)
```after
problem: warning package `errors` "Package 'errors' contains only exception classes"
```

### JAVA-ICB-300 case 1: Split package
settings: enable inspection PackageInMultipleModules
language: plain
```before
modules: a, b
--- a/p/A.java
package p;
class A {}
--- b/p/B.java
package p;
class B {}
```
keys: inspect (batch, whole project)
```after
problem: warning package `p` "Package 'p' has classes in modules 'a' and 'b'"
```

### JAVA-ICB-301 case 1: One class
settings: enable inspection PackageWithTooFewClasses
language: java
```before
--- p/A.java
package p;
class A {}
```
keys: inspect (batch, whole project)
```after
problem: warning package `p` "Package 'p' contains too few classes (1 < 3)"
```

### JAVA-ICB-302 case 1: Above a limit of 2
settings: enable inspection PackageWithTooManyClasses, Maximum number of classes=2
language: java
```before
--- p/A.java
package p;
class A {}
--- p/B.java
package p;
class B {}
--- p/C.java
package p;
class C {}
```
keys: inspect (batch, whole project)
```after
problem: warning package `p` "Package 'p' contains too many classes (3 > 2)"
```

### JAVA-ICB-303 case 1: One argument
settings: default
language: java
```before
import java.util.*;
class A {
    List<String> one = Arrays.<caret>asList("one");
}
```
keys: Alt+Enter, choose "Replace with 'Collections.singletonList()'"
```after
import java.util.*;
class A {
    List<String> one = Collections.singletonList("one");
}
```

### JAVA-ICB-303 case 2: Two arguments
settings: default
language: java
```before
import java.util.*;
class A {
    List<String> two = Arrays.<caret>asList("a", "b");
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-304 case 1: Make boxing explicit
settings: enable inspection AutoBoxing
language: java
```before
class A {
    Integer x = <caret>42;
}
```
keys: Alt+Enter, choose "Make boxing explicit"
```after
class A {
    Integer x = Integer.valueOf(42);
}
```

### JAVA-ICB-305 case 1: Make unboxing explicit
settings: enable inspection AutoUnboxing
language: java
```before
class A {
    int x(Integer boxed) {
        int y = <caret>boxed;
        return y;
    }
}
```
keys: Alt+Enter, choose "Make unboxing explicit"
```after
class A {
    int x(Integer boxed) {
        int y = boxed.intValue();
        return y;
    }
}
```

### JAVA-ICB-306 case 1: Constant argument
settings: default
language: java
```before
class A {
    Boolean b1 = <caret>new Boolean(true);
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    Boolean b1 = Boolean.TRUE;
}
```

### JAVA-ICB-306 case 2: valueOf is fine
settings: default
language: java
```before
class A {
    Boolean b1 = <caret>Boolean.valueOf("true");
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-307 case 1: valueOf of a boxed value
settings: default
language: java
```before
class A {
    void method(Integer i) {}
    void m() {
        Integer value = 1;
        method(Integer.valueOf(<caret>value));
    }
}
```
keys: Alt+Enter, choose "Remove unnecessary boxing"
```after
class A {
    void method(Integer i) {}
    void m() {
        Integer value = 1;
        method(value);
    }
}
```

### JAVA-ICB-307 case 2: valueOf of a primitive
settings: default
language: java
```before
class A {
    void method(Integer i) {}
    void m() {
        int value = 1;
        method(Integer.valueOf(<caret>value));
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-308 case 1: File isFile and lastModified
settings: enable inspection BulkFileAttributesRead
language: java
```before
import java.io.*;
class A {
    boolean isNewFile(File file, long lastModified) throws IOException {
        return file.<caret>isFile() && file.lastModified() > lastModified;
    }
}
```
keys: Alt+Enter, choose "Replace with bulk 'Files.readAttributes()' call"
```after
import java.io.*;
import java.nio.file.Files;
import java.nio.file.attribute.BasicFileAttributes;
class A {
    boolean isNewFile(File file, long lastModified) throws IOException {
        var fileAttributes = Files.readAttributes(file.toPath(), BasicFileAttributes.class);
        return fileAttributes.isRegularFile() && fileAttributes.lastModifiedTime().toMillis() > lastModified;
    }
}
```

### JAVA-ICB-309 case 1: Inline getter
settings: enable inspection CallToSimpleGetterInClass
language: java
```before
public class Salient {
    private String name;
    public String getName() {
        return name;
    }
    @Override
    public String toString() {
        return <caret>getName();
    }
}
```
keys: Alt+Enter, choose "Inline call to getter"
```after
public class Salient {
    private String name;
    public String getName() {
        return name;
    }
    @Override
    public String toString() {
        return name;
    }
}
```

### JAVA-ICB-310 case 1: Inline setter
settings: enable inspection CallToSimpleSetterInClass
language: java
```before
class Foo {
    private int index;
    public Foo(int idx) {
        <caret>setIndex(idx);
    }
    public void setIndex(int idx) {
        index = idx;
    }
}
```
keys: Alt+Enter, choose "Inline call to setter"
```after
class Foo {
    private int index;
    public Foo(int idx) {
        index = idx;
    }
    public void setIndex(int idx) {
        index = idx;
    }
}
```

### JAVA-ICB-311 case 1: Static-capable initializer
settings: enable inspection ClassInitializerMayBeStatic
language: java
```before
class A {
    public static String CONSTANT;
    <caret>{
        CONSTANT = "Hello";
    }
}
```
keys: Alt+Enter, choose "Make 'static'"
```after
class A {
    public static String CONSTANT;
    static {
        CONSTANT = "Hello";
    }
}
```

### JAVA-ICB-312 case 1: addAll after constructor
settings: default
language: java
```before
import java.util.*;
class A {
    void m() {
        Set<String> set = new HashSet<>();
        set.<caret>addAll(Arrays.asList("alpha", "beta", "gamma"));
    }
}
```
keys: Alt+Enter, choose "Replace 'addAll()' call with parametrized constructor call"
```after
import java.util.*;
class A {
    void m() {
        Set<String> set = new HashSet<>(Arrays.asList("alpha", "beta", "gamma"));
    }
}
```

### JAVA-ICB-312 case 2: Statement between creation and addAll
settings: default
language: java
```before
import java.util.*;
class A {
    void m() {
        Set<String> set = new HashSet<>();
        System.out.println();
        set.<caret>addAll(Arrays.asList("alpha"));
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-313 case 1: HashMap without capacity
settings: enable inspection CollectionsMustHaveInitialCapacity
language: java
status: unverified (rendering of #ref for a new expression)
```before
import java.util.*;
class A {
    Object m() {
        return <caret>new HashMap<String, String>();
    }
}
```
keys: inspect
```after
problem: warning `new` "new HashMap() without initial capacity"
```

### JAVA-ICB-314 case 1: replaceAll with literal
settings: default
language: java
status: unverified (generated constant name and placement)
```before
class A {
    String m(String text, String replacement) {
        return text.<caret>replaceAll("abc", replacement);
    }
}
```
keys: Alt+Enter, choose "Replace with call to method of compiled 'Pattern' constant"
```after
import java.util.regex.Matcher;
import java.util.regex.Pattern;
class A {
    private static final Pattern ABC = Pattern.compile("abc", Pattern.LITERAL);
    String m(String text, String replacement) {
        return ABC.matcher(text).replaceAll(Matcher.quoteReplacement(replacement));
    }
}
```

### JAVA-ICB-314 case 2: Non-constant regex
settings: default
language: java
```before
class A {
    String m(String text, String regex, String replacement) {
        return text.<caret>replaceAll(regex, replacement);
    }
}
```
keys: Alt+Enter
```after
the intention list does not contain "Replace with call to method of compiled 'Pattern' constant"; no problem reported
```

### JAVA-ICB-315 case 1: orElse to orElseGet
settings: default
language: java
```before
import java.util.Optional;
class A {
    String createDefaultValue() { return "x"; }
    String m(Optional<String> optional) {
        return optional.orElse(<caret>createDefaultValue());
    }
}
```
keys: Alt+Enter, choose "Use 'orElseGet' method with functional argument"
```after
import java.util.Optional;
class A {
    String createDefaultValue() { return "x"; }
    String m(Optional<String> optional) {
        return optional.orElseGet(this::createDefaultValue);
    }
}
```

### JAVA-ICB-315 case 2: Trivial argument
settings: default
language: java
```before
import java.util.Optional;
class A {
    String m(Optional<String> optional, String dflt) {
        return optional.orElse(<caret>dflt);
    }
}
```
keys: Alt+Enter
```after
the intention list does not contain "Use 'orElseGet' method with functional argument"; no problem reported
```

### JAVA-ICB-316 case 1: Final constant field
settings: enable inspection FieldMayBeStatic
language: java
```before
public class A {
    public final String <caret>str = "sample";
}
```
keys: Alt+Enter, choose "Make 'static'"
```after
public class A {
    public static final String str = "sample";
}
```

### JAVA-ICB-317 case 1: Search flag loop
settings: default
language: java
```before
import java.util.Objects;
class A {
    boolean m(int[] arr, int value) {
        boolean found = false;
        for (int i = 0; i < arr.length; i++) {
            <caret>if (Objects.equals(value, arr[i])) {
                found = true;
            }
        }
        return found;
    }
}
```
keys: Alt+Enter, choose "Add 'break'"
```after
import java.util.Objects;
class A {
    boolean m(int[] arr, int value) {
        boolean found = false;
        for (int i = 0; i < arr.length; i++) {
            if (Objects.equals(value, arr[i])) {
                found = true;
                break;
            }
        }
        return found;
    }
}
```

### JAVA-ICB-317 case 2: Counting loop
settings: default
language: java
```before
class A {
    int m(int[] arr, int value) {
        int count = 0;
        for (int i = 0; i < arr.length; i++) {
            <caret>if (arr[i] == value) {
                count++;
            }
        }
        return count;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-318 case 1: Non-final lazy constant
settings: default
language: java
status: unverified (requires a JDK that ships java.lang.LazyConstant, a preview API of recent JDKs)
```before
class A {
    private java.lang.LazyConstant<String> <caret>x = java.lang.LazyConstant.of(() -> "expensive string");
}
```
keys: Alt+Enter, choose "Make 'x' 'final'"
```after
class A {
    private final java.lang.LazyConstant<String> x = java.lang.LazyConstant.of(() -> "expensive string");
}
```

### JAVA-ICB-318 case 2: Final lazy constant
settings: default
language: java
status: unverified (requires a JDK that ships java.lang.LazyConstant)
```before
class A {
    private final java.lang.LazyConstant<String> <caret>x = java.lang.LazyConstant.of(() -> "expensive string");
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-319 case 1: new X().getClass()
settings: default
language: java
```before
class Sample {}
class A {
    Class<?> c = <caret>new Sample().getClass();
}
```
keys: Alt+Enter, choose "Replace with direct class object access"
```after
class Sample {}
class A {
    Class<?> c = Sample.class;
}
```

### JAVA-ICB-319 case 2: Class literal
settings: default
language: java
```before
class Sample {}
class A {
    Class<?> c = <caret>Sample.class;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-320 case 1: FileInputStream
settings: language level 8
language: java
```before
import java.io.*;
class A {
    InputStream m(File file) throws IOException {
        return new BufferedInputStream(<caret>new FileInputStream(file));
    }
}
```
keys: Alt+Enter, choose "Replace with 'Files.newInputStream()'"
```after
import java.io.*;
import java.nio.file.Files;
class A {
    InputStream m(File file) throws IOException {
        return new BufferedInputStream(Files.newInputStream(file.toPath()));
    }
}
```

### JAVA-ICB-320 case 2: Files.newInputStream already used
settings: language level 8
language: java
```before
import java.io.*;
import java.nio.file.Files;
class A {
    InputStream m(File file) throws IOException {
        return <caret>Files.newInputStream(file.toPath());
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-321 case 1: keySet then get
settings: enable inspection KeySetIterationMayUseEntrySet
language: java
status: unverified (item name and the exact body after the replacement)
```before
import java.util.*;
class A {
    void m(Map<Object, Object> map) {
        for (Object key : map.<caret>keySet()) {
            Object val = map.get(key);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'values()'"
```after
import java.util.*;
class A {
    void m(Map<Object, Object> map) {
        for (Object val : map.values()) {
        }
    }
}
```

### JAVA-ICB-322 case 1: indexOf with string
settings: enable inspection LengthOneStringInIndexOf
language: java
```before
class A {
    int m(String s) {
        return s.indexOf(<caret>"x");
    }
}
```
keys: Alt+Enter, choose "Replace with character"
```after
class A {
    int m(String s) {
        return s.indexOf('x');
    }
}
```

### JAVA-ICB-323 case 1: Concatenated literal
settings: default
language: java
```before
class A {
    String m(String hell) {
        return hell + <caret>"o";
    }
}
```
keys: Alt+Enter, choose "Replace with character"
```after
class A {
    String m(String hell) {
        return hell + 'o';
    }
}
```

### JAVA-ICB-323 case 2: Two-character literal
settings: default
language: java
```before
class A {
    String m(String hell) {
        return hell + <caret>"oo";
    }
}
```
keys: Alt+Enter
```after
the intention list does not contain "Replace with character"; no problem reported
```

### JAVA-ICB-324 case 1: Remove range
settings: default
language: java
```before
import java.util.List;
class A {
    void removeRange(List<String> list, int from, int to) {
        <caret>for (int i = from; i < to; i++) {
            list.remove(from);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'List.subList().clear()'"
```after
import java.util.List;
class A {
    void removeRange(List<String> list, int from, int to) {
        if (to > from) {
            list.subList(from, to).clear();
        }
    }
}
```

### JAVA-ICB-324 case 2: Removing a different index each time
settings: default
language: java
```before
import java.util.List;
class A {
    void m(List<String> list, int from, int to) {
        <caret>for (int i = from; i < to; i++) {
            list.remove(i);
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-325 case 1: Copy loop
settings: default
language: java
```before
class A {
    void m(int[] array, int[] newArray) {
        <caret>for (int i = 0; i < array.length; i++) {
            newArray[i] = array[i];
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'System.arraycopy()'"
```after
class A {
    void m(int[] array, int[] newArray) {
        System.arraycopy(array, 0, newArray, 0, array.length);
    }
}
```

### JAVA-ICB-325 case 2: Transforming loop
settings: default
language: java
```before
class A {
    void m(int[] array, int[] newArray) {
        <caret>for (int i = 0; i < array.length; i++) {
            newArray[i] = array[i] + 1;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-326 case 1: Loop with local variable
settings: default
language: java
```before
import java.util.*;
class A {
    void addAll(List<String> list, String[] arr) {
        <caret>for (int i = 0; i < arr.length; i++) {
            String s = arr[i];
            list.add(s);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'Collections.addAll()'"
```after
import java.util.*;
class A {
    void addAll(List<String> list, String[] arr) {
        Collections.addAll(list, arr);
    }
}
```

### JAVA-ICB-326 case 2: Loop with an extra statement
settings: default
language: java
```before
import java.util.*;
class A {
    void addAll(List<String> list, String[] arr) {
        <caret>for (int i = 0; i < arr.length; i++) {
            System.out.println(arr[i]);
            list.add(arr[i]);
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-327 case 1: HashMap with enum keys
settings: enable inspection MapReplaceableByEnumMap
language: java
```before
import java.util.*;
enum MyEnum { A }
class X {
    Map<MyEnum, String> myEnums = <caret>new HashMap<>();
}
```
keys: Alt+Enter, choose "Replace with 'EnumMap'"
```after
import java.util.*;
enum MyEnum { A }
class X {
    Map<MyEnum, String> myEnums = new EnumMap<>(MyEnum.class);
}
```

### JAVA-ICB-328 case 1: Utility method
settings: enable inspection MethodMayBeStatic
language: java
```before
class A {
    int <caret>twice(int x) {
        return x * 2;
    }
}
```
keys: Alt+Enter, choose "Make 'static'"
```after
class A {
    static int twice(int x) {
        return x * 2;
    }
}
```

### JAVA-ICB-329 case 1: new in loop
settings: enable inspection ObjectAllocationInLoop
language: java
status: unverified (rendering of #ref for the new expression)
```before
import java.util.*;
class A {
    void m(Map<Integer, List<String>> declarationsMap, int[] keys) {
        for (int key : keys) {
            declarationsMap.put(key, <caret>new ArrayList<>());
        }
    }
}
```
keys: inspect
```after
problem: warning `new` "Object allocation new ArrayList() in loop"
```

### JAVA-ICB-330 case 1: Final class without equals
settings: default
language: java
```before
final class Point {}
class A {
    boolean m(Point a, Point b) {
        return a.<caret>equals(b);
    }
}
```
keys: Alt+Enter, choose "Replace 'equals()' with '=='"
```after
final class Point {}
class A {
    boolean m(Point a, Point b) {
        return a == b;
    }
}
```

### JAVA-ICB-330 case 2: Class with own equals
settings: default
language: java
```before
final class Point {
    @Override public boolean equals(Object o) { return true; }
    @Override public int hashCode() { return 1; }
}
class A {
    boolean m(Point a, Point b) {
        return a.<caret>equals(b);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-331 case 1: Array in equals
settings: enable inspection ObjectInstantiationInEqualsHashCode
language: java
```before
import java.util.Arrays;
class Person {
    private String name;
    private int age;
    public boolean equals(Object o) {
        return Arrays.equals(<caret>new Object[] {name, age}, new Object[] {((Person) o).name, ((Person) o).age});
    }
}
```
keys: inspect
```after
problem: warning `new Object[] {name, age}` "Object instantiation inside 'equals()'"
```

### JAVA-ICB-332 case 1: nextDouble times size
settings: enable inspection RandomDoubleForRandomInteger
language: java
```before
import java.util.Random;
class A {
    static final int SIZE = 10;
    int getRandomInt() {
        return (int) ((new Random()).<caret>nextDouble() * SIZE);
    }
}
```
keys: Alt+Enter, choose "Replace with 'nextInt()'"
```after
import java.util.Random;
class A {
    static final int SIZE = 10;
    int getRandomInt() {
        return (new Random()).nextInt(SIZE);
    }
}
```

### JAVA-ICB-333 case 1: println of String.format
settings: default
language: java
```before
class A {
    void m() {
        System.out.println(String.<caret>format("Total count: %d", 42));
    }
}
```
keys: Alt+Enter, choose "Remove redundant call to 'String.format()'"
```after
class A {
    void m() {
        System.out.printf("Total count: %d%n", 42);
    }
}
```

### JAVA-ICB-333 case 2: format result stored
settings: default
language: java
```before
class A {
    String m() {
        String s = String.<caret>format("Total count: %d", 42);
        return s;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-334 case 1: Plain literal
settings: default
language: java
```before
class A {
    String m() {
        return "a,b,c".<caret>replaceAll(",", ";");
    }
}
```
keys: Alt+Enter, choose "Replace with 'replace()'"
```after
class A {
    String m() {
        return "a,b,c".replace(",", ";");
    }
}
```

### JAVA-ICB-334 case 2: Regex pattern
settings: default
language: java
```before
class A {
    String m() {
        return "a,b,c".<caret>replaceAll("[,]", ";");
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-335 case 1: stream().count()
settings: default
language: java
```before
import java.util.List;
class A {
    long m(List<String> list) {
        return list.stream().<caret>count();
    }
}
```
keys: Alt+Enter, choose "Replace with 'Collection.size()'"
```after
import java.util.List;
class A {
    long m(List<String> list) {
        return list.size();
    }
}
```

### JAVA-ICB-335 case 2: filter count
settings: default
language: java
```before
import java.util.List;
class A {
    long m(List<String> list) {
        return list.stream().filter(s -> s.isEmpty()).<caret>count();
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-336 case 1: HashSet of enum
settings: enable inspection SetReplaceableByEnumSet
language: java
```before
import java.util.*;
enum MyEnum { FOO, BAR }
class A {
    Set<MyEnum> enums = <caret>new HashSet<MyEnum>();
}
```
keys: Alt+Enter, choose "Replace with 'EnumSet'"
```after
import java.util.*;
enum MyEnum { FOO, BAR }
class A {
    Set<MyEnum> enums = EnumSet.noneOf(MyEnum.class);
}
```

### JAVA-ICB-337 case 1: removeAll with a list
settings: default
language: java
status: unverified (name of the fix with the generated text)
```before
import java.util.*;
class A {
    Set<String> mySet = new HashSet<>();
    public void check(String... ss) {
        mySet.<caret>removeAll(List.of(ss));
    }
}
```
keys: Alt+Enter, choose "Replace with 'List.of(ss).forEach(mySet::remove)'"
```after
import java.util.*;
class A {
    Set<String> mySet = new HashSet<>();
    public void check(String... ss) {
        List.of(ss).forEach(mySet::remove);
    }
}
```

### JAVA-ICB-337 case 2: removeAll with a set
settings: default
language: java
```before
import java.util.*;
class A {
    Set<String> mySet = new HashSet<>();
    public void check(Set<String> other) {
        mySet.<caret>removeAll(other);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-338 case 1: containsAll on a list
settings: default
language: java
```before
import java.util.*;
class A {
    public boolean check(List<String> list, Collection<String> collection) {
        return list.<caret>containsAll(collection);
    }
}
```
keys: Alt+Enter, choose "Wrap 'list' in 'HashSet' constructor"
```after
import java.util.*;
class A {
    public boolean check(List<String> list, Collection<String> collection) {
        return new HashSet<>(list).containsAll(collection);
    }
}
```

### JAVA-ICB-338 case 2: containsAll on a set
settings: default
language: java
```before
import java.util.*;
class A {
    public boolean check(Set<String> set, Collection<String> collection) {
        return set.<caret>containsAll(collection);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-339 case 1: StringBuilder without capacity
settings: enable inspection StringBufferMustHaveInitialCapacity
language: java
status: unverified (rendering of #ref for a new expression)
```before
class A {
    void m() {
        var sb = <caret>new StringBuilder();
    }
}
```
keys: inspect
```after
problem: warning `new` "new StringBuilder() without initial capacity"
```

### JAVA-ICB-340 case 1: Introduce builder
settings: default
language: java
```before
class A {
    String m() {
        String str = "";
        for (int i = 0; i < 10; i++) {
            str <caret>+= i;
        }
        return str;
    }
}
```
keys: Alt+Enter, choose "Introduce new StringBuilder to update variable 'str'"
```after
class A {
    String m() {
        String str = "";
        StringBuilder strBuilder = new StringBuilder(str);
        for (int i = 0; i < 10; i++) {
            strBuilder.append(i);
        }
        str = strBuilder.toString();
        return str;
    }
}
```

### JAVA-ICB-340 case 2: Concatenation outside loop
settings: default
language: java
```before
class A {
    String m(int i) {
        String str = "";
        str <caret>+= i;
        return str;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-341 case 1: Chain appends
settings: default
language: java
```before
class A {
    void bar(StringBuilder builder, String name) {
        builder.<caret>append("Hello," + name);
    }
}
```
keys: Alt+Enter, choose "Replace with chained 'append()' calls"
```after
class A {
    void bar(StringBuilder builder, String name) {
        builder.append("Hello,").append(name);
    }
}
```

### JAVA-ICB-341 case 2: Constant concatenation
settings: default
language: java
```before
class A {
    void bar(StringBuilder builder) {
        builder.<caret>append("Hello," + "world");
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-342 case 1: Non-null receiver
settings: default
language: java
```before
class A {
    boolean m(String s) {
        return s.<caret>equals("");
    }
}
```
keys: Alt+Enter, choose "Replace with 'isEmpty()'"
```after
class A {
    boolean m(String s) {
        return s.isEmpty();
    }
}
```

### JAVA-ICB-342 case 2: Nullable argument with the default option
settings: default
language: java
```before
class A {
    boolean m(String s) {
        return "".<caret>equals(s);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-343 case 1: Appending in a loop
settings: enable inspection StringReplaceableByStringBuffer
language: java
```before
class A {
    String m(String[] names) {
        String <caret>s = "";
        for (int i = 0; i < names.length; i++) {
            s = s + names[i];
        }
        return s;
    }
}
```
keys: inspect
```after
problem: warning `s` "Non-constant String s should probably be declared as 'StringBuilder'"
```

### JAVA-ICB-344 case 1: Factorial
settings: default
language: java
```before
class A {
    int factorial(int val, int runningVal) {
        if (val == 1) {
            return runningVal;
        } else {
            return <caret>factorial(val - 1, runningVal * val);
        }
    }
}
```
keys: Alt+Enter, choose "Replace tail recursion with iteration"
```after
class A {
    int factorial(int val, int runningVal) {
        while (true) {
            if (val == 1) {
                return runningVal;
            } else {
                runningVal = runningVal * val;
                val = val - 1;
            }
        }
    }
}
```

### JAVA-ICB-344 case 2: Non-tail recursion
settings: default
language: java
```before
class A {
    int sum(int n) {
        if (n == 0) return 0;
        return n + <caret>sum(n - 1);
    }
}
```
keys: Alt+Enter
```after
the intention list does not contain "Replace tail recursion with iteration"; no problem reported
```

### JAVA-ICB-345 case 1: Pre-sized to empty
settings: default
language: java
```before
import java.util.*;
class A {
    String[] m(List<String> c) {
        return c.<caret>toArray(new String[c.size()]);
    }
}
```
keys: Alt+Enter, choose "Replace argument with empty array"
```after
import java.util.*;
class A {
    String[] m(List<String> c) {
        return c.toArray(new String[0]);
    }
}
```

### JAVA-ICB-345 case 2: Already empty
settings: default
language: java
```before
import java.util.*;
class A {
    String[] m(List<String> c) {
        return c.<caret>toArray(new String[0]);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-346 case 1: Temporary Integer
settings: default
language: java
status: unverified (the inspection description shows `Integer.valueOf("3")`, the fix builds the call from the type's parse method, here `parseInt`)
```before
class A {
    int m() {
        return new Integer("3").<caret>intValue();
    }
}
```
keys: Alt+Enter, choose "Replace with 'Integer.parseInt("3")'"
```after
class A {
    int m() {
        return Integer.parseInt("3");
    }
}
```

### JAVA-ICB-346 case 2: Static parse call
settings: default
language: java
```before
class A {
    int m() {
        return Integer.<caret>parseInt("3");
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-347 case 1: Temporary Integer toString
settings: default
language: java
```before
class A {
    String m() {
        return <caret>new Integer(3).toString();
    }
}
```
keys: Alt+Enter, choose "Replace with 'Integer.toString(3)'"
```after
class A {
    String m() {
        return Integer.toString(3);
    }
}
```

### JAVA-ICB-347 case 2: Static toString
settings: default
language: java
```before
class A {
    String m() {
        return Integer.<caret>toString(3);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-348 case 1: Loop with add
settings: default
language: java
```before
import java.util.*;
class A {
    void test(Collection<Integer> numbers) {
        List<Integer> result = new ArrayList<>();
        for (Integer i : numbers) {
            result.<caret>add(i);
        }
    }
}
```
keys: Alt+Enter, choose "Replace iteration with bulk 'addAll()' call"
```after
import java.util.*;
class A {
    void test(Collection<Integer> numbers) {
        List<Integer> result = new ArrayList<>();
        result.addAll(numbers);
    }
}
```

### JAVA-ICB-348 case 2: Transformed element
settings: default
language: java
```before
import java.util.*;
class A {
    void test(Collection<Integer> numbers) {
        List<Integer> result = new ArrayList<>();
        for (Integer i : numbers) {
            result.<caret>add(i + 1);
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-349 case 1: Used as primitive in a loop
settings: default
language: java
```before
class A {
    void needPrimitive(int value) {}
    void needBox(Integer value) {}
    public void example() {
        <caret>Integer value = 12;
        needBox(value);
        for (int i = 0; i < 10; i++) {
            needPrimitive(value);
        }
    }
}
```
keys: Alt+Enter, choose "Convert wrapper type to primitive"
```after
class A {
    void needPrimitive(int value) {}
    void needBox(Integer value) {}
    public void example() {
        int value = 12;
        needBox(value);
        for (int i = 0; i < 10; i++) {
            needPrimitive(value);
        }
    }
}
```

### JAVA-ICB-349 case 2: Only used as an object
settings: default
language: java
```before
class A {
    void needBox(Integer value) {}
    public void example() {
        <caret>Integer value = 12;
        needBox(value);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-350 case 1: Windows path
settings: enable inspection HardcodedFileSeparators
language: java
```before
import java.io.File;
class A {
    File m() {
        return new File(<caret>"C:\\Users\\Name");
    }
}
```
keys: inspect
```after
problem: warning `"C:\\Users\\Name"` "Hardcoded file separator \"C:\\Users\\Name\""
```

### JAVA-ICB-351 case 1: Line separators in a string
settings: enable inspection HardcodedLineSeparators
language: java
```before
class A {
    String count = <caret>"first\nsecond\rthird";
}
```
keys: inspect
```after
problem: warning `"first\nsecond\rthird"` "Hardcoded line separator \"first\nsecond\rthird\""
```

### JAVA-ICB-352 case 1: Native method
settings: enable inspection NativeMethods
language: java
```before
class A {
    public <caret>native void run();
}
```
keys: inspect
```after
problem: warning `native` "Methods declared native are non-portable"
```

### JAVA-ICB-353 case 1: Runtime.exec
settings: enable inspection RuntimeExec
language: java
```before
class A {
    void m() throws java.io.IOException {
        Runtime.getRuntime().<caret>exec("ls");
    }
}
```
keys: inspect
```after
problem: warning `exec` "Call to Runtime.exec() is non-portable"
```

### JAVA-ICB-354 case 1: System.exit
settings: enable inspection SystemExit
language: java
```before
class A {
    void m() {
        System.<caret>exit(1);
    }
}
```
keys: inspect
```after
problem: warning `exit` "Call to System.exit() is non-portable"
```

### JAVA-ICB-355 case 1: System.getenv
settings: enable inspection SystemGetenv
language: java
```before
class A {
    String m() {
        return System.<caret>getenv("HOME");
    }
}
```
keys: inspect
```after
problem: warning `getenv` "Call to System.getenv() is non-portable"
```

### JAVA-ICB-356 case 1: Implementing a peer interface
settings: enable inspection UseOfAWTPeerClass, language level 8
language: java
```before
import java.awt.peer.ButtonPeer;
abstract class Sample implements <caret>ButtonPeer {
}
```
keys: inspect
```after
problem: warning `ButtonPeer` "Use of AWT peer class ButtonPeer is non-portable"
```

### JAVA-ICB-357 case 1: Concrete driver class
settings: enable inspection UseOfJDBCDriverClass
language: java
status: unverified (needs the driver library on the classpath; the interface java.sql.Driver itself is the allowed form)
```before
class A {
    void m() {
        <caret>org.postgresql.Driver d = null;
    }
}
```
keys: inspect
```after
problem: warning `org.postgresql.Driver` "Use of concrete JDBC driver class org.postgresql.Driver is non-portable"
```

### JAVA-ICB-358 case 1: ProcessBuilder
settings: enable inspection UseOfProcessBuilder
language: java
```before
class A {
    void m() {
        <caret>ProcessBuilder pb = new ProcessBuilder("ls");
    }
}
```
keys: inspect
```after
problem: warning `ProcessBuilder` "Use of ProcessBuilder is non-portable"
```

### JAVA-ICB-359 case 1: sun.misc.Unsafe
settings: enable inspection UseOfSunClasses
language: java
```before
class A {
    void m() {
        <caret>sun.misc.Unsafe u = null;
    }
}
```
keys: inspect
```after
problem: warning `sun.misc.Unsafe` "Use of Sun-supplied class sun.misc.Unsafe is non-portable"
```

