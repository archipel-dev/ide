//! Collapsed code-folding regions of a document and the folding commands
//! (Collapse / Expand, Collapse All / Expand All, gutter toggles).

use crate::{Document, IdeState, spec, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_language::{ByteEdit, FoldDefault, FoldKind, FoldRange};
use archipel_ide_text::Selection;
use serde_json::{Value, json};

/// A collapsed region, anchored by byte offsets that follow buffer edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollapsedFold {
    pub start_byte: usize,
    pub end_byte: usize,
    pub placeholder: String,
    /// Collapsed together with the regions of the same non-zero group (the two halves of a
    /// one-line method).
    pub group: usize,
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
            group: fold.group,
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

    /// Collapses the regions the Code Folding settings collapse by default, as IntelliJ does
    /// when a file opens (ED-FOLD-020).
    pub fn collapse_default_regions(&mut self) {
        let defaults = self.fold_defaults;
        for fold in self.generated_folds() {
            if defaults.collapses(fold.default) {
                self.folds.collapse(&fold);
            }
        }
    }

    /// The regions of the syntax tree. With `One-line methods` off the two grouped halves of a
    /// one-line method are one ordinary method body region (ED-FOLD-033).
    pub fn generated_folds(&self) -> Vec<FoldRange> {
        let all = self.analysis.folds_in(0..self.analysis.source.len());
        if self.fold_defaults.one_line_methods {
            return all;
        }
        let mut out: Vec<FoldRange> = Vec::with_capacity(all.len());
        let mut index = 0;
        while index < all.len() {
            let fold = &all[index];
            if fold.group != 0
                && fold.default == FoldDefault::OneLineMethod
                && let Some(second) = all.get(index + 1).filter(|n| n.group == fold.group)
            {
                let source = &self.analysis.source;
                let brace = source
                    .get(fold.start_byte..fold.end_byte)
                    .and_then(|text| text.find('{'))
                    .map_or(fold.start_byte, |offset| fold.start_byte + offset);
                out.push(FoldRange {
                    start_byte: brace,
                    end_byte: second.end_byte,
                    start_line: fold.start_line,
                    end_line: second.end_line,
                    placeholder: "{...}".into(),
                    kind: FoldKind::Block,
                    default: FoldDefault::MethodBody,
                    group: 0,
                });
                index += 2;
                continue;
            }
            out.push(fold.clone());
            index += 1;
        }
        out
    }

    /// Every region of the document with its state: the regions of the syntax tree and the
    /// collapsed regions the user made (Fold Selection).
    pub fn fold_regions(&self) -> Vec<FoldRegion> {
        let generated = self.generated_folds();
        let mut regions: Vec<FoldRegion> = generated
            .iter()
            .map(|range| FoldRegion {
                collapsed: self.folds.is_collapsed(range),
                user: false,
                range: range.clone(),
            })
            .collect();
        let len = self.text.char_to_byte(self.text.len_chars());
        for fold in self.folds.collapsed() {
            if fold.end_byte > len || generated.iter().any(|g| same_range(g, fold)) {
                continue;
            }
            let start = self.text.byte_to_char(fold.start_byte);
            let end = self.text.byte_to_char(fold.end_byte);
            regions.push(FoldRegion {
                collapsed: true,
                user: true,
                range: FoldRange {
                    start_byte: fold.start_byte,
                    end_byte: fold.end_byte,
                    start_line: self.text.char_to_line(start),
                    end_line: self.text.char_to_line(end),
                    placeholder: fold.placeholder.clone(),
                    kind: FoldKind::Block,
                    default: FoldDefault::Expanded,
                    group: fold.group,
                },
            });
        }
        regions.sort_by(|a, b| {
            a.range
                .start_byte
                .cmp(&b.range.start_byte)
                .then(b.range.end_byte.cmp(&a.range.end_byte))
        });
        regions
    }

    /// Expands regions hiding any caret.
    pub fn reveal_carets(&mut self) -> bool {
        let mut changed = false;
        for selection in self.selections.selections().to_vec() {
            changed |= self.folds.reveal(self.text.char_to_byte(selection.head));
        }
        changed
    }

    /// The regions a caret works on: the first (outermost) region starting on its line, and the
    /// regions around it, innermost first (ED-FOLD-010, ED-FOLD-011).
    fn fold_targets(&self, regions: &[FoldRegion], head: usize) -> (Option<usize>, Vec<usize>) {
        let line = self.text.char_to_line(head);
        let head_byte = self.text.char_to_byte(head);
        let starting = regions.iter().position(|r| r.range.start_line == line);
        let mut around: Vec<usize> = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.range.start_byte <= head_byte && head_byte < r.range.end_byte)
            .map(|(i, _)| i)
            .collect();
        around.sort_by_key(|&i| regions[i].range.end_byte - regions[i].range.start_byte);
        (starting, around)
    }

    fn set_collapsed(&mut self, regions: &[FoldRegion], index: usize, collapsed: bool) {
        let group = regions[index].range.group;
        for region in regions {
            let same =
                region.range == regions[index].range || (group != 0 && region.range.group == group);
            if same {
                if collapsed {
                    self.folds.collapse(&region.range);
                } else {
                    self.folds.expand(&region.range);
                }
            }
        }
    }

    /// After a collapse, carets inside hidden text move to the start of the region
    /// (ED-FOLD-018).
    fn move_carets_out_of_folds(&mut self) {
        let collapsed = self.folds.collapsed().to_vec();
        let moved: Vec<Selection> = self
            .selections
            .selections()
            .iter()
            .map(|s| {
                let mut s = *s;
                for fold in &collapsed {
                    let (start, end) = (
                        self.text.byte_to_char(fold.start_byte),
                        self.text.byte_to_char(fold.end_byte),
                    );
                    if start < s.head && s.head < end {
                        s = Selection::caret(start);
                    }
                }
                s
            })
            .collect();
        self.selections.set_all(moved);
    }

    /// Collapse (IntelliJ `CollapseRegion`): per caret, the expanded region that starts on its
    /// line, else the innermost expanded region around it.
    pub fn collapse_at_caret(&mut self) -> bool {
        let regions = self.fold_regions();
        let mut changed = false;
        for head in self
            .selections
            .selections()
            .iter()
            .map(|s| s.head)
            .collect::<Vec<_>>()
        {
            let (starting, around) = self.fold_targets(&regions, head);
            let target = starting
                .filter(|&i| !regions[i].collapsed)
                .or_else(|| around.into_iter().find(|&i| !regions[i].collapsed));
            if let Some(index) = target {
                self.set_collapsed(&regions, index, true);
                changed = true;
            }
        }
        if changed {
            self.move_carets_out_of_folds();
        }
        changed
    }

    /// Expand (IntelliJ `ExpandRegion`): per caret, the collapsed region that starts on its line,
    /// else the outermost collapsed region around it.
    pub fn expand_at_caret(&mut self) -> bool {
        let regions = self.fold_regions();
        let mut changed = false;
        for head in self
            .selections
            .selections()
            .iter()
            .map(|s| s.head)
            .collect::<Vec<_>>()
        {
            let (starting, around) = self.fold_targets(&regions, head);
            let target = starting
                .filter(|&i| regions[i].collapsed)
                .or_else(|| around.into_iter().rev().find(|&i| regions[i].collapsed));
            if let Some(index) = target {
                self.set_collapsed(&regions, index, false);
                changed = true;
            }
        }
        changed
    }

    /// Collapse Recursively / Expand Recursively (ED-FOLD-012): the region the caret works on
    /// and every region inside it.
    pub fn fold_recursively(&mut self, collapse: bool) -> bool {
        let regions = self.fold_regions();
        let mut changed = false;
        for head in self
            .selections
            .selections()
            .iter()
            .map(|s| s.head)
            .collect::<Vec<_>>()
        {
            let (starting, around) = self.fold_targets(&regions, head);
            let wants = |i: usize| regions[i].collapsed != collapse;
            let root = starting.filter(|&i| wants(i)).or_else(|| {
                if collapse {
                    around.iter().copied().find(|&i| wants(i))
                } else {
                    around.iter().rev().copied().find(|&i| wants(i))
                }
            });
            let Some(root) = root else {
                continue;
            };
            let (start, end) = (regions[root].range.start_byte, regions[root].range.end_byte);
            for (index, region) in regions.iter().enumerate() {
                if region.range.start_byte >= start && region.range.end_byte <= end {
                    self.set_collapsed(&regions, index, collapse);
                    changed = true;
                }
            }
        }
        if changed && collapse {
            self.move_carets_out_of_folds();
        }
        changed
    }

    /// Fold Selection (IntelliJ `CollapseSelection`, Ctrl+.): collapses the selection into a
    /// user-defined `...` region, or removes the user-defined region the selection covers
    /// exactly. Without a selection it removes the user-defined region at the caret or toggles
    /// the generated one (ED-FOLD-015).
    pub fn fold_selection(&mut self) -> bool {
        let selection = self.selections.primary();
        if selection.is_caret() {
            let regions = self.fold_regions();
            let (_, around) = self.fold_targets(&regions, selection.head);
            let Some(&index) = around.first() else {
                return false;
            };
            if regions[index].user {
                self.folds.expand(&regions[index].range);
                return true;
            }
            let collapsed = regions[index].collapsed;
            self.set_collapsed(&regions, index, !collapsed);
            if !collapsed {
                self.move_carets_out_of_folds();
            }
            return true;
        }
        let start = selection.start();
        let mut end = selection.end();
        // A trailing line break is not part of the region.
        if end > start && self.text.char_at(end - 1) == Some('\n') {
            end -= 1;
        }
        if end <= start {
            return false;
        }
        let range = FoldRange {
            start_byte: self.text.char_to_byte(start),
            end_byte: self.text.char_to_byte(end),
            start_line: self.text.char_to_line(start),
            end_line: self.text.char_to_line(end),
            placeholder: "...".into(),
            kind: FoldKind::Block,
            default: FoldDefault::Expanded,
            group: 0,
        };
        if self.folds.is_collapsed(&range) {
            // Remove the user-defined region; the selection stays.
            self.folds.expand(&range);
            return true;
        }
        if self
            .generated_folds()
            .iter()
            .any(|generated| same_range(generated, &range))
        {
            return false;
        }
        self.folds.collapse(&range);
        self.selections.set_single(Selection::caret(start));
        true
    }

    /// Toggles the region starting on `line` (gutter marker click).
    pub fn toggle_fold_at_line(&mut self, line: usize) -> bool {
        let regions = self.fold_regions();
        let Some(index) = regions.iter().position(|r| r.range.start_line == line) else {
            return false;
        };
        let collapsed = regions[index].collapsed;
        self.set_collapsed(&regions, index, !collapsed);
        if !collapsed {
            self.move_carets_out_of_folds();
        }
        true
    }

    pub fn collapse_all(&mut self) -> bool {
        let regions = self.fold_regions();
        for region in &regions {
            self.folds.collapse(&region.range);
        }
        self.move_carets_out_of_folds();
        !regions.is_empty()
    }

    pub fn expand_all(&mut self) -> bool {
        let changed = !self.folds.collapsed().is_empty();
        self.folds.clear();
        changed
    }
}

/// A region of the document and its state (ED-FOLD model).
#[derive(Debug, Clone, PartialEq)]
pub struct FoldRegion {
    pub range: FoldRange,
    pub collapsed: bool,
    /// Made by Fold Selection rather than by the syntax tree.
    pub user: bool,
}

fn same_range(a: &FoldRange, b: &impl RangeLike) -> bool {
    a.start_byte == b.start() && a.end_byte == b.end()
}

trait RangeLike {
    fn start(&self) -> usize;
    fn end(&self) -> usize;
}

impl RangeLike for FoldRange {
    fn start(&self) -> usize {
        self.start_byte
    }
    fn end(&self) -> usize {
        self.end_byte
    }
}

impl RangeLike for CollapsedFold {
    fn start(&self) -> usize {
        self.start_byte
    }
    fn end(&self) -> usize {
        self.end_byte
    }
}

/// Which folding regions start collapsed (Settings | Editor | General | Code Folding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldDefaults {
    pub imports: bool,
    pub method_bodies: bool,
    pub one_line_methods: bool,
    pub doc_comments: bool,
    pub file_header: bool,
    pub custom_regions: bool,
}

impl Default for FoldDefaults {
    fn default() -> Self {
        Self {
            imports: true,
            method_bodies: false,
            one_line_methods: true,
            doc_comments: false,
            file_header: true,
            custom_regions: false,
        }
    }
}

impl FoldDefaults {
    pub fn from_settings(settings: &crate::services::Settings) -> Self {
        use archipel_ide_settings::ids;
        let defaults = Self::default();
        let flag = |id: &str, default: bool| settings.bool(id).unwrap_or(default);
        Self {
            imports: flag(ids::FOLD_IMPORTS, defaults.imports),
            method_bodies: flag(ids::FOLD_METHOD_BODIES, defaults.method_bodies),
            one_line_methods: flag(ids::FOLD_ONE_LINE_METHODS, defaults.one_line_methods),
            doc_comments: flag(ids::FOLD_DOC_COMMENTS, defaults.doc_comments),
            file_header: flag(ids::FOLD_FILE_HEADER, defaults.file_header),
            custom_regions: flag(ids::FOLD_CUSTOM_REGIONS, defaults.custom_regions),
        }
    }

    /// Whether a region with this default starts collapsed.
    pub fn collapses(&self, default: FoldDefault) -> bool {
        match default {
            FoldDefault::Expanded => false,
            FoldDefault::Imports => self.imports,
            FoldDefault::MethodBody => self.method_bodies,
            FoldDefault::OneLineMethod => self.one_line_methods,
            FoldDefault::DocComment => self.doc_comments,
            FoldDefault::FileHeader => self.file_header,
            FoldDefault::CustomRegion => self.custom_regions,
            FoldDefault::Collapsed => true,
        }
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
        "editor.collapseRegionRecursively",
        "Collapse Recursively",
        "⌥⌘-",
        |d| d.fold_recursively(true),
    );
    fold_command(
        registry,
        "editor.expandRegionRecursively",
        "Expand Recursively",
        "⌥⌘=",
        |d| d.fold_recursively(false),
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
        // A selection within one line folds too (ED-FOLD-015).
        document.selections.set_single(Selection {
            anchor: 25,
            head: 31,
        });
        assert!(document.fold_selection());
        assert_eq!(document.folds.collapsed().len(), 1);
        assert_eq!(document.selections.primary(), Selection::caret(25));
        document.selections.set_single(Selection {
            anchor: 25,
            head: 31,
        });
        assert!(document.fold_selection(), "the same selection removes it");
        assert!(document.folds.collapsed().is_empty());
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
        // Imports, and the two halves of the one-line method (ED-FOLD-030, ED-FOLD-033).
        assert_eq!(document.folds.collapsed().len(), 3);
        assert_eq!(document.folds.collapsed()[0].placeholder, "...");
        assert_eq!(document.folds.collapsed()[1].placeholder, " { ");
        assert_eq!(document.folds.collapsed()[2].placeholder, " }");
        assert_eq!(
            document.folds.collapsed()[1].group,
            document.folds.collapsed()[2].group
        );

        let caret = SOURCE.find("g();").unwrap();
        document.selections.set_single(Selection::caret(caret));
        assert!(document.collapse_at_caret());
        // The caret moved out of the hidden text to the region start.
        let head = document.selections.primary().head;
        assert_eq!(&SOURCE[head..head + 1], "{");
        assert_eq!(document.folds.collapsed().len(), 4);
        assert!(
            !document.collapse_at_caret(),
            "the only top-level class has no region"
        );
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
