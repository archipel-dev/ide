//! Java folding regions (ED-FOLD-030 to ED-FOLD-040): imports, nested class bodies, method
//! bodies (and the one-line method in two grouped regions), inner blocks, comments, the file
//! header and comment-based custom regions.

use crate::folding::{FoldDefault, FoldKind, FoldRange};
use tree_sitter::{Node, Tree};

/// Width a folded one-line method may have (the right margin).
const RIGHT_MARGIN: usize = 120;

fn text<'a>(source: &'a str, node: Node<'_>) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}

fn region(
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    placeholder: impl Into<String>,
    kind: FoldKind,
    default: FoldDefault,
) -> FoldRange {
    FoldRange {
        start_byte,
        end_byte,
        start_line,
        end_line,
        placeholder: placeholder.into(),
        kind,
        default,
        group: 0,
    }
}

const TYPE_DECLARATIONS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "record_declaration",
    "annotation_type_declaration",
];

/// All regions of a Java file, sorted by start.
pub fn fold_regions(tree: &Tree, source: &str) -> Vec<FoldRange> {
    let root = tree.root_node();
    let mut out = Vec::new();
    let mut cursor = root.walk();
    let children: Vec<Node<'_>> = root.children(&mut cursor).collect();
    imports(&children, source, &mut out);
    let top_level_types = children
        .iter()
        .filter(|n| TYPE_DECLARATIONS.contains(&n.kind()))
        .count();
    visit(root, source, top_level_types, &mut out);
    comments(root, source, &mut out);
    out.retain(|r| {
        r.start_line < r.end_line || r.kind == FoldKind::Block && r.start_byte < r.end_byte
    });
    out.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then(a.start_byte.cmp(&b.start_byte))
            .then(b.end_byte.cmp(&a.end_byte))
    });
    // One region starts on a line, except the two halves of one grouped fold.
    let mut kept: Vec<FoldRange> = Vec::with_capacity(out.len());
    for fold in out {
        match kept.last() {
            Some(previous)
                if previous.start_line == fold.start_line
                    && !(fold.group != 0 && fold.group == previous.group) =>
            {
                continue;
            }
            _ => kept.push(fold),
        }
    }
    kept
}

fn imports(children: &[Node<'_>], source: &str, out: &mut Vec<FoldRange>) {
    let run: Vec<Node<'_>> = children
        .iter()
        .copied()
        .filter(|n| n.kind() == "import_declaration")
        .collect();
    let (Some(first), Some(last)) = (run.first(), run.last()) else {
        return;
    };
    if run.len() < 2 {
        return;
    }
    // After the first `import` keyword and its blank.
    let Some(keyword) = first.child(0) else {
        return;
    };
    let start_byte = keyword.end_byte() + 1;
    if start_byte >= last.end_byte() || !source.is_char_boundary(start_byte) {
        return;
    }
    out.push(region(
        start_byte,
        last.end_byte(),
        first.start_position().row,
        last.end_position().row,
        "...",
        FoldKind::Imports,
        FoldDefault::Imports,
    ));
}

fn visit(node: Node<'_>, source: &str, top_level_types: usize, out: &mut Vec<FoldRange>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if TYPE_DECLARATIONS.contains(&kind) {
            if let Some(body) = child.child_by_field_name("body") {
                let top_level = child.parent().is_some_and(|p| p.kind() == "program");
                // A single top-level class has no region (ED-FOLD-032).
                if !(top_level && top_level_types == 1) {
                    delimited(body, "{...}", FoldDefault::Expanded, out);
                }
            }
        } else if kind == "method_declaration" || kind == "constructor_declaration" {
            if let Some(body) = child.child_by_field_name("body") {
                method_body(child, body, source, out);
            }
        } else if matches!(kind, "block" | "constructor_body" | "switch_block")
            && !child.parent().is_some_and(|p| {
                matches!(p.kind(), "method_declaration" | "constructor_declaration")
            })
        {
            // A nested block with at least one statement.
            if child.named_child_count() > 0 {
                delimited(child, "{...}", FoldDefault::Expanded, out);
            }
        }
        visit(child, source, top_level_types, out);
    }
}

/// `{ ... }` from the opening to the closing token, on different lines.
fn delimited(node: Node<'_>, placeholder: &str, default: FoldDefault, out: &mut Vec<FoldRange>) {
    if node.start_position().row == node.end_position().row {
        return;
    }
    out.push(region(
        node.start_byte(),
        node.end_byte(),
        node.start_position().row,
        node.end_position().row,
        placeholder,
        FoldKind::Block,
        default,
    ));
}

fn method_body(method: Node<'_>, body: Node<'_>, source: &str, out: &mut Vec<FoldRange>) {
    if body.start_position().row == body.end_position().row {
        return;
    }
    let mut cursor = body.walk();
    let statements: Vec<Node<'_>> = body
        .named_children(&mut cursor)
        .filter(|n| !n.kind().contains("comment"))
        .collect();
    if statements.is_empty() {
        return;
    }
    if statements.len() == 1
        && let Some(previous) = body.prev_sibling()
    {
        let statement = statements[0];
        let header_on_one_line = method.start_position().row == body.start_position().row
            || previous.end_position().row == body.start_position().row;
        let tight = statement.start_position().row == body.start_position().row + 1
            && statement.end_position().row == statement.start_position().row
            && body.end_position().row == statement.end_position().row + 1;
        let header_width =
            previous.end_position().column + 3 + text(source, statement).chars().count() + 2;
        if header_on_one_line && tight && header_width <= RIGHT_MARGIN {
            let group = previous.end_byte() + 1;
            let mut first = region(
                previous.end_byte(),
                statement.start_byte(),
                previous.end_position().row,
                statement.start_position().row,
                " { ",
                FoldKind::OneLineBody,
                FoldDefault::OneLineMethod,
            );
            let mut second = region(
                statement.end_byte(),
                body.end_byte(),
                statement.end_position().row,
                body.end_position().row,
                " }",
                FoldKind::OneLineBody,
                FoldDefault::OneLineMethod,
            );
            first.group = group;
            second.group = group;
            out.push(first);
            out.push(second);
            return;
        }
    }
    delimited(body, "{...}", FoldDefault::MethodBody, out);
}

/// The text of a comment without its delimiters and line prefixes.
fn comment_lines(comment: &str) -> Vec<String> {
    comment
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches("/**")
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .trim_start_matches('*')
                .trim()
                .to_owned()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

/// What a custom folding comment says (ED-FOLD-040).
enum Marker {
    Start {
        placeholder: String,
        collapsed: bool,
        style: u8,
    },
    End {
        style: u8,
    },
}

fn marker(comment: &str) -> Option<Marker> {
    let body = comment.trim_start_matches(|c: char| {
        c == '/' || c == '*' || c == '#' || c == '-' || c.is_whitespace()
    });
    let body = body.trim_end_matches("*/").trim_end();
    if let Some(rest) = body.strip_prefix("endregion") {
        let _ = rest;
        return Some(Marker::End { style: 0 });
    }
    if let Some(rest) = body.strip_prefix("region") {
        let rest = rest.trim();
        let placeholder = if rest.is_empty() { "..." } else { rest };
        return Some(Marker::Start {
            placeholder: placeholder.to_owned(),
            collapsed: false,
            style: 0,
        });
    }
    if comment.contains("</editor-fold") {
        return Some(Marker::End { style: 1 });
    }
    if comment.contains("<editor-fold") {
        let desc = comment
            .split("desc=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or("...");
        return Some(Marker::Start {
            placeholder: if desc.is_empty() { "..." } else { desc }.to_owned(),
            collapsed: comment.contains("defaultstate=\"collapsed\""),
            style: 1,
        });
    }
    None
}

fn collect_comments<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().contains("comment") {
            out.push(child);
        } else {
            collect_comments(child, out);
        }
    }
}

fn comments(root: Node<'_>, source: &str, out: &mut Vec<FoldRange>) {
    let mut all = Vec::new();
    collect_comments(root, &mut all);
    // Custom regions pair their comments like brackets.
    let mut stack: Vec<(Node<'_>, String, bool, u8)> = Vec::new();
    let mut markers: Vec<bool> = vec![false; all.len()];
    for (index, comment) in all.iter().enumerate() {
        let body = text(source, *comment);
        match marker(body) {
            Some(Marker::Start {
                placeholder,
                collapsed,
                style,
            }) => {
                markers[index] = true;
                stack.push((*comment, placeholder, collapsed, style));
            }
            Some(Marker::End { style }) => {
                markers[index] = true;
                if let Some(position) = stack.iter().rposition(|(_, _, _, s)| *s == style) {
                    let (start, placeholder, collapsed, _) = stack.remove(position);
                    stack.truncate(position);
                    out.push(region(
                        start.start_byte(),
                        comment.end_byte(),
                        start.start_position().row,
                        comment.end_position().row,
                        placeholder,
                        FoldKind::Comment,
                        if collapsed {
                            FoldDefault::Collapsed
                        } else {
                            FoldDefault::CustomRegion
                        },
                    ));
                }
            }
            None => {}
        }
    }
    // Block comments and runs of line comments.
    let first_code = {
        let mut cursor = root.walk();
        root.children(&mut cursor)
            .find(|n| !n.kind().contains("comment"))
            .map(|n| (n.start_byte(), n.kind().to_owned()))
    };
    let mut index = 0;
    while index < all.len() {
        let comment = all[index];
        if markers[index] {
            index += 1;
            continue;
        }
        let body = text(source, comment);
        let is_header = comment.parent().is_some_and(|p| p.kind() == "program")
            && first_code.as_ref().is_none_or(|(code_start, kind)| {
                comment.end_byte() <= *code_start
                    && (matches!(kind.as_str(), "package_declaration" | "import_declaration")
                        || !body.starts_with("/**"))
            })
            && source[..comment.start_byte()].trim().is_empty();
        if comment.kind() == "block_comment" {
            if comment.start_position().row != comment.end_position().row {
                let doc = body.starts_with("/**");
                let (placeholder, default, kind) = if is_header {
                    (
                        "/.../".to_owned(),
                        FoldDefault::FileHeader,
                        FoldKind::Comment,
                    )
                } else if doc {
                    let lines = comment_lines(body);
                    let first = lines.first().cloned().unwrap_or_default();
                    let placeholder = if lines.len() > 1 {
                        format!("/** {first} ... */")
                    } else {
                        format!("/** {first} */")
                    };
                    (placeholder, FoldDefault::DocComment, FoldKind::Comment)
                } else {
                    (
                        "/*...*/".to_owned(),
                        FoldDefault::Expanded,
                        FoldKind::Comment,
                    )
                };
                out.push(region(
                    comment.start_byte(),
                    comment.end_byte(),
                    comment.start_position().row,
                    comment.end_position().row,
                    placeholder,
                    kind,
                    default,
                ));
            }
            index += 1;
            continue;
        }
        // A run of adjacent line comments (no custom region marker among them).
        let mut last = index;
        while last + 1 < all.len()
            && all[last + 1].kind() == "line_comment"
            && !markers[last + 1]
            && all[last + 1].start_position().row == all[last].end_position().row + 1
        {
            last += 1;
        }
        if last > index {
            let end = all[last];
            out.push(region(
                comment.start_byte(),
                end.end_byte(),
                comment.start_position().row,
                end.end_position().row,
                if is_header { "/.../" } else { "//..." },
                FoldKind::Comment,
                if is_header {
                    FoldDefault::FileHeader
                } else {
                    FoldDefault::Expanded
                },
            ));
        }
        index = last + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyntaxTree, registry};

    fn regions(source: &str) -> Vec<FoldRange> {
        let spec = registry::by_id("java").expect("java");
        let syntax = SyntaxTree::parse(spec, source).expect("parse");
        fold_regions(syntax.tree(), source)
    }

    #[test]
    fn one_line_method_is_two_grouped_regions() {
        let source = "class A {\n    int f() {\n        return 1;\n    }\n}\n";
        let found = regions(source);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].placeholder, " { ");
        assert_eq!(found[1].placeholder, " }");
        assert_ne!(found[0].group, 0);
        assert_eq!(found[0].group, found[1].group);
    }

    #[test]
    fn a_single_top_level_class_has_no_region() {
        let source = "class A {\n    class B {\n        int x;\n    }\n}\n";
        let found = regions(source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].start_line, 1);
    }

    #[test]
    fn custom_regions_pair_their_comments() {
        let source = "class A {\n    // region Helpers\n    void f() {}\n    // endregion\n}\n";
        let found = regions(source);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].placeholder, "Helpers");
        assert_eq!(found[0].default, FoldDefault::CustomRegion);
    }
}
