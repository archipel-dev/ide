//! Query authoring aid: prints the syntax tree of a file, one node per line
//! (`kind [start-end] field:`), so queries can be written against real trees.
//!
//! `cargo run -q -p archipel-ide-language --example sexp -- <path> [language-id]`

use archipel_ide_language::{FileId, FileRev, SyntaxNode, SyntaxSnapshot, registry};
use std::path::Path;

fn dump(node: SyntaxNode<'_>, field: Option<&str>, depth: usize, out: &mut String) {
    let range = node.range();
    let named = if node.is_named() { "" } else { "  (token)" };
    let field = field.map(|f| format!("{f}: ")).unwrap_or_default();
    let text = if node.raw().child_count() == 0 {
        format!(" {:?}", node.text())
    } else {
        String::new()
    };
    out.push_str(&format!(
        "{}{}{} [{}-{}]{}{}\n",
        "  ".repeat(depth),
        field,
        node.kind(),
        range.start,
        range.end,
        text,
        named
    ));
    let mut cursor = node.raw().walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            let name = cursor.field_name();
            let wrapped = node
                .children()
                .find(|c| c.raw().id() == child.id())
                .expect("child");
            if wrapped.is_named() || std::env::var_os("TOKENS").is_some() {
                dump(wrapped, name, depth + 1, out);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("path");
    let source = std::fs::read_to_string(&path).expect("readable file");
    let lang = match args.next() {
        Some(id) => registry::by_id(&id),
        None => registry::detect(Path::new(&path), &source),
    }
    .expect("known language");
    let snap =
        SyntaxSnapshot::parse(FileId(1), FileRev(1), lang, source.as_str().into()).expect("parse");
    let mut out = String::new();
    dump(snap.root(), None, 0, &mut out);
    print!("{out}");
}
