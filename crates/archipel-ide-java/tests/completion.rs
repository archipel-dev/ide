//! Engine-layer tests of native completion beyond the class names: member access after a dot,
//! reference variants, keywords by position, statics on the second press, the weigher chain.
//! JAVA-CMP-001, 003, 008, 011, 016, 017, 022, 027, 028, 035.

mod common;
use archipel_ide_java::complete::{self, Completion, CompletionInput, Item};
use archipel_ide_java::import_edit::ImportSettings;
use common::*;

const MAIN: &str = "src/main/java/p/Main.java";

/// Completes in `src` at `|` (the caret), count `count`.
fn at(p: &Proj, src: &str, count: u32) -> Option<Completion> {
    let offset = src.find('|').expect("caret");
    let text = src.replacen('|', "", 1);
    let settings = ImportSettings::default();
    let path = p.path(MAIN);
    let input = CompletionInput {
        project: &p.project,
        settings: &settings,
        path: &path,
        text: &text,
        offset,
        count,
    };
    complete::complete(&input).expect("indexes are ready")
}

fn names(c: &Option<Completion>) -> Vec<String> {
    c.as_ref()
        .map(|c| c.items.iter().map(|i| i.name.clone()).collect())
        .unwrap_or_default()
}

fn item<'a>(c: &'a Option<Completion>, name: &str) -> &'a Item {
    c.as_ref()
        .and_then(|c| c.items.iter().find(|i| i.name == name))
        .unwrap_or_else(|| panic!("`{name}` is not offered: {:?}", names(c)))
}

fn project_with(text: &str) -> Proj {
    let text = text.replacen('|', "", 1);
    project(&[(MAIN, &text)], &[])
}

fn wrap(body: &str) -> String {
    format!(
        "package p;\n\nimport java.util.*;\n\nclass Main {{\n  void run() {{\n{body}\n  }}\n}}\n"
    )
}

#[test]
fn list_dot_lists_the_instance_members_of_its_static_type() {
    // JAVA-CMP-008: `list.|` with `List<String> list`: `add`, `get`, `size`, `stream`, `forEach`
    // (inherited from `Iterable`) and the members of `Object`, no statics of other classes.
    let src = wrap("    List<String> names = new ArrayList<>();\n    names.|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    let got = names(&c);
    for n in [
        "add", "get", "size", "stream", "forEach", "isEmpty", "toString", "hashCode",
    ] {
        assert!(got.contains(&n.to_string()), "{n} missing: {got:?}");
    }
    assert!(!got.contains(&"emptyList".to_string()), "{got:?}");
    // Static interface methods are not inherited by instances (JLS 8.4.8).
    assert!(!got.contains(&"of".to_string()), "{got:?}");
    // Overloads are one row (JAVA-CMP-016, 035).
    assert_eq!(got.iter().filter(|n| *n == "add").count(), 1, "{got:?}");
    assert_eq!(item(&c, "size").ty, "int");
    assert_eq!(item(&c, "size").signature, "()");
}

#[test]
fn the_return_type_is_seen_through_the_receivers_type_arguments() {
    // JAVA-TYP-013: `List<String>.get(0)` is a `String`.
    let src = wrap("    List<String> names = new ArrayList<>();\n    names.get(0).|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    for n in ["length", "charAt", "substring", "isBlank", "trim"] {
        assert!(names(&c).contains(&n.to_string()), "{n}: {:?}", names(&c));
    }
    assert_eq!(item(&c, "length").ty, "int");
    // `E get(int)` shown with the receiver's argument.
    let src = wrap("    List<String> names = new ArrayList<>();\n    names.|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    assert_eq!(item(&c, "get").ty, "String");
}

#[test]
fn chained_calls_and_this_and_super() {
    let src = "package p;\n\nclass Base {\n  protected int shared;\n  void greet() {}\n}\n\nclass Main extends Base {\n  String name = \"n\";\n  Main self() { return this; }\n  void run() {\n    self().self().|\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    let got = names(&c);
    for n in ["self", "run", "name", "greet", "shared", "toString"] {
        assert!(got.contains(&n.to_string()), "{n}: {got:?}");
    }
    let src2 = src.replace("self().self().|", "this.|");
    let c = at(&p, &src2, 1);
    assert!(names(&c).contains(&"greet".to_string()));
    let src3 = src.replace("self().self().|", "super.|");
    let c = at(&p, &src3, 1);
    let got = names(&c);
    assert!(got.contains(&"greet".to_string()), "{got:?}");
    assert!(got.contains(&"shared".to_string()), "{got:?}");
    // `Main` declares `self`; `super` shows the superclass only.
    assert!(!got.contains(&"self".to_string()), "{got:?}");
}

#[test]
fn a_type_qualifier_lists_statics_and_member_types() {
    // JAVA-CMP-008: `Collections.|` only static members and member types; `Map.|` member types.
    let src = wrap("    Collections.|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    let got = names(&c);
    assert!(got.contains(&"emptyList".to_string()), "{got:?}");
    assert!(got.contains(&"sort".to_string()), "{got:?}");
    assert!(!got.contains(&"toString".to_string()), "{got:?}");
    assert!(got.contains(&"class".to_string()));
    let src = wrap("    Map.|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    assert!(names(&c).contains(&"Entry".to_string()), "{:?}", names(&c));
    assert!(names(&c).contains(&"of".to_string()));
    assert!(!names(&c).contains(&"size".to_string()));
}

#[test]
fn deprecated_members_are_struck_through() {
    // JAVA-CMP-017
    let src = wrap("    Date d = new Date();\n    d.|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    assert!(item(&c, "getYear").deprecated);
    assert!(!item(&c, "getTime").deprecated);
}

#[test]
fn inaccessible_members_are_hidden_then_offered_on_the_second_press() {
    // JAVA-CMP-008: private members of another class.
    let src = "package p;\n\nclass Other {\n  private int secret;\n  int visible;\n  private void hidden() {}\n  public void open() {}\n}\n\nclass Main {\n  void run() {\n    Other o = new Other();\n    o.|\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    // Same top-level class would see private; Other is a different top-level class.
    assert!(
        !names(&c).contains(&"secret".to_string()),
        "{:?}",
        names(&c)
    );
    assert!(!names(&c).contains(&"hidden".to_string()));
    assert!(names(&c).contains(&"visible".to_string()));
    assert!(names(&c).contains(&"open".to_string()));
    let c = at(&p, src, 2);
    assert!(names(&c).contains(&"secret".to_string()));
    assert_eq!(item(&c, "secret").note, "private");
    // Ranked after the reachable ones.
    let all = names(&c);
    let pos = |n: &str| all.iter().position(|x| x == n).unwrap();
    assert!(pos("open") < pos("hidden"), "{all:?}");
}

#[test]
fn an_unknown_receiver_offers_nothing_native() {
    // No false positives: the language server may know what we do not.
    for body in [
        "    Mystery m = null;\n    m.|",
        "    var x = unknownCall();\n    x.|",
        "    nothing.|",
        "    foo().|",
    ] {
        let src = wrap(body);
        let p = project_with(&src);
        let c = at(&p, &src, 1);
        assert!(c.is_none(), "{body}: {:?}", names(&c));
    }
}

#[test]
fn nothing_after_a_number_or_a_primitive() {
    // JAVA-CMP-003
    let src = wrap("    int n = 1.|");
    let p = project_with(&src);
    assert!(at(&p, &src, 1).is_none());
    let src = wrap("    int n = 3;\n    n.|");
    let p = project_with(&src);
    let c = at(&p, &src, 1).expect("answers: nothing");
    assert!(c.items.is_empty() && c.exclusive);
}

#[test]
fn var_literals_new_expressions_and_arrays_have_types() {
    for (body, expect) in [
        ("    var s = \"abc\";\n    s.|", "substring"),
        ("    var l = new ArrayList<String>();\n    l.|", "add"),
        ("    \"x\".|", "charAt"),
        ("    new StringBuilder().|", "append"),
        ("    String[] arr = null;\n    arr.|", "length"),
        ("    int[] arr = null;\n    arr.|", "clone"),
        (
            "    for (String s : List.of(\"a\")) {\n      s.|\n    }",
            "isEmpty",
        ),
        (
            "    Object o = null;\n    if (o instanceof String s) {\n      s.|\n    }",
            "length",
        ),
    ] {
        let src = wrap(body);
        let p = project_with(&src);
        let c = at(&p, &src, 1);
        assert!(
            names(&c).contains(&expect.to_string()),
            "{body}: {:?}",
            names(&c)
        );
    }
}

#[test]
fn generic_receivers_through_type_variables_and_fields() {
    let src = "package p;\n\nimport java.util.*;\n\nclass Box<T extends Comparable<T>> {\n  List<T> items = new ArrayList<>();\n  T first;\n  void run() {\n    first.|\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    assert!(
        names(&c).contains(&"compareTo".to_string()),
        "{:?}",
        names(&c)
    );
    let src2 = src.replace("first.|", "this.items.|");
    let c = at(&p, &src2, 1);
    assert!(names(&c).contains(&"add".to_string()));
    let src3 = src.replace("first.|", "items.get(0).|");
    let c = at(&p, &src3, 1);
    assert!(
        names(&c).contains(&"compareTo".to_string()),
        "{:?}",
        names(&c)
    );
}

#[test]
fn enum_and_record_synthetic_members() {
    let src = "package p;\n\nenum Color { RED, GREEN; int mix() { return 1; } }\nrecord Pt(int x, int y) {}\n\nclass Main {\n  void run() {\n    Color.|\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    for n in ["RED", "GREEN", "values", "valueOf"] {
        assert!(names(&c).contains(&n.to_string()), "{n}: {:?}", names(&c));
    }
    assert!(!names(&c).contains(&"mix".to_string()));
    let src2 = src.replace("Color.|", "Pt pt = null;\n    pt.|");
    let c = at(&p, &src2, 1);
    assert!(names(&c).contains(&"x".to_string()), "{:?}", names(&c));
    assert!(names(&c).contains(&"y".to_string()));
}

#[test]
fn a_package_qualifier_lists_classes_and_subpackages() {
    let src = wrap("    java.util.|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    let got = names(&c);
    assert!(got.contains(&"ArrayList".to_string()), "{got:?}");
    assert!(got.contains(&"concurrent".to_string()), "{got:?}");
    assert!(!got.contains(&"sun".to_string()));
}

#[test]
fn choosing_a_method_inserts_the_call_with_the_caret_inside() {
    // JAVA-CMP-035
    let src = wrap("    List<String> names = new ArrayList<>();\n    names.si|");
    let p = project_with(&src);
    let offset = src.find('|').unwrap();
    let text = src.replacen('|', "", 1);
    let settings = ImportSettings::default();
    let path = p.path(MAIN);
    let input = CompletionInput {
        project: &p.project,
        settings: &settings,
        path: &path,
        text: &text,
        offset,
        count: 1,
    };
    let c = complete::complete(&input).unwrap().unwrap();
    let size = c.items.iter().find(|i| i.name == "size").unwrap();
    let ins = complete::insertion(&input, &size.key).unwrap().unwrap();
    assert_eq!(ins.text, "size()");
    assert!(!ins.snippet);
    let add_src = wrap("    List<String> names = new ArrayList<>();\n    names.ad|");
    let add_text = add_src.replacen('|', "", 1);
    let add_input = CompletionInput {
        text: &add_text,
        offset: add_src.find('|').unwrap(),
        ..input
    };
    let c = complete::complete(&add_input).unwrap().unwrap();
    let add = c.items.iter().find(|i| i.name == "add").unwrap();
    let ins = complete::insertion(&add_input, &add.key).unwrap().unwrap();
    assert_eq!(ins.text, "add($0)");
    assert!(ins.snippet);
}

fn rows(c: &Option<Completion>) -> Vec<(String, String)> {
    c.as_ref()
        .map(|c| {
            c.items
                .iter()
                .map(|i| (i.name.clone(), i.sort.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn pos(c: &Option<Completion>, name: &str) -> usize {
    names(c)
        .iter()
        .position(|n| n == name)
        .unwrap_or_else(|| panic!("`{name}` is not offered: {:?}", names(c)))
}

#[test]
fn variants_come_from_the_scope_walk() {
    // JAVA-CMP-008 position 1, JAVA-RES-001: locals, parameters, fields (own and inherited),
    // methods of the enclosing classes, outer class members from an inner class.
    let src = "package p;\n\nclass Base {\n  protected int inherited;\n}\n\nclass Main extends Base {\n  String title = \"t\";\n  String helper() { return title; }\n  class Inner {\n    int deep;\n    void go(int arg) {\n      long local = 1;\n      |\n    }\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    for n in [
        "local",
        "arg",
        "deep",
        "title",
        "helper",
        "inherited",
        "go",
        "toString",
    ] {
        assert!(names(&c).contains(&n.to_string()), "{n}: {:?}", names(&c));
    }
    assert_eq!(item(&c, "local").ty, "long");
    assert_eq!(item(&c, "arg").ty, "int");
    // Nearer scopes first: the local, the parameter, the member of Inner, the outer ones.
    assert!(pos(&c, "local") < pos(&c, "deep"));
    assert!(pos(&c, "deep") < pos(&c, "title"));
    // Members of Object come last of the methods.
    assert!(pos(&c, "go") < pos(&c, "toString"));
}

#[test]
fn a_local_hides_the_field_of_the_same_name_and_inner_methods_hide_outer_ones() {
    let src = "package p;\n\nclass Main {\n  int value;\n  void f() {}\n  class Inner {\n    void f(int x) {}\n    void run() {\n      String value = \"\";\n      val|\n      f|\n    }\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    let values: Vec<_> = c
        .as_ref()
        .unwrap()
        .items
        .iter()
        .filter(|i| i.name == "value")
        .collect();
    assert_eq!(values.len(), 1, "{:?}", names(&c));
    assert_eq!(values[0].ty, "String");
}

#[test]
fn statements_and_conditions_know_the_context() {
    // The keywords of the statement context (JAVA-CMP-011) and only where legal.
    let src = wrap("    |");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    for n in [
        "if", "for", "while", "try", "return", "int", "this", "new", "final", "var",
    ] {
        assert!(names(&c).contains(&n.to_string()), "{n}: {:?}", names(&c));
    }
    for n in ["break", "continue", "else", "case", "yield", "catch"] {
        assert!(!names(&c).contains(&n.to_string()), "{n}: {:?}", names(&c));
    }
    let src = wrap("    for (int i = 0; i < 3; i++) {\n      |\n    }");
    let c = at(&p, &src, 1);
    assert!(names(&c).contains(&"break".to_string()));
    assert!(names(&c).contains(&"continue".to_string()));
    let src = wrap("    switch (1) {\n      case 1:\n        break;\n      |\n    }");
    let c = at(&p, &src, 1);
    assert!(names(&c).contains(&"case".to_string()), "{:?}", names(&c));
    assert!(names(&c).contains(&"default".to_string()));
    assert!(names(&c).contains(&"break".to_string()));
    let src = wrap("    try {\n    } finally {\n    }\n    |");
    let c = at(&p, &src, 1);
    assert!(!names(&c).contains(&"catch".to_string()));
    let src = wrap("    try {\n    }\n    |");
    let c = at(&p, &src, 1);
    assert!(names(&c).contains(&"catch".to_string()));
    assert!(names(&c).contains(&"finally".to_string()));
}

#[test]
fn modifiers_do_not_conflict_with_those_written() {
    let src = "package p;\n\nclass Main {\n  abstract |\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    for n in ["public", "protected"] {
        assert!(names(&c).contains(&n.to_string()), "{n}");
    }
    for n in ["final", "static", "native", "abstract", "private"] {
        assert!(!names(&c).contains(&n.to_string()), "{n}: {:?}", names(&c));
    }
    let src = "package p;\n\ninterface Main {\n  |\n}\n";
    let c = at(&p, src, 1);
    assert!(names(&c).contains(&"default".to_string()));
    assert!(!names(&c).contains(&"protected".to_string()));
    let src = "package p;\n\nclass Main {\n  void run() {}\n  |\n}\n";
    let c = at(&p, src, 1);
    assert!(!names(&c).contains(&"default".to_string()));
}

#[test]
fn top_level_keywords() {
    let src = "|";
    let p = project_with(src);
    let c = at(&p, src, 1);
    assert!(
        names(&c).contains(&"package".to_string()),
        "{:?}",
        names(&c)
    );
    let src = "package p;\n\n|";
    let c = at(&p, src, 1);
    assert!(names(&c).contains(&"import".to_string()));
    assert!(names(&c).contains(&"class".to_string()));
    assert!(!names(&c).contains(&"package".to_string()));
    let src = "package p;\n\nclass A {}\n\n|";
    let c = at(&p, src, 1);
    assert!(!names(&c).contains(&"import".to_string()));
}

#[test]
fn expected_type_ranks_variables_then_classes() {
    // JAVA-CMP-027/028: in `String s = |` the String variables lead, then the other variables,
    // then methods, then classes; after `new`, the inheritors of the expected type.
    let src = wrap("    String name = \"\";\n    int n = 1;\n    String s = |");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    assert!(pos(&c, "name") < pos(&c, "n"), "{:?}", rows(&c));
    let src = wrap("    List<String> l = new Arr|");
    let c = at(&p, &src, 1);
    assert!(
        pos(&c, "ArrayList") < pos(&c, "ArrayStoreException"),
        "{:?}",
        names(&c)
    );
    // Without an expected type the classes the file already sees come first.
    let src = "package p;\n\nclass Main {\n  void run() {\n    Object l = new Arr|\n  }\n}\n";
    let c = at(&p, src, 1);
    assert!(
        pos(&c, "ArrayStoreException") < pos(&c, "ArrayList"),
        "{:?}",
        names(&c)
    );
    // An argument of a call with one candidate: its parameter type.
    let src = wrap("    String text = \"\";\n    int count = 0;\n    \"a\".repeat(|");
    let c = at(&p, &src, 1);
    assert!(pos(&c, "count") < pos(&c, "text"), "{:?}", names(&c));
    // A return value.
    let src = "package p;\n\nclass Main {\n  int size() {\n    String s = \"\";\n    int n = 0;\n    return |;\n  }\n}\n";
    let c = at(&p, src, 1);
    assert!(pos(&c, "n") < pos(&c, "s"), "{:?}", names(&c));
}

#[test]
fn nothing_wrong_is_offered_for_method_references_and_declarators() {
    // `Type::|` is not a variable position; names of declarators are not operands.
    for body in ["    Object o = Main::|", "    int a = 1, b|"] {
        let src = wrap(body);
        let p = project_with(&src);
        let c = at(&p, &src, 1);
        assert!(c.is_none(), "{body}: {:?}", names(&c));
    }
}

#[test]
fn an_inherited_overload_does_not_decide_the_type() {
    // The nearer class declares `f(int)`; the call passes a String, so the inherited
    // `Object f(String)` is the one that applies: the type is `Object`, never `String`.
    let src = "package p;\n\nclass A {\n  Object f(String s) { return s; }\n}\n\nclass B extends A {\n  String f(int i) { return \"\"; }\n}\n\nclass Main {\n  void run(B b) {\n    b.f(\"x\").|\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    assert!(
        !names(&c).contains(&"length".to_string()),
        "{:?}",
        names(&c)
    );
    assert!(names(&c).contains(&"hashCode".to_string()));
}

#[test]
fn the_second_press_offers_static_members_of_other_classes() {
    // JAVA-CMP-022: not on the first press; `Math.max` once, with the package as its tail.
    let src = wrap("    ma|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    assert!(
        c.as_ref()
            .is_none_or(|c| !c.items.iter().any(|i| i.is_class() || i.display.is_some()))
    );
    let c = at(&p, &src, 2);
    let max: Vec<_> = c
        .as_ref()
        .unwrap()
        .items
        .iter()
        .filter(|i| i.display.as_deref() == Some("Math.max"))
        .collect();
    assert_eq!(max.len(), 1, "{:?}", names(&c));
    assert_eq!(max[0].tail, "java.lang");
    // Choosing it: `Math.max(` and no import for java.lang.
    let offset = src.find('|').unwrap();
    let text = src.replacen('|', "", 1);
    let settings = ImportSettings::default();
    let path = p.path(MAIN);
    let input = CompletionInput {
        project: &p.project,
        settings: &settings,
        path: &path,
        text: &text,
        offset,
        count: 2,
    };
    let ins = complete::insertion(&input, &max[0].key).unwrap().unwrap();
    assert_eq!(ins.text, "Math.max($0)");
    assert!(ins.edits.is_empty());
    // A class outside java.lang is imported.
    let src = "package p;\n\nclass Main {\n  void run() {\n    emptyL|\n  }\n}\n";
    let src = src.to_string();
    let c = at(&p, &src, 2);
    let row = item(&c, "emptyList");
    let text = src.replacen('|', "", 1);
    let input = CompletionInput {
        text: &text,
        offset: src.find('|').unwrap(),
        ..input
    };
    let ins = complete::insertion(&input, &row.key).unwrap().unwrap();
    assert_eq!(ins.text, "Collections.emptyList()");
    assert!(!ins.edits.is_empty());
}

#[test]
fn no_panics_anywhere_in_a_file() {
    let src = "package p;\n\nimport java.util.*;\nimport static java.lang.Math.*;\n\n@Deprecated\npublic class Main<T extends Number> extends Thread implements Runnable {\n  private final List<T> items = new ArrayList<>();\n  enum Mode { A, B; int x() { return 1; } }\n  record Pt(int x, int y) {}\n  @Override public void run() {\n    var local = items.stream().map(i -> i.intValue()).toList();\n    for (T t : items) {\n      if (t instanceof Integer n && n > 0) { System.out.println(n); }\n    }\n    switch (Mode.A) { case A -> items.clear(); default -> {} }\n    Runnable r = () -> { items.size(); };\n    String s = \"a\" + 1.5 + 'c';\n    int[] arr = new int[]{1, 2};\n    arr[0] = (int) 2L;\n    new Thread(r).start();\n  }\n}\n";
    let p = project_with(src);
    let settings = ImportSettings::default();
    let path = p.path(MAIN);
    let mut offsets: Vec<usize> = (0..=src.len())
        .filter(|i| src.is_char_boundary(*i))
        .collect();
    offsets.retain(|i| i % 7 == 0);
    for count in [1, 2] {
        for &offset in &offsets {
            let input = CompletionInput {
                project: &p.project,
                settings: &settings,
                path: &path,
                text: src,
                offset,
                count,
            };
            let _ = complete::complete(&input);
        }
    }
}

#[test]
fn qualified_super_explicit_type_arguments_and_anonymous_classes() {
    // `Greeter.super.|` lists the interface's instance members (its default methods).
    let src = "package p;\n\ninterface Greeter {\n  default void greet() {}\n  static void make() {}\n}\n\nclass Main implements Greeter {\n  public void greet() {\n    Greeter.super.|\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    assert!(names(&c).contains(&"greet".to_string()), "{:?}", names(&c));
    assert!(!names(&c).contains(&"make".to_string()), "{:?}", names(&c));
    // `Collections.<String>|`: explicit type arguments keep it a member access.
    let src = wrap("    Collections.<String>empty|");
    let p = project_with(&src);
    let c = at(&p, &src, 1);
    assert!(
        names(&c).contains(&"emptyList".to_string()),
        "{:?}",
        names(&c)
    );
    // An anonymous class declares members the named type has not: no list is derived from it.
    let src = wrap("    new Object() { int extra; }.|");
    let p = project_with(&src);
    assert!(at(&p, &src, 1).is_none());
}

#[test]
fn a_missing_library_is_noted_or_offers_nothing() {
    // A supertype that does not resolve: the members that are known are offered with a note.
    let src = "package p;\n\nclass Sub extends Missing {\n  void own() {}\n}\n\nclass Main {\n  void run(Sub s) {\n    s.|\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    assert!(names(&c).contains(&"own".to_string()), "{:?}", names(&c));
    assert!(
        c.as_ref()
            .and_then(|c| c.note.as_deref())
            .is_some_and(|n| n.contains("supertypes")),
        "{:?}",
        c.as_ref().map(|c| &c.note)
    );
    // A type variable whose declared bound does not resolve: nothing is known about it.
    let src = "package p;\n\nclass Box<T extends Missing> {\n  T value;\n  void f() {\n    value.|\n  }\n}\n";
    let p = project_with(src);
    assert!(at(&p, src, 1).is_none());
}

#[test]
fn after_new_only_classes_with_an_accessible_constructor() {
    // Spec 06 6.3 position 3: `Arrays` and `java.lang.reflect.Array` have only a private
    // constructor; a class of the same package with a package-private one is offered.
    let src = "package p;\n\nclass Local {\n  Local() {}\n}\n\nclass Main {\n  void run() {\n    Object o = new Arr|\n  }\n}\n";
    let p = project_with(src);
    let c = at(&p, src, 1);
    assert!(
        names(&c).contains(&"ArrayList".to_string()),
        "{:?}",
        names(&c)
    );
    assert!(
        !names(&c).contains(&"Arrays".to_string()),
        "{:?}",
        names(&c)
    );
    assert!(!names(&c).contains(&"Array".to_string()), "{:?}", names(&c));
    let src = src.replace("new Arr|", "new Loc|");
    let c = at(&p, &src, 1);
    assert!(names(&c).contains(&"Local".to_string()), "{:?}", names(&c));
}
