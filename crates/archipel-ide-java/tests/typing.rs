//! Errors while typing (plan M2): the buffer between two keystrokes is syntactically broken, the
//! intact references must still be resolved and the missing ones marked, with IntelliJ's
//! messages; correct code is never marked. JAVA-ERR-002 (per-reference tolerance), JAVA-ERR-007,
//! JAVA-ERR-011, JAVA-ERR-012, JAVA-ERR-019, JAVA-RES-005, JAVA-RES-006, JAVA-RES-007, JAVA-RES-021.

mod common;
use archipel_ide_projectmodel::DependencyScope;
use common::*;

const MAIN: &str = "src/main/java/demo/Main.java";
const TEST: &str = "src/test/java/demo/CalcTest.java";

fn app(body: &str) -> String {
    format!("package demo;\n\npublic class App {{\n    void f() {{\n{body}\n    }}\n}}\n")
}

fn errs(p: &Proj, text: &str) -> Vec<(String, String)> {
    p.analyze(MAIN, text)
}

#[test]
fn owner_cases_while_typing() {
    let p = project(&[], &[]);
    for (body, want) in [
        ("        foo();", ("foo", "Cannot resolve method 'foo()'")),
        ("        foo()", ("foo", "Cannot resolve method 'foo()'")),
        ("        foo(", ("foo", "Cannot resolve method 'foo()'")),
        ("        Bar b;", ("Bar", "Cannot resolve symbol 'Bar'")),
        ("        Bar b", ("Bar", "Cannot resolve symbol 'Bar'")),
        (
            "        Object o = new Missing",
            ("Missing", "Cannot resolve symbol 'Missing'"),
        ),
        (
            "        String s = \"a\";\n        s.nope(",
            ("nope", "Cannot resolve method 'nope()'"),
        ),
    ] {
        let got = errs(&p, &app(body));
        assert_eq!(
            got,
            vec![(want.0.to_string(), want.1.to_string())],
            "for {body:?}"
        );
    }
}

#[test]
fn a_broken_line_does_not_hide_the_rest_of_the_file() {
    let p = project(&[], &[]);
    let text = "package demo;\n\npublic class App {\n    void a() {\n        Foo x = null;\n    }\n    void b() {\n        int y = \n    }\n    void c() {\n        bar();\n    }\n}\n";
    let got = errs(&p, text);
    assert!(
        got.contains(&("Foo".into(), "Cannot resolve symbol 'Foo'".into())),
        "{got:?}"
    );
    assert!(
        got.contains(&("bar".into(), "Cannot resolve method 'bar()'".into())),
        "{got:?}"
    );
}

#[test]
fn receivers_typed_through_generics_and_chains() {
    let p = project(&[], &[]);
    let text = app(
        "        java.util.List<String> l = new java.util.ArrayList<>();\n        l.get(0).nope();\n        l.nope2();\n        var s = new StringBuilder();\n        s.append(1).append(\"a\").nope3();\n        java.util.Map<String, java.util.List<Integer>> m = new java.util.HashMap<>();\n        m.get(\"a\").get(0).nope4();\n        m.get(\"a\").size();\n        l.get(0).length();",
    );
    let got: Vec<String> = errs(&p, &text).into_iter().map(|(r, _)| r).collect();
    assert_eq!(got, ["nope", "nope2", "nope3", "nope4"], "{got:?}");
}

#[test]
fn unresolved_annotations_and_supertypes_make_member_errors_uncertain() {
    let p = project(&[], &[]);
    for text in [
        "package demo;\nimport lombok.Getter;\n@Getter\npublic class App {\n    int run() { return getCount(); }\n}\n",
        "package demo;\n@Generated2\npublic class App {\n    int run() { return getCount(); }\n}\n",
        "package demo;\npublic class App extends Missing {\n    int run() { return getCount() + this.other(); }\n}\n",
    ] {
        let got = errs(&p, text);
        assert!(
            got.iter()
                .all(|(_, m)| !m.starts_with("Cannot resolve method")),
            "{text}\n{got:?}"
        );
    }
}

#[test]
fn source_retained_library_annotations_make_member_errors_uncertain() {
    // A library annotation that only an annotation processor can act on (retention SOURCE).
    let jar = fixture_jar("processor");
    let p = project(&[], &[("processor", jar, DependencyScope::Compile)]);
    let text = "package demo;\nimport proc.Generate;\n@Generate\npublic class App {\n    int run() { return getCount(); }\n}\n";
    assert_eq!(errs(&p, text), vec![]);
    // Without the annotation the same call is an error.
    let plain = "package demo;\npublic class App {\n    int run() { return getCount(); }\n}\n";
    assert_eq!(errs(&p, plain).len(), 1);
}

#[test]
fn junit_assertions_without_import_is_marked_in_a_test() {
    let p = project(
        &[],
        &[("junit5", fixture_jar("junit5"), DependencyScope::Test)],
    );
    let text = "package demo;\nimport org.junit.jupiter.api.Test;\npublic class CalcTest {\n    @Test\n    void t() {\n        Assertions.assertEquals(1, 1);\n    }\n}\n";
    assert_eq!(
        p.analyze(TEST, text),
        vec![(
            "Assertions".into(),
            "Cannot resolve symbol 'Assertions'".into()
        )]
    );
    // The same text with the import is clean.
    let fixed = text.replace(
        "import org.junit.jupiter.api.Test;",
        "import org.junit.jupiter.api.Assertions;\nimport org.junit.jupiter.api.Test;",
    );
    assert_eq!(p.analyze(TEST, &fixed), vec![]);
}

#[test]
fn correct_code_is_clean() {
    let p = project(&[], &[]);
    let text = r#"package demo;

import static java.util.stream.Collectors.toList;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.function.Function;
import java.util.stream.Stream;

public class App extends ArrayList<String> {
    enum Kind { A, B; Kind next() { return values()[(ordinal() + 1) % 2]; } }
    record Pair<A, B>(A first, B second) {}

    class Inner {
        int n = size();
        String head() { return get(0); }
    }

    static <T extends Comparable<T>> T max(List<T> xs) {
        T best = xs.get(0);
        for (T x : xs) {
            if (x.compareTo(best) > 0) {
                best = x;
            }
        }
        return best;
    }

    Map<String, List<Integer>> run(Function<String, Integer> f) {
        Map<String, List<Integer>> out = new HashMap<>();
        for (String s : this) {
            out.computeIfAbsent(s, k -> new ArrayList<>()).add(f.apply(s));
        }
        Optional<String> first = stream().findFirst();
        String shown = first.map(String::trim).orElse("none");
        List<String> upper = stream().map(String::toUpperCase).collect(toList());
        Pair<String, Integer> pair = new Pair<>(shown, upper.size());
        pair.first().length();
        Kind.A.next().name().length();
        Stream.of(1, 2, 3).map(i -> i + 1).forEach(System.out::println);
        Inner in = new Inner();
        in.head().isEmpty();
        int[] arr = {1, 2};
        arr.clone();
        int len = arr.length;
        Object o = this;
        if (o instanceof App a && a.size() > len) {
            a.add("x");
        }
        return out;
    }
}
"#;
    assert_eq!(errs(&p, text), vec![]);
}

#[test]
fn members_of_a_primitive_are_an_error() {
    // JAVA-ERR-019
    let p = project(&[], &[]);
    let text = app(
        "        int n = 1;\n        n.foo();\n        long m = n + 2L;\n        m.bar();\n        Integer boxed = 1;\n        boxed.toString();",
    );
    assert_eq!(
        errs(&p, &text),
        vec![
            (
                "foo".to_string(),
                "Cannot call methods on 'int' type".to_string()
            ),
            (
                "bar".to_string(),
                "Cannot call methods on 'long' type".to_string()
            ),
        ]
    );
}

#[test]
fn try_with_resources_variables_are_in_scope_of_later_resources() {
    let p = project(&[], &[]);
    let text = app(
        "        try (java.io.StringReader a = new java.io.StringReader(\"x\");\n                java.io.BufferedReader b = new java.io.BufferedReader(a)) {\n            b.read();\n        } catch (java.io.IOException e) {\n            e.printStackTrace();\n        }",
    );
    assert_eq!(errs(&p, &text), vec![]);
}

#[test]
fn a_declarator_is_in_scope_of_the_later_declarators_of_its_declaration() {
    let p = project(&[], &[]);
    let text = app(
        "        int[] a = {1, 2}, b = new int[a.length];\n        int n = 1, m = n + 1;\n        b[0] = m;",
    );
    assert_eq!(errs(&p, &text), vec![]);
}

#[test]
fn a_declaration_of_an_earlier_switch_group_is_in_scope_of_the_later_groups() {
    let p = project(&[], &[]);
    let text = app(
        "        int k = 1;\n        switch (k) {\n            case 1:\n                String s = \"a\";\n                s.length();\n                break;\n            case 2:\n                s = \"b\";\n                s.length();\n                break;\n            default:\n                break;\n        }",
    );
    assert_eq!(errs(&p, &text), vec![]);
}
