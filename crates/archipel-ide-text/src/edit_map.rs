//! Offset mapping through a journal of [`ByteEdit`]s.
//!
//! An [`EditMap`] answers "where is old byte offset `p` now?" for any number of
//! edits. Per edit (applied in order, identical to the syntax layer's range
//! mapping): an offset at or before the edit start stays, an offset at or
//! after the old end moves by the length delta, and an offset strictly inside
//! the replaced span collapses to the end of the inserted text.
//!
//! The composition of such maps is a monotone step function made of "shift"
//! pieces (slope 1) and "collapse" pieces (constant). The map stores exactly
//! that normalized form, so mapping costs O(log pieces) however many
//! keystrokes the journal held (typing a word is one piece), and two maps
//! compose ([`EditMap::compose`]) into one with the same answers as mapping
//! through both in turn.

use crate::ByteEdit;
use std::ops::Range;

/// How the offsets of one piece map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceMap {
    /// `offset + delta`.
    Shift(isize),
    /// Every offset collapses to this one (text around them was replaced).
    Collapse(usize),
}

/// The offsets from `start` up to the next piece's start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub start: usize,
    pub map: PieceMap,
}

/// Normalized composition of byte edits; see the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditMap {
    /// Sorted by start; the first starts at 0; adjacent pieces differ.
    pieces: Vec<Piece>,
}

impl Default for EditMap {
    fn default() -> Self {
        Self::new()
    }
}

fn shift(offset: usize, delta: isize) -> usize {
    offset.saturating_add_signed(delta)
}

impl EditMap {
    /// The identity map.
    pub fn new() -> Self {
        Self {
            pieces: vec![Piece {
                start: 0,
                map: PieceMap::Shift(0),
            }],
        }
    }

    /// The map of one edit.
    pub fn of_edit(edit: &ByteEdit) -> Self {
        let start = edit.start_byte;
        let old_end = edit.old_end_byte.max(start);
        let new_end = edit.new_end_byte.max(start);
        let mut pieces = vec![Piece {
            start: 0,
            map: PieceMap::Shift(0),
        }];
        if old_end > start + 1 {
            pieces.push(Piece {
                start: start + 1,
                map: PieceMap::Collapse(new_end),
            });
        }
        let after = old_end.max(start + 1);
        pieces.push(Piece {
            start: after,
            map: PieceMap::Shift(new_end as isize - old_end as isize),
        });
        let mut map = Self { pieces };
        map.normalize();
        map
    }

    /// The map of `edits` applied in order (each in the coordinates left by the
    /// previous one, as `TextDocument::take_edits` journals them).
    pub fn from_edits(edits: &[ByteEdit]) -> Self {
        let mut map = Self::new();
        for edit in edits {
            map.push(edit);
        }
        map
    }

    /// Appends one edit expressed in this map's output coordinates.
    pub fn push(&mut self, edit: &ByteEdit) {
        *self = self.compose(&Self::of_edit(edit));
    }

    /// True when no offset moves.
    pub fn is_identity(&self) -> bool {
        self.pieces.len() == 1 && self.pieces[0].map == PieceMap::Shift(0)
    }

    /// The normalized pieces.
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    /// Length change of the text (the delta of the last piece).
    pub fn delta(&self) -> isize {
        match self.pieces.last().map(|piece| piece.map) {
            Some(PieceMap::Shift(delta)) => delta,
            _ => 0,
        }
    }

    fn piece_index(&self, offset: usize) -> usize {
        self.pieces
            .partition_point(|piece| piece.start <= offset)
            .saturating_sub(1)
    }

    /// Maps an old offset to the new text (see the module docs).
    pub fn map(&self, offset: usize) -> usize {
        match self.pieces[self.piece_index(offset)].map {
            PieceMap::Shift(delta) => shift(offset, delta),
            PieceMap::Collapse(target) => target,
        }
    }

    /// Maps an old offset, or `None` when the edits replaced the text around
    /// it (the offset was strictly inside a replaced span).
    pub fn map_exact(&self, offset: usize) -> Option<usize> {
        match self.pieces[self.piece_index(offset)].map {
            PieceMap::Shift(delta) => Some(shift(offset, delta)),
            PieceMap::Collapse(_) => None,
        }
    }

    /// Maps a range; the end never precedes the start.
    pub fn map_range(&self, range: Range<usize>) -> Range<usize> {
        let start = self.map(range.start);
        start..self.map(range.end).max(start)
    }

    /// True when every offset of `range` (ends included) shifts by the same
    /// delta: no edit happened inside it.
    pub fn untouched(&self, range: Range<usize>) -> bool {
        let first = self.piece_index(range.start);
        let last = self.piece_index(range.end);
        first == last && matches!(self.pieces[first].map, PieceMap::Shift(_))
    }

    /// `self` followed by `next` (whose input is this map's output).
    pub fn compose(&self, next: &EditMap) -> EditMap {
        let mut pieces = Vec::with_capacity(self.pieces.len() + next.pieces.len());
        for (index, piece) in self.pieces.iter().enumerate() {
            let end = self.pieces.get(index + 1).map(|next| next.start);
            match piece.map {
                PieceMap::Collapse(target) => pieces.push(Piece {
                    start: piece.start,
                    map: PieceMap::Collapse(next.map(target)),
                }),
                PieceMap::Shift(delta) => {
                    // Image [start + delta, end + delta) crosses `next`'s pieces.
                    let image_start = shift(piece.start, delta);
                    let image_end = end.map(|end| shift(end, delta));
                    let mut at = next.piece_index(image_start);
                    let mut start = piece.start;
                    loop {
                        let mapped = match next.pieces[at].map {
                            PieceMap::Shift(next_delta) => PieceMap::Shift(delta + next_delta),
                            PieceMap::Collapse(target) => PieceMap::Collapse(target),
                        };
                        pieces.push(Piece { start, map: mapped });
                        let Some(boundary) = next.pieces.get(at + 1).map(|piece| piece.start)
                        else {
                            break;
                        };
                        if image_end.is_some_and(|image_end| boundary >= image_end) {
                            break;
                        }
                        start = shift(boundary, -delta);
                        at += 1;
                    }
                }
            }
        }
        let mut map = EditMap { pieces };
        map.normalize();
        map
    }

    fn normalize(&mut self) {
        let mut out: Vec<Piece> = Vec::with_capacity(self.pieces.len());
        for piece in self.pieces.drain(..) {
            match out.last_mut() {
                Some(last) if last.start == piece.start => *last = piece,
                Some(last) if last.map == piece.map => {}
                _ => out.push(piece),
            }
        }
        self.pieces = out;
    }
}

/// Maps `offset` through one edit (the reference semantics).
pub fn map_through(offset: usize, edit: &ByteEdit) -> usize {
    if offset <= edit.start_byte {
        offset
    } else if offset >= edit.old_end_byte {
        offset + edit.new_end_byte - edit.old_end_byte
    } else {
        edit.new_end_byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BytePoint;
    use proptest::prelude::*;

    fn edit(start: usize, old_end: usize, new_end: usize) -> ByteEdit {
        ByteEdit {
            start_byte: start,
            old_end_byte: old_end,
            new_end_byte: new_end,
            start_point: BytePoint::default(),
            old_end_point: BytePoint::default(),
            new_end_point: BytePoint::default(),
        }
    }

    /// Random valid edit sequence over a text of `len` bytes.
    fn edits(len: usize) -> impl Strategy<Value = (usize, Vec<ByteEdit>)> {
        prop::collection::vec((0.0f64..1.0, 0usize..8, 0usize..8), 0..12).prop_map(move |raw| {
            let mut current = len;
            let mut out = Vec::new();
            for (at, delete, insert) in raw {
                let start = ((current as f64) * at) as usize;
                let old_end = (start + delete).min(current);
                out.push(edit(start, old_end, start + insert));
                current = current - (old_end - start) + insert;
            }
            (len, out)
        })
    }

    fn direct(offset: usize, edits: &[ByteEdit]) -> usize {
        edits.iter().fold(offset, map_through)
    }

    #[test]
    fn maps_like_single_edits() {
        let map = EditMap::from_edits(&[edit(4, 6, 9)]);
        assert_eq!(map.map(0), 0);
        assert_eq!(map.map(4), 4);
        assert_eq!(map.map(5), 9);
        assert_eq!(map.map(6), 9);
        assert_eq!(map.map(8), 11);
        assert_eq!(map.map_exact(5), None);
        assert_eq!(map.map_exact(6), Some(9));
        assert_eq!(map.map_range(5..7), 9..10);
        assert!(map.untouched(0..4));
        assert!(!map.untouched(3..5));
        assert!(map.untouched(6..10));
        assert_eq!(map.pieces().len(), 3);
        assert!(EditMap::new().is_identity());
        assert_eq!(map.delta(), 3);
    }

    #[test]
    fn typing_merges_into_one_hunk() {
        let mut map = EditMap::new();
        for i in 0..100 {
            map.push(&edit(10 + i, 10 + i, 11 + i));
        }
        assert_eq!(map.pieces().len(), 2);
        assert_eq!(map.map(10), 10);
        assert_eq!(map.map(11), 111);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn normalized_map_equals_direct_mapping((len, edits) in edits(40)) {
            let map = EditMap::from_edits(&edits);
            for offset in 0..=len {
                prop_assert_eq!(map.map(offset), direct(offset, &edits), "offset {}", offset);
            }
            for window in map.pieces().windows(2) {
                prop_assert!(window[0].start < window[1].start);
                prop_assert!(window[0].map != window[1].map);
            }
        }

        #[test]
        fn composition_equals_direct_mapping(
            (len, first) in edits(40),
            split in 0usize..12,
        ) {
            let split = split.min(first.len());
            let (a, b) = first.split_at(split);
            let composed = EditMap::from_edits(a).compose(&EditMap::from_edits(b));
            let whole = EditMap::from_edits(&first);
            for offset in 0..=len {
                prop_assert_eq!(composed.map(offset), direct(offset, &first), "offset {}", offset);
                prop_assert_eq!(whole.map(offset), composed.map(offset));
            }
        }
    }
}
