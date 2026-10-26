//! Line arithmetic over a text: byte offset <-> zero-based line, columns and
//! indentation. `\n` and `\r\n` both end a line; offsets are bytes.

use std::ops::Range;

/// Start offsets of every line of a text (built once, O(n)).
#[derive(Debug, Clone, Default)]
pub struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(
            text.bytes()
                .enumerate()
                .filter(|(_, byte)| *byte == b'\n')
                .map(|(index, _)| index + 1),
        );
        Self {
            starts,
            len: text.len(),
        }
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// Start offset of `line` (the text length for lines past the end).
    pub fn line_start(&self, line: usize) -> usize {
        self.starts.get(line).copied().unwrap_or(self.len)
    }

    /// Line containing `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        self.starts.partition_point(|start| *start <= offset) - 1
    }

    /// Offsets of `line` without its line break.
    pub fn line_range(&self, line: usize, text: &str) -> Range<usize> {
        let start = self.line_start(line);
        let mut end = self.line_start(line + 1).min(text.len());
        if end > start && text.as_bytes()[end - 1] == b'\n' {
            end -= 1;
            if end > start && text.as_bytes()[end - 1] == b'\r' {
                end -= 1;
            }
        }
        start..end.max(start)
    }
}

/// Width in columns of the leading whitespace of `line` (tabs advance to the
/// next multiple of `tab_size`).
pub fn indent_columns(line: &str, tab_size: usize) -> usize {
    let tab_size = tab_size.max(1);
    let mut columns = 0;
    for ch in line.chars() {
        match ch {
            ' ' => columns += 1,
            '\t' => columns += tab_size - columns % tab_size,
            _ => break,
        }
    }
    columns
}

/// Byte length of the leading whitespace of `line`.
pub fn indent_bytes(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_columns() {
        let text = "a\n  b\r\n\tc\n";
        let index = LineIndex::new(text);
        assert_eq!(index.line_count(), 4);
        assert_eq!(index.line_of(0), 0);
        assert_eq!(index.line_of(2), 1);
        assert_eq!(index.line_of(text.len()), 3);
        assert_eq!(&text[index.line_range(1, text)], "  b");
        assert_eq!(indent_columns("\t  x", 4), 6);
        assert_eq!(indent_bytes("\t  x"), 3);
        assert_eq!(index.line_start(9), text.len());
    }
}
