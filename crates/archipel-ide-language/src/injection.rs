//! Language injections driven by the grammars' bundled `injections.scm`
//! (Markdown code fences and inline grammar, HTML `<script>`/`<style>`, Rust
//! macro bodies, JS tagged templates). Follows the `tree-sitter-highlight`
//! conventions: `@injection.content`, `@injection.language` or the
//! `injection.language` property, `injection.combined` and
//! `injection.include-children`; legacy captures named after the language.

use crate::registry::{self, LANGUAGES, LanguageSpec};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::OnceLock;
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator, Tree};

/// Nesting limit for injections inside injections (e.g. Rust macros).
pub(crate) const MAX_DEPTH: usize = 4;

/// One injected region: a language and the byte ranges it is parsed from.
pub(crate) struct Injection {
    pub spec: &'static LanguageSpec,
    pub ranges: Vec<tree_sitter::Range>,
}

impl Injection {
    pub fn extent(&self) -> Range<usize> {
        let start = self.ranges.first().map_or(0, |range| range.start_byte);
        let end = self.ranges.last().map_or(0, |range| range.end_byte);
        start..end
    }

    /// Parses the injected ranges with the injected language's grammar,
    /// reusing one parser per language and thread.
    pub fn parse(&self, source: &str) -> Option<Tree> {
        thread_local! {
            static PARSERS: RefCell<HashMap<&'static str, Parser>> = RefCell::new(HashMap::new());
        }
        PARSERS.with(|parsers| {
            let mut parsers = parsers.borrow_mut();
            let parser = match parsers.entry(self.spec.id) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let mut parser = Parser::new();
                    parser.set_language(&self.spec.grammar()).ok()?;
                    entry.insert(parser)
                }
            };
            parser.set_included_ranges(&self.ranges).ok()?;
            let tree = parser.parse(source, None);
            parser.reset();
            tree
        })
    }
}

fn query(spec: &'static LanguageSpec) -> Option<&'static Query> {
    static CACHE: OnceLock<Vec<OnceLock<Option<Query>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| LANGUAGES.iter().map(|_| OnceLock::new()).collect());
    let index = LANGUAGES
        .iter()
        .position(|candidate| std::ptr::eq(candidate, spec))?;
    cache[index]
        .get_or_init(|| {
            spec.injections
                .and_then(|source| Query::new(&spec.grammar(), source).ok())
        })
        .as_ref()
}

/// Ranges of `node` to parse: the whole node, or the node minus its named
/// children (anonymous tokens such as Markdown's inline delimiters stay).
fn node_ranges(node: Node<'_>, include_children: bool) -> Vec<tree_sitter::Range> {
    if include_children || node.named_child_count() == 0 {
        return vec![node.range()];
    }
    let mut ranges = Vec::new();
    let mut start_byte = node.start_byte();
    let mut start_point = node.start_position();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.start_byte() > start_byte {
            ranges.push(tree_sitter::Range {
                start_byte,
                end_byte: child.start_byte(),
                start_point,
                end_point: child.start_position(),
            });
        }
        start_byte = child.end_byte();
        start_point = child.end_position();
    }
    if node.end_byte() > start_byte {
        ranges.push(tree_sitter::Range {
            start_byte,
            end_byte: node.end_byte(),
            start_point,
            end_point: node.end_position(),
        });
    }
    ranges
}

/// Finds the injections whose content intersects `range`.
pub(crate) fn injections(
    spec: &'static LanguageSpec,
    tree: &Tree,
    source: &str,
    range: Range<usize>,
) -> Vec<Injection> {
    let Some(query) = query(spec) else {
        return Vec::new();
    };
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range);
    let mut found = Vec::new();
    let mut combined: HashMap<(usize, &'static str), usize> = HashMap::new();
    let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());
    while let Some(query_match) = matches.next() {
        let properties = query.property_settings(query_match.pattern_index);
        let property = |key: &str| properties.iter().find(|property| &*property.key == key);
        let mut language = property("injection.language")
            .and_then(|property| property.value.as_deref())
            .map(str::to_owned);
        let is_combined = property("injection.combined").is_some();
        let include_children = property("injection.include-children").is_some();
        let mut content = Vec::new();
        for capture in query_match.captures {
            let name = query.capture_names()[capture.index as usize];
            match name {
                "injection.language" => {
                    language = source
                        .get(capture.node.byte_range())
                        .map(|text| text.trim().to_owned());
                }
                "injection.content" => content.push(capture.node),
                name if !name.starts_with('_') && !name.starts_with("injection.") => {
                    language.get_or_insert_with(|| name.to_owned());
                    content.push(capture.node);
                }
                _ => {}
            }
        }
        let Some(injected) = language.as_deref().and_then(registry::by_id) else {
            continue;
        };
        let ranges: Vec<_> = content
            .into_iter()
            .flat_map(|node| node_ranges(node, include_children))
            .filter(|range| range.start_byte < range.end_byte)
            .collect();
        if ranges.is_empty() {
            continue;
        }
        if is_combined {
            let key = (query_match.pattern_index, injected.id);
            if let Some(&index) = combined.get(&key) {
                let injection: &mut Injection = &mut found[index];
                injection.ranges.extend(ranges);
                continue;
            }
            combined.insert(key, found.len());
        }
        found.push(Injection {
            spec: injected,
            ranges,
        });
    }
    for injection in &mut found {
        injection.ranges.sort_by_key(|range| range.start_byte);
        injection
            .ranges
            .dedup_by(|later, earlier| later.start_byte < earlier.end_byte);
    }
    found
}

/// True when no injected region parses with syntax errors. Error recovery in
/// an injected tree can depend on its absolute position, so only error-free
/// injections are guaranteed to re-highlight identically after edits elsewhere.
pub(crate) fn error_free(spec: &'static LanguageSpec, tree: &Tree, source: &str) -> bool {
    injections(spec, tree, source, 0..source.len())
        .iter()
        .all(|injection| {
            injection
                .parse(source)
                .is_some_and(|tree| !tree.root_node().has_error())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_injection_query_compiles() {
        for spec in LANGUAGES.iter().filter(|spec| spec.injections.is_some()) {
            assert!(
                query(spec).is_some(),
                "{} injections do not compile",
                spec.id
            );
        }
    }

    #[test]
    fn markdown_fences_and_html_scripts_are_injected() {
        let source = "# Title\n\n```kotlin\nfun main() {}\n```\n";
        let spec = registry::by_id("markdown").expect("markdown");
        let tree = crate::SyntaxTree::parse(spec, source).expect("parse");
        let found = injections(spec, tree.tree(), source, 0..source.len());
        assert!(found.iter().any(|injection| injection.spec.id == "kotlin"));
        assert!(
            found
                .iter()
                .any(|injection| injection.spec.id == "markdown_inline")
        );

        let source = "<script>let x = 1;</script>";
        let spec = registry::by_id("html").expect("html");
        let tree = crate::SyntaxTree::parse(spec, source).expect("parse");
        let found = injections(spec, tree.tree(), source, 0..source.len());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].spec.id, "javascript");
        assert_eq!(&source[found[0].extent()], "let x = 1;");
    }
}
