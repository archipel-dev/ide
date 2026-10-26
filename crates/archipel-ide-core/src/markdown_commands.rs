//! Markdown editing commands: preview layout, selection formatting and insertions.
//! Transforms live in `archipel-ide-markdown`; this layer applies each as one undoable edit.

use crate::{Document, IdeState, spec, string_param, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_markdown::Layout;
use archipel_ide_markdown::format::{self, Edit, ListKind, Style};
use archipel_ide_text::Selection;
use serde_json::{Value, json};

/// The active document, when it is a Markdown file.
fn active_markdown(state: &IdeState) -> Option<&Document> {
    state
        .active_document
        .and_then(|index| state.documents.get(index))
        .filter(|document| archipel_ide_markdown::is_markdown_path(&document.path))
}

impl IdeState {
    /// On-disk path of a document path (documents are stored relative to the project).
    pub fn absolute_path(&self, path: &std::path::Path) -> std::path::PathBuf {
        match &self.project {
            Some(project) => project.root().join(path),
            None => path.to_owned(),
        }
    }

    /// Layout of the active Markdown document (default Split), if it is one.
    pub fn markdown_layout(&self) -> Option<Layout> {
        let document = active_markdown(self)?;
        Some(
            self.markdown_layouts
                .get(&document.path)
                .copied()
                .unwrap_or_default(),
        )
    }
}

/// Applies `edit` as one replacement (a single undo step) and selects its result.
pub fn apply_edit(document: &mut Document, edit: &Edit) -> Result<(), String> {
    let len = document.text.len_chars();
    let (start, end) = (edit.start.min(len), edit.end.min(len));
    if start != end || !edit.text.is_empty() {
        document
            .text
            .edit(start, end, &edit.text)
            .map_err(|error| error.to_string())?;
    }
    document.selections.set_single(Selection {
        anchor: edit.selection.0,
        head: edit.selection.1,
    });
    Ok(())
}

/// Runs a transform over the active Markdown document's text and primary selection.
fn transform(
    state: &mut IdeState,
    run: impl FnOnce(&str, (usize, usize)) -> Edit,
) -> Result<Value, CommandError> {
    state
        .edit_active(|document| {
            let selection = document.selections.primary();
            let edit = run(&document.text.text(), (selection.anchor, selection.head));
            apply_edit(document, &edit)?;
            Ok(())
        })
        .map_err(CommandError::Failed)?;
    Ok(Value::Null)
}

fn command(
    registry: &mut CommandRegistry<IdeState>,
    id: &str,
    label: &str,
    params: &str,
    run: fn(&mut IdeState, &Value) -> Result<Value, CommandError>,
) {
    registry
        .register(Command::new(
            spec(id, label, "Markdown", None, params),
            |state: &IdeState| active_markdown(state).is_some(),
            move |state: &mut IdeState, params| run(state, &params),
        ))
        .expect("unique built-in id");
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "markdown.layout",
                "Markdown: Editor and Preview Layout",
                "Markdown",
                None,
                r#"{"mode":"editor|split|preview"}"#,
            ),
            |state: &IdeState| active_markdown(state).is_some(),
            |state: &mut IdeState, params| {
                let mode = string_param(&params, "mode")?;
                let layout = Layout::from_id(&mode).ok_or_else(|| {
                    CommandError::InvalidParams(format!("unknown layout `{mode}`"))
                })?;
                let path = active_markdown(state)
                    .map(|document| document.path.clone())
                    .ok_or_else(|| CommandError::Failed("no active Markdown document".into()))?;
                state.markdown_layouts.insert(path, layout);
                Ok(json!({"layout": layout.id()}))
            },
        ))
        .expect("unique built-in id");
    command(registry, "markdown.bold", "Markdown: Bold", "{}", |s, _| {
        transform(s, |t, sel| format::toggle_style(t, sel, Style::Bold))
    });
    command(
        registry,
        "markdown.italic",
        "Markdown: Italic",
        "{}",
        |s, _| transform(s, |t, sel| format::toggle_style(t, sel, Style::Italic)),
    );
    command(
        registry,
        "markdown.strikethrough",
        "Markdown: Strikethrough",
        "{}",
        |s, _| {
            transform(s, |t, sel| {
                format::toggle_style(t, sel, Style::Strikethrough)
            })
        },
    );
    command(
        registry,
        "markdown.code",
        "Markdown: Code Span",
        "{}",
        |s, _| transform(s, |t, sel| format::toggle_style(t, sel, Style::Code)),
    );
    command(
        registry,
        "markdown.link",
        "Markdown: Create Link",
        "{}",
        |s, _| transform(s, format::link),
    );
    command(
        registry,
        "markdown.heading",
        "Markdown: Set Heading Level",
        r#"{"level":"0-6"}"#,
        |s, params| {
            let level = usize_param(params, "level")?;
            if level > 6 {
                return Err(CommandError::InvalidParams("level must be 0-6".into()));
            }
            transform(s, |t, sel| format::set_heading(t, sel, level as u8))
        },
    );
    command(
        registry,
        "markdown.list",
        "Markdown: Create or Change List",
        r#"{"kind":"bullet|ordered|task"}"#,
        |s, params| {
            let kind = match string_param(params, "kind")?.as_str() {
                "bullet" => ListKind::Bullet,
                "ordered" => ListKind::Ordered,
                "task" => ListKind::Task,
                other => {
                    return Err(CommandError::InvalidParams(format!(
                        "unknown list kind `{other}`"
                    )));
                }
            };
            transform(s, |t, sel| format::toggle_list(t, sel, kind))
        },
    );
    command(
        registry,
        "markdown.insertTable",
        "Markdown: Insert Table",
        r#"{"columns":"int?","rows":"int?"}"#,
        |s, params| {
            let columns = usize_param(params, "columns").unwrap_or(3);
            let rows = usize_param(params, "rows").unwrap_or(2);
            transform(s, |t, sel| format::insert_table(t, sel, columns, rows))
        },
    );
    command(
        registry,
        "markdown.insertImage",
        "Markdown: Insert Image",
        r#"{"path":"string?","alt":"string?"}"#,
        |s, params| {
            let text = |name| {
                params
                    .get(name)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            let (path, alt) = (text("path"), text("alt"));
            transform(s, |t, sel| format::insert_image(t, sel, &path, &alt))
        },
    );
    command(
        registry,
        "markdown.insertCodeBlock",
        "Markdown: Insert Code Block",
        r#"{"lang":"string?"}"#,
        |s, params| {
            let lang = params
                .get("lang")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            transform(s, |t, sel| format::insert_code_block(t, sel, &lang))
        },
    );
    command(
        registry,
        "markdown.insertToc",
        "Markdown: Generate Table of Contents",
        "{}",
        |s, _| transform(s, format::insert_toc),
    );
    command(
        registry,
        "markdown.exportHtml",
        "Markdown: Export to HTML",
        "{}",
        |s, _| {
            let document = active_markdown(s)
                .ok_or_else(|| CommandError::Failed("no active Markdown document".into()))?;
            let target = s.absolute_path(&document.path).with_extension("html");
            let title = document
                .path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("document")
                .replace('&', "&amp;")
                .replace('<', "&lt;");
            let body = archipel_ide_markdown::to_html(&document.text.text());
            let html = format!(
                "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>{title}</title></head>\n<body>\n{body}</body></html>\n"
            );
            std::fs::write(&target, html)
                .map_err(|error| CommandError::Failed(format!("{}: {error}", target.display())))?;
            Ok(json!({"path": target.display().to_string()}))
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn state_with(path: &str, text: &str) -> IdeState {
        let mut state = IdeState::default();
        state.documents.push(Document::new(path.into(), text));
        state.active_document = Some(0);
        state
    }

    fn select(state: &mut IdeState, anchor: usize, head: usize) {
        state.documents[0]
            .selections
            .set_single(Selection { anchor, head });
    }

    #[test]
    fn formatting_is_one_undo_step_and_unicode_safe() {
        let commands = built_in_commands();
        let mut state = state_with("n.md", "ünï wörd");
        select(&mut state, 4, 8);
        commands
            .execute("markdown.bold", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "ünï **wörd**");
        let selection = state.documents[0].selections.primary();
        assert_eq!((selection.start(), selection.end()), (6, 10));
        // Toggling again restores the text, and a single undo reverts one command.
        commands
            .execute("markdown.bold", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "ünï wörd");
        assert!(state.documents[0].text.undo());
        assert_eq!(state.documents[0].text.text(), "ünï **wörd**");
    }

    #[test]
    fn heading_list_and_insert_commands_edit_the_document() {
        let commands = built_in_commands();
        let mut state = state_with("n.md", "Title\nitem");
        commands
            .execute("markdown.heading", &mut state, json!({"level": 2}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "## Title\nitem");
        select(&mut state, 9, 13);
        commands
            .execute("markdown.list", &mut state, json!({"kind": "task"}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "## Title\n- [ ] item");
        commands
            .execute(
                "markdown.insertCodeBlock",
                &mut state,
                json!({"lang": "rust"}),
            )
            .unwrap();
        assert!(state.documents[0].text.text().contains("```rust\n"));
        assert!(matches!(
            commands.execute("markdown.heading", &mut state, json!({"level": 9})),
            Err(CommandError::InvalidParams(_))
        ));
        assert!(matches!(
            commands.execute("markdown.list", &mut state, json!({"kind": "x"})),
            Err(CommandError::InvalidParams(_))
        ));
    }

    #[test]
    fn layout_is_per_file_and_commands_need_a_markdown_document() {
        let commands = built_in_commands();
        let mut state = state_with("n.md", "x");
        assert_eq!(state.markdown_layout(), Some(Layout::Split));
        commands
            .execute("markdown.layout", &mut state, json!({"mode": "preview"}))
            .unwrap();
        assert_eq!(state.markdown_layout(), Some(Layout::PreviewOnly));
        assert!(
            commands
                .execute("markdown.layout", &mut state, json!({"mode": "nope"}))
                .is_err()
        );
        let mut rust = state_with("a.rs", "fn main() {}");
        assert_eq!(rust.markdown_layout(), None);
        assert!(!commands.is_enabled("markdown.bold", &rust));
        assert!(matches!(
            commands.execute("markdown.bold", &mut rust, Value::Null),
            Err(CommandError::Disabled(_))
        ));
    }

    #[test]
    fn export_writes_html_next_to_the_file() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-md-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("doc.md");
        let commands = built_in_commands();
        let mut state = state_with(path.to_str().unwrap(), "# Hi\n");
        let result = commands
            .execute("markdown.exportHtml", &mut state, Value::Null)
            .unwrap();
        let html = std::fs::read_to_string(result["path"].as_str().unwrap()).unwrap();
        assert!(html.contains("<h1>Hi</h1>"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
