//! Code folding regions from the syntax tree. A language's `folds.scm` says
//! which nodes fold (`@fold`, `@fold.comment`, `@fold.doc`, `@fold.imports`,
//! `@fold.open`/`@fold.close`, `@fold.marker`; `#set! placeholder`, `#set!
//! collapsed`, `#set! group`); without one, regions come from generic
//! structure: bracket- or tag-delimited nodes, block comments and runs of
//! import declarations that span several lines. Custom `region` /
//! `endregion` comments fold in every language that marks them.

use crate::queryset::{self, QueryKind};
use crate::registry::LanguageSpec;
use std::ops::Range;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

/// What a folding region contains; used for defaults such as collapsing
/// imports when a file opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldKind {
    Block,
    Comment,
    /// A documentation comment (`/** ... */`).
    DocComment,
    Imports,
    /// A method or function body holding a single one-line statement,
    /// shown as `{ statement }`; collapsed by default.
    OneLineBody,
    /// A user-defined region between `region` and `endregion` comments.
    Region,
}

impl FoldKind {
    /// Collapsed when a file opens unless the pattern says otherwise (IntelliJ
    /// Editor | General | Code Folding defaults: imports and one-line bodies).
    pub fn collapsed_by_default(self) -> bool {
        matches!(self, FoldKind::Imports | FoldKind::OneLineBody)
    }
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
    /// Collapsed when the file opens (`#set! collapsed` or the kind's default).
    pub collapsed_by_default: bool,
    /// Regions of one group collapse and expand together (`#set! group "name"`).
    pub group: Option<u32>,
}

impl FoldRange {
    /// A region with the kind's default collapse state and no group.
    pub fn new(
        start_byte: usize,
        end_byte: usize,
        start_line: usize,
        end_line: usize,
        placeholder: String,
        kind: FoldKind,
    ) -> Self {
        Self {
            start_byte,
            end_byte,
            start_line,
            end_line,
            placeholder,
            kind,
            collapsed_by_default: kind.collapsed_by_default(),
            group: None,
        }
    }
}

fn text<'a>(source: &'a str, node: Node<'_>) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}

const OPENERS: &[&str] = &["{", "[", "(", "<", "\"\"\"", "'''", "`"];

fn rows(node: Node<'_>) -> (usize, usize) {
    (node.start_position().row, node.end_position().row)
}

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
        return Some(FoldRange::new(
            first.end_byte(),
            last.start_byte(),
            first.end_position().row,
            last.start_position().row,
            "...".into(),
            FoldKind::Block,
        ));
    }
    if first.is_named() || last.is_named() || !OPENERS.contains(&text(source, first)) {
        return None;
    }
    let (start_line, end_line) = (first.start_position().row, last.end_position().row);
    if let Some(placeholder) = one_line_body(node, source) {
        return Some(FoldRange::new(
            first.start_byte(),
            last.end_byte(),
            start_line,
            end_line,
            placeholder,
            FoldKind::OneLineBody,
        ));
    }
    Some(FoldRange::new(
        first.start_byte(),
        last.end_byte(),
        start_line,
        end_line,
        format!("{}...{}", text(source, first), text(source, last)),
        FoldKind::Block,
    ))
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

fn comment(node: Node<'_>, source: &str, kind: FoldKind) -> Option<FoldRange> {
    if node.start_position().row == node.end_position().row {
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
    let (start_line, end_line) = rows(node);
    Some(FoldRange::new(
        node.start_byte(),
        node.end_byte(),
        start_line,
        end_line,
        format!("{head}...{closing}"),
        kind,
    ))
}

/// A `@fold` node without delimiters (Python blocks, YAML mappings, Markdown
/// sections): folds from the end of the text before the node (the header) to
/// the end of the node.
fn headed(node: Node<'_>, source: &str) -> Option<FoldRange> {
    let start = source.get(..node.start_byte())?.trim_end().len();
    let end = node.end_byte();
    let (_, end_line) = rows(node);
    // Line of the header's last character.
    let start_line = source[..start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    (start_line < end_line && start < end).then(|| {
        FoldRange::new(
            start,
            end,
            start_line,
            end_line,
            "...".into(),
            FoldKind::Block,
        )
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
    Some(FoldRange::new(
        start_byte,
        last.end_byte(),
        first.start_position().row,
        last.end_position().row,
        "...".into(),
        FoldKind::Imports,
    ))
}

fn intersects(node: Node<'_>, range: &Range<usize>) -> bool {
    node.start_byte() < range.end.max(range.start + 1) && node.end_byte() > range.start
}

/// Generic regions for a grammar without a `folds.scm`.
fn collect(node: Node<'_>, source: &str, range: &Range<usize>, out: &mut Vec<FoldRange>) {
    // Single-line nodes cannot contain multi-line regions.
    if node.start_position().row == node.end_position().row || !intersects(node, range) {
        return;
    }
    let comment_fold = node
        .kind()
        .contains("comment")
        .then(|| comment(node, source, FoldKind::Comment))
        .flatten();
    if let Some(fold) = comment_fold.or_else(|| delimited(node, source)) {
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

fn group_id(name: &str) -> u32 {
    name.bytes().fold(2_166_136_261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    })
}

/// Region markers in comments: `region Title` / `endregion`, `#region` /
/// `#endregion`, `<editor-fold desc="Title">` / `</editor-fold>`.
#[derive(Debug, PartialEq, Eq)]
enum Marker {
    Start(String),
    End,
}

fn marker(comment_text: &str) -> Option<Marker> {
    let mut body = comment_text.trim();
    for prefix in ["<!--", "//", "/*", "#", "--", "!", ";"] {
        if let Some(rest) = body.strip_prefix(prefix) {
            body = rest;
            break;
        }
    }
    let body = body.trim_end_matches("-->").trim_end_matches("*/").trim();
    let body = body.strip_prefix('#').unwrap_or(body).trim_start();
    if let Some(rest) = body.strip_prefix("<editor-fold") {
        let title = rest
            .split_once("desc=\"")
            .and_then(|(_, tail)| tail.split_once('"'))
            .map(|(title, _)| title.to_owned())
            .unwrap_or_default();
        return Some(Marker::Start(title));
    }
    if body.starts_with("</editor-fold") {
        return Some(Marker::End);
    }
    if let Some(rest) = body.strip_prefix("endregion") {
        return (rest.is_empty() || rest.starts_with(char::is_whitespace)).then_some(Marker::End);
    }
    let rest = body.strip_prefix("region")?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace))
        .then(|| Marker::Start(rest.trim().to_owned()))
}

fn region_folds(markers: &[Node<'_>], source: &str, out: &mut Vec<FoldRange>) {
    let mut stack: Vec<(Node<'_>, String)> = Vec::new();
    for node in markers {
        match marker(text(source, *node)) {
            Some(Marker::Start(title)) => stack.push((*node, title)),
            Some(Marker::End) => {
                let Some((start, title)) = stack.pop() else {
                    continue;
                };
                let (start_line, end_line) = (start.start_position().row, node.end_position().row);
                if start_line < end_line {
                    let placeholder = if title.is_empty() {
                        "region".to_owned()
                    } else {
                        title
                    };
                    out.push(FoldRange::new(
                        start.start_byte(),
                        node.end_byte(),
                        start_line,
                        end_line,
                        placeholder,
                        FoldKind::Region,
                    ));
                }
            }
            None => {}
        }
    }
}

fn from_query(
    query: &Query,
    tree: &Tree,
    source: &str,
    range: &Range<usize>,
    out: &mut Vec<FoldRange>,
) {
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range.start..range.end.max(range.start + 1));
    let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());
    let mut import_nodes: Vec<(Node<'_>, bool)> = Vec::new();
    let mut markers: Vec<Node<'_>> = Vec::new();
    while let Some(found) = matches.next() {
        let collapsed = queryset::setting(query, found.pattern_index, "collapsed").is_some();
        let placeholder = queryset::setting(query, found.pattern_index, "placeholder");
        let group = queryset::setting(query, found.pattern_index, "group").map(group_id);
        let (mut open, mut close) = (None, None);
        for capture in found.captures {
            let node = capture.node;
            let fold = match names[capture.index as usize] {
                "fold" => delimited(node, source).or_else(|| headed(node, source)),
                "fold.comment" => comment(node, source, FoldKind::Comment),
                "fold.doc" => comment(node, source, FoldKind::DocComment),
                "fold.imports" => {
                    import_nodes.push((node, collapsed));
                    None
                }
                "fold.marker" => {
                    markers.push(node);
                    None
                }
                "fold.open" => {
                    open = Some(node);
                    None
                }
                "fold.close" => {
                    close = Some(node);
                    None
                }
                _ => None,
            };
            if let Some(mut fold) = fold {
                if let Some(placeholder) = placeholder.filter(|value| !value.is_empty()) {
                    fold.placeholder = placeholder.to_owned();
                }
                fold.collapsed_by_default |= collapsed;
                fold.group = group;
                out.push(fold);
            }
        }
        if let (Some(open), Some(close)) = (open, close)
            && open.end_position().row < close.start_position().row
        {
            let placeholder = placeholder.filter(|value| !value.is_empty());
            let mut fold = FoldRange::new(
                open.end_byte(),
                close.start_byte(),
                open.end_position().row,
                close.start_position().row,
                placeholder.unwrap_or("...").to_owned(),
                FoldKind::Block,
            );
            fold.collapsed_by_default |= collapsed;
            fold.group = group;
            out.push(fold);
        }
    }
    // Runs of consecutive sibling imports fold as one region.
    import_nodes.sort_by_key(|(node, _)| node.start_byte());
    import_nodes.dedup_by_key(|(node, _)| node.id());
    let mut run: Vec<Node<'_>> = Vec::new();
    let mut run_collapsed = false;
    let flush = |run: &mut Vec<Node<'_>>, collapsed: bool, out: &mut Vec<FoldRange>| {
        if let Some(mut fold) = imports(run, source) {
            fold.collapsed_by_default |= collapsed;
            out.push(fold);
        }
        run.clear();
    };
    for (node, collapsed) in import_nodes {
        let adjacent = run.last().is_some_and(|last| {
            last.next_named_sibling()
                .is_some_and(|next| next.id() == node.id())
        });
        if !adjacent {
            flush(&mut run, run_collapsed, out);
        }
        run_collapsed = collapsed;
        run.push(node);
    }
    flush(&mut run, run_collapsed, out);
    markers.sort_by_key(|node| node.start_byte());
    region_folds(&markers, source, out);
}

/// Drops regions that cross another one (partial overlap): regions must be
/// disjoint or nested so gutter markers and collapsing stay consistent.
/// `sorted` is ordered by start, outermost first.
fn drop_crossing(sorted: Vec<FoldRange>) -> Vec<FoldRange> {
    let mut kept: Vec<FoldRange> = Vec::with_capacity(sorted.len());
    let mut open: Vec<usize> = Vec::new();
    for fold in sorted {
        while let Some(&top) = open.last() {
            if kept[top].end_byte <= fold.start_byte {
                open.pop();
            } else {
                break;
            }
        }
        if let Some(&top) = open.last()
            && fold.end_byte > kept[top].end_byte
        {
            continue;
        }
        open.push(kept.len());
        kept.push(fold);
    }
    kept
}

/// Folding regions whose syntax nodes intersect the byte `range` (use
/// `0..source.len()` for all), sorted by start; at most one region starts on
/// any line (the outermost wins, as gutter markers are per line) and regions
/// never cross. Only nodes intersecting `range` are visited, so viewport
/// queries stay cheap.
pub fn fold_ranges(
    spec: &'static LanguageSpec,
    tree: &Tree,
    source: &str,
    range: Range<usize>,
) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    match queryset::query(spec, QueryKind::Folds) {
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
    ranges.sort_by(|a, b| {
        a.start_byte
            .cmp(&b.start_byte)
            .then(b.end_byte.cmp(&a.end_byte))
    });
    ranges.dedup_by(|later, earlier| {
        later.start_byte == earlier.start_byte && later.end_byte == earlier.end_byte
    });
    drop_crossing(ranges)
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
        let source = "class A {\n    void f() {\n        g();\n    }\n    void h() {\n        i();\n    }\n}\n";
        let spec = registry::by_id("java").expect("java");
        let syntax = SyntaxTree::parse(spec, source).expect("parse");
        let at = source.find("i();").unwrap();
        let found = fold_ranges(spec, syntax.tree(), source, at..at);
        let lines: Vec<_> = found
            .iter()
            .map(|fold| (fold.start_line, fold.end_line))
            .collect();
        assert_eq!(lines, vec![(0, 7), (4, 6)]);
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
                (5, 7, FoldKind::Comment, "/**...*/"),
                (8, 12, FoldKind::Block, "{...}"),
                (9, 11, FoldKind::OneLineBody, "{ g(); }"),
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
