//! Selection formatting and insertion transforms. They are pure: given the text and a
//! selection (character indices, `(anchor, head)`), each returns one [`Edit`] that the
//! caller applies as a single undoable replacement.

use crate::model::{self, BlockKind};

/// Replace `start..end` (character indices) with `text`, then select `selection`
/// (character indices in the new text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub selection: (usize, usize),
}

impl Edit {
    /// Applies the edit to a string (used by tests and non-rope callers).
    pub fn apply(&self, source: &str) -> String {
        let chars: Vec<char> = source.chars().collect();
        let mut out: String = chars[..self.start.min(chars.len())].iter().collect();
        out.push_str(&self.text);
        out.extend(&chars[self.end.min(chars.len())..]);
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Bold,
    Italic,
    Strikethrough,
    Code,
}

impl Style {
    fn marker(self) -> (char, usize) {
        match self {
            Style::Bold => ('*', 2),
            Style::Italic => ('*', 1),
            Style::Strikethrough => ('~', 2),
            Style::Code => ('`', 1),
        }
    }

    /// Whether a delimiter run of `run` characters includes this style's marker.
    fn in_run(self, run: usize) -> bool {
        match self {
            Style::Italic => run % 2 == 1,
            other => run >= other.marker().1,
        }
    }
}

fn normalize(len: usize, selection: (usize, usize)) -> (usize, usize) {
    let (a, b) = selection;
    (a.min(b).min(len), a.max(b).min(len))
}

fn run_before(chars: &[char], at: usize, c: char) -> usize {
    chars[..at].iter().rev().take_while(|&&x| x == c).count()
}

fn run_after(chars: &[char], at: usize, c: char) -> usize {
    chars[at..].iter().take_while(|&&x| x == c).count()
}

/// Toggles bold, italic, strikethrough or inline code around the selection.
pub fn toggle_style(text: &str, selection: (usize, usize), style: Style) -> Edit {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = normalize(chars.len(), selection);
    let (c, m) = style.marker();
    let marker: String = std::iter::repeat_n(c, m).collect();
    let inside = &chars[start..end];
    // Markers inside the selection: "**x**" selected.
    let lead = inside.iter().take_while(|&&x| x == c).count();
    let trail = inside.iter().rev().take_while(|&&x| x == c).count();
    if inside.len() >= 2 * m && lead < inside.len() && style.in_run(lead) && style.in_run(trail) {
        let inner: String = inside[m..inside.len() - m].iter().collect();
        let len = inner.chars().count();
        return Edit {
            start,
            end,
            text: inner,
            selection: (start, start + len),
        };
    }
    // Markers around the selection: "**" before and after.
    let (before, after) = (run_before(&chars, start, c), run_after(&chars, end, c));
    if start >= m && style.in_run(before) && style.in_run(after) && end > start {
        let inner: String = inside.iter().collect();
        return Edit {
            start: start - m,
            end: end + m,
            selection: (start - m, start - m + inside.len()),
            text: inner,
        };
    }
    let inner: String = inside.iter().collect();
    Edit {
        start,
        end,
        text: format!("{marker}{inner}{marker}"),
        selection: (start + m, start + m + inside.len()),
    }
}

fn is_url(text: &str) -> bool {
    let text = text.trim();
    !text.contains(char::is_whitespace)
        && ["http://", "https://", "www.", "mailto:", "ftp://"]
            .iter()
            .any(|scheme| text.starts_with(scheme))
}

/// Makes the selection a link: `[selection](url)`. A selected URL becomes the target.
pub fn link(text: &str, selection: (usize, usize)) -> Edit {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = normalize(chars.len(), selection);
    let selected: String = chars[start..end].iter().collect();
    let len = selected.chars().count();
    if is_url(&selected) {
        // Caret goes into the empty label.
        return Edit {
            start,
            end,
            text: format!("[]({selected})"),
            selection: (start + 1, start + 1),
        };
    }
    if len == 0 {
        return Edit {
            start,
            end,
            text: "[text](url)".into(),
            selection: (start + 1, start + 5),
        };
    }
    let url_start = start + len + 3;
    Edit {
        start,
        end,
        text: format!("[{selected}](url)"),
        selection: (url_start, url_start + 3),
    }
}

/// Character range of the lines touched by the selection (no trailing newline).
fn line_span(chars: &[char], start: usize, end: usize) -> (usize, usize) {
    let first = chars[..start]
        .iter()
        .rposition(|&c| c == '\n')
        .map_or(0, |i| i + 1);
    // A selection ending at the very start of a line does not include that line.
    let last_at = if end > start && end > 0 && chars[end - 1] == '\n' {
        end - 1
    } else {
        end
    };
    let last = chars[last_at..]
        .iter()
        .position(|&c| c == '\n')
        .map_or(chars.len(), |i| last_at + i);
    (first, last)
}

fn strip_heading(line: &str) -> (&str, usize) {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) {
        let rest = &line[hashes..];
        if rest.is_empty() {
            return ("", hashes);
        }
        if let Some(rest) = rest.strip_prefix(' ') {
            return (rest.trim_start(), hashes);
        }
    }
    (line, 0)
}

/// Sets the heading level (1–6) of the selected lines; `0` makes them paragraphs.
/// Applying the level every line already has removes it.
pub fn set_heading(text: &str, selection: (usize, usize), level: u8) -> Edit {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = normalize(chars.len(), selection);
    let (first, last) = line_span(&chars, start, end);
    let block: String = chars[first..last].iter().collect();
    let lines: Vec<&str> = block.split('\n').collect();
    let level = level.min(6);
    let all_same = level > 0
        && lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .all(|l| strip_heading(l).1 == level as usize);
    let target = if all_same { 0 } else { level as usize };
    let out: Vec<String> = lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                return (*line).to_owned();
            }
            let (body, _) = strip_heading(line);
            if target == 0 {
                body.to_owned()
            } else {
                format!("{} {body}", "#".repeat(target))
            }
        })
        .collect();
    let replaced = out.join("\n");
    let len = replaced.chars().count();
    let selection = if start == end {
        // Caret: keep its column relative to the line text.
        let old_prefix = strip_heading(lines[0]).1;
        let old_prefix = if old_prefix > 0 { old_prefix + 1 } else { 0 };
        let new_prefix = if target > 0 { target + 1 } else { 0 };
        let column = start - first;
        let column = (column + new_prefix).saturating_sub(old_prefix.min(column));
        let column = column.min(len);
        (first + column, first + column)
    } else {
        (first, first + len)
    };
    Edit {
        start: first,
        end: last,
        text: replaced,
        selection,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Bullet,
    Ordered,
    Task,
}

/// Splits a list marker off a line: `(indent, kind, rest)`.
fn split_marker(line: &str) -> (&str, Option<ListKind>, &str) {
    let body = line.trim_start();
    let indent = &line[..line.len() - body.len()];
    for (prefix, kind) in [
        ("- [ ] ", ListKind::Task),
        ("- [x] ", ListKind::Task),
        ("- [X] ", ListKind::Task),
    ] {
        if let Some(rest) = body.strip_prefix(prefix) {
            return (indent, Some(kind), rest);
        }
    }
    for prefix in ["- ", "* ", "+ "] {
        if let Some(rest) = body.strip_prefix(prefix) {
            return (indent, Some(ListKind::Bullet), rest);
        }
    }
    let digits = body.chars().take_while(char::is_ascii_digit).count();
    if digits > 0
        && let Some(rest) = body[digits..]
            .strip_prefix(". ")
            .or_else(|| body[digits..].strip_prefix(") "))
    {
        return (indent, Some(ListKind::Ordered), rest);
    }
    (indent, None, body)
}

/// Turns the selected lines into a bullet, numbered or task list, or removes the list
/// markers when every non-blank line already is that kind of list item.
pub fn toggle_list(text: &str, selection: (usize, usize), kind: ListKind) -> Edit {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = normalize(chars.len(), selection);
    let (first, last) = line_span(&chars, start, end);
    let block: String = chars[first..last].iter().collect();
    let lines: Vec<&str> = block.split('\n').collect();
    let all_same = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .all(|l| split_marker(l).1 == Some(kind));
    let mut number = 0;
    let out: Vec<String> = lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                return (*line).to_owned();
            }
            let (indent, _, rest) = split_marker(line);
            if all_same {
                return format!("{indent}{rest}");
            }
            number += 1;
            match kind {
                ListKind::Bullet => format!("{indent}- {rest}"),
                ListKind::Ordered => format!("{indent}{number}. {rest}"),
                ListKind::Task => format!("{indent}- [ ] {rest}"),
            }
        })
        .collect();
    let replaced = out.join("\n");
    let len = replaced.chars().count();
    Edit {
        start: first,
        end: last,
        text: replaced,
        selection: if start == end {
            (first + len, first + len)
        } else {
            (first, first + len)
        },
    }
}

/// Text to insert at `start` so a block begins on its own line.
fn block_prefix(chars: &[char], start: usize) -> &'static str {
    if start == 0 || chars[start - 1] == '\n' {
        ""
    } else {
        "\n\n"
    }
}

fn block_suffix(chars: &[char], end: usize) -> &'static str {
    match chars.get(end) {
        None | Some('\n') => "\n",
        Some(_) => "\n\n",
    }
}

/// Inserts an empty GFM table with `columns` x `rows` body cells.
pub fn insert_table(text: &str, selection: (usize, usize), columns: usize, rows: usize) -> Edit {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = normalize(chars.len(), selection);
    let columns = columns.clamp(1, 20);
    let rows = rows.clamp(1, 100);
    let row = |cell: &dyn Fn(usize) -> String| {
        format!(
            "|{}|",
            (1..=columns)
                .map(|i| format!(" {} ", cell(i)))
                .collect::<Vec<_>>()
                .join("|")
        )
    };
    let header = row(&|i| format!("Column {i}"));
    let rule = row(&|_| "---".into());
    let body = row(&|_| String::new());
    let mut table = format!("{header}\n{rule}");
    for _ in 0..rows {
        table.push('\n');
        table.push_str(&body);
    }
    let prefix = block_prefix(&chars, start);
    let suffix = block_suffix(&chars, end);
    // Select the first header label for quick renaming.
    let label_start = start + prefix.chars().count() + 2;
    Edit {
        start,
        end,
        text: format!("{prefix}{table}{suffix}"),
        selection: (label_start, label_start + "Column 1".chars().count()),
    }
}

/// Wraps the selection in a fenced code block (or inserts an empty one).
pub fn insert_code_block(text: &str, selection: (usize, usize), lang: &str) -> Edit {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = normalize(chars.len(), selection);
    let selected: String = chars[start..end].iter().collect();
    let lang: String = lang
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '`')
        .collect();
    // A fence longer than any backtick run in the content keeps it intact.
    let longest = selected
        .split(|c| c != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat((longest + 1).max(3));
    let prefix = block_prefix(&chars, start);
    let suffix = block_suffix(&chars, end);
    let head = format!("{prefix}{fence}{lang}\n");
    let body_start = start + head.chars().count();
    let body_len = selected.chars().count();
    Edit {
        start,
        end,
        text: format!("{head}{selected}\n{fence}{suffix}"),
        selection: (body_start, body_start + body_len),
    }
}

/// Inserts an image `![alt](path)`; the selection becomes the alt text.
pub fn insert_image(text: &str, selection: (usize, usize), path: &str, alt: &str) -> Edit {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = normalize(chars.len(), selection);
    let selected: String = chars[start..end].iter().collect();
    let alt = if selected.is_empty() {
        alt
    } else {
        selected.as_str()
    };
    let path = if path.is_empty() { "image.png" } else { path };
    // Spaces would end the destination early.
    let destination = if path.contains(' ') {
        format!("<{path}>")
    } else {
        path.to_owned()
    };
    let path_start = start + alt.chars().count() + 4 + usize::from(path.contains(' '));
    Edit {
        start,
        end,
        text: format!("![{alt}]({destination})"),
        selection: (path_start, path_start + path.chars().count()),
    }
}

/// GitHub-style anchor for a heading title.
pub fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if c == ' ' || c == '-' {
            out.push('-');
        }
    }
    out
}

/// Inserts a table of contents (linked bullet list of the document's headings) at the caret.
pub fn insert_toc(text: &str, selection: (usize, usize)) -> Edit {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = normalize(chars.len(), selection);
    let doc = model::parse(text);
    let headings: Vec<(u8, String)> = doc
        .blocks
        .iter()
        .filter_map(|block| match &block.kind {
            BlockKind::Heading { level, inlines } => Some((*level, model::plain_text(inlines))),
            _ => None,
        })
        .collect();
    let base = headings.iter().map(|(level, _)| *level).min().unwrap_or(1);
    let mut seen = std::collections::HashMap::<String, usize>::new();
    let mut toc = String::new();
    for (level, title) in &headings {
        let mut anchor = slug(title);
        let count = seen.entry(anchor.clone()).or_insert(0);
        if *count > 0 {
            anchor = format!("{anchor}-{count}");
        }
        *count += 1;
        toc.push_str(&format!(
            "{}- [{title}](#{anchor})\n",
            "  ".repeat((level - base) as usize)
        ));
    }
    if toc.is_empty() {
        toc.push_str("<!-- no headings -->\n");
    }
    let prefix = block_prefix(&chars, start);
    let end_pos = start + prefix.chars().count() + toc.chars().count();
    Edit {
        start,
        end,
        text: format!("{prefix}{toc}"),
        selection: (end_pos, end_pos),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(
        text: &str,
        sel: (usize, usize),
        f: impl Fn(&str, (usize, usize)) -> Edit,
    ) -> (String, String) {
        let edit = f(text, sel);
        let out = edit.apply(text);
        let chars: Vec<char> = out.chars().collect();
        let selected: String = chars[edit.selection.0..edit.selection.1].iter().collect();
        (out, selected)
    }

    #[test]
    fn bold_wraps_and_unwraps_selection() {
        let bold = |t: &str, s| toggle_style(t, s, Style::Bold);
        assert_eq!(
            run("a word b", (2, 6), bold),
            ("a **word** b".into(), "word".into())
        );
        // Unwrap with the markers outside the selection.
        assert_eq!(
            run("a **word** b", (4, 8), bold),
            ("a word b".into(), "word".into())
        );
        // Unwrap with the markers inside the selection.
        assert_eq!(
            run("a **word** b", (2, 10), bold),
            ("a word b".into(), "word".into())
        );
        // Empty selection leaves the caret between the markers.
        let (out, _) = run("ab", (1, 1), bold);
        assert_eq!(out, "a****b");
        assert_eq!(bold("ab", (1, 1)).selection, (3, 3));
    }

    #[test]
    fn italic_and_bold_do_not_confuse_each_other() {
        let italic = |t: &str, s| toggle_style(t, s, Style::Italic);
        // Italic on bold text adds a marker instead of stripping bold.
        assert_eq!(run("**x**", (2, 3), italic).0, "***x***");
        assert_eq!(run("***x***", (3, 4), italic).0, "**x**");
        assert_eq!(run("*x*", (1, 2), italic).0, "x");
        assert_eq!(
            run("x", (0, 1), |t, s| toggle_style(t, s, Style::Bold)).0,
            "**x**"
        );
    }

    #[test]
    fn strike_and_code() {
        assert_eq!(
            run("gone", (0, 4), |t, s| toggle_style(
                t,
                s,
                Style::Strikethrough
            ))
            .0,
            "~~gone~~"
        );
        assert_eq!(
            run("`x`", (1, 2), |t, s| toggle_style(t, s, Style::Code)).0,
            "x"
        );
        assert_eq!(
            run("x", (0, 1), |t, s| toggle_style(t, s, Style::Code)).0,
            "`x`"
        );
    }

    #[test]
    fn selection_offsets_are_characters_not_bytes() {
        // "ünï 🎉" — multi-byte characters before and inside the selection.
        let (out, selected) = run("ünï 🎉 x", (4, 5), |t, s| {
            toggle_style(t, s, Style::Bold)
        });
        assert_eq!(out, "ünï **🎉** x");
        assert_eq!(selected, "🎉");
        // Backwards selections and out-of-range offsets are clamped.
        assert_eq!(
            run("日本", (2, 0), |t, s| toggle_style(t, s, Style::Italic)).0,
            "*日本*"
        );
        assert_eq!(
            run("日本", (1, 99), |t, s| toggle_style(t, s, Style::Italic)).0,
            "日*本*"
        );
    }

    #[test]
    fn links() {
        assert_eq!(
            run("see docs", (4, 8), link),
            ("see [docs](url)".into(), "url".into())
        );
        assert_eq!(
            run("https://a.dev", (0, 13), link),
            ("[](https://a.dev)".into(), String::new())
        );
        assert_eq!(run("", (0, 0), link), ("[text](url)".into(), "text".into()));
    }

    #[test]
    fn headings_toggle_and_replace() {
        let h = |level| move |t: &str, s| set_heading(t, s, level);
        assert_eq!(run("Title", (2, 2), h(2)).0, "## Title");
        assert_eq!(set_heading("Title", (2, 2), 2).selection, (5, 5));
        assert_eq!(run("# Title", (0, 0), h(3)).0, "### Title");
        assert_eq!(run("## Title", (0, 0), h(2)).0, "Title");
        assert_eq!(run("# T", (0, 0), h(0)).0, "T");
        assert_eq!(run("a\nb\n\nc", (0, 4), h(1)).0, "# a\n# b\n\nc");
        // Only the selected lines change; a selection ending at a line start excludes it.
        assert_eq!(run("a\nb", (0, 2), h(1)).0, "# a\nb");
        assert_eq!(run("#hashtag", (0, 0), h(1)).0, "# #hashtag");
    }

    #[test]
    fn lists() {
        let l = |kind| move |t: &str, s| toggle_list(t, s, kind);
        assert_eq!(run("a\nb", (0, 3), l(ListKind::Bullet)).0, "- a\n- b");
        assert_eq!(run("- a\n- b", (0, 7), l(ListKind::Bullet)).0, "a\nb");
        assert_eq!(
            run("- a\n- b", (0, 7), l(ListKind::Ordered)).0,
            "1. a\n2. b"
        );
        assert_eq!(
            run("1. a\n2. b", (0, 9), l(ListKind::Task)).0,
            "- [ ] a\n- [ ] b"
        );
        assert_eq!(run("- [x] a", (0, 7), l(ListKind::Task)).0, "a");
        assert_eq!(
            run("  x\n\n  y", (0, 8), l(ListKind::Ordered)).0,
            "  1. x\n\n  2. y"
        );
    }

    #[test]
    fn insertions() {
        let (out, selected) = run("intro", (5, 5), |t, s| insert_table(t, s, 2, 1));
        assert_eq!(
            out,
            "intro\n\n| Column 1 | Column 2 |\n| --- | --- |\n|  |  |\n"
        );
        assert_eq!(selected, "Column 1");
        let (out, selected) = run("", (0, 0), |t, s| insert_code_block(t, s, "rust"));
        assert_eq!(out, "```rust\n\n```\n");
        assert_eq!(selected, "");
        let (out, selected) = run("let x;", (0, 6), |t, s| insert_code_block(t, s, ""));
        assert_eq!(out, "```\nlet x;\n```\n");
        assert_eq!(selected, "let x;");
        // Backticks in the content get a longer fence.
        assert!(
            run("```", (0, 3), |t, s| insert_code_block(t, s, ""))
                .0
                .starts_with("````\n")
        );
        let (out, selected) = run("", (0, 0), |t, s| insert_image(t, s, "img/a b.png", "pic"));
        assert_eq!(out, "![pic](<img/a b.png>)");
        assert_eq!(selected, "img/a b.png");
        let (out, selected) = run("cat", (0, 3), |t, s| insert_image(t, s, "", ""));
        assert_eq!(out, "![cat](image.png)");
        assert_eq!(selected, "image.png");
    }

    #[test]
    fn table_of_contents_links_headings() {
        let source = "# A\n\n## B c\n\n## B c\n\n```\n# not a heading\n```\n";
        let edit = insert_toc(source, (0, 0));
        assert_eq!(edit.text, "- [A](#a)\n  - [B c](#b-c)\n  - [B c](#b-c-1)\n");
        assert_eq!(
            insert_toc("text", (4, 4)).text,
            "\n\n<!-- no headings -->\n"
        );
    }
}
