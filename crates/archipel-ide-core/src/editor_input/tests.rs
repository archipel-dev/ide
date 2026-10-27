//! Unit tests of the input layer on documents without a project; the executable specification
//! cases run in `tests/editor_cases.rs`.

use super::*;
use crate::Document;
use archipel_ide_text::Selection;

fn document(name: &str, text: &str, caret: usize) -> Document {
    let mut document = Document::new(name.into(), text);
    document.selections.set_single(Selection::caret(caret));
    document
}

fn text(document: &Document) -> String {
    document.text.text()
}

#[test]
fn typing_pairs_and_overtypes_brackets_in_code() {
    let opts = EditorOptions::default();
    let mut d = document("A.java", "class A { void f() { foo } }", 24);
    typing::type_text(&mut d, &opts, "(").unwrap();
    assert_eq!(text(&d), "class A { void f() { foo() } }");
    typing::type_text(&mut d, &opts, ")").unwrap();
    assert_eq!(d.selections.primary().head, 26);
    assert_eq!(text(&d), "class A { void f() { foo() } }");
}

#[test]
fn plain_text_never_pairs() {
    let opts = EditorOptions::default();
    let mut d = document("a.txt", "foo", 3);
    typing::type_text(&mut d, &opts, "(").unwrap();
    assert_eq!(text(&d), "foo(");
}

#[test]
fn typed_run_is_one_undo_step_and_restores_the_caret() {
    let opts = EditorOptions::default();
    let mut d = document("a.txt", "", 0);
    for c in ["a", "b", "c"] {
        typing::type_text(&mut d, &opts, c).unwrap();
    }
    assert_eq!(text(&d), "abc");
    let current = d.selections.selections().to_vec();
    let result = d.text.undo_with(&current, false).unwrap();
    assert!(result.text_changed);
    assert_eq!(text(&d), "");
    assert_eq!(result.selections, vec![Selection::caret(0)]);
}

#[test]
fn escape_keeps_the_oldest_caret() {
    let mut d = document("a.txt", "a b c", 0);
    d.selections.add(Selection::caret(2));
    d.selections.add(Selection::caret(4));
    assert!(movement::escape(&mut d));
    assert_eq!(d.selections.selections(), &[Selection::caret(0)]);
}
