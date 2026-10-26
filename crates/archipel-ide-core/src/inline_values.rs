//! Inline variable values: end-of-line `name = value` hints shown in the editor while
//! the debugger is suspended.
//!
//! Pure text analysis over the suspended frame's locals; the debugger session supplies
//! the values, the editor paints the hints.

use crate::IdeState;
use archipel_ide_dap::SessionState;
use std::path::Path;

/// Longest value shown inline; longer ones are cut with an ellipsis.
const MAX_VALUE_CHARS: usize = 40;
/// How far above the suspended line a method body is searched.
const MAX_LOOKBACK: usize = 300;

/// One hint: the text goes after the end of `line` (zero-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineValue {
    pub line: usize,
    pub text: String,
}

/// Hints for the lines from the enclosing method's start through `frame_line`, one per
/// line that mentions a local, like `x = 1, name = "a"`.
pub fn inline_values(
    source: &str,
    frame_line: usize,
    locals: &[(String, String)],
) -> Vec<InlineValue> {
    if locals.is_empty() {
        return Vec::new();
    }
    let lines: Vec<&str> = source.lines().collect();
    if frame_line >= lines.len() {
        return Vec::new();
    }
    let mut start = frame_line;
    let floor = frame_line.saturating_sub(MAX_LOOKBACK);
    while start > floor && !is_method_start(lines[start]) {
        start -= 1;
    }
    // The signature line itself holds the parameters; its hints would repeat the
    // Variables view, so the body starts after it.
    if is_method_start(lines[start]) && start < frame_line {
        start += 1;
    }
    let mut values = Vec::new();
    for (index, line) in lines.iter().enumerate().take(frame_line + 1).skip(start) {
        let code = strip_comments_and_strings(line);
        let mut shown = Vec::new();
        for (name, value) in locals {
            if shown
                .iter()
                .any(|(existing, _): &(&str, &str)| existing == name)
            {
                continue;
            }
            if mentions(&code, name) {
                shown.push((name.as_str(), value.as_str()));
            }
        }
        if shown.is_empty() {
            continue;
        }
        let text = shown
            .iter()
            .map(|(name, value)| format!("{name} = {}", shorten(value)))
            .collect::<Vec<_>>()
            .join(", ");
        values.push(InlineValue { line: index, text });
    }
    values
}

fn shorten(value: &str) -> String {
    let value = value.trim();
    if value.chars().count() <= MAX_VALUE_CHARS {
        return value.to_owned();
    }
    let cut: String = value.chars().take(MAX_VALUE_CHARS).collect();
    format!("{cut}…")
}

/// A Java/Kotlin-like declaration line: `… name(…) {` that is not a control statement.
fn is_method_start(line: &str) -> bool {
    let trimmed = line.trim();
    if !(trimmed.ends_with('{') && trimmed.contains('(') && trimmed.contains(')')) {
        return false;
    }
    let first = trimmed
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("");
    !matches!(
        first,
        "if" | "for"
            | "while"
            | "switch"
            | "catch"
            | "else"
            | "try"
            | "do"
            | "synchronized"
            | "return"
            | "new"
            | "when"
    ) && !trimmed.contains('=')
        || trimmed.starts_with("fun ")
}

/// Blanks `// comments`, string and char literals so names inside them do not match.
fn strip_comments_and_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == '\\' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
                out.push(' ');
            }
            None => match c {
                '"' | '\'' => {
                    quote = Some(c);
                    out.push(' ');
                }
                '/' if chars.peek() == Some(&'/') => break,
                _ => out.push(c),
            },
        }
    }
    out
}

/// Whether `name` occurs as a whole identifier that is not a member access (`a.name`).
fn mentions(code: &str, name: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let mut from = 0;
    while let Some(found) = code[from..].find(name) {
        let start = from + found;
        let end = start + name.len();
        let before = code[..start].chars().next_back();
        let after = code[end..].chars().next();
        let member = code[..start].trim_end().ends_with('.');
        if !before.is_some_and(is_ident) && !after.is_some_and(is_ident) && !member {
            return true;
        }
        from = end;
    }
    false
}

impl IdeState {
    /// Locals of the suspended frame as `(name, value)`, from the first scope that has
    /// variables loaded (Locals before Globals/Registers).
    fn frame_locals(&self) -> Vec<(String, String)> {
        let Some(client) = self.debugger.client.as_ref() else {
            return Vec::new();
        };
        let session = client.session();
        let scope = session
            .scopes()
            .iter()
            .find(|scope| scope.name.to_ascii_lowercase().contains("local"))
            .or_else(|| session.scopes().first());
        scope
            .and_then(|scope| session.variables(scope.reference))
            .map(|variables| {
                variables
                    .iter()
                    .filter(|variable| variable.name != "this")
                    .map(|variable| (variable.name.clone(), variable.value.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Inline value hints for the document at `path` (project-relative) while suspended
    /// in it; empty otherwise.
    pub fn inline_values(&self, path: &Path) -> Vec<InlineValue> {
        let Some(client) = self.debugger.client.as_ref() else {
            return Vec::new();
        };
        if !matches!(client.session().state(), SessionState::Stopped { .. }) {
            return Vec::new();
        }
        let Some(point) = self.execution_point() else {
            return Vec::new();
        };
        if point.path != path {
            return Vec::new();
        }
        let Some(document) = self.documents.iter().find(|document| document.path == path) else {
            return Vec::new();
        };
        let source = document.text.text();
        inline_values(&source, point.line, &self.frame_locals())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locals(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
            .collect()
    }

    const SOURCE: &str = "\
public class App {
    static int square(int n) {
        int result = n * n;
        // result is cached
        String label = \"result\";
        return result;
    }
}
";

    #[test]
    fn hints_follow_the_locals_up_to_the_suspended_line() {
        let values = inline_values(
            SOURCE,
            5,
            &locals(&[("n", "{int} 3"), ("result", "9"), ("label", "\"result\"")]),
        );
        assert_eq!(
            values,
            vec![
                InlineValue {
                    line: 2,
                    text: "n = {int} 3, result = 9".into()
                },
                InlineValue {
                    line: 4,
                    text: "label = \"result\"".into()
                },
                InlineValue {
                    line: 5,
                    text: "result = 9".into()
                },
            ]
        );
    }

    #[test]
    fn comments_strings_and_members_do_not_match() {
        let values = inline_values(SOURCE, 5, &locals(&[("cached", "1"), ("length", "2")]));
        assert!(values.is_empty());
        let source = "void f() {\n    a.n = \"n\"; // n\n}\n";
        assert!(inline_values(source, 1, &locals(&[("n", "1")])).is_empty());
    }

    #[test]
    fn other_methods_above_are_not_annotated() {
        let source = "void a() {\n    int x = 1;\n}\nvoid b() {\n    int x = 2;\n    use(x);\n}\n";
        let values = inline_values(source, 5, &locals(&[("x", "2")]));
        assert_eq!(
            values.iter().map(|v| v.line).collect::<Vec<_>>(),
            vec![4, 5]
        );
    }

    #[test]
    fn long_values_are_cut() {
        let long = "x".repeat(100);
        let source = "void a() {\n    use(v);\n}\n";
        let values = inline_values(source, 1, &locals(&[("v", &long)]));
        assert!(values[0].text.ends_with('…'));
        assert!(values[0].text.chars().count() < 60);
    }

    #[test]
    fn out_of_range_lines_and_empty_locals_are_ignored() {
        assert!(inline_values(SOURCE, 99, &locals(&[("n", "1")])).is_empty());
        assert!(inline_values(SOURCE, 2, &[]).is_empty());
    }
}
