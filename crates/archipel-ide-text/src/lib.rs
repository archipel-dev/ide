use ropey::Rope;
use thiserror::Error;

pub mod edit_map;
pub mod format;
pub mod multicaret;
pub mod ops;
pub mod transform;

pub use edit_map::EditMap;

#[derive(Debug, Clone, PartialEq, Eq)]
struct AtomicEdit {
    original_start: usize,
    applied_start: usize,
    deleted: String,
    inserted: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Transaction {
    edits: Vec<AtomicEdit>,
    /// Produced by typing; later adjacent typing may extend it.
    typing: bool,
    before_state: u64,
    after_state: u64,
}

/// Maximum number of journaled edits kept before consumers must resynchronize
/// from the full text (see [`TextDocument::take_edits`]).
const MAX_JOURNAL_EDITS: usize = 4096;

/// A row/column position where the column counts UTF-8 bytes from line start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BytePoint {
    pub row: usize,
    pub column: usize,
}

/// One applied buffer mutation in UTF-8 byte coordinates, in the shape
/// incremental parsers (Tree-sitter `InputEdit`) expect. Offsets refer to the
/// text as it was immediately before this edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteEdit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_point: BytePoint,
    pub old_end_point: BytePoint,
    pub new_end_point: BytePoint,
}

/// Edits applied since the last [`TextDocument::take_edits`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditJournal {
    /// The consumer must rebuild from the full text (new document, or too many
    /// edits accumulated without being drained).
    Reset,
    /// Edits in application order.
    Edits(Vec<ByteEdit>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextChange {
    pub start_char: usize,
    pub end_char: usize,
    pub inserted: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    pub fn caret(position: usize) -> Self {
        Self {
            anchor: position,
            head: position,
        }
    }

    pub fn start(self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn is_caret(self) -> bool {
        self.anchor == self.head
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSet {
    selections: Vec<Selection>,
    /// Selections added by "Add Selection for Next Occurrence", most recent last,
    /// so "Unselect Occurrence" can remove them in reverse order.
    occurrences: Vec<Selection>,
    /// Whether the occurrence search started from a word at a bare caret
    /// (whole-word matching) rather than from an explicit selection.
    occurrence_whole_words: bool,
    /// Column selection mode toggle and the block it last produced.
    column_mode: bool,
    block: Option<multicaret::BlockSelection>,
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::single(0)
    }
}

impl SelectionSet {
    pub fn single(position: usize) -> Self {
        Self {
            selections: vec![Selection::caret(position)],
            occurrences: Vec::new(),
            occurrence_whole_words: false,
            column_mode: false,
            block: None,
        }
    }

    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    pub fn primary(&self) -> Selection {
        self.selections[0]
    }

    /// Replaces all selections; overlapping ones are merged. An empty input keeps
    /// a caret at offset 0 so there is always a primary selection.
    pub fn set_all(&mut self, selections: Vec<Selection>) {
        self.selections = if selections.is_empty() {
            vec![Selection::caret(0)]
        } else {
            selections
        };
        self.normalize();
    }

    pub fn set_single(&mut self, selection: Selection) {
        self.occurrences.clear();
        self.occurrence_whole_words = false;
        self.block = None;
        self.selections.clear();
        self.selections.push(selection);
    }

    pub fn add(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.normalize();
    }

    pub fn clamp(&mut self, len: usize) {
        for selection in &mut self.selections {
            selection.anchor = selection.anchor.min(len);
            selection.head = selection.head.min(len);
        }
        self.normalize();
    }

    pub fn insert(&mut self, document: &mut TextDocument, text: &str) -> Result<(), TextError> {
        self.normalize();
        let inserted_len = text.chars().count();
        if self.selections.len() == 1 && self.selections[0].is_caret() && !text.is_empty() {
            let position = self.selections[0].head;
            document.type_text(position, text)?;
            self.selections[0] = Selection::caret(position + inserted_len);
            return Ok(());
        }
        if !text.is_empty() && self.selections.iter().all(|s| s.is_caret()) {
            let positions: Vec<usize> = self.selections.iter().map(|s| s.head).collect();
            document.type_text_many(&positions, text)?;
            for (index, selection) in self.selections.iter_mut().enumerate() {
                *selection = Selection::caret(positions[index] + inserted_len * (index + 1));
            }
            return Ok(());
        }
        let mut shift: isize = 0;
        let mut next = Vec::with_capacity(self.selections.len());
        let changes: Vec<_> = self
            .selections
            .iter()
            .map(|selection| {
                let start = selection.start();
                let end = selection.end();
                let applied_start = start.saturating_add_signed(shift);
                shift += inserted_len as isize - (end - start) as isize;
                next.push(Selection::caret(applied_start + inserted_len));
                TextChange {
                    start_char: start,
                    end_char: end,
                    inserted: text.to_owned(),
                }
            })
            .collect();
        document.edit_many_kind(changes, !text.is_empty())?;
        self.selections = next;
        Ok(())
    }

    pub fn backspace(&mut self, document: &mut TextDocument) -> Result<(), TextError> {
        for selection in &mut self.selections {
            if selection.is_caret() && selection.head > 0 {
                selection.anchor = selection.head - 1;
            }
        }
        self.insert(document, "")
    }

    pub fn delete_forward(&mut self, document: &mut TextDocument) -> Result<(), TextError> {
        let len = document.len_chars();
        for selection in &mut self.selections {
            if selection.is_caret() && selection.head < len {
                selection.head += 1;
            }
        }
        self.insert(document, "")
    }

    fn normalize(&mut self) {
        self.selections
            .sort_by_key(|selection| (selection.start(), selection.end()));
        let mut normalized: Vec<Selection> = Vec::with_capacity(self.selections.len());
        for selection in self.selections.drain(..) {
            if let Some(previous) = normalized.last_mut() {
                let duplicate_caret =
                    previous.is_caret() && selection.is_caret() && previous.head == selection.head;
                if selection.start() < previous.end() || duplicate_caret {
                    let start = previous.start().min(selection.start());
                    let end = previous.end().max(selection.end());
                    *previous = Selection {
                        anchor: start,
                        head: end,
                    };
                    continue;
                }
            }
            normalized.push(selection);
        }
        self.selections = normalized;
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TextError {
    #[error("invalid character range {start}..{end} for document length {len}")]
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    #[error("text changes overlap or are not ordered")]
    OverlappingChanges,
    #[error("invalid regular expression: {0}")]
    InvalidPattern(String),
}

/// Revisioned UTF-8 text storage. All public offsets are Unicode scalar-value
/// (character) offsets, matching Ropey's native indexing and never splitting UTF-8.
#[derive(Debug, Clone)]
pub struct TextDocument {
    rope: Rope,
    revision: u64,
    current_state: u64,
    saved_state: u64,
    next_state: u64,
    undo: Vec<Transaction>,
    redo: Vec<Transaction>,
    journal: Vec<ByteEdit>,
    journal_reset: bool,
}

impl TextDocument {
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            revision: 0,
            current_state: 0,
            saved_state: 0,
            next_state: 1,
            undo: Vec::new(),
            redo: Vec::new(),
            journal: Vec::new(),
            journal_reset: true,
        }
    }

    /// Drains the byte-level edit journal. Every mutation (edits, typing,
    /// undo and redo) is journaled; a new document starts with
    /// [`EditJournal::Reset`].
    pub fn take_edits(&mut self) -> EditJournal {
        let edits = std::mem::take(&mut self.journal);
        if std::mem::take(&mut self.journal_reset) {
            EditJournal::Reset
        } else {
            EditJournal::Edits(edits)
        }
    }

    fn byte_point(&self, byte: usize) -> BytePoint {
        let row = self.rope.byte_to_line(byte);
        BytePoint {
            row,
            column: byte - self.rope.line_to_byte(row),
        }
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, line: usize) -> Option<String> {
        (line < self.len_lines()).then(|| self.rope.line(line).to_string())
    }

    /// Text between two character offsets.
    pub fn slice(&self, start: usize, end: usize) -> Result<String, TextError> {
        self.validate_range(start, end)?;
        Ok(self.rope.slice(start..end).to_string())
    }

    pub fn char_at(&self, index: usize) -> Option<char> {
        (index < self.len_chars()).then(|| self.rope.char(index))
    }

    pub fn char_to_line(&self, index: usize) -> usize {
        self.rope.char_to_line(index.min(self.len_chars()))
    }

    pub fn line_to_char(&self, line: usize) -> Option<usize> {
        (line < self.len_lines()).then(|| self.rope.line_to_char(line))
    }

    /// UTF-8 byte offset of a character offset (clamped to the end).
    pub fn char_to_byte(&self, index: usize) -> usize {
        self.rope.char_to_byte(index.min(self.len_chars()))
    }

    /// Character offset of a UTF-8 byte offset (clamped to the end; a byte
    /// inside a character maps to that character).
    pub fn byte_to_char(&self, byte: usize) -> usize {
        self.rope.byte_to_char(byte.min(self.rope.len_bytes()))
    }

    pub fn line_to_byte(&self, line: usize) -> Option<usize> {
        (line < self.len_lines()).then(|| self.rope.line_to_byte(line))
    }

    pub fn char_to_line_column(&self, char_index: usize) -> Result<(usize, usize), TextError> {
        self.validate_range(char_index, char_index)?;
        let line = self.rope.char_to_line(char_index);
        Ok((line, char_index - self.rope.line_to_char(line)))
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        self.current_state != self.saved_state
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn mark_saved(&mut self) {
        self.saved_state = self.current_state;
    }

    /// Converts a rope character offset to the UTF-16 line/character position
    /// required by the Language Server Protocol.
    pub fn lsp_position(&self, char_index: usize) -> Result<(usize, usize), TextError> {
        self.validate_range(char_index, char_index)?;
        let line = self.rope.char_to_line(char_index);
        let line_start = self.rope.line_to_char(line);
        let utf16_character = self
            .rope
            .slice(line_start..char_index)
            .chars()
            .map(char::len_utf16)
            .sum();
        Ok((line, utf16_character))
    }

    /// Converts an LSP UTF-16 line/character position to a rope character
    /// offset. Positions past the end of a line clamp to the line end, and
    /// lines past the end clamp to the document end, as the protocol requires.
    pub fn char_from_lsp_position(&self, line: usize, character: usize) -> usize {
        if line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(line);
        let mut units = 0;
        let mut offset = line_start;
        for c in self.rope.line(line).chars() {
            if units >= character || c == '\n' || c == '\r' {
                break;
            }
            units += c.len_utf16();
            offset += 1;
        }
        offset
    }

    /// Strict variant of [`Self::char_from_lsp_position`] for edits that must
    /// be validated: a character past the end of the line still means the line
    /// end, but a line past the end of the document is an error.
    pub fn try_char_from_lsp_position(
        &self,
        line: usize,
        utf16: usize,
    ) -> Result<usize, TextError> {
        let len = self.len_chars();
        if line >= self.rope.len_lines() {
            return Err(TextError::InvalidRange {
                start: line,
                end: line,
                len,
            });
        }
        let start = self.rope.line_to_char(line);
        let mut units = 0;
        for (index, c) in self.rope.line(line).chars().enumerate() {
            if units >= utf16 || c == '\n' || c == '\r' {
                return Ok(start + index);
            }
            units += c.len_utf16();
        }
        Ok(start + self.rope.line(line).len_chars())
    }

    pub fn edit(
        &mut self,
        start_char: usize,
        end_char: usize,
        inserted: &str,
    ) -> Result<(), TextError> {
        self.edit_many([TextChange {
            start_char,
            end_char,
            inserted: inserted.to_owned(),
        }])
    }

    /// Applies non-overlapping edits against one document snapshot as one undo step.
    /// Input order is irrelevant; offsets always refer to the pre-transaction text.
    pub fn edit_many(
        &mut self,
        changes: impl IntoIterator<Item = TextChange>,
    ) -> Result<(), TextError> {
        self.edit_many_kind(changes, false)
    }

    fn edit_many_kind(
        &mut self,
        changes: impl IntoIterator<Item = TextChange>,
        typing: bool,
    ) -> Result<(), TextError> {
        let mut changes: Vec<_> = changes.into_iter().collect();
        changes.sort_by_key(|change| (change.start_char, change.end_char));
        for change in &changes {
            self.validate_range(change.start_char, change.end_char)?;
        }
        if changes
            .windows(2)
            .any(|pair| pair[0].end_char > pair[1].start_char)
        {
            return Err(TextError::OverlappingChanges);
        }

        let mut shift: isize = 0;
        let mut edits = Vec::new();
        for change in changes {
            let deleted = self
                .rope
                .slice(change.start_char..change.end_char)
                .to_string();
            if deleted == change.inserted {
                continue;
            }
            let applied_start = change.start_char.saturating_add_signed(shift);
            shift += change.inserted.chars().count() as isize
                - (change.end_char - change.start_char) as isize;
            edits.push(AtomicEdit {
                original_start: change.start_char,
                applied_start,
                deleted,
                inserted: change.inserted,
            });
        }
        if edits.is_empty() {
            return Ok(());
        }
        for edit in edits.iter().rev() {
            self.apply(
                edit.original_start,
                edit.original_start + edit.deleted.chars().count(),
                &edit.inserted,
            );
        }
        self.record_transaction(edits, typing);
        Ok(())
    }

    /// Inserts adjacent typing into the previous insertion transaction when safe.
    pub fn type_text(&mut self, position: usize, text: &str) -> Result<(), TextError> {
        self.validate_range(position, position)?;
        if text.is_empty() {
            return Ok(());
        }
        let can_coalesce = self.current_state != self.saved_state
            && self.redo.is_empty()
            && self.undo.last().is_some_and(|transaction| {
                transaction.typing
                    && transaction.edits.len() == 1
                    && transaction.edits[0].applied_start
                        + transaction.edits[0].inserted.chars().count()
                        == position
            });
        self.apply(position, position, text);
        if can_coalesce {
            let after_state = self.allocate_state();
            let transaction = self.undo.last_mut().expect("coalescing requires history");
            transaction.edits[0].inserted.push_str(text);
            transaction.after_state = after_state;
            self.current_state = after_state;
            self.revision = self.revision.saturating_add(1);
        } else {
            self.record_transaction(
                vec![AtomicEdit {
                    original_start: position,
                    applied_start: position,
                    deleted: String::new(),
                    inserted: text.to_owned(),
                }],
                true,
            );
        }
        Ok(())
    }

    /// Multi-caret typing: inserts `text` at every position (ascending, distinct)
    /// as one transaction, and extends the previous multi-caret typing
    /// transaction when every caret continues right after its earlier insertion,
    /// so a typed word at N carets undoes in one step.
    pub fn type_text_many(&mut self, positions: &[usize], text: &str) -> Result<(), TextError> {
        if positions.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(TextError::OverlappingChanges);
        }
        for &position in positions {
            self.validate_range(position, position)?;
        }
        if text.is_empty() || positions.is_empty() {
            return Ok(());
        }
        let can_coalesce = self.current_state != self.saved_state
            && self.redo.is_empty()
            && self.undo.last().is_some_and(|transaction| {
                transaction.typing
                    && transaction.edits.len() == positions.len()
                    && transaction
                        .edits
                        .iter()
                        .zip(positions)
                        .all(|(edit, &position)| {
                            edit.applied_start + edit.inserted.chars().count() == position
                        })
            });
        if !can_coalesce {
            return self.edit_many_kind(
                positions.iter().map(|&position| TextChange {
                    start_char: position,
                    end_char: position,
                    inserted: text.to_owned(),
                }),
                true,
            );
        }
        for &position in positions.iter().rev() {
            self.apply(position, position, text);
        }
        let added = text.chars().count();
        let after_state = self.allocate_state();
        let transaction = self.undo.last_mut().expect("coalescing requires history");
        for (index, edit) in transaction.edits.iter_mut().enumerate() {
            edit.applied_start += added * index;
            edit.inserted.push_str(text);
        }
        transaction.after_state = after_state;
        self.current_state = after_state;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    /// Converts a UI-produced snapshot into one minimal contiguous edit. This keeps
    /// the engine transactional while the first UI shell still uses egui TextEdit.
    pub fn replace_from_snapshot(&mut self, next: &str) -> Result<(), TextError> {
        let current = self.text();
        if current == next {
            return Ok(());
        }
        let old_chars: Vec<char> = current.chars().collect();
        let new_chars: Vec<char> = next.chars().collect();
        let prefix = old_chars
            .iter()
            .zip(&new_chars)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = old_chars[prefix..]
            .iter()
            .rev()
            .zip(new_chars[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let old_end = old_chars.len() - suffix;
        let new_end = new_chars.len() - suffix;
        let inserted: String = new_chars[prefix..new_end].iter().collect();
        self.edit(prefix, old_end, &inserted)
    }

    pub fn undo(&mut self) -> bool {
        self.undo_carets().is_some()
    }

    pub fn redo(&mut self) -> bool {
        self.redo_carets().is_some()
    }

    /// Undoes the last transaction and returns where carets belong afterwards:
    /// one per edit, after the restored text (where the caret was before a
    /// deletion, where typing started for an insertion).
    pub fn undo_carets(&mut self) -> Option<Vec<usize>> {
        let transaction = self.undo.pop()?;
        let carets = transaction
            .edits
            .iter()
            .map(|edit| edit.original_start + edit.deleted.chars().count())
            .collect();
        for edit in transaction.edits.iter().rev() {
            self.apply(
                edit.applied_start,
                edit.applied_start + edit.inserted.chars().count(),
                &edit.deleted,
            );
        }
        self.current_state = transaction.before_state;
        self.redo.push(transaction);
        self.revision = self.revision.saturating_add(1);
        Some(carets)
    }

    /// Redoes the next transaction; carets go after each re-applied edit.
    pub fn redo_carets(&mut self) -> Option<Vec<usize>> {
        let transaction = self.redo.pop()?;
        let carets = transaction
            .edits
            .iter()
            .map(|edit| edit.applied_start + edit.inserted.chars().count())
            .collect();
        for edit in transaction.edits.iter().rev() {
            self.apply(
                edit.original_start,
                edit.original_start + edit.deleted.chars().count(),
                &edit.inserted,
            );
        }
        self.current_state = transaction.after_state;
        self.undo.push(transaction);
        self.revision = self.revision.saturating_add(1);
        Some(carets)
    }

    fn validate_range(&self, start: usize, end: usize) -> Result<(), TextError> {
        let len = self.len_chars();
        if start > end || end > len {
            return Err(TextError::InvalidRange { start, end, len });
        }
        Ok(())
    }

    fn apply(&mut self, start: usize, end: usize, inserted: &str) {
        let start_byte = self.rope.char_to_byte(start);
        let old_end_byte = self.rope.char_to_byte(end);
        let start_point = self.byte_point(start_byte);
        let old_end_point = self.byte_point(old_end_byte);
        if start != end {
            self.rope.remove(start..end);
        }
        if !inserted.is_empty() {
            self.rope.insert(start, inserted);
        }
        if self.journal_reset {
            return;
        }
        if self.journal.len() >= MAX_JOURNAL_EDITS {
            self.journal.clear();
            self.journal_reset = true;
            return;
        }
        let new_end_byte = start_byte + inserted.len();
        self.journal.push(ByteEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_point,
            old_end_point,
            new_end_point: self.byte_point(new_end_byte),
        });
    }

    fn allocate_state(&mut self) -> u64 {
        let state = self.next_state;
        self.next_state = self.next_state.saturating_add(1);
        state
    }

    fn record_transaction(&mut self, edits: Vec<AtomicEdit>, typing: bool) {
        let before_state = self.current_state;
        let after_state = self.allocate_state();
        self.undo.push(Transaction {
            edits,
            typing,
            before_state,
            after_state,
        });
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        self.current_state = after_state;
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn lsp_positions_convert_to_character_offsets() {
        let doc = TextDocument::new("a𝔸b\r\nxyz\n");
        assert_eq!(doc.try_char_from_lsp_position(0, 0).unwrap(), 0);
        // 𝔸 is two UTF-16 units.
        assert_eq!(doc.try_char_from_lsp_position(0, 3).unwrap(), 2);
        // Past the end of the line clamps to the line end, before "\r\n".
        assert_eq!(doc.try_char_from_lsp_position(0, 99).unwrap(), 3);
        assert_eq!(doc.try_char_from_lsp_position(1, 1).unwrap(), 6);
        // The empty last line exists; beyond it is an error.
        assert_eq!(doc.try_char_from_lsp_position(2, 0).unwrap(), 9);
        assert!(doc.try_char_from_lsp_position(3, 0).is_err());
        for offset in 0..doc.len_chars() {
            let (line, character) = doc.lsp_position(offset).unwrap();
            if doc.char_at(offset) != Some('\n') {
                assert_eq!(
                    doc.try_char_from_lsp_position(line, character).unwrap(),
                    offset
                );
            }
        }
    }

    use super::*;

    #[test]
    fn undo_and_redo_report_carets_at_the_change() {
        let mut doc = TextDocument::new("fn a() {}\nfn b() {}\n");
        doc.type_text(10, "let").unwrap();
        doc.type_text(13, " x").unwrap();
        assert_eq!(doc.undo_carets(), Some(vec![10]), "where typing started");
        assert_eq!(doc.redo_carets(), Some(vec![15]), "after the re-typed text");
        doc.edit(0, 3, "").unwrap();
        assert_eq!(doc.undo_carets(), Some(vec![3]), "after restored text");
        doc.edit_many([
            TextChange {
                start_char: 0,
                end_char: 0,
                inserted: "// ".into(),
            },
            TextChange {
                start_char: 10,
                end_char: 10,
                inserted: "// ".into(),
            },
        ])
        .unwrap();
        assert_eq!(doc.undo_carets(), Some(vec![0, 10]), "one caret per edit");
        assert_eq!(doc.redo_carets(), Some(vec![3, 16]));
        let mut empty = TextDocument::new("");
        assert_eq!(empty.undo_carets(), None);
    }
    use proptest::prelude::*;

    #[test]
    fn journal_reports_byte_edits_including_undo() {
        let mut doc = TextDocument::new("ä\nbc\n");
        assert_eq!(doc.take_edits(), EditJournal::Reset);
        assert_eq!(doc.take_edits(), EditJournal::Edits(Vec::new()));
        doc.edit(3, 4, "XY").unwrap();
        let point = |row, column| BytePoint { row, column };
        let expected = ByteEdit {
            start_byte: 4,
            old_end_byte: 5,
            new_end_byte: 6,
            start_point: point(1, 1),
            old_end_point: point(1, 2),
            new_end_point: point(1, 3),
        };
        assert_eq!(doc.take_edits(), EditJournal::Edits(vec![expected]));
        assert!(doc.undo());
        let EditJournal::Edits(edits) = doc.take_edits() else {
            panic!("expected edits");
        };
        assert_eq!(edits.len(), 1);
        assert_eq!(
            (
                edits[0].start_byte,
                edits[0].old_end_byte,
                edits[0].new_end_byte
            ),
            (4, 6, 5)
        );
    }

    #[test]
    fn journal_overflow_requests_reset() {
        let mut doc = TextDocument::new("");
        doc.take_edits();
        for _ in 0..=MAX_JOURNAL_EDITS {
            doc.edit(0, 0, "x").unwrap();
        }
        assert_eq!(doc.take_edits(), EditJournal::Reset);
        doc.edit(0, 0, "y").unwrap();
        assert!(matches!(doc.take_edits(), EditJournal::Edits(edits) if edits.len() == 1));
    }

    #[test]
    fn edits_unicode_without_splitting_and_tracks_history() {
        let mut doc = TextDocument::new("a🦀c\n");
        doc.edit(1, 2, "Rust").unwrap();
        assert_eq!(doc.text(), "aRustc\n");
        assert!(doc.is_dirty());
        assert!(doc.undo());
        assert_eq!(doc.text(), "a🦀c\n");
        assert!(doc.redo());
        assert_eq!(doc.text(), "aRustc\n");
    }

    #[test]
    fn snapshot_replacement_records_only_changed_span() {
        let mut doc = TextDocument::new("alpha beta gamma");
        doc.replace_from_snapshot("alpha delta gamma").unwrap();
        assert_eq!(doc.text(), "alpha delta gamma");
        assert!(doc.undo());
        assert_eq!(doc.text(), "alpha beta gamma");
    }

    #[test]
    fn undo_back_to_saved_state_clears_dirty_and_branching_stays_dirty() {
        let mut doc = TextDocument::new("one");
        doc.edit(3, 3, " two").unwrap();
        doc.mark_saved();
        doc.edit(7, 7, " three").unwrap();
        assert!(doc.is_dirty());
        assert!(doc.undo());
        assert!(!doc.is_dirty());
        assert!(doc.undo());
        assert!(doc.is_dirty());
        doc.edit(0, 3, "ONE").unwrap();
        assert!(doc.is_dirty());
    }

    #[test]
    fn converts_character_offsets_to_lsp_utf16_positions() {
        let doc = TextDocument::new("one\na🦀b\n");
        assert_eq!(doc.lsp_position(4).unwrap(), (1, 0));
        assert_eq!(doc.lsp_position(6).unwrap(), (1, 3));
        assert!(doc.lsp_position(99).is_err());
    }

    #[test]
    fn converts_lsp_utf16_positions_back_to_character_offsets() {
        let doc = TextDocument::new("one\na🦀b\n");
        assert_eq!(doc.char_from_lsp_position(1, 0), 4);
        assert_eq!(doc.char_from_lsp_position(1, 3), 6);
        assert_eq!(doc.char_from_lsp_position(1, 4), 7);
        assert_eq!(doc.char_from_lsp_position(1, 99), 7, "clamps to line end");
        assert_eq!(doc.char_from_lsp_position(9, 0), doc.len_chars());
        for offset in 0..doc.len_chars() {
            let (line, character) = doc.lsp_position(offset).unwrap();
            assert_eq!(doc.char_from_lsp_position(line, character), offset);
        }
    }

    #[test]
    fn multi_cursor_edits_are_one_reversible_transaction() {
        let mut doc = TextDocument::new("alpha beta gamma");
        doc.edit_many([
            TextChange {
                start_char: 0,
                end_char: 5,
                inserted: "A".into(),
            },
            TextChange {
                start_char: 11,
                end_char: 16,
                inserted: "G".into(),
            },
        ])
        .unwrap();
        assert_eq!(doc.text(), "A beta G");
        assert!(doc.undo());
        assert_eq!(doc.text(), "alpha beta gamma");
        assert!(doc.redo());
        assert_eq!(doc.text(), "A beta G");
    }

    #[test]
    fn overlapping_multi_cursor_edits_are_rejected_without_mutation() {
        let mut doc = TextDocument::new("abcdef");
        let error = doc
            .edit_many([
                TextChange {
                    start_char: 1,
                    end_char: 4,
                    inserted: "x".into(),
                },
                TextChange {
                    start_char: 3,
                    end_char: 5,
                    inserted: "y".into(),
                },
            ])
            .unwrap_err();
        assert_eq!(error, TextError::OverlappingChanges);
        assert_eq!(doc.text(), "abcdef");
        assert!(!doc.can_undo());
    }

    #[test]
    fn adjacent_typing_coalesces_until_a_save_boundary() {
        let mut doc = TextDocument::new("");
        doc.type_text(0, "a").unwrap();
        doc.type_text(1, "🦀").unwrap();
        doc.type_text(2, "c").unwrap();
        assert_eq!(doc.text(), "a🦀c");
        assert!(doc.undo());
        assert_eq!(doc.text(), "");

        doc.type_text(0, "x").unwrap();
        doc.mark_saved();
        doc.type_text(1, "y").unwrap();
        assert!(doc.undo());
        assert_eq!(doc.text(), "x");
        assert!(!doc.is_dirty());
    }

    #[test]
    fn selection_set_inserts_at_multiple_carets_and_tracks_new_positions() {
        let mut doc = TextDocument::new("ab cd");
        let mut selections = SelectionSet::single(0);
        selections.add(Selection::caret(3));
        selections.insert(&mut doc, "_").unwrap();
        assert_eq!(doc.text(), "_ab _cd");
        assert_eq!(
            selections.selections(),
            &[Selection::caret(1), Selection::caret(5)]
        );
        assert!(doc.undo());
        assert_eq!(doc.text(), "ab cd");
    }

    #[test]
    fn selection_set_replaces_and_merges_overlapping_ranges() {
        let mut doc = TextDocument::new("abcdef");
        let mut selections = SelectionSet::single(4);
        selections.set_single(Selection { anchor: 4, head: 1 });
        selections.add(Selection { anchor: 3, head: 5 });
        selections.insert(&mut doc, "X").unwrap();
        assert_eq!(doc.text(), "aXf");
        assert_eq!(selections.primary(), Selection::caret(2));
    }

    #[test]
    fn multi_cursor_deletion_and_clamping_are_unicode_safe() {
        let mut doc = TextDocument::new("a🦀 bc");
        let mut selections = SelectionSet::single(2);
        selections.add(Selection::caret(5));
        selections.backspace(&mut doc).unwrap();
        assert_eq!(doc.text(), "a b");
        assert_eq!(
            selections.selections(),
            &[Selection::caret(1), Selection::caret(3)]
        );
        selections.clamp(1);
        assert_eq!(selections.selections(), &[Selection::caret(1)]);
    }

    #[test]
    fn multi_caret_typing_coalesces_into_one_undo_step() {
        let mut doc = TextDocument::new("a\nb\nc");
        let mut selections = SelectionSet::single(1);
        selections.add(Selection::caret(3));
        selections.add(Selection::caret(5));
        for c in ["x", "🦀", "z"] {
            selections.insert(&mut doc, c).unwrap();
        }
        assert_eq!(doc.text(), "ax🦀z\nbx🦀z\ncx🦀z");
        assert_eq!(
            selections.selections(),
            &[
                Selection::caret(4),
                Selection::caret(9),
                Selection::caret(14)
            ]
        );
        assert!(doc.undo());
        assert_eq!(doc.text(), "a\nb\nc");
        assert!(doc.redo());
        assert_eq!(doc.text(), "ax🦀z\nbx🦀z\ncx🦀z");
        assert!(doc.undo());
        // Moving one caret breaks the group.
        selections.set_all(vec![Selection::caret(0), Selection::caret(3)]);
        selections.insert(&mut doc, "1").unwrap();
        selections.set_all(vec![Selection::caret(0), Selection::caret(3)]);
        selections.insert(&mut doc, "2").unwrap();
        assert_eq!(doc.text(), "21a\n2b1\nc");
        assert!(doc.undo());
        assert_eq!(doc.text(), "1a\nb1\nc");
        assert!(doc.type_text_many(&[2, 1], "x").is_err());
    }

    #[test]
    fn typing_over_selections_undoes_with_the_typed_word_but_not_with_cuts() {
        let mut doc = TextDocument::new("one one");
        let mut selections = SelectionSet::default();
        selections.set_all(vec![
            Selection { anchor: 0, head: 3 },
            Selection { anchor: 4, head: 7 },
        ]);
        for c in ["t", "w", "o"] {
            selections.insert(&mut doc, c).unwrap();
        }
        assert_eq!(doc.text(), "two two");
        assert!(doc.undo());
        assert_eq!(doc.text(), "one one");
        // A deletion is not typing: the next keystroke starts a new step.
        let mut doc = TextDocument::new("abc");
        doc.edit(1, 3, "").unwrap();
        doc.type_text(1, "x").unwrap();
        assert!(doc.undo());
        assert_eq!(doc.text(), "a");
    }

    proptest! {
        #[test]
        fn edit_then_undo_restores_original(original in ".{0,200}", replacement in ".{0,80}") {
            let mut doc = TextDocument::new(&original);
            let len = original.chars().count();
            doc.edit(0, len, &replacement).unwrap();
            // Identical replacements do not add an undo transaction.
            prop_assert_eq!(doc.undo(), original != replacement);
            prop_assert_eq!(doc.text(), original);
        }

        #[test]
        fn journal_replays_to_the_final_text(
            original in "[a-zä🦀\\n]{0,40}",
            steps in prop::collection::vec((0usize..60, 0usize..5, "[xé\\n]{0,3}", 0u8..6), 1..12),
        ) {
            let mut doc = TextDocument::new(&original);
            doc.take_edits();
            let mut replay = original.clone().into_bytes();
            for (at, delete, insert, action) in steps {
                let len = doc.len_chars();
                let start = at.min(len);
                match action {
                    0 => { doc.undo(); }
                    1 => { doc.redo(); }
                    _ => doc.edit(start, (start + delete).min(len), &insert).unwrap(),
                }
                let EditJournal::Edits(edits) = doc.take_edits() else {
                    panic!("journal reset");
                };
                for edit in edits {
                    let inserted = doc.text().into_bytes()[edit.start_byte..edit.new_end_byte].to_vec();
                    // Points must agree with the pre-edit text.
                    let before = String::from_utf8(replay.clone()).unwrap();
                    let row = before[..edit.start_byte].matches('\n').count();
                    prop_assert_eq!(edit.start_point.row, row);
                    replay.splice(edit.start_byte..edit.old_end_byte, inserted);
                }
                prop_assert_eq!(String::from_utf8(replay.clone()).unwrap(), doc.text());
            }
        }
    }
}
