//! Code navigation through language servers: Go to Declaration / Type
//! Declaration / Implementation, Find Usages, Show Usages, and the Back /
//! Forward navigation history.
//!
//! Requests are asynchronous: a command sends the request and returns at
//! once; [`IdeState::poll_navigation`] (called from `poll_lsp`) applies the
//! answer: one target is opened, several become [`NavigationState::choices`]
//! for a chooser popup, and usages fill [`NavigationState::usages`].
use crate::{IdeState, spec, string_param, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_lsp::{Feature, LspLocation, RequestHandle, parse_locations};
use archipel_ide_text::{Selection, ops};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// History entries kept per direction.
const HISTORY_LIMIT: usize = 150;
/// Usages listed at most (the reference IDE also stops and warns).
const USAGE_LIMIT: usize = 2_000;
/// Files larger than this are not read to show a usage's line.
const PREVIEW_FILE_LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NavigationKind {
    Declaration,
    TypeDeclaration,
    Implementation,
    FindUsages,
    ShowUsages,
}

impl NavigationKind {
    fn feature(self) -> Feature {
        match self {
            Self::Declaration => Feature::Definition,
            Self::TypeDeclaration => Feature::TypeDefinition,
            Self::Implementation => Feature::Implementation,
            Self::FindUsages | Self::ShowUsages => Feature::References,
        }
    }

    fn not_found(self) -> &'static str {
        match self {
            Self::Declaration => "Cannot find declaration to go to",
            Self::TypeDeclaration => "Cannot find type declaration to go to",
            Self::Implementation => "No implementations found",
            Self::FindUsages | Self::ShowUsages => "No usages found",
        }
    }
}

/// A caret position in a project file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub path: PathBuf,
    pub offset: usize,
}

/// One navigation target or usage, with the text of its line for display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationItem {
    /// Project-relative, or absolute for files outside the project.
    pub path: PathBuf,
    pub external: bool,
    /// 0-based line and character columns.
    pub line: usize,
    pub column: usize,
    pub end_column: usize,
    /// The line's text, trimmed of its line break.
    pub line_text: String,
}

/// Targets for a chooser (several declarations/implementations) or the
/// Show Usages popup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationList {
    pub kind: NavigationKind,
    pub title: String,
    pub items: Vec<NavigationItem>,
    pub truncated: bool,
}

#[derive(Debug)]
struct Pending {
    kind: NavigationKind,
    handle: RequestHandle,
    symbol: String,
    origin: Place,
}

#[derive(Debug, Default)]
pub struct NavigationState {
    back: Vec<Place>,
    forward: Vec<Place>,
    pending: Option<Pending>,
    /// Several targets to choose from, or the Show Usages popup.
    pub choices: Option<NavigationList>,
    /// The latest Find Usages result, shown in the Find tool window.
    pub usages: Option<NavigationList>,
    /// Whether the Find tool window shows the usages instead of text results.
    pub usages_selected: bool,
    /// Where documents were last edited, oldest first: one place per file and run of edits
    /// (Last Edit Location, ED-NAV-005).
    changes: Vec<Place>,
    /// The change place the last Last Edit Location went to.
    change_walk: Option<usize>,
}

impl NavigationState {
    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }
}

impl IdeState {
    fn canonical_root(&self) -> Option<PathBuf> {
        let root = self.project.as_ref()?.root();
        Some(root.canonicalize().unwrap_or_else(|_| root.to_owned()))
    }

    /// The caret position of the active document.
    pub fn current_place(&self) -> Option<Place> {
        let document = self.documents.get(self.active_document?)?;
        Some(Place {
            path: document.path.clone(),
            offset: document.selections.primary().head,
        })
    }

    /// The line of a place in an open document.
    fn place_line(&self, place: &Place) -> Option<usize> {
        let document = self.documents.iter().find(|d| d.path == place.path)?;
        Some(
            document
                .text
                .char_to_line(place.offset.min(document.text.len_chars())),
        )
    }

    /// Records the place a navigation command started at. A place within fewer than four lines
    /// of the previous one in the same file replaces it (ED-NAV-003).
    fn remember(&mut self, place: Place) {
        if self.navigation.back.last() == Some(&place) {
            return;
        }
        let near = self.navigation.back.last().is_some_and(|previous| {
            previous.path == place.path
                && matches!(
                    (self.place_line(previous), self.place_line(&place)),
                    (Some(a), Some(b)) if a.abs_diff(b) < 4
                )
        });
        if near {
            self.navigation.back.pop();
        }
        self.navigation.back.push(place);
        if self.navigation.back.len() > HISTORY_LIMIT {
            self.navigation.back.remove(0);
        }
    }

    /// A navigation command (Go to Line) moved the caret away from `origin`: Back returns there
    /// and the forward places are gone (ED-NAV-002).
    pub(crate) fn navigated_from(&mut self, origin: Option<Place>) {
        let Some(origin) = origin else {
            return;
        };
        if self.current_place().as_ref() != Some(&origin) {
            self.remember(origin);
            self.navigation.forward.clear();
        }
    }

    /// Records the caret place after a document edit for Last Edit Location (ED-NAV-005).
    pub(crate) fn record_change_place(&mut self) {
        let Some(place) = self.current_place() else {
            return;
        };
        // Consecutive changes in the same place merge; other places stay (SRCH-HIS-003).
        let line = self.place_line(&place);
        let same_place = self
            .navigation
            .changes
            .last()
            .is_some_and(|last| last.path == place.path && self.place_line(last) == line);
        let changes = &mut self.navigation.changes;
        if same_place {
            changes.pop();
        }
        changes.push(place);
        if changes.len() > HISTORY_LIMIT {
            changes.remove(0);
        }
        self.navigation.change_walk = None;
    }

    /// Last Edit Location: the most recent place that differs from the current one; repeated
    /// invocation walks back through older places.
    pub fn jump_to_last_change(&mut self) -> Result<(), String> {
        let current = self.current_place();
        let changes = self.navigation.changes.clone();
        let start = match self.navigation.change_walk {
            Some(index) if changes.get(index) == current.as_ref() => index,
            _ => changes.len(),
        };
        let Some(index) = (0..start)
            .rev()
            .find(|&i| Some(&changes[i]) != current.as_ref())
        else {
            return Err("No more edit locations".into());
        };
        self.show_place(&changes[index])?;
        self.navigation.change_walk = Some(index);
        Ok(())
    }

    /// Opens `place` and puts the caret there, without touching the history.
    fn show_place(&mut self, place: &Place) -> Result<(), String> {
        self.open_file(&place.path)?;
        let index = self.active_document.ok_or("no active document")?;
        let document = &mut self.documents[index];
        let offset = place.offset.min(document.text.len_chars());
        document.selections.set_single(Selection::caret(offset));
        Ok(())
    }

    /// Jumps to a navigation item, recording the current place for Back.
    pub fn navigate_to(&mut self, item: &NavigationItem) -> Result<(), String> {
        if item.external {
            return Err(format!(
                "{} is outside the project; opening library sources is not supported yet",
                item.path.display()
            ));
        }
        let origin = self.current_place();
        self.open_file(&item.path)?;
        let index = self.active_document.ok_or("no active document")?;
        let document = &mut self.documents[index];
        let offset = document
            .text
            .line_to_char(item.line.min(document.text.len_lines().saturating_sub(1)))
            .map(|start| start + item.column)
            .unwrap_or(0)
            .min(document.text.len_chars());
        let target = Place {
            path: item.path.clone(),
            offset,
        };
        document.selections.set_single(Selection::caret(offset));
        if let Some(origin) = origin
            && origin != target
        {
            self.remember(origin);
            self.navigation.forward.clear();
        }
        self.navigation.choices = None;
        self.status.clear();
        Ok(())
    }

    /// Back (`forward == false`) or Forward in the navigation history.
    pub fn navigate_history(&mut self, forward: bool) -> Result<Place, String> {
        let current = self.current_place();
        let (from, to) = if forward {
            (&mut self.navigation.forward, &mut self.navigation.back)
        } else {
            (&mut self.navigation.back, &mut self.navigation.forward)
        };
        // Skip entries that equal the current place (e.g. after a manual jump back).
        let place = loop {
            let place = from.pop().ok_or(if forward {
                "Nothing to go forward to"
            } else {
                "Nothing to go back to"
            })?;
            if Some(&place) != current.as_ref() {
                break place;
            }
        };
        if let Some(current) = current {
            to.push(current);
        }
        if let Err(error) = self.show_place(&place) {
            // The file is gone: drop the entry rather than getting stuck on it.
            return Err(format!("{}: {error}", place.path.display()));
        }
        Ok(place)
    }

    /// Sends a navigation request for the caret of the active document.
    pub fn request_navigation(&mut self, kind: NavigationKind) -> Result<u64, String> {
        let origin = self.current_place().ok_or("no active document")?;
        let index = self.active_document.ok_or("no active document")?;
        let document = &self.documents[index];
        let (line, character) = document
            .text
            .lsp_position(origin.offset)
            .map_err(|error| error.to_string())?;
        let (start, end) = ops::word_at(&document.text, origin.offset);
        let symbol = document.text.slice(start, end).unwrap_or_default();
        let absolute = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .join(&document.path);
        let extra = if kind.feature() == Feature::References {
            json!({"context": {"includeDeclaration": kind == NavigationKind::ShowUsages}})
        } else {
            json!({})
        };
        if let Some(pending) = self.navigation.pending.take() {
            self.lsp.cancel_request(&pending.handle);
        }
        let handle = self
            .lsp
            .request(&absolute, kind.feature(), line, character, extra)
            .map_err(|error| error.to_string())?;
        let id = handle.id;
        self.navigation.pending = Some(Pending {
            kind,
            handle,
            symbol,
            origin,
        });
        Ok(id)
    }

    /// Applies a navigation answer that arrived since the last poll.
    pub fn poll_navigation(&mut self) {
        let Some(pending) = &self.navigation.pending else {
            return;
        };
        let Some(result) = self.lsp.take_response(&pending.handle) else {
            return;
        };
        let Some(pending) = self.navigation.pending.take() else {
            return;
        };
        match result {
            Ok(value) => {
                let locations = parse_locations(&value);
                self.apply_navigation(pending.kind, &pending.symbol, &pending.origin, &locations);
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    /// Turns server locations into items: project-relative paths, character
    /// columns, and each line's text.
    pub(crate) fn navigation_items(&self, locations: &[LspLocation]) -> Vec<NavigationItem> {
        let root = self.canonical_root();
        let mut files: HashMap<PathBuf, Option<Vec<String>>> = HashMap::new();
        locations
            .iter()
            .take(USAGE_LIMIT)
            .map(|location| {
                let relative = root
                    .as_deref()
                    .and_then(|root| location.path.strip_prefix(root).ok())
                    .map(Path::to_path_buf);
                let open = relative
                    .as_ref()
                    .and_then(|relative| self.documents.iter().find(|d| &d.path == relative));
                let line_text = match open {
                    Some(document) => document
                        .text
                        .line(location.start_line)
                        .unwrap_or_default()
                        .trim_end_matches(['\r', '\n'])
                        .to_owned(),
                    None => files
                        .entry(location.path.clone())
                        .or_insert_with(|| read_lines(&location.path))
                        .as_ref()
                        .and_then(|lines| lines.get(location.start_line).cloned())
                        .unwrap_or_default(),
                };
                let column = |utf16: usize| utf16_to_char(&line_text, utf16);
                let end_column = if location.end_line == location.start_line {
                    column(location.end_character)
                } else {
                    line_text.chars().count()
                };
                NavigationItem {
                    external: relative.is_none(),
                    path: relative.unwrap_or_else(|| location.path.clone()),
                    line: location.start_line,
                    column: column(location.start_character),
                    end_column,
                    line_text,
                }
            })
            .collect()
    }

    pub(crate) fn apply_navigation(
        &mut self,
        kind: NavigationKind,
        symbol: &str,
        origin: &Place,
        locations: &[LspLocation],
    ) {
        let items = self.navigation_items(locations);
        let truncated = locations.len() > items.len();
        if items.is_empty() {
            self.status = kind.not_found().into();
            return;
        }
        let list = |title: String| NavigationList {
            kind,
            title,
            items: items.clone(),
            truncated,
        };
        match kind {
            NavigationKind::FindUsages => {
                self.navigation.usages = Some(list(format!("Usages of {symbol}")));
                self.navigation.usages_selected = true;
                self.tool_windows.show(crate::ToolWindow::Find);
                self.status = format!("Found {} usages of {symbol}", items.len());
            }
            NavigationKind::ShowUsages => {
                self.navigation.choices = Some(list(format!("Usages of {symbol}")));
            }
            _ if items.len() == 1 => {
                let item = &items[0];
                // Go to Declaration on the declaration itself shows its usages,
                // like the reference IDE.
                let on_origin = kind == NavigationKind::Declaration
                    && item.path == origin.path
                    && self
                        .documents
                        .iter()
                        .find(|document| document.path == origin.path)
                        .and_then(|document| document.text.line_to_char(item.line))
                        .is_some_and(|start| {
                            (start + item.column..=start + item.end_column).contains(&origin.offset)
                        });
                if on_origin {
                    if let Err(error) = self.request_navigation(NavigationKind::ShowUsages) {
                        self.status = error;
                    }
                } else if let Err(error) = self.navigate_to(item) {
                    self.status = error;
                }
            }
            NavigationKind::Implementation => {
                self.navigation.choices = Some(list(format!("Choose Implementation of {symbol}")));
            }
            _ => {
                self.navigation.choices = Some(list(format!("Choose Declaration of {symbol}")));
            }
        }
    }
}

fn read_lines(path: &Path) -> Option<Vec<String>> {
    if std::fs::metadata(path).ok()?.len() > PREVIEW_FILE_LIMIT {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    Some(text.lines().map(str::to_owned).collect())
}

fn utf16_to_char(line: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (index, c) in line.chars().enumerate() {
        if units >= utf16 {
            return index;
        }
        units += c.len_utf16();
    }
    line.chars().count()
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let lsp_ready = |state: &IdeState| state.active_has_language_server();
    for (id, label, shortcut, kind) in [
        (
            "navigate.declaration",
            "Go to Declaration or Usages",
            "⌘B",
            NavigationKind::Declaration,
        ),
        (
            "navigate.typeDeclaration",
            "Go to Type Declaration",
            "⇧⌘B",
            NavigationKind::TypeDeclaration,
        ),
        (
            "navigate.implementation",
            "Go to Implementation(s)",
            "⌥⌘B",
            NavigationKind::Implementation,
        ),
        (
            "navigate.findUsages",
            "Find Usages",
            "⌥F7",
            NavigationKind::FindUsages,
        ),
        (
            "navigate.showUsages",
            "Show Usages",
            "⌥⌘F7",
            NavigationKind::ShowUsages,
        ),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Navigate", Some(shortcut), "{}"),
                lsp_ready,
                move |state: &mut IdeState, _| {
                    let id = state
                        .request_navigation(kind)
                        .map_err(CommandError::Failed)?;
                    Ok(json!({"requestId": id}))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "navigate.declarationAt",
                "Go to Declaration at Position",
                "Navigate",
                None,
                r#"{"line":0,"column":0}"#,
            ),
            lsp_ready,
            |state: &mut IdeState, params| {
                let line = usize_param(&params, "line")?;
                let column = usize_param(&params, "column")?;
                let index = state
                    .active_document
                    .ok_or_else(|| CommandError::Failed("no active document".into()))?;
                let document = &mut state.documents[index];
                let start = document
                    .text
                    .line_to_char(line)
                    .ok_or_else(|| CommandError::InvalidParams(format!("no line {line}")))?;
                let offset = (start + column).min(document.text.len_chars());
                document.selections.set_single(Selection::caret(offset));
                let id = state
                    .request_navigation(NavigationKind::Declaration)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"requestId": id}))
            },
        ))
        .expect("unique built-in id");
    for (id, label, shortcut, forward) in [
        ("navigate.back", "Back", "⌘[", false),
        ("navigate.forward", "Forward", "⌘]", true),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Navigate", Some(shortcut), "{}"),
                move |state: &IdeState| {
                    if forward {
                        state.navigation.can_go_forward()
                    } else {
                        state.navigation.can_go_back()
                    }
                },
                move |state: &mut IdeState, _| {
                    let place = state
                        .navigate_history(forward)
                        .map_err(CommandError::Failed)?;
                    Ok(json!(place))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "navigate.lastEdit",
                "Last Edit Location",
                "Navigate",
                Some("⇧⌘⌫"),
                "{}",
            ),
            |state: &IdeState| !state.navigation.changes.is_empty(),
            |state: &mut IdeState, _| {
                state.jump_to_last_change().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "navigate.goto",
                "Go to Location",
                "Navigate",
                None,
                r#"{"path":"string","line":0,"column":0}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let item = NavigationItem {
                    path: PathBuf::from(string_param(&params, "path")?),
                    external: false,
                    line: usize_param(&params, "line")?,
                    column: usize_param(&params, "column")?,
                    end_column: 0,
                    line_text: String::new(),
                };
                state.navigate_to(&item).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "navigate.results",
                "Navigation Results",
                "Navigate",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                Ok(json!({
                    "pending": state.navigation.is_pending(),
                    "choices": state.navigation.choices,
                    "usages": state.navigation.usages,
                    "canGoBack": state.navigation.can_go_back(),
                    "canGoForward": state.navigation.can_go_forward(),
                }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn project(
        name: &str,
    ) -> (
        IdeState,
        archipel_ide_command::CommandRegistry<IdeState>,
        PathBuf,
    ) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-nav-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.txt"), "fn greet() {}\ngreet();\n").unwrap();
        std::fs::write(root.join("src/b.txt"), "// 𝔸\nlet x = greet();\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "src/a.txt"}))
            .unwrap();
        let root = root.canonicalize().unwrap();
        (state, commands, root)
    }

    fn location(path: PathBuf, line: usize, start: usize, end: usize) -> LspLocation {
        LspLocation {
            path,
            start_line: line,
            start_character: start,
            end_line: line,
            end_character: end,
        }
    }

    fn caret(state: &IdeState) -> Place {
        state.current_place().unwrap()
    }

    #[test]
    fn one_target_jumps_and_back_forward_walk_the_history() {
        let (mut state, commands, root) = project("jump");
        let origin = caret(&state);
        // `greet` in b.txt line 1, after a surrogate pair on line 0.
        let target = location(root.join("src/b.txt"), 1, 8, 13);
        state.apply_navigation(NavigationKind::Declaration, "greet", &origin, &[target]);
        let place = caret(&state);
        assert_eq!(place.path, PathBuf::from("src/b.txt"));
        // Line 0 is "// 𝔸\n" (5 chars, 6 UTF-16 units); column 8 of line 1.
        assert_eq!(place.offset, 5 + 8);
        assert!(state.navigation.can_go_back());
        commands
            .execute("navigate.back", &mut state, json!({}))
            .unwrap();
        assert_eq!(caret(&state), origin);
        commands
            .execute("navigate.forward", &mut state, json!({}))
            .unwrap();
        assert_eq!(caret(&state), place);
        assert!(!commands.is_enabled("navigate.forward", &state));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn several_targets_become_choices_and_usages_fill_the_find_window() {
        let (mut state, commands, root) = project("choices");
        let origin = caret(&state);
        let locations = [
            location(root.join("src/b.txt"), 1, 8, 13),
            location(root.join("src/a.txt"), 1, 0, 5),
            location(PathBuf::from("/usr/lib/rust/lib.rs"), 0, 0, 1),
        ];
        state.apply_navigation(NavigationKind::Implementation, "greet", &origin, &locations);
        let choices = state.navigation.choices.clone().unwrap();
        assert_eq!(choices.title, "Choose Implementation of greet");
        assert_eq!(
            choices.items[0].line_text, "let x = greet();",
            "read from disk"
        );
        assert_eq!(choices.items[0].column, 8);
        assert_eq!(
            choices.items[1].line_text, "greet();",
            "read from the open document"
        );
        assert!(choices.items[2].external);
        assert!(state.navigate_to(&choices.items[2]).is_err());
        state.apply_navigation(
            NavigationKind::FindUsages,
            "greet",
            &origin,
            &locations[..2],
        );
        let usages = state.navigation.usages.as_ref().unwrap();
        assert_eq!(usages.title, "Usages of greet");
        assert_eq!(usages.items.len(), 2);
        assert_eq!(
            state.tool_windows.visible(crate::Anchor::Bottom),
            Some(crate::ToolWindow::Find)
        );
        state.apply_navigation(NavigationKind::Declaration, "nothing", &origin, &[]);
        assert_eq!(state.status, "Cannot find declaration to go to");
        let results = commands
            .execute("navigate.results", &mut state, json!({}))
            .unwrap();
        assert_eq!(results["usages"]["items"][1]["path"], "src/a.txt");
        commands
            .execute(
                "navigate.goto",
                &mut state,
                json!({"path": "src/b.txt", "line": 1, "column": 4}),
            )
            .unwrap();
        assert_eq!(caret(&state).offset, 5 + 4);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn navigation_commands_explain_a_missing_server() {
        let (mut state, commands, root) = project("noserver");
        // `.txt` has no language server: the commands are disabled.
        assert!(!commands.is_enabled("navigate.declaration", &state));
        assert!(
            state
                .request_navigation(NavigationKind::Declaration)
                .unwrap_err()
                .contains("no language server")
        );
        assert!(
            commands
                .execute("navigate.back", &mut state, json!({}))
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
