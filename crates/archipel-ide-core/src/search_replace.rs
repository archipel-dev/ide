//! Replace in Files: previews and applies a replacement over the files Find in
//! Files would search. Regex queries expand `$1`/`${name}` groups.

use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_search::{FileReplacement, Matcher, SearchOptions};
use serde_json::{Value, json};
use std::path::PathBuf;

const PARAMS: &str = r#"{"query":"string","replacement":"string","matchCase":"bool?","wholeWords":"bool?","regex":"bool?","fileMask":"string?","scope":"string?"}"#;
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
                let plan = state.plan_replace(&params)?;
                let root = state
                    .project
                    .as_ref()
                    .map(|project| project.root().to_owned())
                    .ok_or_else(|| CommandError::Failed("no project is open".into()))?;
                let (mut files, mut replaced) = (0, 0);
                let mut skipped = Vec::new();
                for file in plan {
                    // Never overwrite unsaved editor changes.
                    if state.has_unsaved_buffer(&file.path) {
                        skipped.push(file.path);
                        continue;
                    }
                    std::fs::write(root.join(&file.path), &file.new_contents).map_err(|error| {
                        CommandError::Failed(format!(
                            "cannot write {}: {error}",
                            file.path.display()
                        ))
                    })?;
                    files += 1;
                    replaced += file.count();
                }
                // Clean open editors reload the new contents (undoably).
                state.synchronize_all();
                state.status = format!(
                    "{replaced} occurrence{} replaced in {files} file{}{}",
                    if replaced == 1 { "" } else { "s" },
                    if files == 1 { "" } else { "s" },
                    if skipped.is_empty() {
                        String::new()
                    } else {
                        format!(" ({} skipped: unsaved changes)", skipped.len())
                    }
                );
                Ok(json!({"files": files, "replaced": replaced, "skipped": skipped}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;

    #[test]
    fn preview_leaves_files_alone_and_replace_expands_groups_skipping_dirty_buffers() {
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
        // B.kt is open with unsaved edits: it is skipped, not overwritten.
        state.open_file(std::path::Path::new("B.kt")).unwrap();
        state.insert_active_text(0, "// ").unwrap();
        let done = commands
            .execute("search.replaceInFiles", &mut state, params)
            .unwrap();
        assert_eq!(done["files"], 1);
        assert_eq!(done["replaced"], 2);
        assert_eq!(done["skipped"], json!(["B.kt"]));
        assert_eq!(
            std::fs::read_to_string(root.join("A.kt")).unwrap(),
            "val a: Int = 1;\nval b: Int = 2;\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("B.kt")).unwrap(),
            "let c = 3;\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("C.txt")).unwrap(),
            "let d = 4;\n",
            "outside the file mask"
        );
        let error = commands
            .execute(
                "search.replaceInFiles",
                &mut state,
                json!({"query": "(", "replacement": "x", "regex": true}),
            )
            .unwrap_err();
        assert!(error.to_string().contains("invalid regular expression"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
