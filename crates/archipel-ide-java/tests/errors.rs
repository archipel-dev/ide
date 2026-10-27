//! Native unresolved-reference errors: JAVA-ERR-001/002/003/007/011/012/144, JAVA-IMP-001/002/004,
//! JAVA-RES-004..008, 011, 018..023.

mod common;
use archipel_ide_projectmodel::DependencyScope;
use common::*;

const MAIN: &str = "src/main/java/demo/Main.java";
const TEST: &str = "src/test/java/demo/CalcTest.java";

fn no_errors(p: &Proj, rel: &str, text: &str) {
    let errs = p.analyze(rel, text);
    assert!(errs.is_empty(), "unexpected errors in\n{text}\n{errs:?}");
}

#[test]
fn headline_assertthat_without_import_is_one_error_on_the_name() {
    // JAVA-IMP-001
    let Some(aj) = assertj() else { return };
    let p = project(&[], &[("assertj", aj, DependencyScope::Test)]);
    let text = r#"package demo;

import org.junit.Test;

public class CalcTest {
    @Test
    public void adds() {
        String x = "a";
        assertThat(x).isEqualTo("a");
    }
}
"#;
    let errs = p.analyze(TEST, text);
    // `org.junit.Test` is not on this classpath: the import is its own error, then the call.
    let calls: Vec<_> = errs
        .iter()
        .filter(|(_, m)| m.starts_with("Cannot resolve method"))
        .collect();
    assert_eq!(
        calls,
        vec![&(
            "assertThat".to_string(),
            "Cannot resolve method 'assertThat(String)'".to_string()
        )]
    );
}

#[test]
fn argument_types_in_the_message() {
    // JAVA-IMP-002
    let p = project(&[], &[]);
    let text = r#"package demo;
public class Main {
    void f(int n, java.util.List<Integer> l) {
        foo();
        foo(1, "a", unknownVar);
        bar(n, l, 'c', 2L, 1.5, true, null, new StringBuilder(), n + 1, "a" + n);
    }
}
"#;
    let errs = p.analyze(MAIN, text);
    let msgs: Vec<_> = errs.iter().map(|(_, m)| m.as_str()).collect();
    assert!(msgs.contains(&"Cannot resolve method 'foo()'"), "{msgs:?}");
    assert!(
        msgs.contains(&"Cannot resolve method 'foo(int, String, ?)'"),
        "{msgs:?}"
    );
    assert!(
        msgs.contains(&"Cannot resolve symbol 'unknownVar'"),
        "{msgs:?}"
    );
    assert!(
        msgs.contains(&"Cannot resolve method 'bar(int, List<Integer>, char, long, double, boolean, null, StringBuilder, int, String)'"),
        "{msgs:?}"
    );
}

#[test]
fn class_symbols_and_ranges() {
    // JAVA-ERR-012, JAVA-RES-005
    let p = project(&[], &[]);
    let text = r#"package demo;
import java.util.List;
import java.utl.Missing;
import zork.nothere.Foo;
public class Main {
    Strng s;
    List<Nope> l;
    void f() {
        Collections.emptyList();
        java.util.Lisst x;
        Object o = new Unknown(1);
        int y = zzz + 1;
    }
}
"#;
    let errs = p.analyze(MAIN, text);
    let want = [
        ("utl", "Cannot resolve symbol 'utl'"),
        ("zork", "Cannot resolve symbol 'zork'"),
        ("Strng", "Cannot resolve symbol 'Strng'"),
        ("Nope", "Cannot resolve symbol 'Nope'"),
        ("Collections", "Cannot resolve symbol 'Collections'"),
        ("Lisst", "Cannot resolve symbol 'Lisst'"),
        ("Unknown", "Cannot resolve symbol 'Unknown'"),
        ("zzz", "Cannot resolve symbol 'zzz'"),
    ];
    for (r, m) in want {
        assert!(
            errs.iter().any(|(rr, mm)| rr == r && mm == m),
            "missing {r}: {errs:?}"
        );
    }
    assert_eq!(errs.len(), want.len(), "{errs:?}");
}

#[test]
fn valid_code_is_clean() {
    let p = project(
        &[(
            "src/main/java/demo/Other.java",
            "package demo; public class Other { public static int K = 1; public static void s() {} public int inst; }",
        )],
        &[],
    );
    no_errors(
        &p,
        MAIN,
        r#"package demo;

import java.util.*;
import java.util.function.Function;
import static java.lang.Math.max;
import static java.util.Collections.*;
import static java.util.Map.Entry;

public class Main<T extends Comparable<T>> extends Other implements Runnable {
    private final List<String> names = new ArrayList<>();
    static int counter;
    enum Color { RED, GREEN; Color next() { return values()[(ordinal() + 1) % values().length]; } }
    record Pt(int x, int y) { int sum() { return x + y; } }
    interface Shape { double area(); default String name() { return "s"; } }
    class Inner { int v = counter; void g() { run(); names.size(); } }

    public void run() {
        int a = max(1, 2);
        List<String> e = emptyList();
        for (String n : names) { System.out.println(n.length()); }
        for (int i = 0; i < 3; i++) { a += i; }
        try (var sc = new Scanner(System.in)) { sc.nextLine(); } catch (RuntimeException | Error ex) { ex.printStackTrace(); }
        Function<Integer, Integer> f = x -> x + a2();
        Runnable r = () -> System.out.println(K + counter + inst);
        Object o = "x";
        if (o instanceof String s && s.isEmpty()) { s.length(); }
        switch (o) { case Integer n when n > 1 -> n++; case String t -> t.length(); default -> {} }
        Color c = Color.RED; c.next();
        Pt p = new Pt(1, 2); p.x(); p.sum();
        Shape sh = new Shape() { public double area() { return 1; } };
        sh.name(); Other.s(); Math.abs(-1); String.valueOf(1).length();
        var list = new ArrayList<Map.Entry<String, Integer>>();
        list.forEach(en -> en.getKey().length());
        int[] arr = {1, 2}; arr.clone(); int len = arr.length;
        label: for (;;) { break label; }
        T t = null; t.compareTo(t); this.names.add("a"); Main.this.run();
        new Thread(this).start(); Entry<String, Integer> ee = null;
        java.util.Optional.of(1).ifPresent(System.out::println);
        Object[] objs = new Object[2]; objs.getClass();
    }
    int a2() { return 1; }
    static <U> U id(U u) { return u; }
}
"#,
    );
}

#[test]
fn inherited_members_and_hiding() {
    // JAVA-RES-007, JAVA-RES-011
    let p = project(
        &[
            (
                "src/main/java/demo/Base.java",
                "package demo; public class Base { protected int bf; public void bm() {} public static class BN {} }",
            ),
            (
                "src/main/java/demo/Sub.java",
                "package demo; public class Sub extends Base { void t() { bf++; bm(); BN n; super.bm(); } }",
            ),
        ],
        &[],
    );
    no_errors(
        &p,
        "src/main/java/demo/Sub.java",
        "package demo; public class Sub extends Base { void t() { bf++; bm(); BN n; super.bm(); } }",
    );
    // The innermost class level with a candidate decides; an inapplicable one does not fall through.
    let errs = p.analyze(MAIN, "package demo; public class Main { void f(int a) {} class I { void g() { nope(1); this.zork(); } } }");
    assert!(
        errs.iter()
            .any(|(r, m)| r == "nope" && m == "Cannot resolve method 'nope(int)'"),
        "{errs:?}"
    );
}

#[test]
fn unresolved_supertypes_suppress_member_errors() {
    // Spec 04 4.1.3: nothing is reported when a supertype could supply the name.
    let p = project(&[], &[]);
    let errs = p.analyze(
        MAIN,
        "package demo; public class Main extends Missing { void f() { inherited(); x++; } }",
    );
    assert_eq!(
        errs,
        vec![(
            "Missing".to_string(),
            "Cannot resolve symbol 'Missing'".to_string()
        )]
    );
}

#[test]
fn lombok_style_generated_members_are_not_reported() {
    let p = project(&[], &[]);
    let errs = p.analyze(MAIN, "package demo; @lombok.Data public class Main { String name; void f() { setName(\"x\"); getName(); } }");
    assert!(
        errs.iter().all(|(r, _)| r != "setName" && r != "getName"),
        "{errs:?}"
    );
}

#[test]
fn qualified_calls_on_known_types() {
    // JAVA-ERR-007, 011
    let p = project(&[], &[]);
    let errs = p.analyze(
        MAIN,
        r#"package demo;
public class Main {
    void f(String s) {
        s.lenght();
        Math.foo(1);
        s.length();
        Integer.parseInt("1");
    }
}
"#,
    );
    assert!(
        errs.iter()
            .any(|(r, m)| r == "lenght" && m == "Cannot resolve method 'lenght()'"),
        "{errs:?}"
    );
    assert!(
        errs.iter()
            .any(|(r, m)| r == "foo" && m == "Cannot resolve method 'foo(int)' in 'Math'"),
        "{errs:?}"
    );
    assert_eq!(errs.len(), 2, "{errs:?}");
}

#[test]
fn production_file_does_not_see_test_libraries() {
    // JAVA-RES-029
    let Some(aj) = assertj() else { return };
    let p = project(&[], &[("assertj", aj, DependencyScope::Test)]);
    let src = "package demo; import org.assertj.core.api.Assertions; public class Main { void f() { Assertions.assertThat(1); } }";
    let prod = p.analyze(MAIN, src);
    assert!(
        prod.iter()
            .any(|(r, m)| r == "assertj" && m == "Cannot resolve symbol 'assertj'"),
        "{prod:?}"
    );
    let test = p.analyze(TEST, src.replace("Main", "CalcTest").as_str());
    assert!(test.is_empty(), "{test:?}");
}

#[test]
fn name_resolution_order() {
    // JAVA-RES-018, 019, 020
    let p = project(
        &[(
            "src/main/java/demo/List.java",
            "package demo; public class List { }",
        )],
        &[],
    );
    // The package beats an on-demand import.
    no_errors(
        &p,
        MAIN,
        "package demo; import java.util.*; public class Main { List l = new List(); }",
    );
    // Two on-demand imports supplying the same name are ambiguous: no resolution, no second error.
    let errs = p.analyze(MAIN, "package demo; import java.util.*; import java.awt.*; public class Main2 { void f() { Object o = null; } }");
    assert!(errs.is_empty(), "{errs:?}");
}

#[test]
fn while_the_project_loads_unresolved_references_are_pending() {
    // JAVA-ERR-003, JAVA-STUB-026
    let mut p = project(&[], &[]);
    p.project.model_loading = true;
    let a = p.project.analyze_text(
        &p.path(MAIN),
        "package demo; public class Main { void f() { Foo x; } }",
    );
    assert_eq!(a.problems.len(), 1);
    assert_eq!(
        a.problems[0].message,
        "Not resolved until the project is fully loaded"
    );
}
