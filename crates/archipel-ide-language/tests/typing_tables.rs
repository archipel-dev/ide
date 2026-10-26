//! Before/after tables of the typed-editing services: a tiny editor applies
//! `before_typed`, the insertion, `after_typed`, Enter, Complete Current
//! Statement and Move Statement to a text with a caret (`|`), and the result
//! is compared as a string. Language-specific rows document the behavior.

use archipel_ide_language::edit::TypedDecision;
use archipel_ide_language::indent::IndentOptions;
use archipel_ide_language::{FileId, FileRev, SyntaxSnapshot, move_statement, smart_enter, typing};

struct Buffer {
    lang: &'static str,
    text: String,
    caret: usize,
}

impl Buffer {
    fn new(lang: &'static str, marked: &str) -> Self {
        let caret = marked.find('|').expect("caret marker `|`");
        Self {
            lang,
            text: marked.replacen('|', "", 1),
            caret,
        }
    }

    fn snap(&self) -> SyntaxSnapshot {
        let spec = archipel_ide_language::registry::by_id(self.lang).expect("language");
        SyntaxSnapshot::parse(FileId(1), FileRev(1), spec, self.text.as_str().into())
            .expect("parse")
    }

    fn options(&self) -> IndentOptions {
        IndentOptions::detect(&self.text)
    }

    fn show(&self) -> String {
        let mut out = self.text.clone();
        out.insert(self.caret, '|');
        out
    }

    fn apply(&mut self, set: archipel_ide_language::edit::EditSet) {
        self.text = set.apply(&self.text);
        if let Some(caret) = set.caret {
            self.caret = caret;
        }
    }

    fn type_char(&mut self, typed: char) {
        let decision = typing::before_typed(&self.snap(), self.caret, None, typed);
        match decision {
            TypedDecision::Overtype => self.caret += typed.len_utf8(),
            TypedDecision::InsertPair(close) => {
                self.text.insert_str(self.caret, &format!("{typed}{close}"));
                self.caret += typed.len_utf8();
            }
            _ => {
                self.text.insert(self.caret, typed);
                self.caret += typed.len_utf8();
            }
        }
        if !matches!(
            decision,
            TypedDecision::Overtype | TypedDecision::InsertPair(_)
        ) {
            let options = self.options();
            if let Some(set) = typing::after_typed(&self.snap(), self.caret, typed, options) {
                self.apply(set);
            }
        }
    }

    fn enter(&mut self) {
        let set = typing::on_enter(&self.snap(), self.caret..self.caret, self.options())
            .unwrap_or_else(|| archipel_ide_language::edit::EditSet {
                edits: vec![archipel_ide_language::edit::Edit::insert(self.caret, "\n")],
                caret: Some(self.caret + 1),
                selection: None,
            });
        self.apply(set);
    }

    fn complete(&mut self) {
        let set =
            smart_enter::smart_enter(&self.snap(), self.caret, self.options()).expect("completion");
        self.apply(set);
    }

    fn move_statement(&mut self, up: bool) -> bool {
        match move_statement::move_statement(&self.snap(), self.caret..self.caret, up) {
            Some(set) => {
                self.apply(set);
                true
            }
            None => false,
        }
    }
}

fn typed(lang: &'static str, before: &str, keys: &str) -> String {
    let mut buffer = Buffer::new(lang, before);
    for key in keys.chars() {
        if key == '\n' {
            buffer.enter();
        } else {
            buffer.type_char(key);
        }
    }
    buffer.show()
}

#[test]
fn typing_table() {
    let rows: &[(&str, &str, &str, &str)] = &[
        // lang, before, keys, after
        (
            "java",
            "class A { void f() { | } }",
            "(",
            "class A { void f() { (|) } }",
        ),
        (
            "java",
            "class A { void f() { g| } }",
            "(",
            "class A { void f() { g(|) } }",
        ),
        (
            "java",
            "class A { void f() { g(|) } }",
            ")",
            "class A { void f() { g()| } }",
        ),
        (
            "java",
            "class A { String s = |; }",
            "\"",
            "class A { String s = \"|\"; }",
        ),
        (
            "java",
            "class A { String s = \"a|\"; }",
            "\"",
            "class A { String s = \"a\"|; }",
        ),
        (
            "java",
            "class A { String s = \"a|b\"; }",
            "(",
            "class A { String s = \"a(|b\"; }",
        ),
        ("java", "class A { // it|\n}", "'", "class A { // it'|\n}"),
        (
            "java",
            "class A { char c = |; }",
            "'",
            "class A { char c = '|'; }",
        ),
        ("rust", "fn f<'a>(x: &|) {}", "'", "fn f<'a>(x: &'|) {}"),
        (
            "rust",
            "fn f() { let c = |; }",
            "'",
            "fn f() { let c = '|'; }",
        ),
        (
            "rust",
            "fn f() { let v = |; }",
            "[",
            "fn f() { let v = [|]; }",
        ),
        ("kotlin", "val s = \"x|\"", "$", "val s = \"x$|\""),
        ("kotlin", "val s = \"$|\"", "{", "val s = \"${|}\""),
        ("python", "x = |", "\"\"\"", "x = \"\"\"|\"\"\""),
        ("python", "x = f|", "\"", "x = f\"|\""),
        ("python", "d = |", "{", "d = {|}"),
        ("go", "func f() { s := |}", "`", "func f() { s := `|`}"),
        ("javascript", "const s = |;", "`", "const s = `|`;"),
        ("json", "{ \"a\": | }", "[", "{ \"a\": [|] }"),
        ("html", "<p class=|>", "\"", "<p class=\"|\">"),
        ("html", "<div>|", "", "<div>|"),
        ("markdown", "see |", "[", "see [|]"),
        ("properties", "a=|", "(", "a=(|"),
    ];
    let mut failures = Vec::new();
    for (lang, before, keys, after) in rows {
        let got = typed(lang, before, keys);
        if &got != after {
            failures.push(format!(
                "{lang}: {before:?} + {keys:?}\n  expected {after:?}\n  got      {got:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn closers_tags_and_dedent_table() {
    let rows: &[(&str, &str, &str, &str)] = &[
        (
            "java",
            "class A {\n    void f() {\n        g();\n        |\n    }\n}\n",
            "}",
            "class A {\n    void f() {\n        g();\n    }|\n    }\n}\n",
        ),
        (
            "rust",
            "fn f() {\n    if a {\n        b();\n            |\n    }\n}\n",
            "}",
            "fn f() {\n    if a {\n        b();\n    }|\n    }\n}\n",
        ),
        ("html", "<div|", ">", "<div>|</div>"),
        ("html", "<img src=\"a\"|", ">", "<img src=\"a\">|"),
        ("xml", "<a><b|</a>", ">", "<a><b>|</b></a>"),
        (
            "python",
            "if a:\n    b\n    else|",
            ":",
            "if a:\n    b\nelse:|",
        ),
        (
            "python",
            "try:\n    a\n    except|",
            ":",
            "try:\n    a\nexcept:|",
        ),
    ];
    let mut failures = Vec::new();
    for (lang, before, keys, after) in rows {
        let got = typed(lang, before, keys);
        if &got != after {
            failures.push(format!(
                "{lang}: {before:?} + {keys:?}\n  expected {after:?}\n  got      {got:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn enter_table() {
    let rows: &[(&str, &str, &str)] = &[
        (
            "java",
            "class A {\n    void f() {|}\n}\n",
            "class A {\n    void f() {\n        |\n    }\n}\n",
        ),
        (
            "java",
            "class A {\n    int[] a = {|};\n}\n",
            "class A {\n    int[] a = {\n        |\n    };\n}\n",
        ),
        (
            "java",
            "class A {\n    void f() {\n        g(|);\n    }\n}\n",
            "class A {\n    void f() {\n        g(\n            |\n        );\n    }\n}\n",
        ),
        (
            "rust",
            "fn f() {\n    let x = 1;|\n}\n",
            "fn f() {\n    let x = 1;\n    |\n}\n",
        ),
        (
            "rust",
            "/// docs|\nfn f() {}\n",
            "/// docs\n/// |\nfn f() {}\n",
        ),
        (
            "go",
            "func f() {\n\tif a {\n\t\tb()|\n\t}\n}\n",
            "func f() {\n\tif a {\n\t\tb()\n\t\t|\n\t}\n}\n",
        ),
        (
            "python",
            "def f():|\n    pass\n",
            "def f():\n    |\n    pass\n",
        ),
        (
            "python",
            "def f():\n    return 1|\n",
            "def f():\n    return 1\n|\n",
        ),
        (
            "c",
            "int main() {\n    switch (x) {\n    case 1:|\n    }\n}\n",
            "int main() {\n    switch (x) {\n    case 1:\n        |\n    }\n}\n",
        ),
        ("css", "a {|}\n", "a {\n    |\n}\n"),
        ("json", "{\"a\": [|]}\n", "{\"a\": [\n    |\n]}\n"),
        ("bash", "f() {\n  echo a|\n}\n", "f() {\n  echo a\n  |\n}\n"),
    ];
    let mut failures = Vec::new();
    for (lang, before, after) in rows {
        let mut buffer = Buffer::new(lang, before);
        buffer.enter();
        if &buffer.show() != after {
            failures.push(format!(
                "{lang}: {before:?}\n  expected {after:?}\n  got      {:?}",
                buffer.show()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn complete_statement_table() {
    let rows: &[(&str, &str, &str)] = &[
        (
            "java",
            "class A {\n    void f() {\n        g(1|\n    }\n}\n",
            "class A {\n    void f() {\n        g(1);\n        |\n    }\n}\n",
        ),
        (
            "java",
            "class A {\n    void f() {\n        while (x|)\n    }\n}\n",
            "class A {\n    void f() {\n        while (x) {\n            |\n        }\n    }\n}\n",
        ),
        (
            "javascript",
            "function f() {\n  let a = g(h(1|\n}\n",
            "function f() {\n  let a = g(h(1));\n  |\n}\n",
        ),
        (
            "rust",
            "fn f() {\n    for i in 0..3|\n}\n",
            "fn f() {\n    for i in 0..3 {\n        |\n    }\n}\n",
        ),
        (
            "kotlin",
            "fun f() {\n    println(1|\n}\n",
            "fun f() {\n    println(1)\n    |\n}\n",
        ),
        (
            "python",
            "class A:\n    def f(self|\n        pass\n",
            "class A:\n    def f(self):\n        |\n        pass\n",
        ),
        ("python", "for x in y|\n", "for x in y:\n    |\n"),
        ("bash", "while true|\n", "while true; do\n    |\ndone\n"),
    ];
    let mut failures = Vec::new();
    for (lang, before, after) in rows {
        let mut buffer = Buffer::new(lang, before);
        buffer.complete();
        if &buffer.show() != after {
            failures.push(format!(
                "{lang}: {before:?}\n  expected {after:?}\n  got      {:?}",
                buffer.show()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn move_statement_table() {
    let rows: &[(&str, &str, bool, Option<&str>)] = &[
        (
            "rust",
            "fn f() {\n    a();\n    b|();\n}\n",
            true,
            Some("fn f() {\n    b|();\n    a();\n}\n"),
        ),
        (
            "python",
            "def f():\n    a()\n    b|()\n    c()\n",
            false,
            Some("def f():\n    a()\n    c()\n    b|()\n"),
        ),
        ("go", "func f() {\n\ta()\n\tb|()\n}\n", false, None),
        (
            "javascript",
            "class A {\n  one() {}\n\n  tw|o() {}\n}\n",
            true,
            Some("class A {\n  tw|o() {}\n\n  one() {}\n}\n"),
        ),
        (
            "json",
            "{\n  \"a\": 1,\n  \"b|\": 2\n}\n",
            true,
            Some("{\n  \"b|\": 2,\n  \"a\": 1\n}\n"),
        ),
    ];
    let mut failures = Vec::new();
    for (lang, before, up, after) in rows {
        let mut buffer = Buffer::new(lang, before);
        let moved = buffer.move_statement(*up);
        let got = moved.then(|| buffer.show());
        if got.as_deref() != *after {
            failures.push(format!(
                "{lang}: {before:?} up={up}\n  expected {after:?}\n  got      {got:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
