//! Text objects from `textobjects.scm`: selection groups (the elements of an
//! argument list, the statements of a block, ...) that make Extend Selection
//! offer an element, the elements up to it, all of them and then the
//! container; plus function/class objects for navigation.

use crate::node::SyntaxSnapshot;
use crate::queryset::{self, QueryKind};
use std::ops::Range;
use tree_sitter::Node;

/// A list-like container and its elements (named children between the
/// delimiters, comments excluded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectGroup {
    pub container: Range<usize>,
    pub elements: Vec<Range<usize>>,
}

impl SelectGroup {
    /// Index of the element containing `range`.
    pub fn element_containing(&self, range: &Range<usize>) -> Option<usize> {
        self.elements
            .iter()
            .position(|element| element.start <= range.start && range.end <= element.end)
    }
}

fn is_comment(snap: &SyntaxSnapshot, node: Node<'_>) -> bool {
    snap.lang()
        .token_set_table()
        .comments
        .contains(crate::ElementType(node.kind_id()))
}

fn group_of(snap: &SyntaxSnapshot, container: Node<'_>) -> Option<SelectGroup> {
    let mut cursor = container.walk();
    let elements: Vec<Range<usize>> = container
        .named_children(&mut cursor)
        .filter(|child| !is_comment(snap, *child) && !child.is_extra())
        .map(|child| child.byte_range())
        .collect();
    (!elements.is_empty()).then(|| SelectGroup {
        container: container.byte_range(),
        elements,
    })
}

/// The selection groups whose container contains `range`, innermost first.
pub fn groups_around(snap: &SyntaxSnapshot, range: &Range<usize>) -> Vec<SelectGroup> {
    let Some(query) = queryset::query(snap.lang(), QueryKind::TextObjects) else {
        return Vec::new();
    };
    let text = snap.text();
    let mut groups: Vec<SelectGroup> =
        queryset::captures_in(query, snap.tree(), text, range.clone())
            .into_iter()
            .filter(|captured| captured.name == "select.group")
            .filter(|captured| {
                let node = captured.node.byte_range();
                node.start <= range.start && range.end <= node.end
            })
            .filter_map(|captured| group_of(snap, captured.node))
            .collect();
    groups.sort_by_key(|group| group.container.len());
    groups.dedup_by(|a, b| a.container == b.container);
    groups
}

/// Candidate ranges the groups offer for Extend Selection from `range`: the
/// element containing it, the elements from the first up to it, and all of
/// them. Every candidate contains `range`.
pub fn group_candidates(snap: &SyntaxSnapshot, range: &Range<usize>) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    for group in groups_around(snap, range) {
        let (Some(first), Some(last)) = (group.elements.first(), group.elements.last()) else {
            continue;
        };
        if let Some(index) = group.element_containing(range) {
            out.push(group.elements[index].clone());
            if index > 0 {
                out.push(first.start..group.elements[index].end);
            }
        } else if range.start >= first.start && range.end <= last.end {
            // A selection spanning several elements: grow to the covering run.
            let start = group
                .elements
                .iter()
                .rev()
                .find(|element| element.start <= range.start)
                .map(|element| element.start);
            let end = group
                .elements
                .iter()
                .find(|element| element.end >= range.end)
                .map(|element| element.end);
            if let (Some(start), Some(end)) = (start, end) {
                out.push(start..end);
            }
        }
        if group.elements.len() > 1 {
            out.push(first.start..last.end);
        }
    }
    out.retain(|candidate| candidate.start <= range.start && range.end <= candidate.end);
    out
}

/// What a text object captures (`@function.outer`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Function,
    Class,
    Comment,
    ParameterList,
}

/// One text object of the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextObject {
    pub kind: ObjectKind,
    /// `outer` is the whole construct; `inner` its body when the pattern marks one.
    pub outer: Range<usize>,
    pub inner: Option<Range<usize>>,
}

/// Every function, class, comment and parameter-list object of the document.
pub fn objects(snap: &SyntaxSnapshot) -> Vec<TextObject> {
    let Some(query) = queryset::query(snap.lang(), QueryKind::TextObjects) else {
        return Vec::new();
    };
    let text = snap.text();
    let found = queryset::captures_in(query, snap.tree(), text, 0..text.len());
    let mut out: Vec<TextObject> = Vec::new();
    for captured in &found {
        let kind = match captured.name {
            "function.outer" => ObjectKind::Function,
            "class.outer" => ObjectKind::Class,
            "comment.outer" => ObjectKind::Comment,
            "parameter.list" => ObjectKind::ParameterList,
            _ => continue,
        };
        out.push(TextObject {
            kind,
            outer: captured.node.byte_range(),
            inner: None,
        });
    }
    for captured in &found {
        if captured.name != "function.inner" {
            continue;
        }
        let inner = captured.node.byte_range();
        if let Some(object) = out
            .iter_mut()
            .filter(|object| {
                object.kind == ObjectKind::Function
                    && object.outer.start <= inner.start
                    && inner.end <= object.outer.end
            })
            .min_by_key(|object| object.outer.len())
        {
            object.inner = Some(inner);
        }
    }
    out.sort_by_key(|object| (object.outer.start, std::cmp::Reverse(object.outer.end)));
    out.dedup_by(|a, b| a.kind == b.kind && a.outer == b.outer);
    out
}

/// Start offsets of the method/function names of the document, ascending
/// (Next/Previous Method targets).
pub fn method_starts(snap: &SyntaxSnapshot) -> Vec<usize> {
    fn walk(items: &[crate::StructureItem], out: &mut Vec<usize>) {
        for item in items {
            if matches!(
                item.kind,
                crate::SymbolKind::Function
                    | crate::SymbolKind::Method
                    | crate::SymbolKind::Constructor
            ) {
                out.push(item.name_range.start);
            }
            walk(&item.children, out);
        }
    }
    let mut out = Vec::new();
    walk(&snap.structure(), &mut out);
    out.sort_unstable();
    out.dedup();
    out
}

/// The next method name start after `offset`.
pub fn next_method(snap: &SyntaxSnapshot, offset: usize) -> Option<usize> {
    method_starts(snap)
        .into_iter()
        .find(|start| *start > offset)
}

/// The previous method name start before `offset`.
pub fn previous_method(snap: &SyntaxSnapshot, offset: usize) -> Option<usize> {
    method_starts(snap)
        .into_iter()
        .rev()
        .find(|start| *start < offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, source: &str) -> SyntaxSnapshot {
        let spec = crate::registry::by_id(id).expect("language");
        SyntaxSnapshot::parse(Default::default(), Default::default(), spec, source.into())
            .expect("parse")
    }

    #[test]
    fn argument_lists_offer_the_first_to_k_elements() {
        let source = "fn f() { g(aaa, bbb, ccc); }\n";
        let snapshot = snap("rust", source);
        let at = source.find("bbb").unwrap();
        let found = group_candidates(&snapshot, &(at..at + 3));
        let texts: Vec<_> = found.iter().map(|r| &source[r.clone()]).collect();
        assert!(texts.contains(&"bbb"), "{texts:?}");
        assert!(texts.contains(&"aaa, bbb"), "{texts:?}");
        assert!(texts.contains(&"aaa, bbb, ccc"), "{texts:?}");
    }

    #[test]
    fn methods_are_navigation_targets() {
        let source = "class A {\n    void f() {}\n    void g() {}\n}\n";
        let snapshot = snap("java", source);
        let f = source.find("f()").unwrap();
        let g = source.find("g()").unwrap();
        assert_eq!(method_starts(&snapshot), vec![f, g]);
        assert_eq!(next_method(&snapshot, 0), Some(f));
        assert_eq!(next_method(&snapshot, f), Some(g));
        assert_eq!(next_method(&snapshot, g), None);
        assert_eq!(previous_method(&snapshot, g + 5), Some(g));
        assert_eq!(previous_method(&snapshot, f), None);
    }
}
