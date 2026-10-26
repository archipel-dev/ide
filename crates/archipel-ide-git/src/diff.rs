//! Line-level two-way diff with word-level highlights inside modified hunks.
//!
//! The model is UI-independent: a viewer lays out [`FileDiff::old_text`] and
//! [`FileDiff::new_text`] and uses [`Hunk`]s to colour and connect changed
//! line ranges, in either a side-by-side or a unified presentation.

use similar::{Algorithm, ChangeTag, DiffOp, TextDiff};
use std::ops::Range;
use std::path::PathBuf;

/// Hunks whose combined text exceeds this many bytes get no word highlights.
const INLINE_DIFF_LIMIT: usize = 64 * 1024;

/// How whitespace differences count when comparing lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhitespaceMode {
    #[default]
    DoNotIgnore,
    /// Ignore leading and trailing whitespace.
    Trim,
    /// Ignore all whitespace.
    IgnoreAll,
}

impl WhitespaceMode {
    pub const ALL: [WhitespaceMode; 3] = [Self::DoNotIgnore, Self::Trim, Self::IgnoreAll];

    pub fn id(self) -> &'static str {
        match self {
            Self::DoNotIgnore => "none",
            Self::Trim => "trim",
            Self::IgnoreAll => "all",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DoNotIgnore => "Do not ignore",
            Self::Trim => "Trim whitespaces",
            Self::IgnoreAll => "Ignore whitespaces",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.id() == id)
    }

    fn normalize(self, line: &str) -> String {
        match self {
            Self::DoNotIgnore => line.to_owned(),
            Self::Trim => line.trim().to_owned(),
            Self::IgnoreAll => line.chars().filter(|c| !c.is_whitespace()).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkKind {
    /// Lines only present on the new side.
    Inserted,
    /// Lines only present on the old side.
    Deleted,
    /// Old lines replaced by new lines.
    Modified,
}

/// A byte range on one line (0-based line index, byte columns in that line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRange {
    pub line: usize,
    pub columns: Range<usize>,
}

/// A changed region. Line ranges are 0-based and half-open; an empty range
/// marks the position between lines where the other side's lines belong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old: Range<usize>,
    pub new: Range<usize>,
    /// Changed words on the old side (only for [`HunkKind::Modified`]).
    pub old_words: Vec<LineRange>,
    /// Changed words on the new side (only for [`HunkKind::Modified`]).
    pub new_words: Vec<LineRange>,
}

impl Hunk {
    pub fn kind(&self) -> HunkKind {
        match (self.old.is_empty(), self.new.is_empty()) {
            (true, _) => HunkKind::Inserted,
            (_, true) => HunkKind::Deleted,
            _ => HunkKind::Modified,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// Repository-relative path of the compared file.
    pub path: PathBuf,
    /// Short hash of the base revision, or `None` when the file is new.
    pub base_revision: Option<String>,
    pub old_text: String,
    pub new_text: String,
    pub hunks: Vec<Hunk>,
}

impl FileDiff {
    pub fn compute(
        path: PathBuf,
        base_revision: Option<String>,
        old_text: String,
        new_text: String,
    ) -> Self {
        let hunks = compute_hunks(&old_text, &new_text, WhitespaceMode::DoNotIgnore);
        Self {
            path,
            base_revision,
            old_text,
            new_text,
            hunks,
        }
    }

    /// The same texts compared under `mode`.
    pub fn with_whitespace(mut self, mode: WhitespaceMode) -> Self {
        self.hunks = compute_hunks(&self.old_text, &self.new_text, mode);
        self
    }

    /// The old text with the hunks at `indices` replaced by their new lines
    /// (what the index holds after staging those chunks).
    pub fn old_with_hunks(&self, indices: &[usize]) -> String {
        let old: Vec<&str> = self.old_text.split_inclusive('\n').collect();
        let new: Vec<&str> = self.new_text.split_inclusive('\n').collect();
        let mut out = String::new();
        let mut at = 0;
        for (index, hunk) in self.hunks.iter().enumerate() {
            if !indices.contains(&index) {
                continue;
            }
            out.extend(
                old[at.min(old.len())..hunk.old.start.min(old.len())]
                    .iter()
                    .copied(),
            );
            out.extend(
                new[hunk.new.start.min(new.len())..hunk.new.end.min(new.len())]
                    .iter()
                    .copied(),
            );
            at = hunk.old.end;
        }
        out.extend(old[at.min(old.len())..].iter().copied());
        out
    }

    /// The new text with the hunk at `index` replaced by its old lines (copies
    /// the chunk from the left side into the right side).
    pub fn new_with_hunk_reverted(&self, index: usize) -> Option<String> {
        let hunk = self.hunks.get(index)?;
        let old: Vec<&str> = self.old_text.split_inclusive('\n').collect();
        let new: Vec<&str> = self.new_text.split_inclusive('\n').collect();
        let mut out = String::new();
        out.extend(new[..hunk.new.start.min(new.len())].iter().copied());
        out.extend(
            old[hunk.old.start.min(old.len())..hunk.old.end.min(old.len())]
                .iter()
                .copied(),
        );
        out.extend(new[hunk.new.end.min(new.len())..].iter().copied());
        Some(out)
    }

    /// Lines of the old side without their terminators.
    pub fn old_lines(&self) -> Vec<&str> {
        split_lines(&self.old_text)
    }

    /// Lines of the new side without their terminators.
    pub fn new_lines(&self) -> Vec<&str> {
        split_lines(&self.new_text)
    }

    /// Maps a (fractional) new-side line position to the old side, so two
    /// panes scroll in step: unchanged regions map 1:1 and hunks are
    /// stretched or squashed proportionally.
    pub fn new_to_old(&self, position: f32) -> f32 {
        let mut delta = 0.0_f32;
        for hunk in &self.hunks {
            let (new_start, new_end) = (hunk.new.start as f32, hunk.new.end as f32);
            let (old_start, old_end) = (hunk.old.start as f32, hunk.old.end as f32);
            if position < new_start {
                break;
            }
            if position < new_end {
                let t = (position - new_start) / (new_end - new_start);
                return old_start + t * (old_end - old_start);
            }
            delta = old_end - new_end;
        }
        position + delta
    }

    /// Index of the hunk after `current` (the first one when nothing is current).
    pub fn next_hunk(&self, current: Option<usize>) -> Option<usize> {
        match current {
            None => (!self.hunks.is_empty()).then_some(0),
            Some(index) => (index + 1 < self.hunks.len()).then_some(index + 1),
        }
    }

    /// Index of the hunk before `current` (the last one when nothing is current).
    pub fn previous_hunk(&self, current: Option<usize>) -> Option<usize> {
        match current {
            None => self.hunks.len().checked_sub(1),
            Some(index) => index.checked_sub(1),
        }
    }
}

/// Splits text into lines, treating `\n` and `\r\n` as terminators. A final
/// terminator does not start an extra empty line.
pub fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.strip_suffix('\n')
        .unwrap_or(text)
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

fn compute_hunks(old_text: &str, new_text: &str, whitespace: WhitespaceMode) -> Vec<Hunk> {
    let old_lines = split_lines(old_text);
    let new_lines = split_lines(new_text);
    let old_keys: Vec<String> = old_lines.iter().map(|l| whitespace.normalize(l)).collect();
    let new_keys: Vec<String> = new_lines.iter().map(|l| whitespace.normalize(l)).collect();
    let old_keys: Vec<&str> = old_keys.iter().map(String::as_str).collect();
    let new_keys: Vec<&str> = new_keys.iter().map(String::as_str).collect();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_slices(&old_keys, &new_keys);
    let mut hunks: Vec<Hunk> = Vec::new();
    for op in diff.ops() {
        let (old, new) = match *op {
            DiffOp::Equal { .. } => continue,
            DiffOp::Delete {
                old_index,
                old_len,
                new_index,
            } => (old_index..old_index + old_len, new_index..new_index),
            DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => (old_index..old_index, new_index..new_index + new_len),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => (
                old_index..old_index + old_len,
                new_index..new_index + new_len,
            ),
        };
        // Adjacent delete + insert ops form one modified hunk.
        if let Some(last) = hunks.last_mut()
            && last.old.end == old.start
            && last.new.end == new.start
        {
            last.old.end = old.end;
            last.new.end = new.end;
        } else {
            hunks.push(Hunk {
                old,
                new,
                old_words: Vec::new(),
                new_words: Vec::new(),
            });
        }
    }
    for hunk in &mut hunks {
        if hunk.kind() == HunkKind::Modified {
            let (old_words, new_words) = word_changes(
                &old_lines[hunk.old.clone()],
                hunk.old.start,
                &new_lines[hunk.new.clone()],
                hunk.new.start,
            );
            hunk.old_words = old_words;
            hunk.new_words = new_words;
        }
    }
    hunks
}

fn word_changes(
    old: &[&str],
    old_first: usize,
    new: &[&str],
    new_first: usize,
) -> (Vec<LineRange>, Vec<LineRange>) {
    let old_text = old.join("\n");
    let new_text = new.join("\n");
    if old_text.len() + new_text.len() > INLINE_DIFF_LIMIT {
        return (Vec::new(), Vec::new());
    }
    let diff = TextDiff::from_words(old_text.as_str(), new_text.as_str());
    let (mut old_ranges, mut new_ranges) = (Vec::new(), Vec::new());
    let (mut old_offset, mut new_offset) = (0, 0);
    for change in diff.iter_all_changes() {
        let len = change.value().len();
        match change.tag() {
            ChangeTag::Equal => {
                old_offset += len;
                new_offset += len;
            }
            ChangeTag::Delete => {
                old_ranges.push(old_offset..old_offset + len);
                old_offset += len;
            }
            ChangeTag::Insert => {
                new_ranges.push(new_offset..new_offset + len);
                new_offset += len;
            }
        }
    }
    (
        to_line_ranges(&old_text, old_first, &old_ranges),
        to_line_ranges(&new_text, new_first, &new_ranges),
    )
}

/// Splits byte ranges of `text` at line breaks into per-line column ranges,
/// dropping whitespace-only pieces and merging touching ones.
fn to_line_ranges(text: &str, first_line: usize, ranges: &[Range<usize>]) -> Vec<LineRange> {
    let mut line_starts = vec![0];
    line_starts.extend(text.match_indices('\n').map(|(index, _)| index + 1));
    let mut result: Vec<LineRange> = Vec::new();
    for range in ranges {
        let mut start = range.start;
        while start < range.end {
            let line = line_starts.partition_point(|&s| s <= start) - 1;
            let line_end = line_starts
                .get(line + 1)
                .map_or(text.len(), |next| next - 1);
            let end = range.end.min(line_end);
            if end > start && !text[start..end].trim().is_empty() {
                let columns = start - line_starts[line]..end - line_starts[line];
                match result.last_mut() {
                    Some(last)
                        if last.line == first_line + line && last.columns.end == columns.start =>
                    {
                        last.columns.end = columns.end;
                    }
                    _ => result.push(LineRange {
                        line: first_line + line,
                        columns,
                    }),
                }
            }
            start = end.max(start) + 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn whitespace_modes_and_hunk_texts() {
        use super::*;
        let diff = FileDiff::compute(
            "a".into(),
            None,
            "a\n  b\nx\nc\n".into(),
            "a\nb\nx\nC\n".into(),
        );
        assert_eq!(diff.hunks.len(), 2);
        let trimmed = diff.clone().with_whitespace(WhitespaceMode::Trim);
        assert_eq!(trimmed.hunks.len(), 1);
        assert_eq!(diff.old_with_hunks(&[1]), "a\n  b\nx\nC\n");
        assert_eq!(diff.old_with_hunks(&[0, 1]), "a\nb\nx\nC\n");
        assert_eq!(diff.new_with_hunk_reverted(0).unwrap(), "a\n  b\nx\nC\n");
        assert!(diff.new_with_hunk_reverted(9).is_none());
    }

    use super::*;

    fn diff(old: &str, new: &str) -> FileDiff {
        FileDiff::compute(PathBuf::from("a.txt"), None, old.into(), new.into())
    }

    #[test]
    fn splits_lines_without_phantom_last_line() {
        assert_eq!(split_lines(""), Vec::<&str>::new());
        assert_eq!(split_lines("a\nb\n"), vec!["a", "b"]);
        assert_eq!(split_lines("a\r\nb"), vec!["a", "b"]);
        assert_eq!(split_lines("\n"), vec![""]);
    }

    #[test]
    fn classifies_inserted_deleted_and_modified_hunks() {
        let d = diff("a\nb\nc\nd\n", "a\nnew\nb\nC\n");
        let kinds: Vec<_> = d.hunks.iter().map(Hunk::kind).collect();
        assert_eq!(
            kinds,
            vec![HunkKind::Inserted, HunkKind::Modified],
            "{:?}",
            d.hunks
        );
        assert_eq!(d.hunks[0].old, 1..1);
        assert_eq!(d.hunks[0].new, 1..2);
        assert_eq!(d.hunks[1].old, 2..4);
        assert_eq!(d.hunks[1].new, 3..4);
        let deleted = diff("a\nb\n", "a\n");
        assert_eq!(deleted.hunks[0].kind(), HunkKind::Deleted);
        assert_eq!(deleted.hunks[0].new, 1..1);
    }

    #[test]
    fn identical_texts_have_no_hunks() {
        assert!(diff("same\n", "same\n").hunks.is_empty());
        assert!(diff("", "").hunks.is_empty());
    }

    #[test]
    fn new_file_is_one_inserted_hunk() {
        let d = diff("", "x\ny\n");
        assert_eq!(d.hunks.len(), 1);
        assert_eq!(d.hunks[0].kind(), HunkKind::Inserted);
        assert_eq!(d.hunks[0].new, 0..2);
    }

    #[test]
    fn highlights_changed_words_on_both_sides() {
        let d = diff(
            "return \"Hello, \" + name;\n",
            "return \"Hi, \" + name + \"!\";\n",
        );
        let hunk = &d.hunks[0];
        assert_eq!(hunk.kind(), HunkKind::Modified);
        let old_line = d.old_lines()[0];
        let changed_old: Vec<_> = hunk
            .old_words
            .iter()
            .map(|r| &old_line[r.columns.clone()])
            .collect();
        assert!(
            changed_old.iter().any(|w| w.contains("Hello")),
            "{changed_old:?}"
        );
        let new_line = d.new_lines()[0];
        let changed_new: Vec<_> = hunk
            .new_words
            .iter()
            .map(|r| &new_line[r.columns.clone()])
            .collect();
        assert!(
            changed_new.iter().any(|w| w.contains("Hi")),
            "{changed_new:?}"
        );
        assert!(!changed_new.iter().any(|w| w.contains("return")));
    }

    #[test]
    fn word_ranges_stay_on_their_lines() {
        let d = diff("a b\nc d\n", "a X\nc Y\n");
        let hunk = &d.hunks[0];
        let lines: Vec<_> = hunk.new_words.iter().map(|r| r.line).collect();
        assert_eq!(lines, vec![0, 1]);
        for range in &hunk.new_words {
            assert!(range.columns.end <= d.new_lines()[range.line].len());
        }
    }

    #[test]
    fn maps_scroll_positions_across_hunks() {
        // Two lines inserted after line 0.
        let d = diff("a\nb\n", "a\nx\ny\nb\n");
        assert_eq!(d.new_to_old(0.0), 0.0);
        assert_eq!(d.new_to_old(2.0), 1.0);
        assert_eq!(d.new_to_old(3.0), 1.0);
        assert_eq!(d.new_to_old(4.0), 2.0);
    }

    #[test]
    fn hunk_navigation_stops_at_the_ends() {
        let d = diff("a\nb\nc\nd\n", "A\nb\nc\nD\n");
        assert_eq!(d.next_hunk(None), Some(0));
        assert_eq!(d.next_hunk(Some(0)), Some(1));
        assert_eq!(d.next_hunk(Some(1)), None);
        assert_eq!(d.previous_hunk(None), Some(1));
        assert_eq!(d.previous_hunk(Some(0)), None);
    }
}
