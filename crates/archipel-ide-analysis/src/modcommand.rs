//! Edits as data (D8): quick fixes, intentions and refactorings return a
//! [`ModCommand`]; one executor applies it (one undo group, previews for
//! free). Commands are pure values: building one changes nothing.

use crate::file::{FileId, FileRev};
use crate::target::Target;
use archipel_ide_language::Anchor;
use std::ops::Range;
use std::path::PathBuf;

/// Replace `range` (bytes of the expected revision) with `new_text`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextEdit {
    pub range: Range<u32>,
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: Range<u32>, new_text: impl Into<String>) -> Self {
        Self {
            range,
            new_text: new_text.into(),
        }
    }

    pub fn insert(at: u32, text: impl Into<String>) -> Self {
        Self::new(at..at, text)
    }

    pub fn delete(range: Range<u32>) -> Self {
        Self::new(range, "")
    }
}

/// Why edits could not be applied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("edit {0:?} is outside the text or not on a character boundary")]
    BadRange(Range<u32>),
    #[error("edits overlap at {0:?}")]
    Overlap(Range<u32>),
    #[error("file {file:?} changed (expected rev {expected:?}, found {found:?})")]
    Stale {
        file: FileId,
        expected: FileRev,
        found: FileRev,
    },
    #[error("unknown file {0:?}")]
    UnknownFile(FileId),
}

/// Applies non-overlapping `edits` (any order) to `text`.
pub fn apply_edits(text: &str, edits: &[TextEdit]) -> Result<String, EditError> {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| (edit.range.start, edit.range.end));
    for pair in sorted.windows(2) {
        if pair[1].range.start < pair[0].range.end {
            return Err(EditError::Overlap(pair[1].range.clone()));
        }
    }
    let mut out = String::with_capacity(text.len());
    let mut at = 0usize;
    for edit in sorted {
        let (start, end) = (edit.range.start as usize, edit.range.end as usize);
        if start > end
            || end > text.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            return Err(EditError::BadRange(edit.range.clone()));
        }
        out.push_str(&text[at..start]);
        out.push_str(&edit.new_text);
        at = end;
    }
    out.push_str(&text[at..]);
    Ok(out)
}

/// One conflict found while preparing a refactoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub message: String,
    pub location: Option<(FileId, Range<u32>)>,
}

/// A live template to start (variables are tab stops over the inserted text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateSession {
    pub file: FileId,
    pub at: u32,
    pub text: String,
    pub variables: Vec<TemplateVariable>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateVariable {
    pub name: String,
    /// Ranges relative to the template text; all mirror the first.
    pub ranges: Vec<Range<u32>>,
    pub default: String,
}

/// Where to navigate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigateTo {
    Target(Box<Target>),
    Range { file: FileId, range: Range<u32> },
}

/// Sealed edit/command model; see the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModCommand {
    Nothing,
    Composite(Vec<ModCommand>),
    /// Optimistic concurrency: refused when the file is not at `expected_rev`.
    UpdateFile {
        file: FileId,
        edits: Vec<TextEdit>,
        expected_rev: FileRev,
    },
    CreateFile {
        path: PathBuf,
        text: String,
    },
    DeleteFile {
        file: FileId,
    },
    MoveFile {
        file: FileId,
        to: PathBuf,
    },
    Navigate {
        to: NavigateTo,
        select: Option<Range<u32>>,
    },
    StartRename {
        anchor: Anchor,
        name: String,
    },
    StartTemplate(TemplateSession),
    ShowConflicts(Vec<Conflict>),
    Choose(Vec<(String, ModCommand)>),
    DisplayMessage {
        message: String,
        error: bool,
    },
    OpenUrl(String),
    CopyToClipboard(String),
    Highlight {
        file: FileId,
        ranges: Vec<Range<u32>>,
    },
    RegisterTabOut {
        file: FileId,
        range: Range<u32>,
        target: u32,
    },
}

impl ModCommand {
    pub fn is_nothing(&self) -> bool {
        match self {
            ModCommand::Nothing => true,
            ModCommand::Composite(parts) => parts.iter().all(ModCommand::is_nothing),
            _ => false,
        }
    }

    /// `self` then `next`, flattened.
    pub fn then(self, next: ModCommand) -> ModCommand {
        let mut parts = Vec::new();
        for command in [self, next] {
            match command {
                ModCommand::Nothing => {}
                ModCommand::Composite(inner) => parts.extend(inner),
                other => parts.push(other),
            }
        }
        match parts.len() {
            0 => ModCommand::Nothing,
            1 => parts.pop().unwrap_or(ModCommand::Nothing),
            _ => ModCommand::Composite(parts),
        }
    }

    /// Every text edit of `file` in this command (composites included).
    pub fn edits_for(&self, file: FileId) -> Vec<&TextEdit> {
        let mut out = Vec::new();
        self.visit(&mut |command| {
            if let ModCommand::UpdateFile {
                file: target,
                edits,
                ..
            } = command
                && *target == file
            {
                out.extend(edits.iter());
            }
        });
        out
    }

    /// Files whose text or existence this command changes.
    pub fn files_touched(&self) -> Vec<FileId> {
        let mut out = Vec::new();
        self.visit(&mut |command| match command {
            ModCommand::UpdateFile { file, .. }
            | ModCommand::DeleteFile { file }
            | ModCommand::MoveFile { file, .. }
                if !out.contains(file) =>
            {
                out.push(*file)
            }
            _ => {}
        });
        out
    }

    fn visit<'a>(&'a self, f: &mut impl FnMut(&'a ModCommand)) {
        f(self);
        if let ModCommand::Composite(parts) = self {
            for part in parts {
                part.visit(f);
            }
        }
    }

    /// The text of `file` after this command, for previews (`None` when the
    /// command does not edit it).
    pub fn preview(&self, file: FileId, text: &str) -> Option<Result<String, EditError>> {
        let edits: Vec<TextEdit> = self.edits_for(file).into_iter().cloned().collect();
        (!edits.is_empty()).then(|| apply_edits(text, &edits))
    }
}

/// What executing a command did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecOutcome {
    pub files_changed: Vec<FileId>,
    pub messages: Vec<String>,
    pub navigated: Option<NavigateTo>,
}

/// Applies commands (one per app; WP7's executor opens one undo group).
pub trait ModCommandExecutor {
    fn execute(&mut self, command: &ModCommand) -> Result<ExecOutcome, EditError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_apply_in_any_order_and_reject_overlaps() {
        let edits = [TextEdit::new(6..11, "Rust"), TextEdit::insert(0, ">> ")];
        assert_eq!(
            apply_edits("hello world", &edits).as_deref(),
            Ok(">> hello Rust")
        );
        assert!(matches!(
            apply_edits("abc", &[TextEdit::new(0..2, ""), TextEdit::new(1..3, "")]),
            Err(EditError::Overlap(_))
        ));
        assert!(matches!(
            apply_edits("ä", &[TextEdit::delete(1..2)]),
            Err(EditError::BadRange(_))
        ));
    }

    #[test]
    fn composites_flatten_and_preview() {
        let file = FileId(1);
        let command = ModCommand::Nothing
            .then(ModCommand::UpdateFile {
                file,
                edits: vec![TextEdit::new(0..1, "X")],
                expected_rev: FileRev(1),
            })
            .then(ModCommand::DisplayMessage {
                message: "done".into(),
                error: false,
            });
        assert!(matches!(&command, ModCommand::Composite(parts) if parts.len() == 2));
        assert_eq!(command.files_touched(), vec![file]);
        assert_eq!(command.preview(file, "abc"), Some(Ok("Xbc".into())));
        assert_eq!(command.preview(FileId(2), "abc"), None);
        assert!(ModCommand::Composite(vec![ModCommand::Nothing]).is_nothing());
    }
}
