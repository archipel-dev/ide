//! Runnable entry points in source files, for gutter run icons.
//!
//! Finds `main` functions and tests with the tree-sitter grammars of the language
//! registry: Java (`static void main`, `@Test`-style annotations), Kotlin (top-level
//! `fun main`, `@Test`), Rust (`fn main` in binaries, `#[test]`/`#[tokio::test]`
//! functions inside their module path) and Go (`func main` in `package main`,
//! `TestXxx`/`BenchmarkXxx`/`ExampleXxx`). Bazel BUILD files are handled by
//! `archipel-ide-bazel`. Mapping runnables to run configurations is the caller's job.

use std::path::Path;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnableKind {
    /// An application entry point.
    Main,
    /// A single test function or method.
    Test,
    /// A class/module that contains tests (runs all of them).
    TestContainer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runnable {
    /// Zero-based line of the declaration.
    pub line: usize,
    pub kind: RunnableKind,
    pub language: &'static str,
    /// Fully qualified name: `com.example.App` (main class), `com.example.AppTest.greets`
    /// (test method), `tests::adds` (Rust test path within its crate target),
    /// `TestAdd` (Go).
    pub symbol: String,
    /// Short label for menus: `App.main()`, `AppTest.greets()`, `tests::adds`.
    pub label: String,
}

/// Java/Kotlin annotations that mark a test method.
const TEST_ANNOTATIONS: [&str; 6] = [
    "Test",
    "ParameterizedTest",
    "RepeatedTest",
    "TestFactory",
    "TestTemplate",
    "Property",
];

/// Finds runnables in `source`. `module_path` is the Rust module of the file within
/// its crate target (see [`rust_module_path`]); other languages ignore it.
pub fn find(path: &Path, source: &str, module_path: &str) -> Vec<Runnable> {
    let Some(spec) = archipel_ide_language::registry::detect(path, source) else {
        return Vec::new();
    };
    let mut parser = Parser::new();
    if parser.set_language(&spec.grammar()).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    find_in_tree(spec.id, &tree, path, source, module_path)
}

/// Like [`find`], on an existing syntax tree of `source` (the editor's
/// incrementally maintained one), so large files are not reparsed per edit.
/// `language` is the registry id (`java`, `kotlin`, `rust`, `go`).
pub fn find_in_tree(
    language: &str,
    tree: &tree_sitter::Tree,
    path: &Path,
    source: &str,
    module_path: &str,
) -> Vec<Runnable> {
    let root = tree.root_node();
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).unwrap_or_default();
    let mut found = Vec::new();
    match language {
        "java" => java(root, &text, &mut found),
        "kotlin" => kotlin(root, &text, path, &mut found),
        "rust" => rust(root, &text, module_path, path, &mut found),
        "go" => go(root, &text, path, &mut found),
        _ => {}
    }
    found.sort_by_key(|runnable| runnable.line);
    found
}

fn children<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn java<'a>(root: Node<'a>, text: &dyn Fn(Node<'a>) -> &'a str, found: &mut Vec<Runnable>) {
    let package = children(root)
        .into_iter()
        .find(|node| node.kind() == "package_declaration")
        .and_then(|node| children(node).into_iter().next())
        .map(text)
        .unwrap_or_default()
        .to_owned();
    for node in children(root) {
        java_class(node, &package, "", text, found);
    }
}

fn java_class<'a>(
    node: Node<'a>,
    package: &str,
    outer: &str,
    text: &dyn Fn(Node<'a>) -> &'a str,
    found: &mut Vec<Runnable>,
) {
    if !matches!(
        node.kind(),
        "class_declaration" | "record_declaration" | "enum_declaration" | "interface_declaration"
    ) {
        return;
    }
    let Some(name) = node.child_by_field_name("name").map(text) else {
        return;
    };
    let simple = if outer.is_empty() {
        name.to_owned()
    } else {
        format!("{outer}${name}")
    };
    let qualified = qualify(package, &simple);
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let mut tests = 0;
    let mut has_main = false;
    for member in children(body) {
        if member.kind() == "method_declaration"
            && let Some(method) = member.child_by_field_name("name").map(text)
        {
            let modifiers = children(member)
                .into_iter()
                .find(|child| child.kind() == "modifiers");
            let modifier_text = modifiers.map(text).unwrap_or_default();
            let annotations: Vec<&str> = modifiers
                .map(|modifiers| {
                    children(modifiers)
                        .into_iter()
                        .filter(|child| matches!(child.kind(), "marker_annotation" | "annotation"))
                        .filter_map(|annotation| annotation.child_by_field_name("name").map(text))
                        .map(|name| name.rsplit('.').next().unwrap_or(name))
                        .collect()
                })
                .unwrap_or_default();
            // The icon goes on the line with the method name, not its annotations.
            let line = member
                .child_by_field_name("name")
                .map_or(member.start_position().row, |name| {
                    name.start_position().row
                });
            if annotations.iter().any(|a| TEST_ANNOTATIONS.contains(a)) {
                tests += 1;
                found.push(Runnable {
                    line,
                    kind: RunnableKind::Test,
                    language: "java",
                    symbol: format!("{qualified}.{method}"),
                    label: format!("{name}.{method}()"),
                });
            } else if method == "main" && modifier_text.split_whitespace().any(|m| m == "static") {
                has_main = true;
                found.push(Runnable {
                    line,
                    kind: RunnableKind::Main,
                    language: "java",
                    symbol: qualified.clone(),
                    label: format!("{name}.main()"),
                });
            }
        } else {
            java_class(member, package, &simple, text, found);
        }
    }
    // A class declaration with an entry point gets the run icon too (like the reference).
    if has_main && tests == 0 {
        found.push(Runnable {
            line: node.start_position().row,
            kind: RunnableKind::Main,
            language: "java",
            symbol: qualified.clone(),
            label: format!("{name}.main()"),
        });
    }
    if tests > 0 {
        found.push(Runnable {
            line: node.start_position().row,
            kind: RunnableKind::TestContainer,
            language: "java",
            symbol: qualified,
            label: name.to_owned(),
        });
    }
}

fn qualify(package: &str, name: &str) -> String {
    if package.is_empty() {
        name.to_owned()
    } else {
        format!("{package}.{name}")
    }
}

fn kotlin<'a>(
    root: Node<'a>,
    text: &dyn Fn(Node<'a>) -> &'a str,
    path: &Path,
    found: &mut Vec<Runnable>,
) {
    let package = children(root)
        .into_iter()
        .find(|node| node.kind() == "package_header")
        .and_then(|node| children(node).into_iter().next())
        .map(text)
        .unwrap_or_default()
        .to_owned();
    for node in children(root) {
        match node.kind() {
            "function_declaration" if kotlin_name(node, text) == Some("main") => {
                // Top-level functions compile into `<File>Kt`.
                let stem = path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut class = stem.chars();
                let class = class
                    .next()
                    .map(|first| first.to_uppercase().chain(class).collect::<String>() + "Kt")
                    .unwrap_or_default();
                found.push(Runnable {
                    line: node.start_position().row,
                    kind: RunnableKind::Main,
                    language: "kotlin",
                    symbol: qualify(&package, &class),
                    label: "main()".into(),
                });
            }
            "class_declaration" | "object_declaration" => {
                let Some(name) = children(node)
                    .into_iter()
                    .find(|child| child.kind() == "type_identifier")
                    .map(text)
                else {
                    continue;
                };
                let qualified = qualify(&package, name);
                let Some(body) = children(node)
                    .into_iter()
                    .find(|child| child.kind() == "class_body")
                else {
                    continue;
                };
                let mut tests = 0;
                for member in children(body) {
                    if member.kind() != "function_declaration" {
                        continue;
                    }
                    let annotated = children(member)
                        .into_iter()
                        .find(|child| child.kind() == "modifiers")
                        .map(text)
                        .is_some_and(|modifiers| {
                            TEST_ANNOTATIONS.iter().any(|annotation| {
                                modifiers.contains(&format!("@{annotation}"))
                                    && !modifiers.contains(&format!("@{annotation}s"))
                            })
                        });
                    if annotated && let Some(method) = kotlin_name(member, text) {
                        tests += 1;
                        let line = children(member)
                            .into_iter()
                            .find(|child| child.kind() == "simple_identifier")
                            .map_or(member.start_position().row, |n| n.start_position().row);
                        found.push(Runnable {
                            line,
                            kind: RunnableKind::Test,
                            language: "kotlin",
                            symbol: format!("{qualified}.{method}"),
                            label: format!("{name}.{method}()"),
                        });
                    }
                }
                if tests > 0 {
                    found.push(Runnable {
                        line: node.start_position().row,
                        kind: RunnableKind::TestContainer,
                        language: "kotlin",
                        symbol: qualified,
                        label: name.to_owned(),
                    });
                }
            }
            _ => {}
        }
    }
}

fn kotlin_name<'a>(node: Node<'a>, text: &dyn Fn(Node<'a>) -> &'a str) -> Option<&'a str> {
    children(node)
        .into_iter()
        .find(|child| child.kind() == "simple_identifier")
        .map(text)
}

fn rust<'a>(
    root: Node<'a>,
    text: &dyn Fn(Node<'a>) -> &'a str,
    module_path: &str,
    path: &Path,
    found: &mut Vec<Runnable>,
) {
    let is_binary_root = path.file_name().is_some_and(|name| name == "main.rs")
        || path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|dir| dir == "bin")
        || path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|dir| dir == "examples");
    rust_items(root, text, module_path, is_binary_root, found);
}

fn rust_items<'a>(
    node: Node<'a>,
    text: &dyn Fn(Node<'a>) -> &'a str,
    module: &str,
    binary_root: bool,
    found: &mut Vec<Runnable>,
) {
    let items = children(node);
    for (index, item) in items.iter().enumerate() {
        match item.kind() {
            "function_item" => {
                let Some(name) = item.child_by_field_name("name").map(text) else {
                    continue;
                };
                // Attributes are preceding siblings: `#[test] fn x()`.
                let is_test = items[..index]
                    .iter()
                    .rev()
                    .take_while(|sibling| {
                        matches!(sibling.kind(), "attribute_item" | "line_comment")
                    })
                    .filter(|sibling| sibling.kind() == "attribute_item")
                    .any(|attribute| {
                        let attribute = text(*attribute);
                        let inner = attribute
                            .trim_start_matches("#[")
                            .trim_end_matches(']')
                            .trim();
                        inner == "test"
                            || inner.ends_with("::test")
                            || inner.starts_with("test(")
                            || inner.contains("::test(")
                    });
                if is_test {
                    let symbol = join_module(module, name);
                    found.push(Runnable {
                        line: item.start_position().row,
                        kind: RunnableKind::Test,
                        language: "rust",
                        label: symbol.clone(),
                        symbol,
                    });
                } else if name == "main" && module.is_empty() && binary_root {
                    found.push(Runnable {
                        line: item.start_position().row,
                        kind: RunnableKind::Main,
                        language: "rust",
                        symbol: "main".into(),
                        label: "main()".into(),
                    });
                }
            }
            "mod_item" => {
                if let (Some(name), Some(body)) = (
                    item.child_by_field_name("name").map(text),
                    item.child_by_field_name("body"),
                ) {
                    let before = found.len();
                    rust_items(body, text, &join_module(module, name), false, found);
                    let module_tests = found[before..]
                        .iter()
                        .filter(|runnable| runnable.kind == RunnableKind::Test)
                        .count();
                    if module_tests > 0 {
                        let symbol = join_module(module, name);
                        found.push(Runnable {
                            line: item.start_position().row,
                            kind: RunnableKind::TestContainer,
                            language: "rust",
                            label: symbol.clone(),
                            symbol,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

fn join_module(module: &str, name: &str) -> String {
    if module.is_empty() {
        name.to_owned()
    } else {
        format!("{module}::{name}")
    }
}

/// Module path of a Rust file inside its crate target: `src/lib.rs` → ``,
/// `src/net/http.rs` → `net::http`, `src/net/mod.rs` → `net`. Files of other
/// targets (`tests/x.rs`, `src/bin/y.rs`) are their own crate roots.
pub fn rust_module_path(crate_root: &Path, file: &Path) -> String {
    let Ok(relative) = file.strip_prefix(crate_root.join("src")) else {
        return String::new();
    };
    if relative.starts_with("bin") {
        return String::new();
    }
    let mut parts: Vec<String> = relative
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect();
    if let Some(last) = parts.pop() {
        let stem = last.trim_end_matches(".rs").to_owned();
        if !matches!(stem.as_str(), "lib" | "main" | "mod") {
            parts.push(stem);
        }
    }
    parts.join("::")
}

fn go<'a>(
    root: Node<'a>,
    text: &dyn Fn(Node<'a>) -> &'a str,
    path: &Path,
    found: &mut Vec<Runnable>,
) {
    let package = children(root)
        .into_iter()
        .find(|node| node.kind() == "package_clause")
        .and_then(|node| children(node).into_iter().next())
        .map(text)
        .unwrap_or_default();
    let test_file = path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().ends_with("_test.go"));
    for node in children(root) {
        if node.kind() != "function_declaration" {
            continue;
        }
        let Some(name) = node.child_by_field_name("name").map(text) else {
            continue;
        };
        let is_test = test_file
            && ["Test", "Benchmark", "Example", "Fuzz"]
                .iter()
                .any(|prefix| {
                    name.strip_prefix(prefix).is_some_and(|rest| {
                        rest.is_empty() || !rest.starts_with(char::is_lowercase)
                    })
                });
        if is_test {
            found.push(Runnable {
                line: node.start_position().row,
                kind: RunnableKind::Test,
                language: "go",
                symbol: name.to_owned(),
                label: name.to_owned(),
            });
        } else if name == "main" && package == "main" {
            found.push(Runnable {
                line: node.start_position().row,
                kind: RunnableKind::Main,
                language: "go",
                symbol: "main".into(),
                label: "main()".into(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(found: &[Runnable]) -> Vec<(usize, RunnableKind, &str)> {
        found
            .iter()
            .map(|runnable| (runnable.line, runnable.kind, runnable.symbol.as_str()))
            .collect()
    }

    #[test]
    fn java_main_and_junit_tests() {
        let source = r#"package com.example;

import org.junit.jupiter.api.Test;

class AppTest {
    @Test
    void greets() {}

    @org.junit.jupiter.api.ParameterizedTest
    void many(int x) {}

    void helper() {}

    static class Nested {
        public static void main(String[] args) {}
    }
}

public class App {
    public static void main(String[] args) {}
    public void main(int notAnEntry) {}
}
"#;
        let found = find(Path::new("src/App.java"), source, "");
        assert_eq!(
            summary(&found),
            [
                (4, RunnableKind::TestContainer, "com.example.AppTest"),
                (6, RunnableKind::Test, "com.example.AppTest.greets"),
                (9, RunnableKind::Test, "com.example.AppTest.many"),
                (13, RunnableKind::Main, "com.example.AppTest$Nested"),
                (14, RunnableKind::Main, "com.example.AppTest$Nested"),
                (18, RunnableKind::Main, "com.example.App"),
                (19, RunnableKind::Main, "com.example.App"),
            ]
        );
        assert_eq!(found[1].label, "AppTest.greets()");
    }

    #[test]
    fn kotlin_top_level_main_and_tests() {
        let source = "package com.example\n\nfun main() {}\n\nclass CalcTest {\n    @Test\n    fun adds() {}\n    fun helper() {}\n}\n";
        let found = find(Path::new("src/main/kotlin/app.kt"), source, "");
        assert_eq!(
            summary(&found),
            [
                (2, RunnableKind::Main, "com.example.AppKt"),
                (4, RunnableKind::TestContainer, "com.example.CalcTest"),
                (6, RunnableKind::Test, "com.example.CalcTest.adds"),
            ]
        );
    }

    #[test]
    fn rust_tests_get_module_paths_and_main_only_in_binaries() {
        let source = "fn main() {}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn adds() {}\n\n    #[tokio::test]\n    async fn waits() {}\n\n    fn helper() {}\n}\n";
        let found = find(Path::new("src/main.rs"), source, "");
        assert_eq!(
            summary(&found),
            [
                (0, RunnableKind::Main, "main"),
                (3, RunnableKind::TestContainer, "tests"),
                (7, RunnableKind::Test, "tests::adds"),
                (10, RunnableKind::Test, "tests::waits"),
            ]
        );
        let library = find(Path::new("src/net/http.rs"), source, "net::http");
        assert_eq!(library[0].symbol, "net::http::tests");
        assert!(
            library
                .iter()
                .all(|runnable| runnable.kind != RunnableKind::Main)
        );
    }

    #[test]
    fn rust_module_paths() {
        let root = Path::new("/w/calc");
        assert_eq!(rust_module_path(root, Path::new("/w/calc/src/lib.rs")), "");
        assert_eq!(
            rust_module_path(root, Path::new("/w/calc/src/net/http.rs")),
            "net::http"
        );
        assert_eq!(
            rust_module_path(root, Path::new("/w/calc/src/net/mod.rs")),
            "net"
        );
        assert_eq!(
            rust_module_path(root, Path::new("/w/calc/src/bin/tool.rs")),
            ""
        );
        assert_eq!(rust_module_path(root, Path::new("/w/calc/tests/it.rs")), "");
    }

    #[test]
    fn go_tests_and_main() {
        let test = "package calc\n\nfunc TestAdd(t *testing.T) {}\nfunc Testing() {}\nfunc BenchmarkAdd(b *testing.B) {}\nfunc helper() {}\n";
        let found = find(Path::new("calc_test.go"), test, "");
        assert_eq!(
            summary(&found),
            [
                (2, RunnableKind::Test, "TestAdd"),
                (4, RunnableKind::Test, "BenchmarkAdd")
            ]
        );
        let main = find(Path::new("main.go"), "package main\n\nfunc main() {}\n", "");
        assert_eq!(summary(&main), [(2, RunnableKind::Main, "main")]);
        assert!(find(Path::new("lib.go"), "package lib\nfunc main() {}\n", "").is_empty());
    }

    #[test]
    fn unknown_or_broken_sources_yield_nothing() {
        assert!(find(Path::new("notes.txt"), "fn main() {}", "").is_empty());
        let broken = find(Path::new("A.java"), "class { void main(", "");
        assert!(broken.is_empty(), "{broken:?}");
    }
}
