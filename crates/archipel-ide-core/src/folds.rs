//! Collapsed code-folding regions of a document and the folding commands
//! (Collapse / Expand, Collapse All / Expand All, gutter toggles).

use crate::{Document, IdeState, spec, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_language::{ByteEdit, FoldKind, FoldRange};
use archipel_ide_text::Selection;
use serde_json::{Value, json};

/// A collapsed region, anchored by byte offsets that follow buffer edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollapsedFold {
    pub start_byte: usize,
    pub end_byte: usize,
    pub placeholder: String,
}

/// The collapsed regions of one document.
#[derive(Debug, Clone, Default)]
pub struct FoldState {
    collapsed: Vec<CollapsedFold>,
}

impl FoldState {
    /// Collapsed regions sorted by start.
    pub fn collapsed(&self) -> &[CollapsedFold] {
        &self.collapsed
    }

    pub fn is_collapsed(&self, fold: &FoldRange) -> bool {
        self.collapsed
            .iter()
            .any(|c| c.start_byte == fold.start_byte && c.end_byte == fold.end_byte)
    }

    pub fn collapse(&mut self, fold: &FoldRange) {
        if self.is_collapsed(fold) {
            return;
        }
        self.collapsed.push(CollapsedFold {
            start_byte: fold.start_byte,
            end_byte: fold.end_byte,
            placeholder: fold.placeholder.clone(),
        });
        self.collapsed
            .sort_by_key(|c| (c.start_byte, std::cmp::Reverse(c.end_byte)));
    }

    pub fn expand(&mut self, fold: &FoldRange) {
        self.collapsed
            .retain(|c| !(c.start_byte == fold.start_byte && c.end_byte == fold.end_byte));
    }

    pub fn clear(&mut self) {
        self.collapsed.clear();
    }

    /// Expands every collapsed region that hides `byte` (strictly inside).
    /// Returns true if anything was expanded.
    pub fn reveal(&mut self, byte: usize) -> bool {
        let before = self.collapsed.len();
        self.collapsed
            .retain(|c| !(c.start_byte < byte && byte < c.end_byte));
        self.collapsed.len() != before
    }

    /// Follows a buffer edit: regions after it shift, regions it touches
    /// expand (as IntelliJ does when hidden text changes).
    pub fn apply_edit(&mut self, edit: &ByteEdit) {
        self.collapsed.retain_mut(|c| {
            if edit.old_end_byte <= c.start_byte && edit.start_byte < c.start_byte {
                c.start_byte = c.start_byte + edit.new_end_byte - edit.old_end_byte;
                c.end_byte = c.end_byte + edit.new_end_byte - edit.old_end_byte;
                true
            } else {
                edit.start_byte >= c.end_byte
            }
        });
    }
}

impl Document {
    /// Folding regions intersecting the given character range.
    pub fn folds_in_chars(&self, start: usize, end: usize) -> Vec<FoldRange> {
        let start = self.text.char_to_byte(start);
        let end = self.text.char_to_byte(end);
        self.analysis.folds_in(start..end)
    }

    /// Collapses imports and one-line method bodies, as IntelliJ does when a
    /// file opens.
    pub fn collapse_default_regions(&mut self) {
        for fold in self.analysis.folds_in(0..self.analysis.source.len()) {
            if fold.collapsed_by_default {
                self.folds.collapse(&fold);
            }
        }
    }

    /// Expands regions hiding any caret.
    pub fn reveal_carets(&mut self) -> bool {
        let mut changed = false;
        for selection in self.selections.selections().to_vec() {
            changed |= self.folds.reveal(self.text.char_to_byte(selection.head));
        }
        changed
    }

    fn caret_line(&self) -> usize {
        self.text.char_to_line(self.selections.primary().head)
    }

    /// Regions whose lines contain the primary caret, innermost last.
    fn folds_at_caret(&self) -> Vec<FoldRange> {
        let head = self.selections.primary().head;
        let line = self.caret_line();
        let line_start = self.text.line_to_char(line).unwrap_or(head);
        let line_end = self
            .text
            .line_to_char(line + 1)
            .unwrap_or(self.text.len_chars());
        self.folds_in_chars(line_start, line_end)
            .into_iter()
            .filter(|fold| fold.start_line <= line && line <= fold.end_line)
            .collect()
    }

    /// Collapse (IntelliJ `CollapseRegion`): the innermost expanded region at the caret.
    pub fn collapse_at_caret(&mut self) -> bool {
        let Some(fold) = self
            .folds_at_caret()
            .into_iter()
            .rev()
            .find(|fold| !self.folds.is_collapsed(fold))
        else {
            return false;
        };
        self.folds.collapse(&fold);
        let head = self.text.char_to_byte(self.selections.primary().head);
        if fold.start_byte < head && head < fold.end_byte {
            let caret = self.text.byte_to_char(fold.start_byte);
            self.selections.set_single(Selection::caret(caret));
        }
        true
    }

    /// Expand (IntelliJ `ExpandRegion`): the collapsed region at the caret.
    pub fn expand_at_caret(&mut self) -> bool {
        let line = self.caret_line();
        let Some(fold) = self
            .folds_at_caret()
            .into_iter()
            .find(|fold| fold.start_line == line && self.folds.is_collapsed(fold))
        else {
            return false;
        };
        self.folds.expand(&fold);
        true
    }

    /// Fold Selection (IntelliJ `CollapseSelection`, Ctrl+.): collapses the
    /// selected lines into a custom `...` region, or expands the region that
    /// selection already folded. Without a selection it toggles the region at
    /// the caret. A selection within one line cannot be folded.
    pub fn fold_selection(&mut self) -> bool {
        let selection = self.selections.primary();
        if selection.is_caret() {
            return self.expand_at_caret() || self.collapse_at_caret();
        }
        let (start, end) = (selection.start(), selection.end());
        let (start_line, end_line) = (self.text.char_to_line(start), self.text.char_to_line(end));
        if start_line == end_line {
            return false;
        }
        let fold = FoldRange::new(
            self.text.char_to_byte(start),
            self.text.char_to_byte(end),
            start_line,
            end_line,
            "...".into(),
            FoldKind::Block,
        );
        if self.folds.is_collapsed(&fold) {
            self.folds.expand(&fold);
        } else {
            self.folds.collapse(&fold);
        }
        self.selections.set_single(Selection::caret(start));
        true
    }

    /// Toggles the region starting on `line` (gutter marker click).
    pub fn toggle_fold_at_line(&mut self, line: usize) -> bool {
        let start = self.text.line_to_char(line).unwrap_or(0);
        let end = self
            .text
            .line_to_char(line + 1)
            .unwrap_or(self.text.len_chars());
        let Some(fold) = self
            .folds_in_chars(start, end)
            .into_iter()
            .find(|fold| fold.start_line == line)
        else {
            return false;
        };
        if self.folds.is_collapsed(&fold) {
            self.folds.expand(&fold);
        } else {
            self.folds.collapse(&fold);
            self.reveal_carets_outside(&fold);
        }
        true
    }

    fn reveal_carets_outside(&mut self, fold: &FoldRange) {
        let head = self.text.char_to_byte(self.selections.primary().head);
        if fold.start_byte < head && head < fold.end_byte {
            let caret = self.text.byte_to_char(fold.start_byte);
            self.selections.set_single(Selection::caret(caret));
        }
    }

    pub fn collapse_all(&mut self) -> bool {
        let folds = self.analysis.folds_in(0..self.analysis.source.len());
        for fold in &folds {
            self.folds.collapse(fold);
        }
        if let Some(outer) = folds
            .iter()
            .find(|fold| {
                let head = self.text.char_to_byte(self.selections.primary().head);
                fold.start_byte < head && head < fold.end_byte
            })
            .cloned()
        {
            self.reveal_carets_outside(&outer);
        }
        !folds.is_empty()
    }

    pub fn expand_all(&mut self) -> bool {
        let changed = !self.folds.collapsed().is_empty();
        self.folds.clear();
        changed
    }
}

fn fold_command(
    registry: &mut CommandRegistry<IdeState>,
    id: &str,
    label: &str,
    shortcut: &str,
    run: fn(&mut Document) -> bool,
) {
    registry
        .register(Command::new(
            spec(id, label, "Folding", Some(shortcut), "{}"),
            |state: &IdeState| state.active_document.is_some(),
            move |state: &mut IdeState, _| {
                let document = state
                    .active_document
                    .and_then(|index| state.documents.get_mut(index))
                    .ok_or_else(|| CommandError::Disabled("no active document".into()))?;
                Ok(json!({"changed": run(document)}))
            },
        ))
        .expect("unique built-in id");
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    fold_command(
        registry,
        "editor.collapseRegion",
        "Collapse",
        "⌘-",
        Document::collapse_at_caret,
    );
    fold_command(
        registry,
        "editor.expandRegion",
        "Expand",
        "⌘=",
        Document::expand_at_caret,
    );
    fold_command(
        registry,
        "editor.foldSelection",
        "Fold Selection",
        "⌘.",
        Document::fold_selection,
    );
    fold_command(
        registry,
        "editor.collapseAllRegions",
        "Collapse All",
        "⇧⌘-",
        Document::collapse_all,
    );
    fold_command(
        registry,
        "editor.expandAllRegions",
        "Expand All",
        "⇧⌘=",
        Document::expand_all,
    );
    registry
        .register(Command::new(
            spec(
                "editor.toggleFold",
                "Toggle Fold at Line",
                "Folding",
                None,
                r#"{"line": "zero-based line of the region start"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params: Value| {
                let line = usize_param(&params, "line")?;
                let document = state
                    .active_document
                    .and_then(|index| state.documents.get_mut(index))
                    .ok_or_else(|| CommandError::Disabled("no active document".into()))?;
                Ok(json!({"changed": document.toggle_fold_at_line(line)}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "import java.util.List;\nimport java.util.Map;\n\nclass A {\n    void f() {\n        g();\n        h();\n    }\n    String s(String n) {\n        return n + \"!\";\n    }\n}\n";

    #[test]
    fn fold_selection_collapses_and_expands_the_selected_lines() {
        let mut document = Document::new("A.java".into(), SOURCE);
        // A one-line selection cannot be folded.
        document.selections.set_single(Selection {
            anchor: 25,
            head: 31,
        });
        assert!(!document.fold_selection());
        let start = document.text.line_to_char(5).unwrap();
        let end = document.text.line_to_char(7).unwrap();
        document.selections.set_single(Selection {
            anchor: start,
            head: end,
        });
        assert!(document.fold_selection());
        assert_eq!(document.folds.collapsed().len(), 1);
        assert_eq!(document.folds.collapsed()[0].placeholder, "...");
        assert!(document.selections.primary().is_caret());
        // The same selection again expands it.
        document.selections.set_single(Selection {
            anchor: start,
            head: end,
        });
        assert!(document.fold_selection());
        assert!(document.folds.collapsed().is_empty());
    }

    #[test]
    fn imports_collapse_by_default_and_commands_fold_at_caret() {
        let mut document = Document::new("A.java".into(), SOURCE);
        document.collapse_default_regions();
        assert_eq!(document.folds.collapsed().len(), 2);
        assert_eq!(document.folds.collapsed()[0].placeholder, "...");
        assert_eq!(
            document.folds.collapsed()[1].placeholder,
            "{ return n + \"!\"; }",
            "one-line method bodies fold with their statement"
        );

        let caret = SOURCE.find("g();").unwrap();
        document.selections.set_single(Selection::caret(caret));
        assert!(document.collapse_at_caret());
        // The caret moved out of the hidden text to the region start.
        let head = document.selections.primary().head;
        assert_eq!(&SOURCE[head..head + 1], "{");
        assert!(document.collapse_at_caret(), "outer class body next");
        assert_eq!(document.folds.collapsed().len(), 4);
        assert!(document.expand_all());
        assert!(document.folds.collapsed().is_empty());

        assert!(document.toggle_fold_at_line(4));
        assert!(document.toggle_fold_at_line(4));
        assert!(document.folds.collapsed().is_empty());
        assert!(!document.toggle_fold_at_line(5));
    }

    #[test]
    fn edits_shift_or_expand_collapsed_regions() {
        let mut document = Document::new("A.java".into(), SOURCE);
        assert!(document.toggle_fold_at_line(4));
        let before = document.folds.collapsed()[0].clone();
        // Typing above the region shifts it.
        document.text.edit(0, 0, "// x\n").unwrap();
        let source = document.text.text();
        document.refresh_analysis(&source);
        let after = &document.folds.collapsed()[0];
        assert_eq!(after.start_byte, before.start_byte + 5);
        assert!(
            document.toggle_fold_at_line(5),
            "still a region, now on line 5"
        );
        assert!(document.folds.collapsed().is_empty());
        // Editing inside a collapsed region expands it.
        assert!(document.toggle_fold_at_line(5));
        let inside = source.find("g();").unwrap();
        document.text.edit(inside, inside, "h();").unwrap();
        let source = document.text.text();
        document.refresh_analysis(&source);
        assert!(document.folds.collapsed().is_empty());
    }

    #[test]
    fn carets_inside_collapsed_regions_reveal_them() {
        let mut document = Document::new("A.java".into(), SOURCE);
        document.collapse_all();
        assert!(!document.folds.collapsed().is_empty());
        let caret = SOURCE.find("g();").unwrap();
        document.selections.set_single(Selection::caret(caret));
        assert!(document.reveal_carets());
        assert!(
            document
                .folds
                .collapsed()
                .iter()
                .all(|fold| !(fold.start_byte < caret && caret < fold.end_byte))
        );
    }
}
