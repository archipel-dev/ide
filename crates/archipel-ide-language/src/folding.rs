//! Code folding regions from the syntax tree. A grammar's bundled
//! `folds.scm` is used when it ships one; otherwise regions come from
//! generic structure: bracket- or tag-delimited nodes, block comments and
//! runs of import declarations that span several lines.

use crate::registry::{LANGUAGES, LanguageSpec};
use std::ops::Range;
use std::sync::OnceLock;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

/// What a folding region contains; used for defaults such as collapsing
/// imports when a file opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldKind {
    Block,
    Comment,
    Imports,
    /// A method or function body holding a single one-line statement,
    /// shown as `{ statement }`; collapsed by default.
    OneLineBody,
}

/// Which setting decides whether a region starts collapsed (Settings | Editor | General |
/// Code Folding, ED-FOLD-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FoldDefault {
    /// Starts expanded.
    #[default]
    Expanded,
    Imports,
    MethodBody,
    OneLineMethod,
    DocComment,
    FileHeader,
    CustomRegion,
    /// Starts collapsed whatever the settings (a NetBeans `defaultstate="collapsed"`).
    Collapsed,
}

/// A foldable region. When collapsed, the editor shows the text before
/// `start_byte` on `start_line`, then `placeholder`, then the text after
/// `end_byte` on `end_line`; lines `start_line + 1 ..= end_line` are hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldRange {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub placeholder: String,
    pub kind: FoldKind,
    /// The setting that decides the first-time state.
    pub default: FoldDefault,
    /// Regions with the same non-zero group expand and collapse together (the two halves of a
    /// one-line method).
    pub group: usize,
}

fn folds_query(spec: &'static LanguageSpec) -> Option<&'static Query> {
    static CACHE: OnceLock<Vec<OnceLock<Option<Query>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| LANGUAGES.iter().map(|_| OnceLock::new()).collect());
    let index = LANGUAGES
        .iter()
        .position(|candidate| std::ptr::eq(candidate, spec))?;
    cache[index]
        .get_or_init(|| {
            spec.folds
                .and_then(|source| Query::new(&spec.grammar(), source).ok())
        })
        .as_ref()
}

fn text<'a>(source: &'a str, node: Node<'_>) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}

const OPENERS: &[&str] = &["{", "[", "(", "<", "\"\"\"", "'''", "`"];

/// A node delimited by an opening and closing token on different lines
/// (`{ … }`, `[ … ]`, `<tag> … </tag>`).
fn delimited(node: Node<'_>, source: &str) -> Option<FoldRange> {
    let count = node.child_count();
    if count < 2 {
        return None;
    }
    let first = node.child(0)?;
    let last = node.child(count - 1)?;
    if first.end_position().row >= last.start_position().row {
        return None;
    }
    let tag =
        |child: Node<'_>| child.is_named() && child.kind().to_ascii_lowercase().contains("tag");
    if tag(first) && tag(last) {
        // `<a …>` … `</a>`: keep both tags visible around the placeholder.
        return Some(FoldRange {
            start_byte: first.end_byte(),
            end_byte: last.start_byte(),
            start_line: first.end_position().row,
            end_line: last.start_position().row,
            placeholder: "...".into(),
            kind: FoldKind::Block,
            default: FoldDefault::Expanded,
            group: 0,
        });
    }
    if first.is_named() || last.is_named() || !OPENERS.contains(&text(source, first)) {
        return None;
    }
    if let Some(placeholder) = one_line_body(node, source) {
        return Some(FoldRange {
            start_byte: first.start_byte(),
            end_byte: last.end_byte(),
            start_line: first.start_position().row,
            end_line: last.end_position().row,
            placeholder,
            kind: FoldKind::OneLineBody,
            default: FoldDefault::OneLineMethod,
            group: 0,
        });
    }
    Some(FoldRange {
        start_byte: first.start_byte(),
        end_byte: last.end_byte(),
        start_line: first.start_position().row,
        end_line: last.end_position().row,
        placeholder: format!("{}...{}", text(source, first), text(source, last)),
        kind: FoldKind::Block,
        default: FoldDefault::Expanded,
        group: 0,
    })
}

/// `{ statement }` for a method/function body block holding exactly one
/// statement that fits on one line (and is short enough to read inline).
fn one_line_body(block: Node<'_>, source: &str) -> Option<String> {
    let parent = block.parent()?;
    let parent_kind = parent.kind();
    if !(parent_kind.contains("method") || parent_kind.contains("function")) {
        return None;
    }
    let mut cursor = block.walk();
    let mut statements = block.named_children(&mut cursor);
    let statement = statements.next()?;
    if statements.next().is_some()
        || statement.kind().contains("comment")
        || statement.start_position().row != statement.end_position().row
        || statement.end_byte() - statement.start_byte() > 80
    {
        return None;
    }
    Some(format!("{{ {} }}", text(source, statement)))
}

fn comment(node: Node<'_>, source: &str) -> Option<FoldRange> {
    if !node.kind().contains("comment") || node.start_position().row == node.end_position().row {
        return None;
    }
    let body = text(source, node);
    let first_line = body.lines().next().unwrap_or_default().trim_end();
    let closing = if body.trim_end().ends_with("*/") {
        "*/"
    } else {
        ""
    };
    let head: String = first_line.chars().take(60).collect();
    Some(FoldRange {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: node.start_position().row,
        end_line: node.end_position().row,
        placeholder: format!("{head}...{closing}"),
        kind: FoldKind::Comment,
        default: FoldDefault::Expanded,
        group: 0,
    })
}

fn is_import(node: Node<'_>) -> bool {
    let kind = node.kind();
    node.is_named() && (kind.contains("import") || kind.contains("using_declaration"))
}

/// Collapses a run of consecutive import nodes into one `import ...` region.
fn imports(run: &[Node<'_>], source: &str) -> Option<FoldRange> {
    let first = *run.first()?;
    let last = *run.last()?;
    if first.start_position().row >= last.end_position().row {
        return None;
    }
    // Keep the leading keyword (`import`, `from`, …) visible: the first leaf
    // token, also when the run is a single container such as `import_list`.
    let mut keyword = first.child(0)?;
    while let Some(child) = keyword.child(0) {
        keyword = child;
    }
    if keyword.end_byte() >= first.end_byte() {
        return None;
    }
    let start_byte = keyword.end_byte() + 1;
    if start_byte >= last.end_byte() || !source.is_char_boundary(start_byte) {
        return None;
    }
    Some(FoldRange {
        start_byte,
        end_byte: last.end_byte(),
        start_line: first.start_position().row,
        end_line: last.end_position().row,
        placeholder: "...".into(),
        kind: FoldKind::Imports,
        default: FoldDefault::Imports,
        group: 0,
    })
}

fn intersects(node: Node<'_>, range: &Range<usize>) -> bool {
    node.start_byte() < range.end.max(range.start + 1) && node.end_byte() > range.start
}

fn collect(node: Node<'_>, source: &str, range: &Range<usize>, out: &mut Vec<FoldRange>) {
    // Single-line nodes cannot contain multi-line regions.
    if node.start_position().row == node.end_position().row || !intersects(node, range) {
        return;
    }
    if let Some(fold) = comment(node, source).or_else(|| delimited(node, source)) {
        out.push(fold);
    }
    let mut cursor = node.walk();
    // Skip children that end before the range (an import run that starts
    // above the range is only reported when the range covers its start).
    if cursor.goto_first_child_for_byte(range.start).is_none() {
        return;
    }
    let mut run: Vec<Node<'_>> = Vec::new();
    loop {
        let child = cursor.node();
        if is_import(child) {
            run.push(child);
        } else {
            if !run.is_empty() {
                out.extend(imports(&run, source).filter(|fold| fold.end_byte > range.start));
                run.clear();
            }
            if child.start_byte() >= range.end.max(range.start + 1) {
                break;
            }
            collect(child, source, range, out);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    out.extend(imports(&run, source).filter(|fold| fold.end_byte > range.start));
}

fn from_query(
    query: &Query,
    tree: &Tree,
    source: &str,
    range: &Range<usize>,
    out: &mut Vec<FoldRange>,
) {
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range.start..range.end.max(range.start + 1));
    let mut captures = cursor.captures(query, tree.root_node(), source.as_bytes());
    while let Some((query_match, index)) = captures.next() {
        let node = query_match.captures[*index].node;
        let (start, end) = (node.start_position(), node.end_position());
        if start.row >= end.row {
            continue;
        }
        // Fold after the node's first line: `def f(x):...`.
        let line_end = source[node.start_byte()..node.end_byte()]
            .find('\n')
            .map_or(node.end_byte(), |offset| node.start_byte() + offset);
        out.push(FoldRange {
            start_byte: line_end,
            end_byte: node.end_byte(),
            start_line: start.row,
            end_line: end.row,
            placeholder: "...".into(),
            kind: FoldKind::Block,
            default: FoldDefault::Expanded,
            group: 0,
        });
    }
}

/// Folding regions whose syntax nodes intersect the byte `range` (use
/// `0..source.len()` for all), sorted by start; at most one region starts on
/// any line (the outermost wins, as gutter markers are per line). Only nodes
/// intersecting `range` are visited, so viewport queries stay cheap.
pub fn fold_ranges(
    spec: &'static LanguageSpec,
    tree: &Tree,
    source: &str,
    range: Range<usize>,
) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    if spec.id == "java" {
        let mut regions = crate::folding_java::fold_regions(tree, source);
        regions
            .retain(|r| r.start_byte < range.end.max(range.start + 1) && r.end_byte > range.start);
        return regions;
    }
    match folds_query(spec) {
        Some(query) => from_query(query, tree, source, &range, &mut ranges),
        None => collect(tree.root_node(), source, &range, &mut ranges),
    }
    ranges.retain(|range| range.start_line < range.end_line && range.start_byte <= range.end_byte);
    ranges.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then(b.end_byte.cmp(&a.end_byte))
    });
    ranges.dedup_by(|later, earlier| later.start_line == earlier.start_line);
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyntaxTree, registry};

    fn folds(id: &str, source: &str) -> Vec<FoldRange> {
        let spec = registry::by_id(id).expect("language");
        let syntax = SyntaxTree::parse(spec, source).expect("parse");
        fold_ranges(spec, syntax.tree(), source, 0..source.len())
    }

    #[test]
    fn range_queries_return_enclosing_and_contained_regions() {
        let source = "class A {\n    void f() {\n        g();\n        g();\n    }\n    void h() {\n        i();\n        j();\n    }\n}\n";
        let spec = registry::by_id("java").expect("java");
        let syntax = SyntaxTree::parse(spec, source).expect("parse");
        let at = source.find("i();").unwrap();
        let found = fold_ranges(spec, syntax.tree(), source, at..at);
        let lines: Vec<_> = found
            .iter()
            .map(|fold| (fold.start_line, fold.end_line))
            .collect();
        assert_eq!(lines, vec![(5, 8)]);
    }

    #[test]
    fn java_blocks_comments_and_imports() {
        let source = "package a;\n\nimport java.util.List;\nimport java.util.Map;\n\n/**\n * Doc.\n */\nclass A {\n    void f() {\n        g();\n    }\n}\n";
        let found = folds("java", source);
        let summary: Vec<_> = found
            .iter()
            .map(|fold| {
                (
                    fold.start_line,
                    fold.end_line,
                    fold.kind,
                    fold.placeholder.as_str(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                (2, 3, FoldKind::Imports, "..."),
                (5, 7, FoldKind::Comment, "/** Doc. */"),
                (9, 10, FoldKind::OneLineBody, " { "),
                (10, 11, FoldKind::OneLineBody, " }"),
            ]
        );
        assert_eq!(
            &source[found[0].start_byte..found[0].start_byte + 4],
            "java"
        );
    }

    #[test]
    fn kotlin_xml_and_starlark_regions() {
        let kotlin = "import a.B\nimport a.C\n\nclass M {\n    fun f() {\n    }\n}\n";
        let found = folds("kotlin", kotlin);
        let imports = found
            .iter()
            .find(|fold| fold.kind == FoldKind::Imports)
            .expect("imports");
        assert_eq!(imports.start_line, 0);
        assert_eq!(&kotlin[..imports.start_byte], "import ");
        assert!(
            found
                .iter()
                .any(|fold| fold.start_line == 3 && fold.end_line == 6)
        );

        let xml = "<a>\n  <b>\n    text\n  </b>\n</a>\n";
        let found = folds("xml", xml);
        assert_eq!(found.len(), 2);
        assert_eq!(&xml[..found[0].start_byte], "<a>");
        assert!(xml[found[0].end_byte..].starts_with("</a>"));

        let bazel =
            "java_binary(\n    name = \"app\",\n    srcs = [\n        \"A.java\",\n    ],\n)\n";
        let found = folds("starlark", bazel);
        assert!(
            found
                .iter()
                .any(|fold| fold.start_line == 0 && fold.end_line == 5)
        );
        assert!(
            found
                .iter()
                .any(|fold| fold.start_line == 2 && fold.end_line == 4)
        );
    }

    #[test]
    fn single_line_and_plain_text_have_no_regions() {
        assert!(folds("java", "class A { void f() {} }\n").is_empty());
        assert!(folds("json", "{\"a\": 1}").is_empty());
    }
}
