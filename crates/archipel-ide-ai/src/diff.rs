//! Line diffs for previewing AI-proposed edits before they are applied.

use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Context,
    Removed,
    Added,
    /// Elided unchanged lines.
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
    /// 1-based line numbers in the old/new text.
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
}

/// Unified-style line diff keeping `context` unchanged lines around each change.
pub fn line_diff(before: &str, after: &str, context: usize) -> Vec<DiffLine> {
    let diff = TextDiff::from_lines(before, after);
    let mut lines = Vec::new();
    for (group_index, group) in diff.grouped_ops(context).iter().enumerate() {
        if group_index > 0 {
            lines.push(DiffLine {
                kind: DiffKind::Skipped,
                text: "…".into(),
                old_line: None,
                new_line: None,
            });
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let kind = match change.tag() {
                    ChangeTag::Equal => DiffKind::Context,
                    ChangeTag::Delete => DiffKind::Removed,
                    ChangeTag::Insert => DiffKind::Added,
                };
                lines.push(DiffLine {
                    kind,
                    text: change.value().trim_end_matches(['\n', '\r']).to_owned(),
                    old_line: change.old_index().map(|i| i + 1),
                    new_line: change.new_index().map(|i| i + 1),
                });
            }
        }
    }
    lines
}

/// (added, removed) line counts.
pub fn stats(lines: &[DiffLine]) -> (usize, usize) {
    lines
        .iter()
        .fold((0, 0), |(added, removed), line| match line.kind {
            DiffKind::Added => (added + 1, removed),
            DiffKind::Removed => (added, removed + 1),
            _ => (added, removed),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_keeps_context_and_elides_distant_lines() {
        let before = "a\nb\nc\nd\ne\nf\ng\nh\n";
        let after = "a\nB\nc\nd\ne\nf\ng\nH\n";
        let lines = line_diff(before, after, 1);
        let kinds: Vec<_> = lines.iter().map(|l| l.kind).collect();
        use DiffKind::*;
        assert_eq!(
            kinds,
            [
                Context, Removed, Added, Context, Skipped, Context, Removed, Added
            ]
        );
        assert_eq!(lines[2].new_line, Some(2));
        assert_eq!(stats(&lines), (2, 2));
        assert!(line_diff("same\n", "same\n", 3).is_empty());
    }
}
