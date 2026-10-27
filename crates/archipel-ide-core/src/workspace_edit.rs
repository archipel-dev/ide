//! Applies LSP `WorkspaceEdit`s (code actions, `workspace/applyEdit`, rename).
//!
//! Application is all-or-nothing: every edit of every file is converted and
//! validated (positions, overlaps, document versions) before the first file
//! changes. Files that are not open are opened as background documents (they
//! appear as modified tabs, like the reference IDE's changed files), so each
//! file's change is one undo step and the language server sees it.
use crate::{Document, IdeState};
use archipel_ide_text::{Selection, TextChange};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use url::Url;

/// What an applied workspace edit changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditSummary {
    /// Project-relative paths of the changed files.
    pub files: Vec<PathBuf>,
    pub edits: usize,
}

struct FileEdits {
    relative: PathBuf,
    version: Option<i64>,
    edits: Vec<(Value, String)>,
}

fn text_edits(value: &Value) -> Result<Vec<(Value, String)>, String> {
    value
        .as_array()
        .ok_or("text edits must be an array")?
        .iter()
        .map(|edit| {
            let range = edit
                .get("range")
                .cloned()
                .ok_or("text edit without range")?;
            let text = edit
                .get("newText")
                .and_then(Value::as_str)
                .ok_or("text edit without newText")?;
            Ok((range, text.to_owned()))
        })
        .collect()
}

fn position(range: &Value, name: &str) -> Result<(usize, usize), String> {
    let field = |field: &str| {
        range
            .get(name)
            .and_then(|position| position.get(field))
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| format!("invalid range {range}"))
    };
    Ok((field("line")?, field("character")?))
}

impl IdeState {
    /// Project-relative path for a `file:` URI inside the project.
    fn relative_from_uri(&self, uri: &str) -> Result<PathBuf, String> {
        let url = Url::parse(uri).map_err(|error| format!("invalid URI {uri}: {error}"))?;
        let path = url
            .to_file_path()
            .map_err(|_| format!("{uri} is not a local file"))?;
        let root = self.project.as_ref().ok_or("no project is open")?.root();
        let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        canonical
            .strip_prefix(&canonical_root)
            .or_else(|_| path.strip_prefix(root))
            .map(Path::to_path_buf)
            .map_err(|_| format!("{} is outside the project", path.display()))
    }

    fn collect_edits(&self, edit: &Value) -> Result<Vec<FileEdits>, String> {
        let mut files = Vec::new();
        if let Some(changes) = edit.get("documentChanges").and_then(Value::as_array) {
            for change in changes {
                if let Some(kind) = change.get("kind").and_then(Value::as_str) {
                    return Err(format!(
                        "the edit wants to {kind} a file; file operations are not supported yet"
                    ));
                }
                let document = change
                    .get("textDocument")
                    .ok_or("invalid document change")?;
                let uri = document
                    .get("uri")
                    .and_then(Value::as_str)
                    .ok_or("document change without uri")?;
                files.push(FileEdits {
                    relative: self.relative_from_uri(uri)?,
                    version: document.get("version").and_then(Value::as_i64),
                    edits: text_edits(change.get("edits").unwrap_or(&Value::Null))?,
                });
            }
        } else if let Some(changes) = edit.get("changes").and_then(Value::as_object) {
            for (uri, edits) in changes {
                files.push(FileEdits {
                    relative: self.relative_from_uri(uri)?,
                    version: None,
                    edits: text_edits(edits)?,
                });
            }
        }
        Ok(files)
    }

    /// The index of the document for `relative`, opening it in the background
    /// (without activating it) when needed.
    pub(crate) fn ensure_document(&mut self, relative: &Path) -> Result<usize, String> {
        if let Some(index) = self.documents.iter().position(|d| d.path == relative) {
            return Ok(index);
        }
        let project = self.project.as_ref().ok_or("no project is open")?;
        let text = project.read_text(relative).map_err(|e| e.to_string())?;
        self.documents
            .push(Document::new(relative.to_owned(), &text));
        self.sync_document_lsp(relative, &text, 0);
        Ok(self.documents.len() - 1)
    }

    /// Applies a `WorkspaceEdit` atomically; see the module documentation.
    pub fn apply_workspace_edit(&mut self, edit: &Value) -> Result<EditSummary, String> {
        let files = self.collect_edits(edit)?;
        // Validate everything before changing anything.
        let mut planned: Vec<(usize, Vec<TextChange>)> = Vec::new();
        for file in &files {
            let open = self.documents.iter().position(|d| d.path == file.relative);
            if let (Some(index), Some(version)) = (open, file.version) {
                let current = self.documents[index].text.revision().min(i32::MAX as u64) as i64;
                if version != current {
                    return Err(format!(
                        "{} changed since the server computed the edit; try again",
                        file.relative.display()
                    ));
                }
            }
            let index = self.ensure_document(&file.relative)?;
            let text = &self.documents[index].text;
            let mut changes = Vec::new();
            for (range, inserted) in &file.edits {
                let (start_line, start_character) = position(range, "start")?;
                let (end_line, end_character) = position(range, "end")?;
                let invalid = |error: archipel_ide_text::TextError| {
                    format!("{}: {error}", file.relative.display())
                };
                let start_char = text
                    .try_char_from_lsp_position(start_line, start_character)
                    .map_err(invalid)?;
                let end_char = text
                    .try_char_from_lsp_position(end_line, end_character)
                    .map_err(invalid)?;
                if end_char < start_char {
                    return Err(format!(
                        "{}: reversed range {range}",
                        file.relative.display()
                    ));
                }
                changes.push(TextChange {
                    start_char,
                    end_char,
                    inserted: inserted.replace("\r\n", "\n"),
                });
            }
            changes.sort_by_key(|change| (change.start_char, change.end_char));
            if changes
                .windows(2)
                .any(|pair| pair[0].end_char > pair[1].start_char)
            {
                return Err(format!("{}: overlapping edits", file.relative.display()));
            }
            match planned.iter_mut().find(|(planned, _)| *planned == index) {
                Some((_, existing)) => existing.extend(changes),
                None => planned.push((index, changes)),
            }
        }
        let mut summary = EditSummary::default();
        for (index, changes) in planned {
            let document = &mut self.documents[index];
            let caret = document.selections.primary().head;
            // Keep the caret on the same text: shift it by the edits before it.
            let shift: isize = changes
                .iter()
                .filter(|change| change.end_char <= caret)
                .map(|change| {
                    change.inserted.chars().count() as isize
                        - (change.end_char - change.start_char) as isize
                })
                .sum();
            summary.edits += changes.len();
            let revision = document.text.revision();
            document
                .text
                .edit_many(changes)
                .map_err(|error| format!("{}: {error}", document.path.display()))?;
            if document.text.revision() == revision {
                continue;
            }
            let caret = caret
                .saturating_add_signed(shift)
                .min(document.text.len_chars());
            document.selections.set_single(Selection::caret(caret));
            let text = document.text.text();
            document.refresh_analysis(&text);
            let path = document.path.clone();
            let version = document.text.revision().min(i32::MAX as u64) as i32;
            summary.files.push(path.clone());
            self.sync_document_lsp(&path, &text, version);
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;
    use std::path::PathBuf;

    fn project(name: &str) -> (IdeState, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-wsedit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "let 𝔸 = old;\nold();\n").unwrap();
        std::fs::write(root.join("b.txt"), "use old;\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        (state, root.canonicalize().unwrap())
    }

    fn range(line: u64, start: u64, end: u64) -> serde_json::Value {
        json!({"start": {"line": line, "character": start}, "end": {"line": line, "character": end}})
    }

    #[test]
    fn edits_several_files_with_utf16_ranges_as_one_undo_step_each() {
        let (mut state, root) = project("multi");
        // Caret after "old();" on line 1.
        state.documents[0]
            .selections
            .set_single(archipel_ide_text::Selection::caret(14 + 5));
        let a = format!("file://{}", root.join("a.txt").display());
        let b = format!("file://{}", root.join("b.txt").display());
        let edit = json!({"changes": {
            // "old" after "let 𝔸 = " starts at UTF-16 column 9 (𝔸 is two units).
            a.clone(): [{"range": range(0, 9, 12), "newText": "newer"},
                        {"range": range(1, 0, 3), "newText": "newer"}],
            b: [{"range": range(0, 4, 7), "newText": "newer"}]
        }});
        let summary = state.apply_workspace_edit(&edit).unwrap();
        assert_eq!(summary.edits, 3);
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = newer;\nnewer();\n");
        // b.txt was opened in the background, a.txt stays active.
        assert_eq!(state.active_document, Some(0));
        assert_eq!(state.documents[1].path, PathBuf::from("b.txt"));
        assert_eq!(state.documents[1].text.text(), "use newer;\n");
        assert!(state.documents[1].text.is_dirty());
        // The caret followed its text.
        assert_eq!(state.documents[0].selections.primary().head, 15 + 8);
        assert!(state.documents[0].text.undo());
        assert_eq!(state.documents[0].text.text(), "let 𝔸 = old;\nold();\n");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_edits_change_nothing() {
        let (mut state, root) = project("atomic");
        let a = format!("file://{}", root.join("a.txt").display());
        let original = state.documents[0].text.text();
        for edit in [
            // Overlapping edits.
            json!({"changes": {a.clone(): [{"range": range(0, 0, 5), "newText": "x"},
                                           {"range": range(0, 3, 6), "newText": "y"}]}}),
            // A line past the end.
            json!({"changes": {a.clone(): [{"range": range(0, 0, 1), "newText": "x"},
                                           {"range": range(9, 0, 1), "newText": "y"}]}}),
            // Outside the project.
            json!({"changes": {"file:///etc/hosts": [{"range": range(0, 0, 0), "newText": "x"}]}}),
            // File operations.
            json!({"documentChanges": [{"kind": "create", "uri": a.clone()}]}),
            // A stale document version.
            json!({"documentChanges": [{"textDocument": {"uri": a.clone(), "version": 99},
                "edits": [{"range": range(0, 0, 0), "newText": "x"}]}]}),
        ] {
            assert!(state.apply_workspace_edit(&edit).is_err(), "{edit}");
            assert_eq!(state.documents[0].text.text(), original);
        }
        let error = state
            .apply_workspace_edit(&json!({"documentChanges": [{"kind": "rename"}]}))
            .unwrap_err();
        assert!(error.contains("not supported yet"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }
}
