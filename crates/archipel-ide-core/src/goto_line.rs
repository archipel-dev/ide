//! Go to Line:Column (`Ctrl+G`, spec `docs/specs/search/05-recent-and-structure-popups.md`
//! section 4): what the dialog shows and how its text is read. The window and the terminal
//! frontend both ask [`IdeState::goto_line_target`] and dispatch `editor.gotoLine` with the
//! result.

use crate::IdeState;

/// The text the dialog is prefilled with: `<line>:<column>` of the caret, 1-based (SRCH-GTL-001).
pub fn prefill(line: usize, column: usize) -> String {
    format!("{line}:{column}")
}

/// Reads the dialog text (SRCH-GTL-002): `12`, `12:5`, `12,5`, `:5` (current line), `12:`, a
/// blank text (the current line) and relative `+N` / `-N` (logical lines from the caret line,
/// clamped to `line_count`). Returns the 1-based `(line, column)`, or `None` when the text is
/// not understood or the line is 0 (the dialog stays open).
pub fn parse(input: &str, caret_line: usize, line_count: usize) -> Option<(usize, usize)> {
    let text = input.trim();
    if text.is_empty() {
        return Some((caret_line, 1));
    }
    if let Some(sign @ ('+' | '-')) = text.chars().next() {
        let amount: usize = text[1..].trim().parse().ok()?;
        let line = if sign == '+' {
            caret_line.saturating_add(amount)
        } else {
            caret_line.saturating_sub(amount)
        };
        return Some((line.clamp(1, line_count.max(1)), 1));
    }
    let (line_part, column_part) = match text.find([':', ',']) {
        Some(at) => (text[..at].trim(), Some(text[at + 1..].trim())),
        None => (text, None),
    };
    let line = if line_part.is_empty() {
        caret_line
    } else {
        line_part.parse::<usize>().ok()?
    };
    if line == 0 {
        return None;
    }
    let column = match column_part {
        Some(c) if !c.is_empty() => c.parse::<usize>().ok()?.max(1),
        _ => 1,
    };
    Some((line, column))
}

impl IdeState {
    /// The 1-based `(line, column)` of the caret of the active document.
    pub fn caret_line_column(&self) -> Option<(usize, usize)> {
        let doc = self.active_doc()?;
        let (line, column) = doc
            .text
            .char_to_line_column(doc.selections.primary().head)
            .ok()?;
        Some((line + 1, column + 1))
    }

    /// Reads Go to Line text against the active document (SRCH-GTL-002).
    pub fn goto_line_target(&self, input: &str) -> Option<(usize, usize)> {
        let doc = self.active_doc()?;
        let (line, _) = self.caret_line_column()?;
        parse(input, line, doc.text.len_lines())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_text_forms() {
        // SRCH-GTL-002
        assert_eq!(parse("12", 7, 30), Some((12, 1)));
        assert_eq!(parse("12:5", 7, 30), Some((12, 5)));
        assert_eq!(parse("12,5", 7, 30), Some((12, 5)));
        assert_eq!(parse(":5", 7, 30), Some((7, 5)));
        assert_eq!(parse("+3", 7, 30), Some((10, 1)));
        assert_eq!(parse("abc", 7, 30), None);
        assert_eq!(parse("0", 7, 30), None);
        assert_eq!(prefill(7, 4), "7:4");
    }
}
