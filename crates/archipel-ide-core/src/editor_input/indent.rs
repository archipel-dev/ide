//! Indentation: the indent unit of a file (code style or detected, ED-STYLE-001, ED-STYLE-010)
//! and the indent the code style gives a line (ED-TYPE-062, ED-TYPE-046).

use super::{EditorOptions, View};

/// One indent step: the text to insert and its width in columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unit {
    pub text: String,
    pub width: usize,
}

impl Unit {
    pub fn spaces(width: usize) -> Self {
        Self {
            text: " ".repeat(width),
            width,
        }
    }
}

/// Columns of leading blanks (a tab counts as `tab_width` columns up to the next stop).
pub(crate) fn columns(indent: &str, tab_width: usize) -> usize {
    let mut col = 0;
    for c in indent.chars() {
        if c == '\t' {
            col += tab_width - col % tab_width;
        } else {
            col += 1;
        }
    }
    col
}

/// The indent unit: tabs when the code style says so, else the indent size, which is detected
/// from the file when `Detect and use existing file indents` is on and the file shows one.
pub(crate) fn unit(view: &View, opts: &EditorOptions) -> Unit {
    if opts.detect_indents
        && let Some(detected) = detect(view)
    {
        return match detected {
            Detected::Tabs => Unit {
                text: "\t".into(),
                width: opts.indent_size,
            },
            Detected::Spaces(width) => Unit::spaces(width),
        };
    }
    if opts.use_tab {
        Unit {
            text: "\t".into(),
            width: opts.indent_size,
        }
    } else {
        Unit::spaces(opts.indent_size)
    }
}

enum Detected {
    Tabs,
    Spaces(usize),
}

/// The indent step the file uses: the most common increase of the indent from one non-blank line
/// to the next.
fn detect(view: &View) -> Option<Detected> {
    let mut counts = [0usize; 9];
    let mut tabs = 0usize;
    let mut previous: Option<usize> = None;
    let mut start = 0;
    let mut sampled = 0;
    while start <= view.len() && sampled < 2000 {
        let end = view.line_end(start);
        if !view.is_blank_line(start) {
            let indent = view.indent_at(start);
            if indent.starts_with('\t') {
                if previous.is_some_and(|p| p == 0) {
                    tabs += 1;
                }
                previous = Some(columns(&indent, 4));
            } else {
                let width = indent.len();
                if let Some(p) = previous
                    && width > p
                    && width - p <= 8
                {
                    counts[width - p] += 1;
                }
                previous = Some(width);
            }
            sampled += 1;
        }
        if end >= view.len() {
            break;
        }
        start = end + 1;
    }
    let (best, count) = counts
        .iter()
        .enumerate()
        .skip(1)
        .max_by_key(|(width, count)| (**count, std::cmp::Reverse(*width)))
        .map(|(width, count)| (width, *count))?;
    if tabs > count && tabs > 0 {
        return Some(Detected::Tabs);
    }
    // Only the usual 2 to 4 columns count as a style; a lone deep indent is a mistake in the
    // file, not an indent size.
    ((2..=4).contains(&best) && count > 0).then_some(Detected::Spaces(best))
}

/// The indent of the line starting at `line_start` as the code style computes it for an empty
/// line there: one unit deeper than the line that opens the innermost unclosed `{`, a
/// continuation indent inside `(` and `[`, none at the top level (ED-TYPE-062).
pub(crate) fn proper(view: &View, opts: &EditorOptions, line_start: usize) -> String {
    if !view.lang.braces() {
        return String::new();
    }
    let unit = unit(view, opts);
    let open = view.open_brackets(line_start);
    let Some((offset, bracket)) = open.last().copied() else {
        return String::new();
    };
    let opener_indent = view.indent_at(view.line_start(offset));
    match bracket {
        '{' => format!("{opener_indent}{}", unit.text),
        _ => format!("{opener_indent}{}{}", unit.text, unit.text),
    }
}

/// The indent for a line that already has text: like [`proper`], one level out when the line
/// starts with a closing bracket.
pub(crate) fn proper_for_line(view: &View, opts: &EditorOptions, line_start: usize) -> String {
    let first = view.first_non_blank(line_start);
    if matches!(view.at(first), Some('}' | ')' | ']')) {
        let open = view.open_brackets(line_start);
        if let Some((offset, _)) = open.last() {
            return view.indent_at(view.line_start(*offset));
        }
    }
    proper(view, opts, line_start)
}

/// Whether the language gives a proper indent (otherwise Backspace falls back to the nearest
/// indent position and Enter copies the indent).
pub(crate) fn has_formatter(view: &View) -> bool {
    view.lang.braces()
}

/// The indent a line has when every enclosing block is indented by the code style: one unit per
/// unclosed `{`, a continuation indent per unclosed `(` or `[` (Auto-Indent Lines, ED-LINE-053).
pub(crate) fn absolute(view: &View, unit: &Unit, line_start: usize) -> String {
    let mut open = view.open_brackets(line_start);
    let first = view.first_non_blank(line_start);
    if matches!(view.at(first), Some('}' | ')' | ']')) {
        open.pop();
    }
    let mut out = String::new();
    for (_, bracket) in open {
        let steps = if bracket == '{' { 1 } else { 2 };
        for _ in 0..steps {
            out.push_str(&unit.text);
        }
    }
    out
}
