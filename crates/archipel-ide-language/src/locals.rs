//! File-local name resolution from `locals.scm` (tier G1).
//!
//! Captures: `@local.scope` (a node that opens a scope), `@local.definition`
//! and `@local.definition.<kind>` (a declared name; `#set! scope "parent"`
//! declares it in the scope around the node that owns the name, so a
//! function's name lives outside the function) and `@local.reference` (a use).
//!
//! A reference resolves to the nearest enclosing scope that declares its name:
//! within a scope the latest declaration before the use wins (shadowing);
//! functions, classes, methods, imports and constants are visible before their
//! declaration. Names that follow a receiver (`a.name`, `Foo::name`) and
//! keyword-argument names are not references.

use crate::node::SyntaxSnapshot;
use crate::queryset::{self, QueryKind};
use std::ops::Range;
use tree_sitter::Node;

/// A declared name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDef {
    pub name: String,
    /// The part after `local.definition.` (`variable`, `parameter`, ...), empty if none.
    pub kind: String,
    pub range: Range<usize>,
    /// The scope the name is visible in.
    pub scope: Range<usize>,
}

/// A use of a name and the declaration it resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRef {
    pub name: String,
    pub range: Range<usize>,
    /// Index into [`Locals::defs`].
    pub def: Option<usize>,
}

/// Scopes, declarations and resolved references of one file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Locals {
    pub scopes: Vec<Range<usize>>,
    pub defs: Vec<LocalDef>,
    pub refs: Vec<LocalRef>,
}

impl Locals {
    /// The declaration the name at `offset` refers to (or is).
    pub fn definition_at(&self, offset: usize) -> Option<&LocalDef> {
        if let Some(def) = self
            .defs
            .iter()
            .find(|d| d.range.start <= offset && offset <= d.range.end)
        {
            return Some(def);
        }
        let reference = self
            .refs
            .iter()
            .find(|r| r.range.start <= offset && offset <= r.range.end)?;
        reference.def.and_then(|index| self.defs.get(index))
    }

    /// Every use of `def` (indices into [`Locals::defs`]).
    pub fn uses_of(&self, def: usize) -> impl Iterator<Item = &LocalRef> {
        self.refs.iter().filter(move |r| r.def == Some(def))
    }
}

const HOISTED: &[&str] = &[
    "function", "method", "class", "import", "constant", "type", "field", "module",
];

fn is_selector(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if parent.kind().ends_with("_suffix") {
        return true;
    }
    let is_field = |name: &str| {
        parent
            .child_by_field_name(name)
            .is_some_and(|c| c.id() == node.id())
    };
    if parent.kind() == "keyword_argument" && is_field("name") {
        return true;
    }
    let member = ["attribute", "field", "property", "member", "name"]
        .iter()
        .any(|f| is_field(f));
    let has_receiver = ["object", "operand", "receiver", "scope", "path"]
        .iter()
        .any(|f| parent.child_by_field_name(f).is_some());
    member && has_receiver
}

/// Resolves the names of `snap`; empty without a `locals.scm`.
pub fn locals(snap: &SyntaxSnapshot) -> Locals {
    let Some(query) = queryset::query(snap.lang(), QueryKind::Locals) else {
        return Locals::default();
    };
    let text = snap.text();
    let found = queryset::captures_in(query, snap.tree(), text, 0..text.len());
    let mut scopes: Vec<Range<usize>> = Vec::new();
    let mut raw_defs: Vec<(String, Range<usize>, bool)> = Vec::new();
    let mut raw_refs: Vec<(String, Range<usize>, usize)> = Vec::new();
    let mut def_nodes: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for captured in &found {
        let range = captured.node.byte_range();
        if captured.name == "local.scope" {
            scopes.push(range);
        } else if let Some(rest) = captured.name.strip_prefix("local.definition") {
            let kind = rest.trim_start_matches('.').to_owned();
            let parent = queryset::setting(query, captured.pattern, "scope") == Some("parent");
            def_nodes.insert(captured.node.id());
            raw_defs.push((kind, range, parent));
        } else if captured.name == "local.reference" && !is_selector(captured.node) {
            raw_refs.push((text[range.clone()].to_owned(), range, captured.node.id()));
        }
    }
    scopes.sort_by_key(|s| (s.start, std::cmp::Reverse(s.end)));
    scopes.dedup();
    // Scopes containing `range`, innermost first.
    let containing = |range: &Range<usize>| -> Vec<Range<usize>> {
        let mut list: Vec<Range<usize>> = scopes
            .iter()
            .filter(|s| s.start <= range.start && range.end <= s.end)
            .cloned()
            .collect();
        list.sort_by_key(|s| s.len());
        list
    };
    let mut defs: Vec<LocalDef> = Vec::new();
    for (kind, range, parent) in raw_defs {
        let chain = containing(&range);
        let scope = if parent {
            chain.get(1).or(chain.first())
        } else {
            chain.first()
        };
        let Some(scope) = scope else { continue };
        let name = text[range.clone()].to_owned();
        if name.is_empty() {
            continue;
        }
        defs.push(LocalDef {
            name,
            kind,
            range,
            scope: scope.clone(),
        });
    }
    defs.sort_by_key(|d| d.range.start);
    defs.dedup_by(|a, b| a.range == b.range);
    let mut refs: Vec<LocalRef> = Vec::new();
    for (name, range, id) in raw_refs {
        if def_nodes.contains(&id) || defs.iter().any(|d| d.range == range) {
            continue;
        }
        let mut resolved = None;
        for scope in containing(&range) {
            let candidates: Vec<usize> = defs
                .iter()
                .enumerate()
                .filter(|(_, d)| d.scope == scope && d.name == name)
                .map(|(index, _)| index)
                .collect();
            let before = candidates
                .iter()
                .copied()
                .filter(|index| defs[*index].range.start <= range.start)
                .max_by_key(|index| defs[*index].range.start);
            let hoisted = candidates
                .iter()
                .copied()
                .find(|index| HOISTED.contains(&defs[*index].kind.as_str()));
            if let Some(found) = before.or(hoisted) {
                resolved = Some(found);
                break;
            }
        }
        refs.push(LocalRef {
            name,
            range,
            def: resolved,
        });
    }
    refs.sort_by_key(|r| r.range.start);
    refs.dedup_by(|a, b| a.range == b.range);
    Locals { scopes, defs, refs }
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
    fn shadowing_and_hoisting_resolve_to_the_nearest_declaration() {
        let source = "fn main() {\n    let x = 1;\n    {\n        let x = 2;\n        use_it(x);\n    }\n    use_it(x);\n    helper();\n}\nfn helper() {}\nfn use_it(_v: i32) {}\n";
        let found = locals(&snap("rust", source));
        let at = |needle: &str, nth: usize| source.match_indices(needle).nth(nth).unwrap().0;
        let inner_use = at("x)", 0);
        let outer_use = at("x)", 1);
        let inner_def = found.definition_at(inner_use).expect("inner");
        assert_eq!(inner_def.range.start, at("x = 2", 0));
        let outer_def = found.definition_at(outer_use).expect("outer");
        assert_eq!(outer_def.range.start, at("x = 1", 0));
        let call = found
            .definition_at(at("helper();", 0))
            .expect("hoisted function");
        assert_eq!(call.kind, "function");
    }

    #[test]
    fn member_names_are_not_references() {
        let source = "def f(a):\n    b = a.value\n    return b\n";
        let found = locals(&snap("python", source));
        assert!(found.refs.iter().all(|r| r.name != "value"));
        let b = source.rfind('b').unwrap();
        assert_eq!(
            found.definition_at(b).map(|d| d.kind.as_str()),
            Some("variable")
        );
    }
}
