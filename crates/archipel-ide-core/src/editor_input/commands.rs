//! The typed commands of the input layer and the dispatch of reference editor actions
//! (`EditorLeft`, `EditorEnter`, ...) onto [`super`]'s functions.

use super::{EditorOptions, enter, lines, movement, paste, typing};
use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};

impl IdeState {
    /// The editor settings (Smart Keys, Code Style, Caret Movement) as they are now.
    pub fn editor_options(&self) -> EditorOptions {
        EditorOptions::from_settings(&self.settings().read())
    }

    /// Types `text` into the active editor as the user would: pairs, overtype, surround,
    /// overwrite mode (ED-TYPE-001 to ED-TYPE-037). The window calls it for every typed text
    /// event; the terminal UI for every printable key.
    pub fn editor_type(&mut self, text: &str) -> Result<(), String> {
        let opts = self.editor_options();
        self.edit_active(|doc| typing::type_text(doc, &opts, text).map(|_| ()))?;
        self.snippet_after_change();
        Ok(())
    }

    /// Runs the reference editor action `action` (`EditorLeft`, `EditorBackSpace`, ...) on the
    /// active editor. `Err` carries a message for the status bar.
    pub fn editor_action(&mut self, action: &str) -> Result<Value, String> {
        let opts = self.editor_options();
        let handled = self.edit_active(|doc| Ok(perform(doc, &opts, action)))?;
        match handled {
            Some(result) => {
                result?;
                self.snippet_after_caret_move();
                Ok(Value::Null)
            }
            None => Err(format!("`{action}` is not an editor action")),
        }
    }
}

/// Performs a document-level editor action. `None` when `action` is not one.
pub(crate) fn perform(
    doc: &mut crate::Document,
    opts: &EditorOptions,
    action: &str,
) -> Option<Result<(), String>> {
    if movement::run(doc, opts, action).is_some() {
        doc.text.break_undo_merge();
        return Some(Ok(()));
    }
    let outcome = match action {
        "EditorBackSpace" => typing::backspace(doc, opts).map(|_| ()),
        "EditorDelete" => typing::delete_forward(doc).map(|_| ()),
        "EditorDeleteToWordStart" => typing::delete_word(doc, opts, false).map(|_| ()),
        "EditorDeleteToWordEnd" => typing::delete_word(doc, opts, true).map(|_| ()),
        "EditorHungryBackSpace" => typing::hungry_backspace(doc, opts).map(|_| ()),
        "EditorStartNewLine" => enter::start_new_line(doc, opts).map(|_| ()),
        "EditorStartNewLineBefore" => enter::start_new_line_before(doc, opts).map(|_| ()),
        "EditorSplitLine" => enter::split_line(doc, opts).map(|_| ()),
        "EditorCompleteStatement" => enter::complete_statement(doc, opts).map(|_| ()),
        "EditorIndentLineOrSelection" => lines::indent_lines(doc, opts).map(|_| ()),
        "EditorDuplicateLines" => lines::duplicate_lines(doc).map(|_| ()),
        "EditorTranspose" => lines::transpose(doc).map(|_| ()),
        "CommentByBlockComment" => lines::comment_block(doc, opts).map(|_| ()),
        "AutoIndentLines" => lines::auto_indent(doc, opts).map(|_| ()),
        "EditorToggleInsertState" => {
            doc.input.overwrite = !doc.input.overwrite;
            Ok(())
        }
        "EditorToggleStickySelection" => {
            doc.input.sticky = !doc.input.sticky;
            Ok(())
        }
        "EditorEscape" => {
            movement::escape(doc);
            Ok(())
        }
        "$SelectAll" | "EditorSelectAll" => {
            movement::select_all(doc);
            Ok(())
        }
        "EditorSelectLine" => {
            movement::select_line(doc);
            Ok(())
        }
        "EditorMatchBrace" => {
            movement::match_brace(doc);
            Ok(())
        }
        "EditorUnSelectWord" => {
            movement::shrink_selection(doc);
            Ok(())
        }
        _ => return None,
    };
    Some(outcome)
}

fn needs_document(state: &IdeState) -> bool {
    state.active_document.is_some()
}

fn clipboard_json(result: Option<(String, bool)>) -> Value {
    match result {
        Some((text, whole_line)) => json!({"text": text, "wholeLine": whole_line}),
        None => json!({"text": "", "wholeLine": false, "skipped": true}),
    }
}

pub fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "editor.type",
                "Type Text",
                "Editor",
                None,
                r#"{"text":"string"}"#,
            ),
            needs_document,
            |state: &mut IdeState, params| {
                let text = string_param(&params, "text")?;
                state.editor_type(&text).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.action",
                "Editor Action",
                "Editor",
                None,
                r#"{"action":"string"}"#,
            ),
            needs_document,
            |state: &mut IdeState, params| {
                let action = string_param(&params, "action")?;
                state.editor_action(&action).map_err(CommandError::Failed)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.enter", "Enter", "Editor", None, "{}"),
            needs_document,
            |state: &mut IdeState, _| {
                // A live template in progress takes Enter (ED-TMPL-031).
                if state.snippet.is_some() {
                    state.snippet_next();
                    return Ok(Value::Null);
                }
                let opts = state.editor_options();
                state
                    .edit_active(|doc| enter::enter(doc, &opts).map(|_| ()))
                    .map_err(CommandError::Failed)?;
                state.snippet_after_change();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.tab", "Tab", "Editor", None, "{}"),
            needs_document,
            |state: &mut IdeState, _| {
                state.editor_tab().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.unindent",
                "Unindent Line or Selection",
                "Editor",
                Some("⇧⇥"),
                "{}",
            ),
            needs_document,
            |state: &mut IdeState, _| {
                if state.snippet.is_some() {
                    state.snippet_previous();
                    return Ok(Value::Null);
                }
                let opts = state.editor_options();
                state
                    .edit_active(|doc| lines::unindent_lines(doc, &opts).map(|_| ()))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.copy", "Copy", "Editor", Some("⌘C"), "{}"),
            needs_document,
            |state: &mut IdeState, _| {
                let opts = state.editor_options();
                let result = state
                    .edit_active(|doc| {
                        doc.input.sticky = false;
                        Ok(paste::copy_text(doc, &opts))
                    })
                    .map_err(CommandError::Failed)?;
                Ok(clipboard_json(result))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.cut", "Cut", "Editor", Some("⌘X"), "{}"),
            needs_document,
            |state: &mut IdeState, _| {
                let opts = state.editor_options();
                let result = state
                    .edit_active(|doc| paste::cut(doc, &opts))
                    .map_err(CommandError::Failed)?;
                Ok(clipboard_json(result))
            },
        ))
        .expect("unique built-in id");
    for (id, label, forward) in [
        ("editor.cutLineEnd", "Cut up to Line End", true),
        ("editor.cutLineBackward", "Cut Line Backward", false),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Editor", None, "{}"),
                needs_document,
                move |state: &mut IdeState, _| {
                    let text = state
                        .edit_active(|doc| lines::cut_to_line_boundary(doc, forward))
                        .map_err(CommandError::Failed)?;
                    Ok(json!({"text": text, "wholeLine": false}))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "editor.paste",
                "Paste",
                "Editor",
                Some("⌘V"),
                r#"{"text":"string","wholeLine":"bool?","raw":"bool?"}"#,
            ),
            needs_document,
            |state: &mut IdeState, params| {
                let text = string_param(&params, "text")?;
                let whole_line = params
                    .get("wholeLine")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let raw = params.get("raw").and_then(Value::as_bool).unwrap_or(false);
                let opts = state.editor_options();
                state
                    .edit_active(|doc| paste::paste(doc, &opts, &text, whole_line, raw).map(|_| ()))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    for (id, label, shortcut, action) in [
        ("editor.splitLine", "Split Line", "⌘↩", "EditorSplitLine"),
        (
            "editor.startNewLineBefore",
            "Start New Line Before Current",
            "⌥⌘↩",
            "EditorStartNewLineBefore",
        ),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Editor", Some(shortcut), "{}"),
                needs_document,
                move |state: &mut IdeState, _| {
                    state.editor_action(action).map_err(CommandError::Failed)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec("editor.indent", "Indent Selection", "Editor", None, "{}"),
            needs_document,
            |state: &mut IdeState, _| {
                let opts = state.editor_options();
                state
                    .edit_active(|doc| lines::indent_lines(doc, &opts).map(|_| ()))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.commentLine",
                "Comment with Line Comment",
                "Editor",
                Some("⌘/"),
                "{}",
            ),
            needs_document,
            |state: &mut IdeState, _| {
                let opts = state.editor_options();
                state
                    .edit_active(|doc| lines::comment_line(doc, &opts).map(|_| ()))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

impl IdeState {
    /// Tab: the next stop of a live template in progress, a tab-out over a typed closing bracket
    /// or quote, a live template abbreviation, an indent of the selection, or an indent step
    /// (ED-TYPE-030, ED-TYPE-031, ED-TYPE-080, ED-TYPE-081).
    pub fn editor_tab(&mut self) -> Result<(), String> {
        if self.snippet.is_some() {
            self.snippet_next();
            return Ok(());
        }
        let opts = self.editor_options();
        let moved = self.edit_active(|doc| Ok(typing::tab_out(doc, &opts)))?;
        if moved {
            return Ok(());
        }
        let has_selection = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .is_some_and(lines::selection_has_text);
        if !has_selection && opts.tab_expands_templates && self.expand_template_at_caret()? {
            return Ok(());
        }
        self.edit_active(|doc| {
            if lines::selection_has_text(doc) {
                lines::indent_lines(doc, &opts).map(|_| ())
            } else {
                lines::tab_at_caret(doc, &opts).map(|_| ())
            }
        })
    }

    /// Undo with the editor state restore of ED-UNDO-011 and ED-UNDO-012.
    pub fn undo_active(&mut self) -> bool {
        let transparent = self.editor_options().undo_transparent_caret;
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get_mut(index))
        else {
            return false;
        };
        let current = document.selections.selections().to_vec();
        let Some(result) = document.text.undo_with(&current, transparent) else {
            return false;
        };
        let len = document.text.len_chars();
        let selections: Vec<_> = result
            .selections
            .into_iter()
            .map(|s| archipel_ide_text::Selection {
                anchor: s.anchor.min(len),
                head: s.head.min(len),
            })
            .collect();
        document.selections.set_all(selections);
        document.input.goal_reset();
        true
    }

    /// Redo; the counterpart of [`IdeState::undo_active`].
    pub fn redo_active(&mut self) -> bool {
        let transparent = self.editor_options().undo_transparent_caret;
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get_mut(index))
        else {
            return false;
        };
        let current = document.selections.selections().to_vec();
        let Some(result) = document.text.redo_with(&current, transparent) else {
            return false;
        };
        let len = document.text.len_chars();
        let selections: Vec<_> = result
            .selections
            .into_iter()
            .map(|s| archipel_ide_text::Selection {
                anchor: s.anchor.min(len),
                head: s.head.min(len),
            })
            .collect();
        document.selections.set_all(selections);
        document.input.goal_reset();
        true
    }
}
