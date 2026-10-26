//! Quick fixes built from functions that *recompute* their edits from the
//! current syntax at the problem's current range, so a fix stays correct
//! after the file changed (highlights are shifted, edits never are).

use archipel_ide_analysis::inspect::ProblemDescriptor;
use archipel_ide_analysis::intention::{FixCx, QuickFix};
use archipel_ide_analysis::{Cancelled, ModCommand, TextEdit};
use std::sync::Arc;

/// Computes the edits of a fix for the problem at `d.range`; `None` = the
/// code no longer matches (nothing to do).
pub type EditsFn = fn(&ProblemDescriptor, &FixCx<'_>) -> Option<Vec<TextEdit>>;

/// A quick fix defined by an [`EditsFn`].
pub struct EditFix {
    pub family: &'static str,
    pub name: String,
    pub edits: EditsFn,
    pub batch: bool,
}

impl EditFix {
    pub fn arc(family: &'static str, name: impl Into<String>, edits: EditsFn) -> Arc<dyn QuickFix> {
        Arc::new(Self {
            family,
            name: name.into(),
            edits,
            batch: true,
        })
    }
}

impl QuickFix for EditFix {
    fn family(&self) -> &str {
        self.family
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn fix(&self, d: &ProblemDescriptor, cx: &FixCx<'_>) -> Result<ModCommand, Cancelled> {
        cx.cancel.check()?;
        Ok(match (self.edits)(d, cx) {
            Some(edits) if !edits.is_empty() => ModCommand::UpdateFile {
                file: cx.file,
                edits,
                expected_rev: cx.syntax.rev(),
            },
            _ => ModCommand::DisplayMessage {
                message: format!("`{}` no longer applies here", self.name),
                error: false,
            },
        })
    }

    fn batch_ok(&self) -> bool {
        self.batch
    }
}

/// `range` widened to cover its whole line(s) when only whitespace shares
/// them (deleting a statement should not leave a blank indented line).
pub fn whole_line_if_alone(text: &str, range: std::ops::Range<u32>) -> std::ops::Range<u32> {
    let (start, end) = (range.start as usize, (range.end as usize).min(text.len()));
    if start > end || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return range;
    }
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let before_blank = text[line_start..start]
        .chars()
        .all(|c| c == ' ' || c == '\t');
    let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
    let after_blank = text[end..line_end]
        .chars()
        .all(|c| c == ' ' || c == '\t' || c == '\r');
    if before_blank && after_blank {
        let end = (line_end + 1).min(text.len());
        line_start as u32..end as u32
    } else {
        range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statements_alone_on_a_line_take_the_line_with_them() {
        let text = "a;\n    b = b;\nc;\n";
        assert_eq!(whole_line_if_alone(text, 7..13), 3..14);
        // Shares the line with other code: only the range.
        let text = "a; b = b; c;";
        assert_eq!(whole_line_if_alone(text, 3..9), 3..9);
    }
}
