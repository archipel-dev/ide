//! Replace in Files: previews and applies a replacement over the files Find in
//! Files would search. The replacement itself is the model of
//! [`find_in_files`](crate::find_in_files) (spec `docs/specs/search/02-find-in-files.md`,
//! SRCH-FIP-030 and 071 to 073): its validation messages, templates and Preserve case.

use crate::find_in_files::{self, FindRequest, ProjectFile, Scope};
use crate::{IdeState, spec, string_param, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_search::{FileReplacement, Matcher, SearchOptions};
use archipel_ide_text::TextChange;
use serde_json::{Value, json};
use std::path::PathBuf;

const PARAMS: &str = r#"{"query":"string","replacement":"string","matchCase":"bool?","wholeWords":"bool?","regex":"bool?","preserveCase":"bool?","fileMask":"string?","scope":"string?"}"#;
/// Files above this size are not replaced in (the search skips them as well).
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Lines listed per file in a preview; counts always cover every line.
const PREVIEW_LINES: usize = 50;

impl IdeState {
    /// Plans the replacement without touching any file.
    fn plan_replace(&mut self, params: &Value) -> Result<Vec<FileReplacement>, CommandError> {
        let query = string_param(params, "query")?;
        let replacement = string_param(params, "replacement")?;
        let flag = |name: &str| params.get(name).and_then(Value::as_bool).unwrap_or(false);
        let options = SearchOptions {
            match_case: flag("matchCase"),
            whole_words: flag("wholeWords"),
            regex: flag("regex"),
            file_mask: params
                .get("fileMask")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        };
        let matcher = Matcher::new(&query, &options)
            .map_err(|error| CommandError::InvalidParams(error.to_string()))?;
        self.project_view.search_scope = params
            .get("scope")
            .and_then(Value::as_str)
            .filter(|scope| !scope.is_empty())
            .map(PathBuf::from);
        let root = self
            .project
            .as_ref()
            .ok_or_else(|| CommandError::Failed("no project is open".into()))?
            .root()
            .to_owned();
        Ok(archipel_ide_search::plan_replacements(
            &root,
            &self.search_files(),
            &matcher,
            &replacement,
        ))
    }

    /// Whether an open editor holds unsaved changes for project-relative `path`.
    fn has_unsaved_buffer(&self, path: &std::path::Path) -> bool {
        self.documents
            .iter()
            .any(|document| document.path == path && document.text.is_dirty())
    }
}

/// The one edit that turns `old` into `new`: the differing middle, in chars.
fn minimal_edit(old: &str, new: &str) -> Option<TextChange> {
    let old: Vec<char> = old.chars().collect();
    let new: Vec<char> = new.chars().collect();
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    if prefix == old.len() && prefix == new.len() {
        return None;
    }
    let max_suffix = old.len().min(new.len()) - prefix;
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    Some(TextChange {
        start_char: prefix,
        end_char: old.len() - suffix,
        inserted: new[prefix..new.len() - suffix].iter().collect(),
    })
}

impl IdeState {
    /// The request of Find in Files for command parameters, validated like the popup does
    /// (SRCH-FIP-030): the error is the message the popup shows.
    fn find_request(params: &Value) -> Result<FindRequest, CommandError> {
        let flag = |name: &str| params.get(name).and_then(Value::as_bool).unwrap_or(false);
        let file_mask = params
            .get("fileMask")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let request = FindRequest {
            query: string_param(params, "query")?,
            options: SearchOptions {
                match_case: flag("matchCase"),
                whole_words: flag("wholeWords"),
                regex: flag("regex"),
                file_mask: file_mask.clone(),
            },
            file_mask_on: !file_mask.is_empty(),
            scope: Scope::Project,
        };
        find_in_files::validate(&request, |_| true).map_err(CommandError::InvalidParams)?;
        Ok(request)
    }

    /// Puts the replaced text of `path` where it belongs: an editor with unsaved changes gets it
    /// as one undoable edit; any other file is written (and a clean editor reloads it).
    fn store_replacement(
        &mut self,
        root: &std::path::Path,
        path: &std::path::Path,
        new_text: &str,
    ) -> Result<(), CommandError> {
        let Some(document) = self.documents.iter_mut().find(|d| d.path == path) else {
            return write(root, path, new_text);
        };
        if !document.text.is_dirty() {
            return write(root, path, new_text);
        }
        let Some(change) = minimal_edit(&document.text.text(), new_text) else {
            return Ok(());
        };
        document
            .text
            .edit_many([change])
            .map_err(|error| CommandError::Failed(format!("{}: {error}", path.display())))?;
        let len = document.text.len_chars();
        document.selections.clamp(len);
        let text = document.text.text();
        document.refresh_analysis(&text);
        let version = document.text.revision().min(i32::MAX as u64) as i32;
        self.sync_document_lsp(path, &text, version);
        Ok(())
    }

    /// The text of the project file `path` as the search sees it: the editor's buffer when open.
    fn current_text(&self, path: &std::path::Path) -> Result<String, CommandError> {
        if let Some(document) = self.documents.iter().find(|d| d.path == path) {
            return Ok(document.text.text());
        }
        self.project
            .as_ref()
            .ok_or_else(|| CommandError::Failed("no project is open".into()))?
            .read_text(path)
            .map_err(|error| CommandError::Failed(error.to_string()))
    }

    /// Replace All over the files of the current search scope. Open editors are replaced in
    /// their buffer (one undoable edit, unsaved); the other files are written.
    fn replace_all_in_scope(
        &mut self,
        request: &FindRequest,
        replacement: &str,
        preserve_case: bool,
    ) -> Result<find_in_files::ReplaceOutcome, CommandError> {
        let root = self
            .project
            .as_ref()
            .ok_or_else(|| CommandError::Failed("no project is open".into()))?
            .root()
            .to_owned();
        let files: Vec<ProjectFile> = self
            .search_files()
            .into_iter()
            .filter_map(|path| {
                let full = root.join(&path);
                if std::fs::metadata(&full).ok()?.len() > MAX_FILE_BYTES {
                    return None;
                }
                Some(ProjectFile {
                    bytes: std::fs::read(full).ok()?,
                    path,
                    excluded: false,
                })
            })
            .collect();
        let open: Vec<(PathBuf, String)> = self
            .documents
            .iter()
            .map(|document| (document.path.clone(), document.text.text()))
            .collect();
        let outcome =
            find_in_files::replace_all(&files, &open, request, replacement, preserve_case)
                .map_err(|error| CommandError::InvalidParams(error.to_string()))?;
        for (path, new_text) in &outcome.changed {
            self.store_replacement(&root, path, new_text)?;
        }
        // Clean open editors reload the new contents (undoably).
        self.synchronize_all();
        self.status = outcome.status.clone();
        Ok(outcome)
    }
}

fn write(root: &std::path::Path, path: &std::path::Path, text: &str) -> Result<(), CommandError> {
    std::fs::write(root.join(path), text)
        .map_err(|error| CommandError::Failed(format!("cannot write {}: {error}", path.display())))
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "search.replacePreview",
                "Preview Replace in Files",
                "Search",
                None,
                PARAMS,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let plan = state.plan_replace(&params)?;
                let files: Vec<Value> = plan
                    .iter()
                    .map(|file| {
                        json!({
                            "path": file.path,
                            "count": file.count(),
                            "unsaved": state.has_unsaved_buffer(&file.path),
                            "lines": file.lines.iter().take(PREVIEW_LINES).map(|line| json!({
                                "line": line.line,
                                "before": line.before,
                                "after": line.after,
                            })).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                let total: usize = plan.iter().map(FileReplacement::count).sum();
                Ok(json!({"files": files, "replacements": total}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "search.replaceInFiles",
                "Replace in Files",
                "Search",
                Some("⇧⌘R"),
                PARAMS,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let replacement = string_param(&params, "replacement")?;
                let request = IdeState::find_request(&params)?;
                state.project_view.search_scope = params
                    .get("scope")
                    .and_then(Value::as_str)
                    .filter(|scope| !scope.is_empty())
                    .map(PathBuf::from);
                let preserve = params
                    .get("preserveCase")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let outcome = state.replace_all_in_scope(&request, &replacement, preserve)?;
                Ok(json!({
                    "files": outcome.changed.iter().map(|(path, _)| path).collect::<Vec<_>>(),
                    "replaced": outcome.replaced,
                    "status": outcome.status,
                    "error": outcome.error,
                    "notification": outcome.notification,
                    "historyLabel": outcome.history_label,
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "search.replaceRow",
                "Replace Occurrence in Files",
                "Search",
                None,
                r#"{"query":"string","replacement":"string","path":"string","line":"usize","column":"usize","length":"usize","matchCase":"bool?","wholeWords":"bool?","regex":"bool?","preserveCase":"bool?"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let replacement = string_param(&params, "replacement")?;
                let request = IdeState::find_request(&params)?;
                let path = PathBuf::from(string_param(&params, "path")?);
                let at = |name: &str| usize_param(&params, name);
                let preserve = params
                    .get("preserveCase")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let root = state
                    .project
                    .as_ref()
                    .ok_or_else(|| CommandError::Failed("no project is open".into()))?
                    .root()
                    .to_owned();
                let text = state.current_text(&path)?;
                let new_text = find_in_files::replace_row(
                    &text,
                    &request,
                    &replacement,
                    preserve,
                    (at("line")?, at("column")?, at("length")?),
                )
                .map_err(|message| {
                    CommandError::InvalidParams(format!("Malformed replacement string: {message}"))
                })?;
                state.store_replacement(&root, &path, &new_text)?;
                state.synchronize_all();
                state.status = "1 occurrence replaced".into();
                Ok(json!({"replaced": 1}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;

    #[test]
    fn preview_leaves_files_alone_and_replace_expands_groups_in_dirty_buffers() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-replace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("A.kt"), "let a = 1;\nlet b = 2;\n").unwrap();
        std::fs::write(root.join("B.kt"), "let c = 3;\n").unwrap();
        std::fs::write(root.join("C.txt"), "let d = 4;\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        let params = json!({
            "query": r"let (\w+) = (\d+)",
            "replacement": "val $1: Int = $2",
            "regex": true,
            "fileMask": "*.kt",
        });
        let preview = commands
            .execute("search.replacePreview", &mut state, params.clone())
            .unwrap();
        assert_eq!(preview["replacements"], 3);
        assert_eq!(preview["files"].as_array().unwrap().len(), 2);
        assert_eq!(
            std::fs::read_to_string(root.join("A.kt")).unwrap(),
            "let a = 1;\nlet b = 2;\n",
            "preview writes nothing"
        );
        // B.kt is open with unsaved edits: the replacement goes into the buffer, not the file.
        state.open_file(std::path::Path::new("B.kt")).unwrap();
        state.insert_active_text(0, "// ").unwrap();
        let done = commands
            .execute("search.replaceInFiles", &mut state, params)
            .unwrap();
        assert_eq!(done["files"], json!(["A.kt", "B.kt"]));
        assert_eq!(done["replaced"], 3);
        assert_eq!(state.status(), "3 occurrences replaced");
        assert_eq!(
            std::fs::read_to_string(root.join("A.kt")).unwrap(),
            "val a: Int = 1;\nval b: Int = 2;\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("B.kt")).unwrap(),
            "let c = 3;\n",
            "the unsaved buffer is not written"
        );
        let buffer = state.active_doc().unwrap();
        assert_eq!(buffer.text.text(), "// val c: Int = 3;\n");
        assert!(buffer.text.is_dirty());
        assert_eq!(
            std::fs::read_to_string(root.join("C.txt")).unwrap(),
            "let d = 4;\n",
            "outside the file mask"
        );
        // Replace (selected) takes one occurrence of one row.
        std::fs::write(root.join("D.txt"), "let d = 4; let e = 5;\nlet f = 6;\n").unwrap();
        state.open_project(&root).unwrap();
        commands
            .execute(
                "search.replaceRow",
                &mut state,
                json!({
                    "query": "let", "replacement": "var", "path": "D.txt",
                    "line": 1, "column": 12, "length": 3,
                }),
            )
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("D.txt")).unwrap(),
            "let d = 4; var e = 5;\nlet f = 6;\n"
        );
        assert_eq!(state.status(), "1 occurrence replaced");
        let error = commands
            .execute(
                "search.replaceInFiles",
                &mut state,
                json!({"query": "(", "replacement": "x", "regex": true}),
            )
            .unwrap_err();
        assert!(error.to_string().contains("Bad pattern"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }
}
