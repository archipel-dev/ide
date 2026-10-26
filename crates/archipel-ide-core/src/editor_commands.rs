//! Editor actions exposed as typed commands (menus, Find Action, keymap, CLI).

use crate::{IdeState, spec, string_param, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_text::ops::{self, FindOptions};
use archipel_ide_text::{Selection, TextError, multicaret, transform};
use serde_json::{Value, json};
use std::path::Path;

/// Line comment prefix for a file, by extension.
pub fn line_comment_prefix(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("py" | "sh" | "bash" | "zsh" | "toml" | "yaml" | "yml" | "rb" | "pl" | "r") => "#",
        Some("sql" | "lua" | "hs") => "--",
        Some("lisp" | "clj" | "el") => ";",
        Some("bat" | "cmd") => "REM",
        _ => "//",
    }
}

fn text_error(error: TextError) -> String {
    error.to_string()
}

fn bool_param(params: &Value, name: &str) -> bool {
    params.get(name).and_then(Value::as_bool).unwrap_or(false)
}

/// Find options from command parameters; an invalid regex is rejected here
/// so every find/replace command reports it the same way.
fn find_options(params: &Value, query: &str) -> Result<FindOptions, CommandError> {
    let options = FindOptions {
        match_case: bool_param(params, "matchCase"),
        whole_words: bool_param(params, "wholeWords"),
        regex: bool_param(params, "regex"),
    };
    ops::validate_query(query, options)
        .map_err(|error| CommandError::InvalidParams(error.to_string()))?;
    Ok(options)
}

/// Registers a parameterless action that edits the active document.
fn action(
    registry: &mut CommandRegistry<IdeState>,
    id: &str,
    label: &str,
    shortcut: Option<&str>,
    run: fn(&mut crate::Document) -> Result<(), TextError>,
) {
    registry
        .register(Command::new(
            spec(id, label, "Editor", shortcut, "{}"),
            |state: &IdeState| state.active_document.is_some(),
            move |state: &mut IdeState, _| {
                state
                    .edit_active(|document| run(document).map_err(text_error))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

/// Selects the next (or previous) match after the primary selection, wrapping around.
fn select_match(
    document: &mut crate::Document,
    query: &str,
    options: FindOptions,
    backwards: bool,
) -> Value {
    let text = document.text.text();
    let matches = ops::find_all(&text, query, options);
    let primary = document.selections.primary();
    let chosen = if backwards {
        matches
            .iter()
            .rev()
            .find(|(start, _)| *start < primary.start())
            .or(matches.last())
    } else {
        matches
            .iter()
            .find(|(start, _)| {
                *start > primary.start() || (*start == primary.start() && primary.is_caret())
            })
            .or(matches.first())
    };
    let Some(&(start, end)) = chosen else {
        return json!({"matches": 0, "index": null});
    };
    document.selections.set_single(Selection {
        anchor: start,
        head: end,
    });
    let index = matches.iter().position(|m| *m == (start, end));
    json!({"matches": matches.len(), "index": index})
}

/// Caret and selection actions (PLATFORM-EDIT-007..012). They never edit text.
fn register_multicaret(registry: &mut CommandRegistry<IdeState>) {
    action(
        registry,
        "editor.selectNextOccurrence",
        "Add Selection for Next Occurrence",
        Some("⌃G"),
        |d| {
            multicaret::select_next_occurrence(&d.text, &mut d.selections);
            Ok(())
        },
    );
    action(
        registry,
        "editor.unselectOccurrence",
        "Unselect Occurrence",
        Some("⌃⇧G"),
        |d| {
            multicaret::unselect_occurrence(&mut d.selections);
            Ok(())
        },
    );
    action(
        registry,
        "editor.selectAllOccurrences",
        "Select All Occurrences",
        Some("⌃⌘G"),
        |d| {
            multicaret::select_all_occurrences(&d.text, &mut d.selections);
            Ok(())
        },
    );
    action(
        registry,
        "editor.cloneCaretAbove",
        "Clone Caret Above",
        None,
        |d| {
            multicaret::clone_caret(&d.text, &mut d.selections, true);
            Ok(())
        },
    );
    action(
        registry,
        "editor.cloneCaretBelow",
        "Clone Caret Below",
        None,
        |d| {
            multicaret::clone_caret(&d.text, &mut d.selections, false);
            Ok(())
        },
    );
    action(
        registry,
        "editor.addCaretsToLineEnds",
        "Add Carets to Ends of Selected Lines",
        Some("⌥⇧G"),
        |d| {
            multicaret::carets_at_line_ends(&d.text, &mut d.selections);
            Ok(())
        },
    );
    registry
        .register(Command::new(
            spec(
                "editor.toggleColumnMode",
                "Column Selection Mode",
                "Editor",
                Some("⇧⌘8"),
                r#"{"enabled":"bool?"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let enabled = state
                    .edit_active(|d| {
                        let enabled = params
                            .get("enabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(!d.selections.column_mode());
                        d.selections.set_column_mode(enabled);
                        Ok(enabled)
                    })
                    .map_err(CommandError::Failed)?;
                Ok(json!({"columnMode": enabled}))
            },
        ))
        .expect("unique built-in id");
}

/// Extend Selection: grows every selection to the next syntactic unit.
/// Returns false when nothing could grow (plain text or whole file selected).
pub fn extend_selection(document: &mut crate::Document) -> bool {
    let source = document.text.text();
    let before = document.selections.selections().to_vec();
    let next: Vec<Selection> = before
        .iter()
        .map(|selection| {
            let bytes = document.text.char_to_byte(selection.start())
                ..document.text.char_to_byte(selection.end());
            match document.analysis.expand_selection(&source, bytes) {
                Some(range) => Selection {
                    anchor: document.text.byte_to_char(range.start),
                    head: document.text.byte_to_char(range.end),
                },
                None => *selection,
            }
        })
        .collect();
    if next == before {
        return false;
    }
    // History only continues while the selection is the last extended one.
    let continues = document.selection_history.last().is_some_and(|previous| {
        previous.len() == before.len()
            && previous
                .iter()
                .zip(&before)
                .all(|(inner, outer)| outer.start() <= inner.start() && inner.end() <= outer.end())
    });
    if !continues {
        document.selection_history.clear();
    }
    document.selection_history.push(before);
    document.selections.set_all(next);
    true
}

/// Shrink Selection: steps back through Extend Selection history, as long as
/// each remembered selection still lies inside the current one.
pub fn shrink_selection(document: &mut crate::Document) -> bool {
    let current = document.selections.selections().to_vec();
    let Some(previous) = document.selection_history.pop() else {
        return false;
    };
    let nested = previous.len() == current.len()
        && previous
            .iter()
            .zip(&current)
            .all(|(inner, outer)| outer.start() <= inner.start() && inner.end() <= outer.end());
    if !nested {
        document.selection_history.clear();
        return false;
    }
    document.selections.set_all(previous);
    true
}

fn selection_command(
    registry: &mut CommandRegistry<IdeState>,
    id: &str,
    label: &str,
    shortcut: &str,
    run: fn(&mut crate::Document) -> bool,
) {
    registry
        .register(Command::new(
            spec(id, label, "Editor", Some(shortcut), "{}"),
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

/// Line and text transformations (PLATFORM-EDIT-040/041/043/044).
fn register_transforms(registry: &mut CommandRegistry<IdeState>) {
    action(
        registry,
        "editor.joinLines",
        "Join Lines",
        Some("⌃⇧J"),
        |d| {
            let prefix = line_comment_prefix(&d.path);
            transform::join_lines(&mut d.text, &mut d.selections, Some(prefix))
        },
    );
    action(
        registry,
        "editor.toggleCase",
        "Toggle Case",
        Some("⇧⌘U"),
        |d| transform::toggle_case(&mut d.text, &mut d.selections),
    );
    action(registry, "editor.sortLines", "Sort Lines", None, |d| {
        transform::sort_lines(&mut d.text, &mut d.selections)
    });
    action(
        registry,
        "editor.reverseLines",
        "Reverse Lines",
        None,
        |d| transform::reverse_lines(&mut d.text, &mut d.selections),
    );
    action(
        registry,
        "editor.splitLine",
        "Split Line",
        Some("⌘↩"),
        |d| transform::split_line(&mut d.text, &mut d.selections),
    );
    action(
        registry,
        "editor.startNewLineBefore",
        "Start New Line Before Current",
        Some("⌥⌘↩"),
        |d| transform::start_new_line_before(&mut d.text, &mut d.selections),
    );
}

/// Moves the active editor's caret to 1-based `line`:`column` (clamped to the
/// document) and selects `length` characters from there, staying on that line.
pub(crate) fn select_in_active(
    state: &mut IdeState,
    line: usize,
    column: usize,
    length: usize,
) -> Result<(), String> {
    let line = line.max(1) - 1;
    let column = column.max(1) - 1;
    state.edit_active(|d| {
        let line = line.min(d.text.len_lines().saturating_sub(1));
        let (start, end, _) = ops::line_bounds(&d.text, line);
        let anchor = (start + column).min(end);
        let head = (anchor + length).min(end);
        d.selections.set_single(Selection { anchor, head });
        Ok(())
    })
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    selection_command(
        registry,
        "editor.extendSelection",
        "Extend Selection",
        "⌥↑",
        extend_selection,
    );
    selection_command(
        registry,
        "editor.shrinkSelection",
        "Shrink Selection",
        "⌥↓",
        shrink_selection,
    );
    action(
        registry,
        "editor.duplicate",
        "Duplicate Line or Selection",
        Some("⌘D"),
        |d| ops::duplicate(&mut d.text, &mut d.selections),
    );
    action(
        registry,
        "editor.deleteLine",
        "Delete Line",
        Some("⌘⌫"),
        |d| ops::delete_lines(&mut d.text, &mut d.selections),
    );
    action(
        registry,
        "editor.moveLineUp",
        "Move Line Up",
        Some("⌥⇧↑"),
        |d| ops::move_lines(&mut d.text, &mut d.selections, true),
    );
    action(
        registry,
        "editor.moveLineDown",
        "Move Line Down",
        Some("⌥⇧↓"),
        |d| ops::move_lines(&mut d.text, &mut d.selections, false),
    );
    action(registry, "editor.indent", "Indent Selection", None, |d| {
        ops::indent_lines(&mut d.text, &mut d.selections)
    });
    action(
        registry,
        "editor.unindent",
        "Unindent Line or Selection",
        Some("⇧⇥"),
        |d| ops::unindent_lines(&mut d.text, &mut d.selections),
    );
    action(
        registry,
        "editor.commentLine",
        "Comment with Line Comment",
        Some("⌘/"),
        |d| {
            let prefix = line_comment_prefix(&d.path);
            ops::toggle_line_comment(&mut d.text, &mut d.selections, prefix)
        },
    );
    action(
        registry,
        "editor.selectAll",
        "Select All",
        Some("⌘A"),
        |d| {
            let len = d.text.len_chars();
            d.selections.set_single(Selection {
                anchor: 0,
                head: len,
            });
            Ok(())
        },
    );

    register_multicaret(registry);
    register_transforms(registry);

    registry
        .register(Command::new(
            spec("editor.copy", "Copy", "Editor", Some("⌘C"), "{}"),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, _| {
                let (text, whole_line) = state
                    .edit_active(|d| Ok(ops::copy_text(&d.text, &d.selections)))
                    .map_err(CommandError::Failed)?;
                Ok(json!({"text": text, "wholeLine": whole_line}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.cut", "Cut", "Editor", Some("⌘X"), "{}"),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, _| {
                let (text, whole_line) = state
                    .edit_active(|d| {
                        let whole_line = d.selections.selections().iter().all(|s| s.is_caret());
                        let text = ops::cut(&mut d.text, &mut d.selections).map_err(text_error)?;
                        Ok((text, whole_line))
                    })
                    .map_err(CommandError::Failed)?;
                Ok(json!({"text": text, "wholeLine": whole_line}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.paste",
                "Paste",
                "Editor",
                Some("⌘V"),
                r#"{"text":"string","wholeLine":"bool?"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let text = string_param(&params, "text")?;
                let whole_line = bool_param(&params, "wholeLine");
                state
                    .edit_active(|d| {
                        ops::paste(&mut d.text, &mut d.selections, &text, whole_line)
                            .map_err(text_error)
                    })
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.gotoLine",
                "Go to Line:Column",
                "Navigate",
                Some("⌘L"),
                r#"{"line":1,"column":"number?"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let line = usize_param(&params, "line")?;
                let column = params.get("column").and_then(Value::as_u64).unwrap_or(1);
                select_in_active(state, line, column as usize, 0).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    for (id, label, shortcut, backwards) in [
        ("editor.findNext", "Find Next Occurrence", "⌘G", false),
        (
            "editor.findPrevious",
            "Find Previous Occurrence",
            "⇧⌘G",
            true,
        ),
    ] {
        registry
            .register(Command::new(
                spec(
                    id,
                    label,
                    "Editor",
                    Some(shortcut),
                    r#"{"query":"string","matchCase":"bool?","wholeWords":"bool?","regex":"bool?"}"#,
                ),
                |state: &IdeState| state.active_document.is_some(),
                move |state: &mut IdeState, params| {
                    let query = string_param(&params, "query")?;
                    let options = find_options(&params, &query)?;
                    state
                        .edit_active(|d| Ok(select_match(d, &query, options, backwards)))
                        .map_err(CommandError::Failed)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "editor.replace",
                "Replace Occurrence",
                "Editor",
                None,
                r#"{"query":"string","replacement":"string","matchCase":"bool?","wholeWords":"bool?","regex":"bool?"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let query = string_param(&params, "query")?;
                let replacement = string_param(&params, "replacement")?;
                let options = find_options(&params, &query)?;
                state
                    .edit_active(|d| {
                        let primary = d.selections.primary();
                        let selected = d
                            .text
                            .slice(primary.start(), primary.end())
                            .map_err(text_error)?;
                        // The selection is replaced only when it is exactly one match;
                        // regex replacements expand `$1`/`${name}` for that match.
                        let replaced = if options.regex {
                            ops::regex_replacements(&d.text.text(), &query, options, &replacement)
                                .map_err(text_error)?
                                .into_iter()
                                .find(|(s, e, _)| *s == primary.start() && *e == primary.end())
                                .map(|(_, _, expanded)| expanded)
                        } else {
                            ops::find_all(&selected, &query, options)
                                .first()
                                .is_some_and(|&(s, e)| s == 0 && e == selected.chars().count())
                                .then(|| replacement.clone())
                        };
                        if let Some(replaced) = replaced {
                            ops::replace_ranges(
                                &mut d.text,
                                &[(primary.start(), primary.end())],
                                &replaced,
                            )
                            .map_err(text_error)?;
                            let caret = primary.start() + replaced.chars().count();
                            d.selections.set_single(Selection::caret(caret));
                        }
                        Ok(select_match(d, &query, options, false))
                    })
                    .map_err(CommandError::Failed)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.replaceAll",
                "Replace All",
                "Editor",
                None,
                r#"{"query":"string","replacement":"string","matchCase":"bool?","wholeWords":"bool?","regex":"bool?"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let query = string_param(&params, "query")?;
                let replacement = string_param(&params, "replacement")?;
                let options = find_options(&params, &query)?;
                let count = state
                    .edit_active(|d| {
                        let count = if options.regex {
                            let changes = ops::regex_replacements(
                                &d.text.text(),
                                &query,
                                options,
                                &replacement,
                            )
                            .map_err(text_error)?;
                            let count = changes.len();
                            d.text
                                .edit_many(changes.into_iter().map(|(start, end, inserted)| {
                                    archipel_ide_text::TextChange {
                                        start_char: start,
                                        end_char: end,
                                        inserted,
                                    }
                                }))
                                .map_err(text_error)?;
                            count
                        } else {
                            let ranges = ops::find_all(&d.text.text(), &query, options);
                            ops::replace_ranges(&mut d.text, &ranges, &replacement)
                                .map_err(text_error)?;
                            ranges.len()
                        };
                        let len = d.text.len_chars();
                        d.selections.clamp(len);
                        Ok(count)
                    })
                    .map_err(CommandError::Failed)?;
                state.status = format!("{count} occurrences replaced");
                Ok(json!({"replaced": count}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn state_with(
        text: &str,
        name: &str,
    ) -> (IdeState, CommandRegistry<IdeState>, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-editor-cmd-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(name), text).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": name}))
            .unwrap();
        (state, commands, root)
    }

    #[test]
    fn undo_puts_the_caret_back_where_the_edit_happened() {
        let (mut state, commands, root) = state_with("one\ntwo\nthree\nfour\n", "u.txt");
        commands
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 8, "text": "new line\n"}),
            )
            .unwrap();
        // The caret wanders off before undoing.
        commands
            .execute("editor.gotoLine", &mut state, json!({"line": 1}))
            .unwrap();
        commands
            .execute("editor.undo", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "one\ntwo\nthree\nfour\n");
        assert_eq!(state.documents[0].selections.primary().head, 8);
        commands
            .execute("editor.redo", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].selections.primary().head, 17);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn open_file_lands_on_and_selects_a_search_match() {
        let (mut state, commands, root) = state_with("a\n  val greeting = 1\n", "found.kt");
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "found.kt", "line": 2, "column": 7, "length": 5}),
            )
            .unwrap();
        let primary = state.documents[0].selections.primary();
        assert_eq!((primary.anchor, primary.head), (8, 13));
        assert_eq!(state.documents[0].text.slice(8, 13).unwrap(), "greet");
        // Out-of-range targets clamp to the line instead of failing.
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "found.kt", "line": 99, "column": 99, "length": 3}),
            )
            .unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn opening_a_nested_file_reveals_it_in_the_tree() {
        let (mut state, commands, root) = state_with("x", "top.txt");
        std::fs::create_dir_all(root.join("src/main/java")).unwrap();
        std::fs::write(root.join("src/main/java/App.java"), "class App {}").unwrap();
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "src/main/java/App.java"}),
            )
            .unwrap();
        for dir in ["src", "src/main", "src/main/java"] {
            assert!(state.expanded_dirs.contains(Path::new(dir)), "{dir}");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn regex_find_replace_and_replace_all_expand_groups() {
        let (mut state, commands, root) = state_with("let a = 1;\nlet bb = 22;\n", "regex.kt");
        let found = commands
            .execute(
                "editor.findNext",
                &mut state,
                json!({"query": r"let (\w+)", "regex": true}),
            )
            .unwrap();
        assert_eq!(found["matches"], 2);
        let params = json!({"query": r"let (\w+) = (\d+)", "replacement": "val $1: Int = $2", "regex": true});
        commands
            .execute("editor.gotoLine", &mut state, json!({"line": 1}))
            .unwrap();
        commands
            .execute("editor.findNext", &mut state, params.clone())
            .unwrap();
        commands
            .execute("editor.replace", &mut state, params.clone())
            .unwrap();
        assert_eq!(
            state.documents[0].text.text(),
            "val a: Int = 1;\nlet bb = 22;\n"
        );
        let all = commands
            .execute("editor.replaceAll", &mut state, params)
            .unwrap();
        assert_eq!(all["replaced"], 1);
        assert_eq!(
            state.documents[0].text.text(),
            "val a: Int = 1;\nval bb: Int = 22;\n"
        );
        let error = commands
            .execute(
                "editor.findNext",
                &mut state,
                json!({"query": "(", "regex": true}),
            )
            .unwrap_err();
        assert!(matches!(error, CommandError::InvalidParams(_)), "{error}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn extend_and_shrink_selection_follow_the_syntax_tree() {
        let source = "class A {\n    int f() { return 1 + 42; }\n}\n";
        let (mut state, commands, root) = state_with(source, "A.java");
        let caret = source.find("42").unwrap();
        state.documents[0]
            .selections
            .set_single(Selection::caret(caret));
        let selected = |state: &IdeState| {
            let selection = state.documents[0].selections.primary();
            state.documents[0]
                .text
                .slice(selection.start(), selection.end())
                .unwrap()
        };
        let extend = |state: &mut IdeState| {
            commands
                .execute("editor.extendSelection", state, Value::Null)
                .unwrap()
        };
        assert_eq!(extend(&mut state)["changed"], true);
        assert_eq!(selected(&state), "42");
        extend(&mut state);
        assert_eq!(selected(&state), "1 + 42");
        extend(&mut state);
        assert_eq!(selected(&state), "return 1 + 42;");
        commands
            .execute("editor.shrinkSelection", &mut state, Value::Null)
            .unwrap();
        assert_eq!(selected(&state), "1 + 42");
        commands
            .execute("editor.shrinkSelection", &mut state, Value::Null)
            .unwrap();
        assert_eq!(selected(&state), "42");
        // Moving the caret elsewhere invalidates the history.
        state.documents[0]
            .selections
            .set_single(Selection::caret(0));
        let shrunk = commands
            .execute("editor.shrinkSelection", &mut state, Value::Null)
            .unwrap();
        assert_eq!(shrunk["changed"], false);
        for _ in 0..20 {
            extend(&mut state);
        }
        assert_eq!(selected(&state), source);
        assert_eq!(extend(&mut state)["changed"], false);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn comment_prefix_depends_on_language() {
        assert_eq!(line_comment_prefix(Path::new("a.rs")), "//");
        assert_eq!(line_comment_prefix(Path::new("Cargo.toml")), "#");
        assert_eq!(line_comment_prefix(Path::new("q.SQL")), "--");
    }

    #[test]
    fn line_actions_run_through_the_registry() {
        let (mut state, commands, root) = state_with("a = 1\nb = 2\n", "x.py");
        commands
            .execute("editor.commentLine", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "# a = 1\nb = 2\n");
        commands
            .execute("editor.duplicate", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "# a = 1\nb = 2\nb = 2\n");
        commands
            .execute("editor.deleteLine", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "# a = 1\nb = 2\n");
        commands
            .execute("editor.gotoLine", &mut state, json!({"line": 2}))
            .unwrap();
        let copied = commands
            .execute("editor.copy", &mut state, Value::Null)
            .unwrap();
        assert_eq!(copied, json!({"text": "b = 2\n", "wholeLine": true}));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn multicaret_and_transform_actions_run_through_the_registry() {
        let (mut state, commands, root) = state_with("val a = a\n    // x\n    // y\n", "m.kt");
        let run =
            |state: &mut IdeState, id: &str| commands.execute(id, state, Value::Null).unwrap();
        commands
            .execute(
                "editor.gotoLine",
                &mut state,
                json!({"line": 1, "column": 5}),
            )
            .unwrap();
        run(&mut state, "editor.selectNextOccurrence");
        run(&mut state, "editor.selectNextOccurrence");
        assert_eq!(state.documents[0].selections.selections().len(), 2);
        run(&mut state, "editor.unselectOccurrence");
        assert_eq!(state.documents[0].selections.selections().len(), 1);
        run(&mut state, "editor.selectAllOccurrences");
        run(&mut state, "editor.toggleCase");
        assert_eq!(
            state.documents[0].text.text(),
            "val A = A\n    // x\n    // y\n"
        );
        commands
            .execute("editor.gotoLine", &mut state, json!({"line": 2}))
            .unwrap();
        run(&mut state, "editor.joinLines");
        assert_eq!(state.documents[0].text.text(), "val A = A\n    // x y\n");
        run(&mut state, "editor.cloneCaretAbove");
        assert_eq!(state.documents[0].selections.selections().len(), 2);
        let mode = run(&mut state, "editor.toggleColumnMode");
        assert_eq!(mode, json!({"columnMode": true}));
        assert!(state.documents[0].selections.column_mode());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn find_replace_and_goto_line() {
        let (mut state, commands, root) = state_with("foo bar\nfoo baz\n", "f.txt");
        let found = commands
            .execute("editor.findNext", &mut state, json!({"query": "foo"}))
            .unwrap();
        assert_eq!(found, json!({"matches": 2, "index": 0}));
        let found = commands
            .execute("editor.findNext", &mut state, json!({"query": "foo"}))
            .unwrap();
        assert_eq!(found["index"], 1);
        let found = commands
            .execute("editor.findNext", &mut state, json!({"query": "foo"}))
            .unwrap();
        assert_eq!(found["index"], 0, "wraps around");
        commands
            .execute(
                "editor.replace",
                &mut state,
                json!({"query": "foo", "replacement": "qux"}),
            )
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "qux bar\nfoo baz\n");
        let replaced = commands
            .execute(
                "editor.replaceAll",
                &mut state,
                json!({"query": "BA", "replacement": "_"}),
            )
            .unwrap();
        assert_eq!(replaced["replaced"], 2);
        assert_eq!(state.documents[0].text.text(), "qux _r\nfoo _z\n");
        commands
            .execute(
                "editor.gotoLine",
                &mut state,
                json!({"line": 2, "column": 3}),
            )
            .unwrap();
        assert_eq!(state.documents[0].selections.primary().head, 9);
        std::fs::remove_dir_all(root).unwrap();
    }
}
