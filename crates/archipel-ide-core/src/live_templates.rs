//! Live templates and postfix completion: template items in the completion
//! popup, Tab expansion, Insert Live Template, Surround With Live Template
//! and the user's template store. Expansions are LSP snippets, so Tab stops
//! reuse the snippet session.

use crate::completion::CompletionSession;
use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_lsp::{
    completion::{MatchClass, RankedItem, is_identifier_char},
    lsp_types::{
        CompletionItem, CompletionItemKind, CompletionTextEdit, InsertTextFormat, Position, Range,
        TextEdit,
    },
};
use archipel_ide_settings::{SettingValue, ids};
use archipel_ide_templates::{
    Context, ExpansionContext, Language, Template, TemplateRegistry, UserTemplate,
    abbreviation_before, contexts_at, expression_before, instantiate, scan, template_key,
};
use archipel_ide_text::{Selection, TextDocument};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::Path;

/// Marks completion items that come from live templates.
const ITEM_MARKER: &str = "archipelTemplate";

pub(crate) fn is_template_item(item: &CompletionItem) -> bool {
    item.data
        .as_ref()
        .is_some_and(|data| data.get(ITEM_MARKER).is_some())
}

/// Context for expanding at `offset` (chars) in `text`.
fn expansion_context(path: &Path, text: &str, offset: usize) -> ExpansionContext {
    let byte = text
        .char_indices()
        .nth(offset)
        .map_or(text.len(), |(byte, _)| byte);
    let scanned = scan(&text[..byte]);
    let method = scanned.method();
    // Names in use nearby, for `suggestIndexName()`.
    let from = text[..byte]
        .char_indices()
        .rev()
        .nth(4_000)
        .map_or(0, |(index, _)| index);
    let to = text[byte..]
        .char_indices()
        .nth(2_000)
        .map_or(text.len(), |(index, _)| byte + index);
    ExpansionContext {
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        // The completion insertion indents continuation lines like the caret's line.
        line_indent: String::new(),
        indent_unit: "\t".into(),
        selection: None,
        expression: None,
        argument: None,
        surrounding: text[from..to].to_owned(),
        class_name: scanned.class_name(),
        method_name: method.as_ref().map(|(name, _)| name.clone()),
        method_parameters: method.map(|(_, params)| params).unwrap_or_default(),
    }
}

fn template_item(
    document: &archipel_ide_text::TextDocument,
    template: &Template,
    context: &ExpansionContext,
    postfix: bool,
    range: (usize, usize),
) -> Option<CompletionItem> {
    let (start_line, start_character) = document.lsp_position(range.0).ok()?;
    let (end_line, end_character) = document.lsp_position(range.1).ok()?;
    let snippet = instantiate(template, context).to_snippet();
    Some(CompletionItem {
        label: template.abbreviation.clone(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(template.description.clone()),
        filter_text: Some(template.abbreviation.clone()),
        sort_text: Some(format!("0{}", template.abbreviation)),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: Range::new(
                Position::new(start_line as u32, start_character as u32),
                Position::new(end_line as u32, end_character as u32),
            ),
            new_text: snippet,
        })),
        // Not empty: the popup must not ask a language server to resolve it.
        additional_text_edits: Some(Vec::new()),
        data: Some(json!({ ITEM_MARKER: if postfix { "postfix" } else { "live" } })),
        ..Default::default()
    })
}

/// Start of the identifier that ends at `caret`.
fn word_start(text: &TextDocument, caret: usize) -> usize {
    let mut start = caret;
    while start > 0 && text.char_at(start - 1).is_some_and(is_identifier_char) {
        start -= 1;
    }
    start
}

/// The statement surrounders Java offers in Surround With (ED-TMPL-042).
fn java_surrounders() -> Vec<Template> {
    [
        ("if", "if ($END$) {\n    $SELECTION$\n}"),
        ("if / else", "if ($END$) {\n    $SELECTION$\n} else {\n}"),
        ("while", "while ($END$) {\n    $SELECTION$\n}"),
        ("do / while", "do {\n    $SELECTION$\n} while ($END$);"),
        ("for", "for ($END$) {\n    $SELECTION$\n}"),
        (
            "try / catch",
            "try {\n    $SELECTION$\n} catch (Exception e) {\n    $END$\n}",
        ),
        (
            "try / finally",
            "try {\n    $SELECTION$\n} finally {\n    $END$\n}",
        ),
        (
            "try / catch / finally",
            "try {\n    $SELECTION$\n} catch (Exception e) {\n    $END$\n} finally {\n}",
        ),
        ("synchronized", "synchronized ($END$) {\n    $SELECTION$\n}"),
        (
            "Runnable",
            "Runnable runnable = () -> {\n    $SELECTION$\n};",
        ),
        ("{ }", "{\n    $SELECTION$\n}"),
    ]
    .into_iter()
    .map(|(name, text)| Template {
        abbreviation: name.to_owned(),
        description: name.to_owned(),
        text: text.to_owned(),
        ..Default::default()
    })
    .collect()
}

/// The word before `anchor`, separated by at most one blank, and where it starts.
fn argument_before(text: &TextDocument, anchor: usize) -> Option<(String, usize)> {
    let mut end = anchor;
    if end > 0 && text.char_at(end - 1) == Some(' ') {
        end -= 1;
    }
    let start = word_start(text, end);
    (start < end).then(|| (text.slice(start, end).unwrap_or_default(), start))
}

impl IdeState {
    /// Bundled live templates plus the user's, honoring the enable toggles.
    pub fn live_templates(&self) -> TemplateRegistry {
        let settings = self.settings();
        let settings = settings.read();
        TemplateRegistry::bundled().with_user(
            settings.list(ids::LIVE_TEMPLATES_USER),
            settings.list(ids::LIVE_TEMPLATES_DISABLED),
        )
    }

    /// Live template and postfix items for the identifier `anchor..caret` of
    /// the active document (an empty list when none apply).
    pub(crate) fn template_completion_items(&self, anchor: usize) -> Vec<CompletionItem> {
        self.template_items(anchor, false)
    }

    /// `listing`: the user asked for the list (Insert Live Template), so
    /// templates show even without a typed prefix in files of unknown language.
    fn template_items(&self, anchor: usize, listing: bool) -> Vec<CompletionItem> {
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return Vec::new();
        };
        let caret = document.selections.primary().head;
        if anchor > caret {
            return Vec::new();
        }
        // Completion in files without a language would list the general
        // templates for every empty prefix: only offer them once typing started.
        if !listing && anchor == caret && Language::of(&document.path) == Language::Other {
            return Vec::new();
        }
        let registry = self.live_templates();
        let text = document.text.text();
        let path = &document.path;
        let mut items = Vec::new();
        if anchor > 0 && document.text.char_at(anchor - 1) == Some('.') {
            let dot = anchor - 1;
            if let Some(start) = expression_before(&document.text, dot)
                && let Ok(expression) = document.text.slice(start, dot)
                && !scan(&text.chars().take(start).collect::<String>()).in_literal
            {
                let mut context = expansion_context(path, &text, start);
                context.expression = Some(expression);
                for template in registry.postfix_for(path) {
                    items.extend(template_item(
                        &document.text,
                        template,
                        &context,
                        true,
                        (start, caret),
                    ));
                }
            }
            return items;
        }
        let contexts = contexts_at(path, &text, anchor);
        let context = expansion_context(path, &text, anchor);
        for template in registry.templates_for(path) {
            if template.applies_in(&contexts) {
                // A template with `$ARG$` takes the word before its abbreviation (ED-TMPL-012).
                let (context, start) = match argument_before(&document.text, anchor) {
                    Some((word, from)) if template.text.contains("$ARG$") => {
                        let mut with_argument = context.clone();
                        with_argument.argument = Some(word);
                        (with_argument, from)
                    }
                    _ => (context.clone(), anchor),
                };
                items.extend(template_item(
                    &document.text,
                    template,
                    &context,
                    false,
                    (start, caret),
                ));
            }
        }
        items
    }

    /// Opens the popup on a fixed list of template items (no language server).
    fn open_template_popup(&mut self, items: Vec<CompletionItem>, anchor: usize) {
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return;
        };
        let path = document.path.clone();
        let caret = document.selections.primary().head;
        let prefix = document.text.slice(anchor, caret).unwrap_or_default();
        self.completion = Some(CompletionSession {
            path,
            anchor,
            request_caret: caret,
            items,
            ranked: Vec::new(),
            selected: 0,
            prefix,
            incomplete: false,
            waiting: false,
            explicit: true,
            resolved: Default::default(),
            indexing: None,
            requery_when_ready: false,
        });
        self.refresh_completion();
    }

    /// Insert Live Template: lists the templates that apply at the caret.
    pub fn insert_live_template(&mut self) -> Result<(), String> {
        let document = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .ok_or("no active document")?;
        let anchor = word_start(&document.text, document.selections.primary().head);
        let items: Vec<_> = self
            .template_items(anchor, true)
            .into_iter()
            .filter(|item| {
                item.data
                    .as_ref()
                    .is_some_and(|data| data[ITEM_MARKER] == "live")
            })
            .collect();
        if items.is_empty() {
            self.status = "No live templates apply here".into();
            return Ok(());
        }
        self.open_template_popup(items, anchor);
        Ok(())
    }

    /// Surround With Live Template: lists templates that wrap the selection
    /// (or the current line when nothing is selected).
    pub fn surround_with_live_template(&mut self) -> Result<(), String> {
        self.surround_with_popup(false)
    }

    /// Surround With (Ctrl+Alt+T): the language's surrounders, then the live templates that
    /// contain `$SELECTION$` (ED-TMPL-040).
    pub fn surround_with(&mut self) -> Result<(), String> {
        self.surround_with_popup(true)
    }

    fn surround_with_popup(&mut self, surrounders: bool) -> Result<(), String> {
        let index = self.active_document.ok_or("no active document")?;
        let registry = self.live_templates();
        let document = self
            .documents
            .get_mut(index)
            .ok_or("active document does not exist")?;
        let primary = document.selections.primary();
        let (mut start, mut end) = (primary.start(), primary.end());
        if start == end {
            let line = document.text.char_to_line(start);
            let content = document.text.line(line).unwrap_or_default();
            let content = content.trim_end_matches(['\n', '\r']);
            let first = document.text.line_to_char(line).unwrap_or(0);
            let indent = content.len() - content.trim_start().len();
            start = first + indent;
            end = first + content.trim_end().chars().count();
        }
        if start >= end {
            self.status = "Select the code to surround first".into();
            return Ok(());
        }
        let selection = document
            .text
            .slice(start, end)
            .map_err(|error| error.to_string())?;
        let text = document.text.text();
        let contexts = contexts_at(&document.path, &text, start);
        let mut context = expansion_context(&document.path, &text, start);
        context.selection = Some(selection);
        let mut items: Vec<_> = Vec::new();
        if surrounders && document.path.extension().is_some_and(|ext| ext == "java") {
            items.extend(java_surrounders().iter().filter_map(|template| {
                template_item(&document.text, template, &context, false, (start, end))
            }));
        }
        items.extend(
            registry
                .surround_templates(&document.path, &contexts)
                .filter_map(|template| {
                    template_item(&document.text, template, &context, false, (start, end))
                }),
        );
        if items.is_empty() {
            self.status = "No surround templates apply here".into();
            return Ok(());
        }
        // The popup edits after the surrounded code: collapse the selection to its end.
        document.selections.set_single(Selection::caret(end));
        self.open_template_popup(items, end);
        Ok(())
    }

    /// Tab: expands the abbreviation before the caret (live template or
    /// `expr.key` postfix). Returns false when there is nothing to expand.
    pub fn expand_template_at_caret(&mut self) -> Result<bool, String> {
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return Ok(false);
        };
        let primary = document.selections.primary();
        if !primary.is_caret() {
            return Ok(false);
        }
        let caret = primary.head;
        let start = word_start(&document.text, caret);
        if start == caret {
            return Ok(false);
        }
        let key = document.text.slice(start, caret).unwrap_or_default();
        let registry = self.live_templates();
        let is_postfix = start > 0 && document.text.char_at(start - 1) == Some('.');
        let known = if is_postfix {
            registry.find_postfix(&document.path, &key).is_some()
        } else {
            let text = document.text.text();
            let contexts = contexts_at(&document.path, &text, start);
            abbreviation_before(&document.text, caret)
                .is_some_and(|(_, abbreviation)| abbreviation == key)
                && registry.find_in(&document.path, &contexts, &key).is_some()
        };
        if !known {
            return Ok(false);
        }
        let items = self.template_completion_items(start);
        let Some(row) = items.iter().position(|item| item.label == key) else {
            return Ok(false);
        };
        self.completion = Some(CompletionSession {
            path: self.documents[self.active_document.unwrap_or_default()]
                .path
                .clone(),
            anchor: start,
            request_caret: caret,
            items,
            ranked: vec![RankedItem {
                index: row,
                class: MatchClass::Exact,
                score: 0,
                label_positions: Vec::new(),
            }],
            selected: 0,
            prefix: key,
            incomplete: false,
            waiting: false,
            explicit: true,
            resolved: HashSet::new(),
            indexing: None,
            requery_when_ready: false,
        });
        self.accept_completion(false)?;
        Ok(true)
    }

    fn user_template_lines(&self) -> Vec<String> {
        self.settings().read().list(ids::LIVE_TEMPLATES_USER)
    }

    fn store_list(&mut self, id: &str, lines: Vec<String>) -> Result<(), CommandError> {
        let settings = self.settings();
        let mut settings = settings.write();
        settings
            .set(id, SettingValue::List(lines))
            .and_then(|_| settings.store.save())
            .map_err(|error| CommandError::Failed(error.to_string()))
    }

    /// Creates or replaces a user template (also how bundled ones are edited).
    pub fn save_user_template(&mut self, user: UserTemplate) -> Result<(), CommandError> {
        let user = UserTemplate {
            group: user.group.trim().to_owned(),
            ..user
        };
        if user.group.is_empty() || user.group.contains('/') {
            return Err(CommandError::InvalidParams(
                "the group name must not be empty or contain `/`".into(),
            ));
        }
        let candidate =
            UserTemplate::from_line(&user.to_line()).map_err(CommandError::InvalidParams)?;
        let same = |line: &String| {
            UserTemplate::from_line(line).is_ok_and(|other| {
                other.group.eq_ignore_ascii_case(&candidate.group)
                    && other.postfix == candidate.postfix
                    && other.template.abbreviation == candidate.template.abbreviation
            })
        };
        let mut lines = self.user_template_lines();
        lines.retain(|line| !same(line));
        lines.push(candidate.to_line());
        self.store_list(ids::LIVE_TEMPLATES_USER, lines)
    }

    /// Removes a user template (bundled templates it overrode return). A
    /// bundled template without an override is switched off instead.
    pub fn delete_user_template(
        &mut self,
        group: &str,
        abbreviation: &str,
        postfix: bool,
    ) -> Result<(), CommandError> {
        let mut lines = self.user_template_lines();
        let before = lines.len();
        lines.retain(|line| {
            !UserTemplate::from_line(line).is_ok_and(|other| {
                other.group.eq_ignore_ascii_case(group)
                    && other.postfix == postfix
                    && other.template.abbreviation == abbreviation
            })
        });
        if lines.len() != before {
            return self.store_list(ids::LIVE_TEMPLATES_USER, lines);
        }
        let template = Template {
            abbreviation: abbreviation.to_owned(),
            ..Default::default()
        };
        self.set_template_enabled(&template_key(group, &template, postfix), false)
    }

    /// Switches a group (`Java`) or template (`Java/sout`, `Java/.if`) on or off.
    pub fn set_template_enabled(&mut self, key: &str, enabled: bool) -> Result<(), CommandError> {
        let mut disabled = self.settings().read().list(ids::LIVE_TEMPLATES_DISABLED);
        disabled.retain(|line| line != key);
        if !enabled {
            disabled.push(key.to_owned());
        }
        self.store_list(ids::LIVE_TEMPLATES_DISABLED, disabled)
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let has_document = |state: &IdeState| state.active_document.is_some();
    registry
        .register(Command::new(
            spec(
                "template.expand",
                "Expand Live Template or Indent",
                "Code",
                None,
                "{}",
            ),
            has_document,
            |state: &mut IdeState, _| {
                if state
                    .expand_template_at_caret()
                    .map_err(CommandError::Failed)?
                {
                    return Ok(json!({"expanded": true}));
                }
                // Nothing to expand: Tab inserts one indentation level.
                state
                    .edit_active(|document| {
                        document
                            .selections
                            .insert(&mut document.text, "    ")
                            .map_err(|error| error.to_string())
                    })
                    .map_err(CommandError::Failed)?;
                Ok(json!({"expanded": false}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "template.insert",
                "Insert Live Template…",
                "Code",
                Some("Ctrl+J"),
                "{}",
            ),
            has_document,
            |state: &mut IdeState, _| {
                state.insert_live_template().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "template.surround",
                "Surround with Live Template…",
                "Code",
                Some("Ctrl+Alt+J"),
                "{}",
            ),
            has_document,
            |state: &mut IdeState, _| {
                state
                    .surround_with_live_template()
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "template.surroundWith",
                "Surround With…",
                "Code",
                Some("Ctrl+Alt+T"),
                "{}",
            ),
            has_document,
            |state: &mut IdeState, _| {
                state.surround_with().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "template.save",
                "Save Live Template",
                "Settings",
                None,
                r#"{"group":"string","postfix":"bool?","abbreviation":"string","description":"string?","text":"string","contexts":"array?","variables":"array?"}"#,
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, params| {
                let template = Template {
                    abbreviation: string_param(&params, "abbreviation")?.trim().to_owned(),
                    description: params
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    text: string_param(&params, "text")?,
                    variables: serde_json::from_value(
                        params.get("variables").cloned().unwrap_or(json!([])),
                    )
                    .map_err(|error| CommandError::InvalidParams(error.to_string()))?,
                    contexts: serde_json::from_value(
                        params.get("contexts").cloned().unwrap_or(json!([])),
                    )
                    .map_err(|error| CommandError::InvalidParams(error.to_string()))?,
                };
                state.save_user_template(UserTemplate {
                    group: string_param(&params, "group")?,
                    postfix: params
                        .get("postfix")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    template,
                })?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "template.delete",
                "Delete Live Template",
                "Settings",
                None,
                r#"{"group":"string","abbreviation":"string","postfix":"bool?"}"#,
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, params| {
                state.delete_user_template(
                    &string_param(&params, "group")?,
                    &string_param(&params, "abbreviation")?,
                    params
                        .get("postfix")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "template.setEnabled",
                "Enable or Disable Live Template",
                "Settings",
                None,
                r#"{"key":"string","enabled":"bool"}"#,
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, params| {
                let enabled = params
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| CommandError::InvalidParams("enabled must be a bool".into()))?;
                state.set_template_enabled(&string_param(&params, "key")?, enabled)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

/// Contexts shown for a template in settings.
pub fn context_labels(template: &Template) -> String {
    if template.contexts.is_empty() {
        return "Everywhere in this language".into();
    }
    template
        .contexts
        .iter()
        .map(|context: &Context| context.label())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use archipel_ide_lsp::{CompletionResult, CompletionTrigger};
    use std::path::PathBuf;

    /// A project with `Main.java` containing `text`, caret at the end.
    fn java_state(text: &str) -> (IdeState, archipel_ide_command::CommandRegistry<IdeState>) {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root: PathBuf = std::env::temp_dir().join(format!(
            "archipel-ide-templates-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Main.java"), text).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "Main.java"}))
            .unwrap();
        let end = state.documents[0].text.len_chars();
        state.documents[0]
            .selections
            .set_single(Selection::caret(end));
        (state, commands)
    }

    const BODY: &str = "class Main {\n    void run() {\n        ";

    fn text(state: &IdeState) -> String {
        state.documents[0].text.text()
    }

    fn type_text(state: &mut IdeState, typed: &str) {
        let document = &mut state.documents[0];
        document
            .selections
            .insert(&mut document.text, typed)
            .unwrap();
    }

    #[test]
    fn abbreviation_shows_in_the_popup_and_enter_expands_it() {
        let (mut state, _) = java_state(&format!("{BODY}sout"));
        state
            .start_completion(CompletionTrigger::Invoked, false)
            .unwrap();
        // With a language server installed the popup waits for its answer.
        state.receive_completion(CompletionResult::default());
        let session = state.completion.as_ref().unwrap();
        let item = session.selected_item().unwrap();
        assert_eq!(item.label, "sout");
        assert_eq!(item.kind, Some(CompletionItemKind::SNIPPET));
        assert_eq!(
            item.detail.as_deref(),
            Some("Prints a string to System.out")
        );
        state.accept_completion(false).unwrap();
        assert_eq!(text(&state), format!("{BODY}System.out.println();"));
        let caret = state.documents[0].selections.primary().head;
        assert_eq!(state.documents[0].text.char_at(caret), Some(')'));
        assert!(state.snippet.is_none());
    }

    #[test]
    fn tab_expands_psvm_with_the_line_indentation() {
        let (mut state, commands) = java_state("class Main {\n    psvm");
        let result = commands
            .execute("template.expand", &mut state, Value::Null)
            .unwrap();
        assert_eq!(result["expanded"], true);
        assert_eq!(
            text(&state),
            "class Main {\n    public static void main(String[] args) {\n        \n    }"
        );
        let caret = state.documents[0].selections.primary().head;
        assert_eq!(state.documents[0].text.char_to_line(caret), 2);
        // Statement templates are not offered among declarations.
        let (mut state, commands) = java_state("class Main {\n    sout");
        let result = commands
            .execute("template.expand", &mut state, Value::Null)
            .unwrap();
        assert_eq!(result["expanded"], false);
        assert_eq!(text(&state), "class Main {\n    sout    ");
    }

    #[test]
    fn fori_walks_its_variables_with_tab() {
        let (mut state, commands) = java_state(&format!("{BODY}fori"));
        commands
            .execute("template.expand", &mut state, Value::Null)
            .unwrap();
        assert_eq!(
            text(&state),
            format!("{BODY}for (int i = 0; i < ; i++) {{\n            \n        }}")
        );
        let selected = |state: &IdeState| {
            let document = &state.documents[0];
            document
                .selections
                .selections()
                .iter()
                .map(|s| document.text.slice(s.start(), s.end()).unwrap())
                .collect::<Vec<_>>()
        };
        // One caret in the first occurrence; the others are mirrors (ED-TMPL-002).
        assert_eq!(selected(&state), ["i"]);
        type_text(&mut state, "k");
        state.snippet_after_change();
        assert!(
            text(&state).contains("for (int k = 0; k < ; k++)"),
            "{}",
            text(&state)
        );
        commands
            .execute("snippet.next", &mut state, Value::Null)
            .unwrap();
        type_text(&mut state, "n");
        assert!(text(&state).contains("k < n;"), "{}", text(&state));
    }

    #[test]
    fn postfix_var_rewrites_the_expression() {
        let (mut state, commands) = java_state(&format!("{BODY}new ArrayList<String>().var"));
        let result = commands
            .execute("template.expand", &mut state, Value::Null)
            .unwrap();
        assert_eq!(result["expanded"], true);
        assert_eq!(
            text(&state),
            format!("{BODY}ArrayList<String> arrayList = new ArrayList<String>();")
        );
        // The popup after `expr.` offers the postfix keys.
        let (mut state, _) = java_state(&format!("{BODY}done."));
        state
            .start_completion(CompletionTrigger::Character(".".into()), false)
            .unwrap();
        // With a language server installed the popup waits for its answer.
        state.receive_completion(CompletionResult::default());
        let session = state.completion.as_ref().unwrap();
        let labels: Vec<_> = session
            .ranked
            .iter()
            .map(|row| session.items[row.index].label.as_str())
            .collect();
        for key in [
            "if", "else", "for", "fori", "var", "not", "null", "nn", "return", "sout", "try",
            "cast", "par",
        ] {
            assert!(labels.contains(&key), "{key} in {labels:?}");
        }
        assert!(!labels.contains(&"val"), "Java has no .val");
    }

    #[test]
    fn insert_and_surround_are_commands_over_the_popup() {
        let (mut state, commands) = java_state(BODY);
        commands
            .execute("template.insert", &mut state, Value::Null)
            .unwrap();
        let session = state.completion.as_ref().unwrap();
        assert!(session.ranked.len() > 10);
        assert!(session.items.iter().all(|item| item.label != "psvm"));
        state.close_completion();

        let (mut state, commands) = java_state(&format!("{BODY}work();\n    }}\n}}"));
        let start = text(&state).find("work").unwrap();
        state.documents[0].selections.set_single(Selection {
            anchor: start,
            head: start + "work();".len(),
        });
        commands
            .execute("template.surround", &mut state, Value::Null)
            .unwrap();
        let session = state.completion.as_mut().unwrap();
        let row = session
            .ranked
            .iter()
            .position(|row| session.items[row.index].label == "T")
            .expect("try/catch surround");
        session.selected = row;
        state.accept_completion(false).unwrap();
        assert!(
            text(&state).contains("try {\n            work();\n        } catch (Exception e) {\n            \n        }"),
            "{}",
            text(&state)
        );
    }

    #[test]
    fn user_templates_persist_override_and_can_be_disabled() {
        let (mut state, commands) = java_state(&format!("{BODY}sout"));
        commands
            .execute(
                "template.save",
                &mut state,
                json!({"group": "Java", "abbreviation": "sout", "description": "log", "text": "log($END$);"}),
            )
            .unwrap();
        assert_eq!(
            state
                .live_templates()
                .find(Path::new("A.java"), "sout")
                .unwrap()
                .text,
            "log($END$);"
        );
        commands
            .execute("template.expand", &mut state, Value::Null)
            .unwrap();
        assert_eq!(text(&state), format!("{BODY}log();"));
        // Invalid input is a structured error, not a panic.
        assert!(
            commands
                .execute(
                    "template.save",
                    &mut state,
                    json!({"group": "x/y", "abbreviation": "a", "text": "t"})
                )
                .is_err()
        );
        assert!(
            commands
                .execute(
                    "template.save",
                    &mut state,
                    json!({"group": "x", "abbreviation": " ", "text": "t"})
                )
                .is_err()
        );
        commands
            .execute(
                "template.delete",
                &mut state,
                json!({"group": "Java", "abbreviation": "sout"}),
            )
            .unwrap();
        assert_eq!(
            state
                .live_templates()
                .find(Path::new("A.java"), "sout")
                .unwrap()
                .text,
            "System.out.println($END$);"
        );
        commands
            .execute(
                "template.setEnabled",
                &mut state,
                json!({"key": "Java/sout", "enabled": false}),
            )
            .unwrap();
        assert!(
            state
                .live_templates()
                .find(Path::new("A.java"), "sout")
                .is_none()
        );
        commands
            .execute(
                "template.setEnabled",
                &mut state,
                json!({"key": "Java/sout", "enabled": true}),
            )
            .unwrap();
        assert!(
            state
                .live_templates()
                .find(Path::new("A.java"), "sout")
                .is_some()
        );
    }
}
