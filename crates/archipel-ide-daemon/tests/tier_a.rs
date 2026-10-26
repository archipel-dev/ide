//! Tier-A inspections on every G0 language (no resolve involved).

mod common;
use common::{Harness, only};

#[test]
fn java() {
    let src = "class A {\n  void f(int a) {\n    if (a = 1) { } else { }\n    if (true) { x(); } else { x(); }\n    try { g(); } catch (Exception e) { }\n    int y = ((1));\n    a = a;\n    return;\n    foo();\n    bar();\n  }\n}\n";
    let found = Harness::new().inspect("A.java", src);
    assert_eq!(only(&found, "AssignmentInCondition"), ["a = 1"]);
    assert_eq!(only(&found, "ConstantCondition"), ["true"]);
    assert_eq!(only(&found, "EmptyCatchBlock"), ["catch (Exception e) { }"]);
    assert_eq!(only(&found, "RedundantParentheses"), ["(1)"]);
    assert_eq!(only(&found, "SelfAssignment"), ["a = a"]);
    assert_eq!(only(&found, "UnreachableCode"), ["foo();\n    bar();"]);
    assert_eq!(only(&found, "DuplicateBranches"), ["{ x(); }"]);
    assert_eq!(only(&found, "EmptyStatementBody"), ["{ }", "{ }"]);
}

#[test]
fn rust() {
    let src = "fn f(a: i32) -> i32 {\n    if (a > 1) { } else { }\n    if true { x(); } else { x(); }\n    a = a;\n    let y = ((1));\n    return 1;\n    foo();\n}\n";
    let found = Harness::new().inspect("a.rs", src);
    assert_eq!(
        only(&found, "RedundantParentheses"),
        ["(a > 1)", "((1))", "(1)"]
    );
    assert_eq!(only(&found, "ConstantCondition"), ["true"]);
    assert_eq!(only(&found, "SelfAssignment"), ["a = a"]);
    assert_eq!(only(&found, "UnreachableCode"), ["foo();"]);
    assert_eq!(only(&found, "DuplicateBranches"), ["{ x(); }"]);
    assert_eq!(only(&found, "EmptyStatementBody"), ["{ }", "{ }"]);
}

#[test]
fn python() {
    let src = "def f(a):\n    if (a):\n        pass\n    else:\n        pass\n    a = a\n    if False:\n        x()\n    return 1\n    foo()\n";
    let found = Harness::new().inspect("a.py", src);
    assert_eq!(only(&found, "RedundantParentheses"), ["(a)"]);
    assert_eq!(only(&found, "SelfAssignment"), ["a = a"]);
    assert_eq!(only(&found, "ConstantCondition"), ["False"]);
    assert_eq!(only(&found, "UnreachableCode"), ["foo()"]);
    assert_eq!(only(&found, "DuplicateBranches").len(), 1);
}

#[test]
fn javascript_and_typescript() {
    let js = "function f(a) {\n  if (a = 1) { } else { x(); }\n  try { g(); } catch (e) { }\n  a = a;\n  return 1;\n  foo();\n}\n";
    for path in ["a.js", "a.ts", "a.tsx"] {
        let found = Harness::new().inspect(path, js);
        assert_eq!(only(&found, "AssignmentInCondition"), ["a = 1"], "{path}");
        assert_eq!(only(&found, "EmptyCatchBlock").len(), 1, "{path}");
        assert_eq!(only(&found, "SelfAssignment"), ["a = a"], "{path}");
        assert_eq!(only(&found, "UnreachableCode"), ["foo();"], "{path}");
        assert_eq!(only(&found, "EmptyStatementBody"), ["{ }"], "{path}");
    }
    let ts = "function f(a: number): number {\n  if (a) { return 1; } else { return 1; }\n  throw new Error(\"x\");\n  foo();\n}\n";
    let found = Harness::new().inspect("b.ts", ts);
    assert_eq!(only(&found, "DuplicateBranches"), ["{ return 1; }"]);
    assert_eq!(only(&found, "UnreachableCode"), ["foo();"]);
}

#[test]
fn go_c_cpp() {
    let go = "package main\nfunc f(a int) int {\n\tif a > 1 { } else { x() }\n\ta = a\n\treturn 1\n\tfoo()\n}\n";
    let found = Harness::new().inspect("a.go", go);
    assert_eq!(only(&found, "SelfAssignment"), ["a = a"]);
    assert_eq!(only(&found, "UnreachableCode"), ["foo()"]);
    assert_eq!(only(&found, "EmptyStatementBody"), ["{ }"]);
    let c = "int f(int a) {\n  if (a = 1) { } else { x(); }\n  a = a;\n  return 1;\n  foo();\n}\n";
    let found = Harness::new().inspect("a.c", c);
    assert_eq!(only(&found, "AssignmentInCondition"), ["a = 1"]);
    assert_eq!(only(&found, "SelfAssignment"), ["a = a"]);
    assert_eq!(only(&found, "UnreachableCode"), ["foo();"]);
    let cpp = "int f(int a) {\n  try { g(); } catch (const E& e) { }\n  switch (a) { case 1: return 1; foo(); default: break; }\n}\n";
    let found = Harness::new().inspect("a.cpp", cpp);
    assert_eq!(only(&found, "EmptyCatchBlock").len(), 1);
    assert_eq!(only(&found, "UnreachableCode"), ["foo();"]);
}

#[test]
fn kotlin_and_groovy() {
    let kt = "fun f(a: Int): Int {\n    if (a > 1) { }\n    if (true) { return 1 }\n    try { g() } catch (e: Exception) { }\n    a = a\n    return 1\n    foo()\n}\n";
    let found = Harness::new().inspect("a.kt", kt);
    assert_eq!(only(&found, "EmptyCatchBlock").len(), 1);
    assert_eq!(only(&found, "ConstantCondition"), ["true"]);
    assert_eq!(only(&found, "SelfAssignment"), ["a = a"]);
    assert_eq!(only(&found, "UnreachableCode"), ["foo()"]);
    let groovy = "def f(a) {\n  if (a = 1) { } else { x() }\n  try { g() } catch (Exception e) { }\n  return 1\n  foo()\n}\n";
    let found = Harness::new().inspect("a.groovy", groovy);
    assert_eq!(only(&found, "AssignmentInCondition"), ["a = 1"]);
    assert_eq!(only(&found, "EmptyCatchBlock").len(), 1);
    assert_eq!(only(&found, "UnreachableCode"), ["foo()"]);
}

#[test]
fn duplicate_keys_in_data_files() {
    let mut h = Harness::new();
    let found = h.inspect(
        "a.json",
        "{\"a\": 1, \"a\": 2, \"b\": {\"c\": 1, \"c\": 2}}",
    );
    assert_eq!(only(&found, "DuplicateKey"), ["\"a\"", "\"c\""]);
    let found = h.inspect("a.yaml", "a: 1\na: 2\nb:\n  c: 1\n  c: 2\n");
    assert_eq!(only(&found, "DuplicateKey"), ["a", "c"]);
    let found = h.inspect("a.toml", "a = 1\na = 2\n[t]\nb = 1\nb = 2\n[t]\n");
    assert_eq!(only(&found, "DuplicateKey"), ["a", "b", "t"]);
    let found = h.inspect("a.properties", "a=1\nb=2\na=3\n");
    assert_eq!(only(&found, "DuplicateKey"), ["a"]);
    let found = h.inspect(
        "a.xml",
        "<r a=\"1\" a=\"2\" b=\"3\"><c x=\"1\" y=\"2\"/></r>",
    );
    assert_eq!(
        only(&found, "DuplicateKey"),
        ["Name"].iter().map(|_| "a".to_string()).collect::<Vec<_>>()
    );
    // Distinct keys: nothing.
    assert!(only(&h.inspect("b.json", "{\"a\": 1, \"b\": 2}"), "DuplicateKey").is_empty());
}

#[test]
fn todo_comments_in_every_language_with_comments() {
    let mut h = Harness::new();
    for (path, src) in [
        ("a.rs", "// TODO: fix this\nfn f() {}\n"),
        ("a.java", "/* FIXME remove */ class A {}\n"),
        ("a.py", "# TODO later\nx = 1\n"),
        ("a.yaml", "# FIXME broken\na: 1\n"),
        ("a.toml", "# TODO\na = 1\n"),
        ("a.xml", "<!-- TODO xml --><r/>"),
        ("a.go", "package a // TODO go\n"),
    ] {
        let found = h.inspect(path, src);
        assert_eq!(only(&found, "TodoComment").len(), 1, "{path}: {found:?}");
    }
    let found = h.inspect("a.rs", "// TODOS are not todos\n// notTODO\nfn f() {}\n");
    assert!(only(&found, "TodoComment").is_empty());
}

#[test]
fn whitespace_checks_run_on_any_language_and_respect_options() {
    use archipel_ide_daemon::inspection::profile::{ProfileFile, ProfileStore};
    let mut h = Harness::new();
    let src = "fn f() {  \n \tlet x = 1;\n    let long = 1111111111111111111111111111111111111111111111;\n}\n";
    // Off by default: LongLine and TrailingWhitespace stay quiet, MixedIndentation fires.
    let found = h.inspect("a.rs", src);
    assert!(only(&found, "TrailingWhitespace").is_empty() && only(&found, "LongLine").is_empty());
    assert_eq!(only(&found, "MixedIndentation"), [" \t"]);
    // Forced on, with a custom limit from a profile.
    let mut store = ProfileStore::new();
    store.insert(
        "P",
        archipel_ide_daemon::inspection::profile::Origin::Project,
        ProfileFile::from_toml("[tools.LongLine.options]\nmax = 40\n").unwrap(),
    );
    let profile = store.resolve("P").unwrap();
    let found = h.inspect_with("a.rs", src, &profile, &["TrailingWhitespace", "LongLine"]);
    let texts: Vec<_> = found
        .iter()
        .map(|f| {
            (
                f.tool,
                &src[f.problem.range.start as usize..f.problem.range.end as usize],
            )
        })
        .collect();
    assert!(texts.contains(&("TrailingWhitespace", "  ")), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|(t, s)| *t == "LongLine" && s.ends_with("11;")),
        "{texts:?}"
    );
    // Markdown is exempt from the whitespace checks (two trailing spaces are a line break).
    let found = h.inspect_with("a.md", "text  \nmore\n", &profile, &["TrailingWhitespace"]);
    assert!(found.iter().all(|f| f.tool != "TrailingWhitespace"));
}

#[test]
fn suppression_comments_silence_problems() {
    let src = "class A {\n  void f() {\n    // noinspection EmptyCatchBlock\n    try { g(); } catch (Exception e) { }\n    try { g(); } catch (Exception e2) { }\n  }\n}\n";
    let found = Harness::new().inspect("A.java", src);
    assert_eq!(
        only(&found, "EmptyCatchBlock"),
        ["catch (Exception e2) { }"]
    );
    let java = "@SuppressWarnings(\"EmptyCatchBlock\")\nclass A { void f() { try { g(); } catch (Exception e) { } } }\n";
    assert!(only(&Harness::new().inspect("A.java", java), "EmptyCatchBlock").is_empty());
    let rust =
        "#[allow(archipel::SelfAssignment)]\nfn f(a: i32) { a = a; }\nfn g(a: i32) { a = a; }\n";
    assert_eq!(
        only(&Harness::new().inspect("a.rs", rust), "SelfAssignment").len(),
        1
    );
}

#[test]
fn no_problems_in_clean_code_of_every_language() {
    let mut h = Harness::new();
    for (path, src) in [
        (
            "a.java",
            "class A {\n  int f(int a) {\n    if (a > 1) { return 1; } else { return 2; }\n  }\n}\n",
        ),
        (
            "a.rs",
            "fn f(a: i32) -> i32 {\n    if a > 1 { a } else { 2 }\n}\n",
        ),
        (
            "a.py",
            "def f(a):\n    if a:\n        return 1\n    return 2\n",
        ),
        (
            "a.go",
            "package main\nfunc f(a int) int {\n\tif a > 1 {\n\t\treturn 1\n\t}\n\treturn 2\n}\n",
        ),
        ("a.json", "{\"a\": 1, \"b\": [1, 2]}"),
        ("a.yaml", "a: 1\nb:\n  - x\n"),
    ] {
        let found = h.inspect(path, src);
        assert!(found.is_empty(), "{path}: {found:?}");
    }
}

#[test]
fn tier_a_tools_cover_all_g0_languages() {
    use archipel_ide_analysis::inspect::LocalInspection;
    let h = Harness::new();
    let snap = h.host.snapshot();
    let mut with_tools = 0;
    let mut markdown = 0;
    for lang in archipel_ide_language::LANGUAGES.iter() {
        let tools = snap
            .registry()
            .all::<dyn LocalInspection>(lang.language_id());
        // Markdown keeps its trailing spaces (hard line breaks) and has no code comments.
        if lang.id.starts_with("markdown") {
            markdown += 1;
            continue;
        }
        assert!(
            tools.iter().any(|t| t.id() == "TodoComment"
                || t.id() == "DuplicateKey"
                || t.id() == "MixedIndentation"),
            "{} has no Tier-A tools",
            lang.id
        );
        with_tools += 1;
    }
    assert_eq!(
        with_tools + markdown,
        archipel_ide_language::LANGUAGES.len()
    );
}
