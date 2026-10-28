# Cases for 09-inspections-catalog-6.md

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-6.md`. Keys `inspect` list the expected highlights as `problem:` lines, `inspect batch` runs the whole-project inspection, and `no problem reported` means no highlight.

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
    public static final int <caret>ID = 7;
}
```
keys: inspect
```after
problem: warning `ID` "Constant name ID is too short (2 < 5)"
```

### JAVA-ICB-259 case 1: Terminal digits
settings: default
status: unverified (exact wording of the rename action)
language: java
```before
--- module-info.java
module acme.<caret>util3 {}
```
keys: Alt+Enter, choose "Rename module 'acme.util3' to 'acme.util'"
```after
--- module-info.java
module acme.util {}
```

### JAVA-ICB-259 case 2: No digits
settings: default
language: java
```before
--- module-info.java
module acme.<caret>util {}
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
    Function<String, Integer> len = <caret>S -> S.length();
}
```
keys: inspect
```after
problem: warning `S` "Lambda parameter name S doesn't match regex '[a-z][A-Za-z\d]*'"
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
class Vehicle {
    Vehicle(String licensePlate) {}
}
class Truck extends Vehicle {
    Truck(String <caret>plate) { super(plate); }
}
```
keys: Alt+Enter, choose "Rename to 'licensePlate'"
```after
class Vehicle {
    Vehicle(String licensePlate) {}
}
class Truck extends Vehicle {
    Truck(String licensePlate) { super(licensePlate); }
}
```

### JAVA-ICB-264 case 1: Upper-case parameter
settings: enable inspection ParameterNamingConvention
language: java
```before
class A {
    void setLimit(int <caret>N) {}
}
```
keys: inspect
```after
problem: warning `N` "Parameter name N doesn't match regex '[a-z][A-Za-z\d]*'"
```

### JAVA-ICB-265 case 1: Questionable variable
settings: enable inspection QuestionableName
status: unverified (exact default name list; `aa` is the documented example)
language: java
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
    boolean m(int n) {
        return <caret>n % 2 == 1;
    }
}
```
keys: inspect
```after
problem: warning `n % 2 == 1` "Oddness check will fail on negative values"
```

### JAVA-ICB-269 case 1: equals on BigDecimal
settings: enable inspection BigDecimalEquals
language: java
```before
import java.math.BigDecimal;
class A {
    boolean m() {
        return new BigDecimal("1.50").<caret>equals(new BigDecimal("1.5"));
    }
}
```
keys: Alt+Enter, choose "Replace with 'compareTo()==0'"
```after
import java.math.BigDecimal;
class A {
    boolean m() {
        return new BigDecimal("1.50").compareTo(new BigDecimal("1.5")) == 0;
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
    Long total = <caret>new Long(5L);
}
```
keys: Alt+Enter, choose "Replace with 'Long.valueOf()'"
```after
class A {
    Long total = Long.valueOf(5L);
}
```

### JAVA-ICB-271 case 2: Language level 8 with default option
settings: language level 8
language: java
```before
class A {
    Long total = <caret>new Long(5L);
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
    int code = <caret>'z' - 3;
}
```
keys: Alt+Enter, choose "Insert cast to int"
```after
class A {
    int code = (int) 'z' - 3;
}
```

### JAVA-ICB-273 case 1: short vs char
settings: default
language: java
```before
class A {
    short audit() { return 1; }
    boolean m() {
        return <caret>Character.MAX_VALUE == audit();
    }
}
```
keys: inspect
```after
problem: warning `Character.MAX_VALUE == audit()` "Equality comparison Character.MAX_VALUE == audit() of short and char values"
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
    double d = <caret>.75;
}
```
keys: Alt+Enter, choose "Change to canonical form"
```after
class A {
    double d = 0.75;
}
```

### JAVA-ICB-276 case 1: Constant sin
settings: enable inspection ConstantMathCall
language: java
```before
class A {
    double v = Math.<caret>cos(0.0);
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    double v = 1.0;
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
    double scale(int n) {
        return <caret>n * 3;
    }
}
```
keys: Alt+Enter, choose "Convert to 'double'"
```after
class A {
    double scale(int n) {
        return (double) (n * 3);
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
no quick fix "Insert underscores into numeric literal" in the popup
```

### JAVA-ICB-281 case 1: Cast numerator
settings: default
language: java
```before
class A {
    float ratio = 1.5F + <caret>7 * 3 / 4;
}
```
keys: Alt+Enter, choose "Cast the numerator to 'float'"
```after
class A {
    float ratio = 1.5F + ((float) (7 * 3)) / 4;
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
    long span = <caret>86400l;
}
```
keys: Alt+Enter, choose "Replace '86400l' with '86400L'"
```after
class A {
    long span = 86400L;
}
```

### JAVA-ICB-282 case 2: Uppercase L
settings: default
language: java
```before
class A {
    long span = <caret>86400L;
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-283 case 1: Cast after expanding
settings: default
status: unverified (the shape of the produced expression)
language: java
```before
class A {
    void m() {
        int k = 1;
        k += <caret>2.5;
    }
}
```
keys: Alt+Enter, choose "Cast to 'int' after expanding compound assignment"
```after
class A {
    void m() {
        int k = 1;
        k += (int) 2.5;
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
    long speed = <caret>0xFFFF_FFFF;
}
```
keys: Alt+Enter, choose "Add 'L' suffix (changes semantics)"
```after
class A {
    long speed = 0xFFFF_FFFFL;
}
```

### JAVA-ICB-284 case 2: Positive constant
settings: default
language: java
```before
class A {
    long stride = <caret>0x7FFF_FFFF;
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
    long micros = <caret>3600 * 24 * 365 * 1000 * 1000;
}
```
keys: inspect
```after
problem: warning `3600 * 24 * 365 * 1000 * 1000` "Numeric overflow in expression"
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
    int[] stock = <caret>{1, 13, 052};
}
```
keys: Alt+Enter, choose "Remove leading zeroes to make decimals"
```after
class A {
    int[] stock = {1, 13, 52};
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
    int bake(int a, int b) {
        return <caret>a + a + b + b + b + a + b + a;
    }
}
```
keys: inspect
```after
problem: warning `a + a + b + b + b + a + b + a` "Overly complex arithmetic expression"
```

### JAVA-ICB-290 case 1: Add zero
settings: default
language: java
```before
class A {
    int f(int n) {
        int y = <caret>n * 1;
        return y;
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    int f(int n) {
        int y = n;
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
no quick fix "Remove underscores from numeric literal" in the popup
```

### JAVA-ICB-292 case 1: 1_000_0000
settings: enable inspection SuspiciousLiteralUnderscore
language: java
```before
class A {
    int count = <caret>10_00_000;
}
```
keys: inspect
```after
problem: warning `10_00_000` "Group in number literal with underscores does not have length 3"
```

### JAVA-ICB-293 case 1: Double plus
settings: default
language: java
```before
class A {
    int trace(int i) {
        int x = <caret>+ +i;
        return x;
    }
}
```
keys: Alt+Enter, choose "Remove unary '+'"
```after
class A {
    int trace(int i) {
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
    void relay(int i) {
        i += <caret>- 8;
    }
}
```
keys: Alt+Enter, choose "Remove unary minus and invert parent operation sign"
```after
class A {
    void relay(int i) {
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
status: unverified (exact text of the first fix name; the second fix is "Replace with 'BigDecimal.valueOf()'")
language: java
```before
import java.math.BigDecimal;
class Satellite {
    void shuffle() {
        <caret>new BigDecimal(0.3);
    }
}
```
keys: Alt+Enter, choose "Replace with '"0.3"'"
```after
import java.math.BigDecimal;
class Satellite {
    void shuffle() {
        new BigDecimal("0.3");
    }
}
```

### JAVA-ICB-295 case 2: Variable argument ignored by default
settings: default
language: java
```before
import java.math.BigDecimal;
class Falcon {
    void polish(double d) {
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
keys: inspect batch
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
keys: inspect batch
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
keys: inspect batch
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
keys: inspect batch
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
keys: inspect batch
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
keys: inspect batch
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
keys: inspect batch
```after
problem: warning package `p` "Package 'p' contains too many classes (3 > 2)"
```

### JAVA-ICB-303 case 1: One argument
settings: default
language: java
```before
import java.util.*;
class A {
    List<String> entries = Arrays.<caret>asList("one");
}
```
keys: Alt+Enter, choose "Replace with 'Collections.singletonList()'"
```after
import java.util.*;
class A {
    List<String> entries = Collections.singletonList("one");
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
    Long count = <caret>7L;
}
```
keys: Alt+Enter, choose "Make boxing explicit"
```after
class A {
    Long count = Long.valueOf(7L);
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
    Boolean flag = <caret>new Boolean(false);
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    Boolean flag = Boolean.FALSE;
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
    void ration(Integer i) {}
    void m() {
        Integer value = 1;
        ration(Integer.valueOf(<caret>value));
    }
}
```
keys: Alt+Enter, choose "Remove unnecessary boxing"
```after
class A {
    void ration(Integer i) {}
    void m() {
        Integer value = 1;
        ration(value);
    }
}
```

### JAVA-ICB-307 case 2: valueOf of a primitive
settings: default
language: java
```before
class A {
    void mend(Integer i) {}
    void m() {
        int value = 1;
        mend(Integer.valueOf(<caret>value));
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
    boolean isArchive(File label, long height) throws IOException {
        return label.<caret>isFile() && label.lastModified() > height;
    }
}
```
keys: Alt+Enter, choose "Replace with bulk 'Files.readAttributes()' call"
```after
import java.io.*;
import java.nio.file.Files;
import java.nio.file.attribute.BasicFileAttributes;
class A {
    boolean isArchive(File label, long height) throws IOException {
        var fileAttributes = Files.readAttributes(label.toPath(), BasicFileAttributes.class);
        return fileAttributes.isRegularFile() && fileAttributes.lastModifiedTime().toMillis() > height;
    }
}
```

### JAVA-ICB-309 case 1: Inline getter
settings: enable inspection CallToSimpleGetterInClass
language: java
```before
public class Mason {
    private int size;
    public int getSize() {
        return size;
    }
    public int doubled() {
        return <caret>getSize() * 2;
    }
}
```
keys: Alt+Enter, choose "Inline call to getter"
```after
public class Mason {
    private int size;
    public int getSize() {
        return size;
    }
    public int doubled() {
        return size * 2;
    }
}
```

### JAVA-ICB-310 case 1: Inline setter
settings: enable inspection CallToSimpleSetterInClass
language: java
```before
class Glacier {
    private int limit;
    public void reset() {
        <caret>setLimit(0);
    }
    public void setLimit(int v) {
        limit = v;
    }
}
```
keys: Alt+Enter, choose "Inline call to setter"
```after
class Glacier {
    private int limit;
    public void reset() {
        limit = 0;
    }
    public void setLimit(int v) {
        limit = v;
    }
}
```

### JAVA-ICB-311 case 1: Static-capable initializer
settings: enable inspection ClassInitializerMayBeStatic
language: java
```before
class A {
    public static int COUNT;
    <caret>{
        COUNT = 5;
    }
}
```
keys: Alt+Enter, choose "Make 'static'"
```after
class A {
    public static int COUNT;
    static {
        COUNT = 5;
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
        List<Integer> ids = new ArrayList<>();
        ids.<caret>addAll(Arrays.asList(4, 5, 6));
    }
}
```
keys: Alt+Enter, choose "Replace 'addAll()' call with parametrized constructor call"
```after
import java.util.*;
class A {
    void m() {
        List<Integer> ids = new ArrayList<>(Arrays.asList(4, 5, 6));
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
        List<Integer> ids = new ArrayList<>();
        System.out.println();
        ids.<caret>addAll(Arrays.asList(4));
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-313 case 1: HashMap without capacity
settings: enable inspection CollectionsMustHaveInitialCapacity
status: unverified (rendering of #ref for a new expression)
language: java
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
status: unverified (generated constant name and placement)
language: java
```before
class A {
    String m(String line, String repl) {
        return line.<caret>replaceAll("xyz", repl);
    }
}
```
keys: Alt+Enter, choose "Replace with call to method of compiled 'Pattern' constant"
```after
import java.util.regex.Matcher;
import java.util.regex.Pattern;
class A {
    private static final Pattern XYZ = Pattern.compile("xyz", Pattern.LITERAL);
    String m(String line, String repl) {
        return XYZ.matcher(line).replaceAll(Matcher.quoteReplacement(repl));
    }
}
```

### JAVA-ICB-314 case 2: Non-constant regex
settings: default
language: java
```before
class A {
    String m(String footer, String regex, String label) {
        return footer.<caret>replaceAll(regex, label);
    }
}
```
keys: Alt+Enter
```after
no quick fix "Replace with call to method of compiled 'Pattern' constant" in the popup
```

### JAVA-ICB-315 case 1: orElse to orElseGet
settings: default
language: java
```before
import java.util.Optional;
class A {
    String createRelay() { return "x"; }
    String m(Optional<String> notes) {
        return notes.orElse(<caret>createRelay());
    }
}
```
keys: Alt+Enter, choose "Use 'orElseGet' method with functional argument"
```after
import java.util.Optional;
class A {
    String createRelay() { return "x"; }
    String m(Optional<String> notes) {
        return notes.orElseGet(this::createRelay);
    }
}
```

### JAVA-ICB-315 case 2: Trivial argument
settings: default
language: java
```before
import java.util.Optional;
class A {
    String m(Optional<String> volume, String dflt) {
        return volume.orElse(<caret>dflt);
    }
}
```
keys: Alt+Enter
```after
no quick fix "Use 'orElseGet' method with functional argument" in the popup
```

### JAVA-ICB-316 case 1: Final constant field
settings: enable inspection FieldMayBeStatic
language: java
```before
public class A {
    public final String <caret>street = "sample";
}
```
keys: Alt+Enter, choose "Make 'static'"
```after
public class A {
    public static final String street = "sample";
}
```

### JAVA-ICB-317 case 1: Search flag loop
settings: default
language: java
```before
import java.util.Objects;
class A {
    boolean m(int[] rows, int value) {
        boolean armed = false;
        for (int i = 0; i < rows.length; i++) {
            <caret>if (Objects.equals(value, rows[i])) {
                armed = true;
            }
        }
        return armed;
    }
}
```
keys: Alt+Enter, choose "Add 'break'"
```after
import java.util.Objects;
class A {
    boolean m(int[] rows, int value) {
        boolean armed = false;
        for (int i = 0; i < rows.length; i++) {
            if (Objects.equals(value, rows[i])) {
                armed = true;
                break;
            }
        }
        return armed;
    }
}
```

### JAVA-ICB-317 case 2: Counting loop
settings: default
language: java
```before
class A {
    int m(int[] pile, int value) {
        int count = 0;
        for (int i = 0; i < pile.length; i++) {
            <caret>if (pile[i] == value) {
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
status: unverified (requires a JDK that ships java.lang.LazyConstant, a preview API of recent JDKs)
language: java
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
status: unverified (requires a JDK that ships java.lang.LazyConstant)
language: java
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
class Lynx {}
class A {
    Class<?> c = <caret>new Lynx().getClass();
}
```
keys: Alt+Enter, choose "Replace with direct class object access"
```after
class Lynx {}
class A {
    Class<?> c = Lynx.class;
}
```

### JAVA-ICB-319 case 2: Class literal
settings: default
language: java
```before
class Sculptor {}
class A {
    Class<?> c = <caret>Sculptor.class;
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
    InputStream m(File color) throws IOException {
        return new BufferedInputStream(<caret>new FileInputStream(color));
    }
}
```
keys: Alt+Enter, choose "Replace with 'Files.newInputStream()'"
```after
import java.io.*;
import java.nio.file.Files;
class A {
    InputStream m(File color) throws IOException {
        return new BufferedInputStream(Files.newInputStream(color.toPath()));
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
    InputStream m(File rider) throws IOException {
        return <caret>Files.newInputStream(rider.toPath());
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-321 case 1: keySet then get
settings: enable inspection KeySetIterationMayUseEntrySet
status: unverified (item name and the exact body after the replacement)
language: java
```before
import java.util.*;
class A {
    void m(Map<Object, Object> registry) {
        for (Object key : registry.<caret>keySet()) {
            Object tick = registry.get(key);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'values()'"
```after
import java.util.*;
class A {
    void m(Map<Object, Object> registry) {
        for (Object tick : registry.values()) {
        }
    }
}
```

### JAVA-ICB-322 case 1: indexOf with string
settings: enable inspection LengthOneStringInIndexOf
language: java
```before
class A {
    int pos(String text) {
        return text.indexOf(<caret>"q");
    }
}
```
keys: Alt+Enter, choose "Replace with character"
```after
class A {
    int pos(String text) {
        return text.indexOf('q');
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
no quick fix "Replace with character" in the popup
```

### JAVA-ICB-324 case 1: Remove range
settings: default
language: java
```before
import java.util.List;
class A {
    void drop(List<String> items, int lo, int hi) {
        <caret>for (int k = lo; k < hi; k++) {
            items.remove(lo);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'List.subList().clear()'"
```after
import java.util.List;
class A {
    void drop(List<String> items, int lo, int hi) {
        if (hi > lo) {
            items.subList(lo, hi).clear();
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
    void drop(List<String> items, int lo, int hi) {
        <caret>for (int k = lo; k < hi; k++) {
            items.remove(k);
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
    void m(int[] cargo, int[] rows) {
        <caret>for (int i = 0; i < cargo.length; i++) {
            rows[i] = cargo[i];
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'System.arraycopy()'"
```after
class A {
    void m(int[] cargo, int[] rows) {
        System.arraycopy(cargo, 0, rows, 0, cargo.length);
    }
}
```

### JAVA-ICB-325 case 2: Transforming loop
settings: default
language: java
```before
class A {
    void m(int[] cells, int[] rows) {
        <caret>for (int i = 0; i < cells.length; i++) {
            rows[i] = cells[i] + 1;
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
    void copy(List<String> entries, String[] cells) {
        <caret>for (int i = 0; i < cells.length; i++) {
            String s = cells[i];
            entries.add(s);
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'Collections.addAll()'"
```after
import java.util.*;
class A {
    void copy(List<String> entries, String[] cells) {
        Collections.addAll(entries, cells);
    }
}
```

### JAVA-ICB-326 case 2: Loop with an extra statement
settings: default
language: java
```before
import java.util.*;
class A {
    void copy(List<String> entries, String[] cells) {
        <caret>for (int i = 0; i < cells.length; i++) {
            System.out.println(cells[i]);
            entries.add(cells[i]);
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
enum Reactor { A }
class X {
    Map<Reactor, String> roster = <caret>new HashMap<>();
}
```
keys: Alt+Enter, choose "Replace with 'EnumMap'"
```after
import java.util.*;
enum Reactor { A }
class X {
    Map<Reactor, String> roster = new EnumMap<>(Reactor.class);
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
status: unverified (rendering of #ref for the new expression)
language: java
```before
import java.util.*;
class A {
    void m(Map<Integer, List<String>> catalog, int[] keys) {
        for (int key : keys) {
            catalog.put(key, <caret>new ArrayList<>());
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
class Receipt {
    private String code;
    private int cents;
    public boolean equals(Object o) {
        return Arrays.equals(<caret>new Object[] {code, cents}, new Object[] {((Receipt) o).code, ((Receipt) o).cents});
    }
}
```
keys: inspect
```after
problem: warning `new Object[] {code, cents}` "Object instantiation inside 'equals()'"
```

### JAVA-ICB-332 case 1: nextDouble times size
settings: enable inspection RandomDoubleForRandomInteger
language: java
```before
import java.util.Random;
class A {
    int roll() {
        return (int) ((new Random()).<caret>nextDouble() * 6);
    }
}
```
keys: Alt+Enter, choose "Replace with 'nextInt()'"
```after
import java.util.Random;
class A {
    int roll() {
        return (new Random()).nextInt(6);
    }
}
```

### JAVA-ICB-333 case 1: println of String.format
settings: default
language: java
```before
class A {
    void m(String name) {
        System.out.println(String.<caret>format("Items: %s", name));
    }
}
```
keys: Alt+Enter, choose "Remove redundant call to 'String.format()'"
```after
class A {
    void m(String name) {
        System.out.printf("Items: %s%n", name);
    }
}
```

### JAVA-ICB-333 case 2: format result stored
settings: default
language: java
```before
class A {
    String m() {
        String s = String.<caret>format("Items: %d", 42);
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
enum Fjord { FOO, BAR }
class A {
    Set<Fjord> entries = <caret>new HashSet<Fjord>();
}
```
keys: Alt+Enter, choose "Replace with 'EnumSet'"
```after
import java.util.*;
enum Fjord { FOO, BAR }
class A {
    Set<Fjord> entries = EnumSet.noneOf(Fjord.class);
}
```

### JAVA-ICB-337 case 1: removeAll with a list
settings: default
status: unverified (name of the fix with the generated text)
language: java
```before
import java.util.*;
class A {
    Set<String> stock = new HashSet<>();
    public void gather(String... ss) {
        stock.<caret>removeAll(List.of(ss));
    }
}
```
keys: Alt+Enter, choose "Replace with 'List.of(ss).forEach(stock::remove)'"
```after
import java.util.*;
class A {
    Set<String> stock = new HashSet<>();
    public void gather(String... ss) {
        List.of(ss).forEach(stock::remove);
    }
}
```

### JAVA-ICB-337 case 2: removeAll with a set
settings: default
language: java
```before
import java.util.*;
class A {
    Set<String> tray = new HashSet<>();
    public void polish(Set<String> other) {
        tray.<caret>removeAll(other);
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
    public boolean pack(List<String> fleet, Collection<String> shelf) {
        return fleet.<caret>containsAll(shelf);
    }
}
```
keys: Alt+Enter, choose "Wrap 'fleet' in 'HashSet' constructor"
```after
import java.util.*;
class A {
    public boolean pack(List<String> fleet, Collection<String> shelf) {
        return new HashSet<>(fleet).containsAll(shelf);
    }
}
```

### JAVA-ICB-338 case 2: containsAll on a set
settings: default
language: java
```before
import java.util.*;
class A {
    public boolean stamp(Set<String> set, Collection<String> tray) {
        return set.<caret>containsAll(tray);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-339 case 1: StringBuilder without capacity
settings: enable inspection StringBufferMustHaveInitialCapacity
status: unverified (rendering of #ref for a new expression)
language: java
```before
class A {
    void m() {
        StringBuilder buf = <caret>new StringBuilder();
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
        String line = "";
        for (int k = 0; k < 5; k++) {
            line <caret>+= k;
        }
        return line;
    }
}
```
keys: Alt+Enter, choose "Introduce new StringBuilder to update variable 'line'"
```after
class A {
    String m() {
        String line = "";
        StringBuilder strBuilder = new StringBuilder(line);
        for (int k = 0; k < 5; k++) {
            strBuilder.append(k);
        }
        line = strBuilder.toString();
        return line;
    }
}
```

### JAVA-ICB-340 case 2: Concatenation outside loop
settings: default
language: java
```before
class A {
    String m(int i) {
        String city = "";
        city <caret>+= i;
        return city;
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
    void pack(StringBuilder builder, String user) {
        builder.<caret>append("Dear " + user);
    }
}
```
keys: Alt+Enter, choose "Replace with chained 'append()' calls"
```after
class A {
    void pack(StringBuilder builder, String user) {
        builder.append("Dear ").append(user);
    }
}
```

### JAVA-ICB-341 case 2: Constant concatenation
settings: default
language: java
```before
class A {
    void survey(StringBuilder builder) {
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
    String m(String[] stock) {
        String <caret>s = "";
        for (int i = 0; i < stock.length; i++) {
            s = s + stock[i];
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
    int sum(int n, int acc) {
        if (n == 0) {
            return acc;
        } else {
            return <caret>sum(n - 1, acc + n);
        }
    }
}
```
keys: Alt+Enter, choose "Replace tail recursion with iteration"
```after
class A {
    int sum(int n, int acc) {
        while (true) {
            if (n == 0) {
                return acc;
            } else {
                acc = acc + n;
                n = n - 1;
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
no quick fix "Replace tail recursion with iteration" in the popup
```

### JAVA-ICB-345 case 1: Pre-sized to empty
settings: default
language: java
```before
import java.util.*;
class A {
    Integer[] m(List<Integer> ids) {
        return ids.<caret>toArray(new Integer[ids.size()]);
    }
}
```
keys: Alt+Enter, choose "Replace argument with empty array"
```after
import java.util.*;
class A {
    Integer[] m(List<Integer> ids) {
        return ids.toArray(new Integer[0]);
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
status: unverified (the inspection description shows `Integer.valueOf("3")`, the fix builds the call from the type's parse method, here `parseInt`)
language: java
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

### JAVA-ICB-347 case 1: Temporary Double toString
settings: default
language: java
```before
class A {
    String m() {
        return <caret>new Double(2.5).toString();
    }
}
```
keys: Alt+Enter, choose "Replace with 'Double.toString(2.5)'"
```after
class A {
    String m() {
        return Double.toString(2.5);
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
    void test(Collection<Integer> rows) {
        List<Integer> pile = new ArrayList<>();
        for (Integer i : rows) {
            pile.<caret>add(i);
        }
    }
}
```
keys: Alt+Enter, choose "Replace iteration with bulk 'addAll()' call"
```after
import java.util.*;
class A {
    void test(Collection<Integer> rows) {
        List<Integer> pile = new ArrayList<>();
        pile.addAll(rows);
    }
}
```

### JAVA-ICB-348 case 2: Transformed element
settings: default
language: java
```before
import java.util.*;
class A {
    void test(Collection<Integer> roster) {
        List<Integer> entries = new ArrayList<>();
        for (Integer i : roster) {
            entries.<caret>add(i + 1);
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
    void usePrim(long v) {}
    void useBoxed(Long v) {}
    public void run() {
        <caret>Long total = 12L;
        useBoxed(total);
        for (int i = 0; i < 10; i++) {
            usePrim(total);
        }
    }
}
```
keys: Alt+Enter, choose "Convert wrapper type to primitive"
```after
class A {
    void usePrim(long v) {}
    void useBoxed(Long v) {}
    public void run() {
        long total = 12L;
        useBoxed(total);
        for (int i = 0; i < 10; i++) {
            usePrim(total);
        }
    }
}
```

### JAVA-ICB-349 case 2: Only used as an object
settings: default
language: java
```before
class A {
    void useBoxed(Long v) {}
    public void run() {
        <caret>Long total = 12L;
        useBoxed(total);
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
    String tag = <caret>"first\nsecond\rthird";
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
abstract class Citadel implements <caret>ButtonPeer {
}
```
keys: inspect
```after
problem: warning `ButtonPeer` "Use of AWT peer class ButtonPeer is non-portable"
```

### JAVA-ICB-357 case 1: Concrete driver class
settings: enable inspection UseOfJDBCDriverClass
status: unverified (needs the driver library on the classpath; the interface java.sql.Driver itself is the allowed form)
language: java
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
