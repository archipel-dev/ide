//! Edit descriptions returned by the typing services: pure data over byte
//! offsets, applied by the host in one undo group.

use std::ops::Range;

/// Replace the bytes `start..end` of the document with `text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

impl Edit {
    pub fn insert(at: usize, text: impl Into<String>) -> Self {
        Self {
            start: at,
            end: at,
            text: text.into(),
        }
    }

    pub fn replace(range: Range<usize>, text: impl Into<String>) -> Self {
        Self {
            start: range.start,
            end: range.end,
            text: text.into(),
        }
    }

    pub fn delete(range: Range<usize>) -> Self {
        Self::replace(range, "")
    }
}

/// Edits (ascending, non-overlapping, in the coordinates of the document
/// they were computed for) plus the caret and selection after applying them
/// (in the coordinates of the result).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditSet {
    pub edits: Vec<Edit>,
    pub caret: Option<usize>,
    pub selection: Option<Range<usize>>,
}

impl EditSet {
    /// Applies the edits to `text`; the result is what the host's document
    /// holds afterwards (used by tests and by hosts without incremental edits).
    pub fn apply(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for edit in &self.edits {
            out.push_str(&text[at..edit.start]);
            out.push_str(&edit.text);
            at = edit.end;
        }
        out.push_str(&text[at..]);
        out
    }
}

/// What typing `typed` should do before the character is inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypedDecision {
    /// Insert the character as typed.
    #[default]
    Default,
    /// Step over the existing closer instead of inserting.
    Overtype,
    /// Insert the character and this closer after it, caret between.
    InsertPair(char),
    /// Surround the selection.
    Wrap { open: char, close: char },
    /// Swallow the character.
    Consume,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_sets_apply_in_order() {
        let set = EditSet {
            edits: vec![
                Edit::insert(0, "a"),
                Edit::replace(2..3, "XY"),
                Edit::delete(4..5),
            ],
            ..EditSet::default()
        };
        assert_eq!(set.apply("01234 6"), "a01XY3 6");
    }
}
