//! File format detection: line separators and indentation. Scans are bounded
//! so per-frame status bar queries stay cheap on very large files.

use crate::TextDocument;

/// Lines inspected by [`detect_indent`].
const INDENT_SAMPLE_LINES: usize = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineSeparator {
    Lf,
    CrLf,
    Cr,
}

impl LineSeparator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }

    /// Status bar label: `LF`, `CRLF` or `CR`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
            Self::Cr => "CR",
        }
    }
}

/// The separator of the first line break; LF for single-line documents.
pub fn detect_line_separator(doc: &TextDocument) -> LineSeparator {
    let Some(first_end) = doc.line_to_char(1) else {
        return LineSeparator::Lf;
    };
    // `first_end` is just past the first line break.
    match (
        first_end.checked_sub(2).and_then(|i| doc.char_at(i)),
        doc.char_at(first_end - 1),
    ) {
        (Some('\r'), Some('\n')) => LineSeparator::CrLf,
        (_, Some('\r')) => LineSeparator::Cr,
        _ => LineSeparator::Lf,
    }
}

/// [`detect_line_separator`] for a plain string.
pub fn detect_line_separator_str(text: &str) -> LineSeparator {
    match text.find(['\r', '\n']) {
        Some(at) if text[at..].starts_with("\r\n") => LineSeparator::CrLf,
        Some(at) if text[at..].starts_with('\r') => LineSeparator::Cr,
        _ => LineSeparator::Lf,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indent {
    Tabs,
    Spaces(usize),
}

impl Indent {
    /// Status bar label: `Tab`, `4 spaces`.
    pub fn label(self) -> String {
        match self {
            Self::Tabs => "Tab".into(),
            Self::Spaces(1) => "1 space".into(),
            Self::Spaces(n) => format!("{n} spaces"),
        }
    }

    pub fn unit(self) -> String {
        match self {
            Self::Tabs => "\t".into(),
            Self::Spaces(n) => " ".repeat(n),
        }
    }
}

/// Detects the file's indentation from its first lines, like "Detect and use
/// existing file indents": tabs when most indented lines start with a tab,
/// otherwise the most common positive indent step between consecutive lines.
/// Returns `None` when the sample has no indented lines.
pub fn detect_indent(doc: &TextDocument) -> Option<Indent> {
    let mut tab_lines = 0usize;
    let mut space_lines = 0usize;
    let mut steps = [0usize; 9];
    let mut previous: Option<usize> = None;
    for line in 0..doc.len_lines().min(INDENT_SAMPLE_LINES) {
        let Some(text) = doc.line(line) else {
            break;
        };
        let content = text.trim_end_matches(['\r', '\n']);
        if content.trim().is_empty() {
            continue;
        }
        if content.starts_with('\t') {
            tab_lines += 1;
            previous = None;
            continue;
        }
        let width = content.chars().take_while(|c| *c == ' ').count();
        if width > 0 {
            space_lines += 1;
        }
        if let Some(previous) = previous
            && width > previous
            && width - previous < steps.len()
        {
            steps[width - previous] += 1;
        }
        previous = Some(width);
    }
    if tab_lines == 0 && space_lines == 0 {
        return None;
    }
    if tab_lines > space_lines {
        return Some(Indent::Tabs);
    }
    let best = (1..steps.len())
        .filter(|&step| steps[step] > 0)
        .max_by_key(|&step| (steps[step], std::cmp::Reverse(step)))
        .unwrap_or(4);
    Some(Indent::Spaces(best))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_line_separators_from_the_first_break() {
        let detect = |text: &str| detect_line_separator(&TextDocument::new(text));
        assert_eq!(detect("a\r\nb\n"), LineSeparator::CrLf);
        assert_eq!(detect("a\nb\r\n"), LineSeparator::Lf);
        assert_eq!(detect("a\rb"), LineSeparator::Cr);
        assert_eq!(detect("single"), LineSeparator::Lf);
        assert_eq!(detect(""), LineSeparator::Lf);
        assert_eq!(LineSeparator::CrLf.label(), "CRLF");
    }

    #[test]
    fn detects_indentation_style_and_size() {
        let detect = |text: &str| detect_indent(&TextDocument::new(text));
        assert_eq!(
            detect("a {\n  b {\n    c\n  }\n}\n"),
            Some(Indent::Spaces(2))
        );
        assert_eq!(
            detect("class A {\n    void f() {\n        x();\n    }\n}\n"),
            Some(Indent::Spaces(4))
        );
        assert_eq!(detect("fn a() {\n\tb();\n\tc();\n}\n"), Some(Indent::Tabs));
        assert_eq!(detect("flat\ntext\n"), None);
        assert_eq!(Indent::Spaces(2).label(), "2 spaces");
        assert_eq!(Indent::Tabs.unit(), "\t");
    }
}
