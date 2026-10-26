//! Inspection suppression.
//!
//! Generic form (every language with comments): a comment
//! `noinspection <id>[, <id>...]` (`ALL` suppresses everything) on the line(s)
//! directly above a statement or declaration, or trailing on its first line,
//! suppresses that tool for the statement and everything inside it. A comment
//! `noinspection-file <id>...` before the first code of the file suppresses
//! the tool for the whole file. Language-native forms: Java
//! `@SuppressWarnings("id")`, Rust `#[allow(id)]`/`#[allow(archipel::id)]`
//! (and the rustc lint names mapped in [`rust_lint_aliases`]).
//!
//! The generic "Suppress for statement/declaration/file" quick fixes are
//! generated from the same rules by [`suppress_edit`].

use super::kinds::{block_comment, kinds_of, line_comment_prefix};
use archipel_ide_analysis::inspect::InspectionSuppressor;
use archipel_ide_analysis::{FileId, ModCommand, SyntaxNode, SyntaxSnapshot, TextEdit};
use std::ops::Range;

/// What a suppression comment covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressScope {
    Statement,
    Declaration,
    File,
}

/// Ids listed after `marker` in `comment`, split on commas/spaces.
fn listed_ids<'a>(comment: &'a str, marker: &str) -> Option<Vec<&'a str>> {
    let at = comment.find(marker)?;
    let rest = &comment[at + marker.len()..];
    // `noinspection` must not be the prefix of `noinspection-file`.
    if marker == "noinspection" && rest.starts_with("-file") {
        return None;
    }
    let rest = rest
        .trim_start_matches([' ', '\t', ':'])
        .trim_end_matches(['*', '/', '-', '>', ' ', '\t', '\r', '\n']);
    let line = rest.lines().next().unwrap_or("");
    Some(
        line.split([',', ' ', '\t'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

fn comment_suppresses(comment: &str, marker: &str, tool: &str) -> bool {
    listed_ids(comment, marker).is_some_and(|ids| ids.iter().any(|id| *id == tool || *id == "ALL"))
}

fn is_comment(node: SyntaxNode<'_>) -> bool {
    let lang = node.snapshot().lang();
    lang.token_set_table()
        .comments
        .contains(node.element_type())
        || node.kind().contains("comment")
}

/// Whether only whitespace with at most one line break separates
/// `from..to` in the file text.
fn adjacent(text: &str, from: usize, to: usize) -> bool {
    from <= to
        && text.get(from..to).is_some_and(|gap| {
            gap.chars().all(char::is_whitespace) && gap.matches('\n').count() <= 1
        })
}

fn is_statement(node: SyntaxNode<'_>) -> bool {
    let lang = node.snapshot().lang();
    lang.token_set_table()
        .statement_like
        .contains(node.element_type())
        || node.parent().is_some_and(|parent| {
            kinds_of(lang.id).is_some_and(|k| k.containers.contains(&parent.kind()))
        })
}

fn is_declaration(node: SyntaxNode<'_>) -> bool {
    let kind = node.kind();
    node.snapshot()
        .lang()
        .token_set_table()
        .declaration_like
        .contains(node.element_type())
        || [
            "_declaration",
            "_definition",
            "_item",
            "_method",
            "_function",
        ]
        .iter()
        .any(|suffix| kind.ends_with(suffix))
}

/// Is `node` something a comment can annotate (statement or declaration)?
fn annotatable(node: SyntaxNode<'_>) -> bool {
    let sets = node.snapshot().lang().token_set_table();
    if sets.statement_like.is_empty() && sets.declaration_like.is_empty() {
        return node.is_named();
    }
    is_statement(node)
        || is_declaration(node)
        || node.parent().is_some_and(|p| p.parent().is_none())
}

/// Comments attached to `node`: the contiguous ones directly above and a
/// trailing one on its first line.
fn attached_comments<'s>(node: SyntaxNode<'s>) -> Vec<SyntaxNode<'s>> {
    let text = node.snapshot().text();
    let mut out = Vec::new();
    // Python's block starts where its first statement does: comments above
    // the block annotate that statement.
    let mut node = node;
    while let Some(parent) = node.parent()
        && parent.range().start == node.range().start
    {
        node = parent;
    }
    let mut start = node.range().start as usize;
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        if is_comment(sibling) && adjacent(text, sibling.range().end as usize, start) {
            start = sibling.range().start as usize;
            out.push(sibling);
            prev = sibling.prev_sibling();
        } else {
            break;
        }
    }
    let end = node.range().end as usize;
    let mut next = node.next_sibling();
    while let Some(sibling) = next {
        let gap = text
            .get(end..sibling.range().start as usize)
            .unwrap_or("\n");
        if is_comment(sibling) && !gap.contains('\n') {
            out.push(sibling);
            break;
        }
        if !sibling.is_named() || is_comment(sibling) {
            next = sibling.next_sibling();
            continue;
        }
        break;
    }
    out
}

/// Leading comments of the file (before the first non-comment node).
fn file_header_comments<'s>(root: SyntaxNode<'s>) -> Vec<SyntaxNode<'s>> {
    root.children()
        .take_while(|child| is_comment(*child) || !child.is_named())
        .filter(|child| is_comment(*child))
        .collect()
}

/// The language-agnostic comment suppressor (registered for every language).
#[derive(Debug, Default, Clone, Copy)]
pub struct CommentSuppressor;

impl InspectionSuppressor for CommentSuppressor {
    fn is_suppressed(&self, node: SyntaxNode<'_>, tool_id: &str) -> bool {
        let root = node.snapshot().root();
        if file_header_comments(root)
            .iter()
            .any(|c| comment_suppresses(c.text(), "noinspection-file", tool_id))
        {
            return true;
        }
        let mut current = Some(node);
        while let Some(n) = current {
            if annotatable(n)
                && attached_comments(n)
                    .iter()
                    .any(|c| comment_suppresses(c.text(), "noinspection", tool_id))
            {
                return true;
            }
            current = n.parent();
        }
        false
    }
}

fn quoted_strings(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        out.push(&after[..close]);
        rest = &after[close + 1..];
    }
    out
}

/// Java: `@SuppressWarnings("id")` / `({"a", "b"})` on a declaration.
#[derive(Debug, Default, Clone, Copy)]
pub struct JavaSuppressor;

impl InspectionSuppressor for JavaSuppressor {
    fn is_suppressed(&self, node: SyntaxNode<'_>, tool_id: &str) -> bool {
        let mut current = Some(node);
        while let Some(n) = current {
            if n.kind().ends_with("_declaration") {
                for child in n.children() {
                    if child.kind() != "modifiers" {
                        continue;
                    }
                    for annotation in child.children() {
                        let text = annotation.text();
                        if text.starts_with("@SuppressWarnings")
                            && quoted_strings(text)
                                .iter()
                                .any(|id| *id == tool_id || *id == "all")
                        {
                            return true;
                        }
                    }
                }
            }
            current = n.parent();
        }
        false
    }
}

/// Our ids -> the rustc/clippy lint names that also suppress them.
pub fn rust_lint_aliases(tool_id: &str) -> &'static [&'static str] {
    match tool_id {
        "UnreachableCode" => &["unreachable_code"],
        "SelfAssignment" => &["clippy::self_assignment"],
        "RedundantParentheses" => &["unused_parens"],
        "ConstantCondition" => &["clippy::overly_complex_bool_expr"],
        "DuplicateBranches" => &["clippy::if_same_then_else"],
        "EmptyStatementBody" => &["clippy::needless_bool"],
        _ => &[],
    }
}

/// Rust: `#[allow(archipel::Id)]`, `#[allow(Id)]`, mapped lint names, on the
/// item or an enclosing one, or `#![allow(..)]` at the top of the file.
#[derive(Debug, Default, Clone, Copy)]
pub struct RustSuppressor;

fn allows(attribute: &str, tool_id: &str) -> bool {
    let Some(open) = attribute
        .find("allow(")
        .or_else(|| attribute.find("expect("))
    else {
        return false;
    };
    let inner = &attribute[open..];
    let inner = inner.split_once('(').map_or("", |(_, rest)| rest);
    let inner = inner.rsplit_once(')').map_or(inner, |(body, _)| body);
    let aliases = rust_lint_aliases(tool_id);
    inner.split(',').map(str::trim).any(|lint| {
        lint == tool_id || lint == format!("archipel::{tool_id}") || aliases.contains(&lint)
    })
}

impl InspectionSuppressor for RustSuppressor {
    fn is_suppressed(&self, node: SyntaxNode<'_>, tool_id: &str) -> bool {
        let root = node.snapshot().root();
        if root
            .children()
            .filter(|c| c.kind() == "inner_attribute_item")
            .any(|c| allows(c.text(), tool_id))
        {
            return true;
        }
        let mut current = Some(node);
        while let Some(n) = current {
            let mut prev = n.prev_sibling();
            while let Some(sibling) = prev {
                match sibling.kind() {
                    "attribute_item" => {
                        if allows(sibling.text(), tool_id) {
                            return true;
                        }
                    }
                    k if k.contains("comment") => {}
                    _ => break,
                }
                prev = sibling.prev_sibling();
            }
            current = n.parent();
        }
        false
    }
}

/// The text of the suppression comment for `tool_id` in `lang`, or `None`
/// when the language has no comment syntax we know.
fn comment_text(lang: &str, marker: &str, ids: &str) -> Option<String> {
    if let Some(prefix) = line_comment_prefix(lang) {
        return Some(format!("{prefix} {marker} {ids}"));
    }
    block_comment(lang).map(|(open, close)| format!("{open} {marker} {ids} {close}"))
}

fn line_indent(text: &str, offset: usize) -> &str {
    let start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let end = text[start..]
        .find(|c: char| c != ' ' && c != '\t')
        .map_or(text.len(), |i| start + i);
    &text[start..end.min(offset.max(start))]
}

/// The innermost ancestor-or-self of the node at `range` that is a
/// statement (`Statement`) or declaration (`Declaration`).
fn target_node<'s>(
    syntax: &'s SyntaxSnapshot,
    range: &Range<u32>,
    scope: SuppressScope,
) -> Option<SyntaxNode<'s>> {
    let mut current = Some(syntax.covering(range.clone()));
    while let Some(n) = current {
        let hit = match scope {
            SuppressScope::Statement => is_statement(n),
            SuppressScope::Declaration => is_declaration(n),
            SuppressScope::File => return None,
        };
        if hit {
            return Some(n);
        }
        current = n.parent();
    }
    None
}

/// Edit suppressing `tool_id` for the statement/declaration around `range`
/// or for the whole file (`None` = not possible in this language/place). An
/// existing suppression comment directly above gets the id appended.
pub fn suppress_edit(
    syntax: &SyntaxSnapshot,
    range: &Range<u32>,
    tool_id: &str,
    scope: SuppressScope,
) -> Option<Vec<TextEdit>> {
    let lang = syntax.lang().id;
    let text = syntax.text();
    if scope == SuppressScope::File {
        let header = file_header_comments(syntax.root());
        if let Some(existing) = header
            .iter()
            .find(|c| c.text().contains("noinspection-file"))
        {
            let end = existing.range().end as usize;
            let trimmed = text[..end]
                .trim_end_matches(['*', '/', '-', '>', ' '])
                .len();
            return Some(vec![TextEdit::insert(
                trimmed as u32,
                format!(", {tool_id}"),
            )]);
        }
        let comment = comment_text(lang, "noinspection-file", tool_id)?;
        return Some(vec![TextEdit::insert(0, format!("{comment}\n"))]);
    }
    let target = target_node(syntax, range, scope)?;
    let start = target.range().start as usize;
    if let Some(existing) = attached_comments(target)
        .into_iter()
        .find(|c| listed_ids(c.text(), "noinspection").is_some() && c.range().end as usize <= start)
    {
        let comment_end = existing.range().end as usize;
        let trimmed = text[..comment_end]
            .trim_end_matches(['*', '/', '-', '>', ' '])
            .len();
        return Some(vec![TextEdit::insert(
            trimmed as u32,
            format!(", {tool_id}"),
        )]);
    }
    let comment = comment_text(lang, "noinspection", tool_id)?;
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let indent = line_indent(text, start);
    // Only when the statement starts its line; otherwise a comment above
    // would annotate the wrong thing.
    if !text[line_start..start]
        .chars()
        .all(|c| c == ' ' || c == '\t')
    {
        return None;
    }
    Some(vec![TextEdit::insert(
        line_start as u32,
        format!("{indent}{comment}\n"),
    )])
}

/// [`suppress_edit`] as a command for `file`.
pub fn suppress_command(
    file: FileId,
    syntax: &SyntaxSnapshot,
    range: &Range<u32>,
    tool_id: &str,
    scope: SuppressScope,
) -> Option<ModCommand> {
    suppress_edit(syntax, range, tool_id, scope).map(|edits| ModCommand::UpdateFile {
        file,
        edits,
        expected_rev: syntax.rev(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_analysis::FileRev;
    use archipel_ide_analysis::modcommand::apply_edits;
    use archipel_ide_language::registry;
    use std::sync::Arc;

    fn parse(lang: &str, text: &str) -> SyntaxSnapshot {
        let def = registry::by_id(lang).unwrap();
        SyntaxSnapshot::parse(FileId(1), FileRev(1), def, Arc::from(text)).unwrap()
    }

    fn suppressed(lang: &str, text: &str, needle: &str, tool: &str) -> bool {
        let syntax = parse(lang, text);
        let at = text.find(needle).unwrap() as u32;
        let node = syntax.covering(at..at + needle.len() as u32);
        let mut all: Vec<Box<dyn InspectionSuppressor>> = vec![Box::new(CommentSuppressor)];
        match lang {
            "java" => all.push(Box::new(JavaSuppressor)),
            "rust" => all.push(Box::new(RustSuppressor)),
            _ => {}
        }
        all.iter().any(|s| s.is_suppressed(node, tool))
    }

    #[test]
    fn comment_above_a_statement_suppresses_it_and_what_is_inside() {
        let src = "class A {\n  void f() {\n    // noinspection EmptyCatchBlock, Other\n    try { g(); } catch (Exception e) { }\n    try { g(); } catch (Exception e2) { }\n  }\n}\n";
        assert!(suppressed("java", src, "{ }", "EmptyCatchBlock"));
        assert!(!suppressed("java", src, "e2) { }", "EmptyCatchBlock"));
        assert!(suppressed("java", src, "{ }", "Other"));
        assert!(!suppressed("java", src, "{ }", "Unrelated"));
        let all = src.replace("EmptyCatchBlock, Other", "ALL");
        assert!(suppressed("java", &all, "{ }", "Anything"));
    }

    #[test]
    fn a_comment_on_a_declaration_covers_its_body() {
        let src = "// noinspection UnreachableCode\nfn f() {\n    return;\n    g();\n}\nfn h() {\n    return;\n    g();\n}\n";
        let first = src.find("g();").unwrap();
        let second = src.rfind("g();").unwrap();
        let syntax = parse("rust", src);
        let covered = |at: usize| {
            CommentSuppressor
                .is_suppressed(syntax.covering(at as u32..at as u32 + 4), "UnreachableCode")
        };
        assert!(covered(first));
        assert!(!covered(second));
    }

    #[test]
    fn hash_comment_languages_and_trailing_comments_work() {
        let src =
            "a: 1\n# noinspection DuplicateKey\na: 2\nb: 1\nb: 2 # noinspection DuplicateKey\n";
        assert!(suppressed("yaml", src, "a: 2", "DuplicateKey"));
        assert!(suppressed("yaml", src, "b: 2", "DuplicateKey"));
        assert!(!suppressed("yaml", src, "b: 1", "DuplicateKey"));
        let py = "def f():\n    # noinspection ConstantCondition\n    if True:\n        pass\n";
        assert!(suppressed("python", py, "True", "ConstantCondition"));
    }

    #[test]
    fn file_level_comment_suppresses_everywhere() {
        let src = "// noinspection-file LongLine, TodoComment\nclass A {}\n";
        assert!(suppressed("java", src, "class A", "TodoComment"));
        assert!(!suppressed("java", src, "class A", "EmptyBlock"));
        // Not a header comment once code came first.
        let late = "class A {}\n// noinspection-file TodoComment\n";
        assert!(!suppressed("java", late, "class A", "TodoComment"));
    }

    #[test]
    fn java_suppress_warnings_annotation() {
        let src = "class A {\n  @SuppressWarnings({\"UnreachableCode\", \"x\"})\n  void f() { return; }\n  void g() { return; }\n}\n";
        assert!(suppressed("java", src, "return;", "UnreachableCode"));
        assert!(!suppressed("java", src, "return; }\n}", "UnreachableCode"));
        let all = "@SuppressWarnings(\"all\")\nclass A { void f() { } }\n";
        assert!(suppressed("java", all, "void f", "Whatever"));
    }

    #[test]
    fn rust_allow_attributes_and_aliases() {
        let src = "#[allow(archipel::SelfAssignment)]\nfn f() { a = a; }\n#[allow(unreachable_code)]\nfn g() { return; h(); }\nfn k() { return; h(); }\n";
        assert!(suppressed("rust", src, "a = a", "SelfAssignment"));
        let g = src.find("h();").unwrap();
        let syntax = parse("rust", src);
        assert!(
            RustSuppressor
                .is_suppressed(syntax.covering(g as u32..g as u32 + 4), "UnreachableCode")
        );
        let k = src.rfind("h();").unwrap();
        let syntax = parse("rust", src);
        assert!(
            !RustSuppressor
                .is_suppressed(syntax.covering(k as u32..k as u32 + 4), "UnreachableCode")
        );
        let inner = "#![allow(archipel::DuplicateBranches)]\nfn f() {}\n";
        assert!(suppressed("rust", inner, "fn f", "DuplicateBranches"));
    }

    #[test]
    fn suppress_fixes_insert_or_extend_comments() {
        let src = "fn f() {\n    return;\n    g();\n}\n";
        let syntax = parse("rust", src);
        let at = src.find("g();").unwrap() as u32;
        let edits = suppress_edit(
            &syntax,
            &(at..at + 4),
            "UnreachableCode",
            SuppressScope::Statement,
        )
        .unwrap();
        let out = apply_edits(src, &edits).unwrap();
        assert_eq!(
            out,
            "fn f() {\n    return;\n    // noinspection UnreachableCode\n    g();\n}\n"
        );
        // The inserted comment suppresses the problem.
        let again = parse("rust", &out);
        let at = out.find("g();").unwrap() as u32;
        assert!(CommentSuppressor.is_suppressed(again.covering(at..at + 4), "UnreachableCode"));
        // A second id extends the comment instead of adding another.
        let edits =
            suppress_edit(&again, &(at..at + 4), "Other", SuppressScope::Statement).unwrap();
        let extended = apply_edits(&out, &edits).unwrap();
        assert!(
            extended.contains("// noinspection UnreachableCode, Other\n"),
            "{extended}"
        );
        // Declaration and file scope.
        let edits = suppress_edit(&syntax, &(at..at + 4), "X", SuppressScope::Declaration)
            .unwrap_or_default();
        assert!(
            edits.is_empty()
                || apply_edits(src, &edits)
                    .unwrap()
                    .starts_with("// noinspection X\nfn f")
        );
        let file = suppress_edit(&syntax, &(0..1), "X", SuppressScope::File).unwrap();
        assert_eq!(
            apply_edits(src, &file).unwrap(),
            format!("// noinspection-file X\n{src}")
        );
        let xml = parse("xml", "<a/>");
        let file = suppress_edit(&xml, &(0..1), "X", SuppressScope::File).unwrap();
        assert!(
            apply_edits("<a/>", &file)
                .unwrap()
                .starts_with("<!-- noinspection-file X -->")
        );
        // JSON has no comments: no fix.
        assert!(suppress_edit(&parse("json", "{}"), &(0..1), "X", SuppressScope::File).is_none());
    }
}
