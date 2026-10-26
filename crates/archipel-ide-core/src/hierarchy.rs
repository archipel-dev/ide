//! Type Hierarchy (Ctrl+H), Call Hierarchy (Ctrl+Alt+H) and Go to Super
//! Method (Ctrl+U), from `textDocument/prepareTypeHierarchy` /
//! `typeHierarchy/*` and `textDocument/prepareCallHierarchy` / `callHierarchy/*`.
//!
//! The hierarchies fill the Hierarchy tool window with a lazily expanded tree:
//! a node's children are requested when it is first expanded. The direction
//! button switches between supertypes / subtypes and callers / callees.
//!
//! Go to Super Method walks the supertypes breadth-first (the server answers
//! direct supertypes only) and jumps to the first one that declares a method
//! with the name under the caret; on a type name it jumps to the superclass.
use crate::navigation::{NavigationItem, Place};
use crate::{IdeState, ToolWindow, spec, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_lsp::{Feature, RequestHandle, parse_locations};
use archipel_ide_text::ops;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::VecDeque;

/// Supertype levels Go to Super Method follows.
const SUPER_DEPTH: usize = 8;
/// Children shown per node (huge hierarchies stay usable).
const CHILD_LIMIT: usize = 500;
/// Files larger than this are not scanned for a super method.
const SCAN_LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HierarchyKind {
    Type,
    Call,
}

/// Which way the tree grows from the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HierarchyDirection {
    /// Supertypes / callers.
    Up,
    /// Subtypes / callees.
    Down,
}

impl HierarchyKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Type => "Type Hierarchy",
            Self::Call => "Call Hierarchy",
        }
    }

    pub fn direction_label(self, direction: HierarchyDirection) -> &'static str {
        match (self, direction) {
            (Self::Type, HierarchyDirection::Up) => "Supertypes",
            (Self::Type, HierarchyDirection::Down) => "Subtypes",
            (Self::Call, HierarchyDirection::Up) => "Callers",
            (Self::Call, HierarchyDirection::Down) => "Callees",
        }
    }

    fn prepare(self) -> Feature {
        match self {
            Self::Type => Feature::PrepareTypeHierarchy,
            Self::Call => Feature::PrepareCallHierarchy,
        }
    }

    fn children(self, direction: HierarchyDirection) -> Feature {
        match (self, direction) {
            (Self::Type, HierarchyDirection::Up) => Feature::TypeHierarchySupertypes,
            (Self::Type, HierarchyDirection::Down) => Feature::TypeHierarchySubtypes,
            (Self::Call, HierarchyDirection::Up) => Feature::CallHierarchyIncoming,
            (Self::Call, HierarchyDirection::Down) => Feature::CallHierarchyOutgoing,
        }
    }
}

/// One row of the hierarchy tree.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchyNode {
    pub label: String,
    /// Package, file or signature detail the server reports.
    pub detail: Option<String>,
    /// LSP `SymbolKind`.
    pub symbol_kind: u64,
    pub target: Option<NavigationItem>,
    pub parent: Option<usize>,
    pub depth: usize,
    pub children: Vec<usize>,
    pub expanded: bool,
    pub loaded: bool,
    pub loading: bool,
    #[serde(skip)]
    raw: Value,
}

/// The hierarchy shown in the tool window.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hierarchy {
    pub kind: HierarchyKind,
    pub direction: HierarchyDirection,
    pub nodes: Vec<HierarchyNode>,
    pub selected: usize,
    /// Bumped when the tree is rebuilt; late answers for old trees are dropped.
    generation: u64,
}

/// A visible row of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HierarchyRow {
    pub node: usize,
    pub depth: usize,
}

impl Hierarchy {
    /// The rows in display order (expanded nodes show their children).
    pub fn rows(&self) -> Vec<HierarchyRow> {
        let mut rows = Vec::new();
        let mut stack = vec![0usize];
        while let Some(node) = stack.pop() {
            let Some(entry) = self.nodes.get(node) else {
                continue;
            };
            rows.push(HierarchyRow {
                node,
                depth: entry.depth,
            });
            if entry.expanded {
                stack.extend(entry.children.iter().rev());
            }
        }
        rows
    }

    pub fn title(&self) -> String {
        let root = self.nodes.first().map_or("", |node| node.label.as_str());
        format!("{} of {root}", self.kind.direction_label(self.direction))
    }
}

#[derive(Debug)]
enum Job {
    Prepare { kind: HierarchyKind },
    Children { node: usize, generation: u64 },
}

#[derive(Debug)]
struct Pending {
    handle: RequestHandle,
    job: Job,
}

/// Breadth-first search for a super method.
#[derive(Debug)]
struct SuperSearch {
    word: String,
    handle: RequestHandle,
    /// Waiting for the prepare answer (`depth == 0`) or a supertypes answer.
    depth: usize,
    queue: VecDeque<(Value, usize)>,
}

#[derive(Debug, Default)]
pub struct HierarchyState {
    pub current: Option<Hierarchy>,
    pending: Vec<Pending>,
    generation: u64,
    super_search: Option<SuperSearch>,
}

/// Parses a type item or call-hierarchy item into a node.
fn node_from(raw: &Value, parent: Option<usize>, depth: usize) -> HierarchyNode {
    let text = |key: &str| raw.get(key).and_then(Value::as_str).map(str::to_owned);
    HierarchyNode {
        label: text("name").unwrap_or_else(|| "?".into()),
        detail: text("detail").filter(|detail| !detail.is_empty()),
        symbol_kind: raw.get("kind").and_then(Value::as_u64).unwrap_or(0),
        target: None,
        parent,
        depth,
        children: Vec::new(),
        expanded: false,
        loaded: false,
        loading: false,
        raw: raw.clone(),
    }
}

/// The `Location` of an item's name.
fn item_location(raw: &Value) -> Value {
    json!({
        "uri": raw.get("uri").cloned().unwrap_or(Value::Null),
        "range": raw.get("selectionRange").or_else(|| raw.get("range")).cloned().unwrap_or(Value::Null),
    })
}

impl IdeState {
    fn hierarchy_target(&self, raw: &Value) -> Option<NavigationItem> {
        let locations = parse_locations(&item_location(raw));
        self.navigation_items(&locations).into_iter().next()
    }

    fn caret_request(
        &mut self,
        feature: Feature,
    ) -> Result<(RequestHandle, String, Place), String> {
        let origin = self.current_place().ok_or("no active document")?;
        let document = &self.documents[self.active_document.ok_or("no active document")?];
        let (line, character) = document
            .text
            .lsp_position(origin.offset)
            .map_err(|error| error.to_string())?;
        let (start, end) = ops::word_at(&document.text, origin.offset);
        let word = document.text.slice(start, end).unwrap_or_default();
        let absolute = self.active_absolute_path().ok_or("no project is open")?;
        let handle = self
            .lsp
            .request(&absolute, feature, line, character, json!({}))
            .map_err(|error| error.to_string())?;
        Ok((handle, word, origin))
    }

    /// Ctrl+H / Ctrl+Alt+H: opens the hierarchy of the symbol at the caret.
    pub fn request_hierarchy(&mut self, kind: HierarchyKind) -> Result<(), String> {
        let (handle, _, _) = self.caret_request(kind.prepare())?;
        self.hierarchy.pending.push(Pending {
            handle,
            job: Job::Prepare { kind },
        });
        Ok(())
    }

    /// Expands or collapses a node, requesting its children first.
    pub fn toggle_hierarchy_node(&mut self, node: usize) -> Result<(), String> {
        let hierarchy = self
            .hierarchy
            .current
            .as_mut()
            .ok_or("no hierarchy is open")?;
        let entry = hierarchy
            .nodes
            .get_mut(node)
            .ok_or("no such hierarchy node")?;
        if entry.loaded {
            entry.expanded = !entry.expanded;
            hierarchy.selected = node;
            return Ok(());
        }
        self.load_hierarchy_children(node)
    }

    fn load_hierarchy_children(&mut self, node: usize) -> Result<(), String> {
        let hierarchy = self
            .hierarchy
            .current
            .as_mut()
            .ok_or("no hierarchy is open")?;
        let feature = hierarchy.kind.children(hierarchy.direction);
        let entry = hierarchy
            .nodes
            .get_mut(node)
            .ok_or("no such hierarchy node")?;
        if entry.loading {
            return Ok(());
        }
        let raw = entry.raw.clone();
        entry.loading = true;
        hierarchy.selected = node;
        let generation = hierarchy.generation;
        let absolute = self.hierarchy_server_path()?;
        let result = self
            .lsp
            .request_with_params(&absolute, feature, json!({"item": raw}));
        match result {
            Ok(handle) => {
                self.hierarchy.pending.push(Pending {
                    handle,
                    job: Job::Children { node, generation },
                });
                Ok(())
            }
            Err(error) => {
                if let Some(entry) = self
                    .hierarchy
                    .current
                    .as_mut()
                    .and_then(|hierarchy| hierarchy.nodes.get_mut(node))
                {
                    entry.loading = false;
                }
                Err(error.to_string())
            }
        }
    }

    /// A file served by the server that answered the hierarchy: the item's
    /// own file, else the active document.
    fn hierarchy_server_path(&self) -> Result<std::path::PathBuf, String> {
        let root = self
            .hierarchy
            .current
            .as_ref()
            .and_then(|hierarchy| hierarchy.nodes.first())
            .and_then(|node| node.target.as_ref())
            .filter(|target| !target.external)
            .and_then(|target| {
                let root = self.project.as_ref()?.root();
                Some(root.join(&target.path))
            });
        root.or_else(|| self.active_absolute_path())
            .ok_or_else(|| "no project is open".into())
    }

    /// Switches supertypes ↔ subtypes or callers ↔ callees and reloads.
    pub fn toggle_hierarchy_direction(&mut self) -> Result<(), String> {
        let hierarchy = self
            .hierarchy
            .current
            .as_mut()
            .ok_or("no hierarchy is open")?;
        hierarchy.direction = match hierarchy.direction {
            HierarchyDirection::Up => HierarchyDirection::Down,
            HierarchyDirection::Down => HierarchyDirection::Up,
        };
        self.hierarchy.generation += 1;
        hierarchy.generation = self.hierarchy.generation;
        hierarchy.nodes.truncate(1);
        hierarchy.selected = 0;
        if let Some(root) = hierarchy.nodes.first_mut() {
            root.children.clear();
            root.loaded = false;
            root.loading = false;
            root.expanded = true;
        }
        self.load_hierarchy_children(0)
    }

    /// Jumps to the node's declaration (double-click / Enter).
    pub fn open_hierarchy_node(&mut self, node: usize) -> Result<(), String> {
        let target = self
            .hierarchy
            .current
            .as_mut()
            .and_then(|hierarchy| {
                hierarchy.selected = node;
                hierarchy.nodes.get(node)
            })
            .and_then(|entry| entry.target.clone())
            .ok_or("This element has no source to open")?;
        self.navigate_to(&target)
    }

    /// Highlights a row without expanding it.
    pub fn hierarchy_select(&mut self, node: usize) {
        if let Some(hierarchy) = self.hierarchy.current.as_mut()
            && node < hierarchy.nodes.len()
        {
            hierarchy.selected = node;
        }
    }

    pub fn close_hierarchy(&mut self) {
        self.hierarchy.current = None;
        self.hierarchy.pending.clear();
        self.tool_windows.hide(ToolWindow::Hierarchy);
    }

    /// Ctrl+U: Go to Super Method (Super Class on a type name).
    pub fn go_to_super_method(&mut self) -> Result<(), String> {
        let (handle, word, _) = self.caret_request(Feature::PrepareTypeHierarchy)?;
        if word.is_empty() {
            return Err("Place the caret on a method or class name".into());
        }
        if let Some(previous) = self.hierarchy.super_search.take() {
            self.lsp.cancel_request(&previous.handle);
        }
        self.hierarchy.super_search = Some(SuperSearch {
            word,
            handle,
            depth: 0,
            queue: VecDeque::new(),
        });
        Ok(())
    }

    /// Finishes hierarchy and super-method requests whose answers arrived.
    pub fn poll_hierarchy(&mut self) {
        let pending = std::mem::take(&mut self.hierarchy.pending);
        for entry in pending {
            match self.lsp.take_response(&entry.handle) {
                None => self.hierarchy.pending.push(entry),
                Some(result) => {
                    let result = result.map_err(|error| error.to_string());
                    self.finish_hierarchy(entry.job, result);
                }
            }
        }
        if let Some(search) = self.hierarchy.super_search.take() {
            match self.lsp.take_response(&search.handle) {
                None => self.hierarchy.super_search = Some(search),
                Some(result) => {
                    let result = result.map_err(|error| error.to_string());
                    self.advance_super_search(search, result);
                }
            }
        }
    }

    fn finish_hierarchy(&mut self, job: Job, result: Result<Value, String>) {
        match (job, result) {
            (Job::Prepare { kind }, Ok(value)) => {
                let Some(raw) = value.as_array().and_then(|items| items.first()) else {
                    self.status = format!("{} is not available for this element", kind.title());
                    return;
                };
                let mut root = node_from(raw, None, 0);
                root.target = self.hierarchy_target(raw);
                root.expanded = true;
                self.hierarchy.generation += 1;
                self.hierarchy.current = Some(Hierarchy {
                    kind,
                    direction: match kind {
                        HierarchyKind::Type => HierarchyDirection::Down,
                        HierarchyKind::Call => HierarchyDirection::Up,
                    },
                    nodes: vec![root],
                    selected: 0,
                    generation: self.hierarchy.generation,
                });
                self.tool_windows.show(ToolWindow::Hierarchy);
                if let Err(error) = self.load_hierarchy_children(0) {
                    self.status = error;
                }
            }
            (Job::Children { node, generation }, Ok(value)) => {
                self.attach_children(node, generation, &value);
            }
            (Job::Prepare { .. }, Err(error)) => self.status = error,
            (Job::Children { node, generation }, Err(error)) => {
                if let Some(entry) = self
                    .hierarchy
                    .current
                    .as_mut()
                    .filter(|hierarchy| hierarchy.generation == generation)
                    .and_then(|hierarchy| hierarchy.nodes.get_mut(node))
                {
                    entry.loading = false;
                }
                self.status = error;
            }
        }
    }

    fn attach_children(&mut self, node: usize, generation: u64, value: &Value) {
        let kind_direction = match self.hierarchy.current.as_ref() {
            Some(hierarchy) if hierarchy.generation == generation => {
                (hierarchy.kind, hierarchy.direction)
            }
            _ => return,
        };
        // Call hierarchy answers wrap the item: `from` for callers, `to` for callees.
        let field = match kind_direction {
            (HierarchyKind::Call, HierarchyDirection::Up) => Some("from"),
            (HierarchyKind::Call, HierarchyDirection::Down) => Some("to"),
            _ => None,
        };
        let items: Vec<Value> = value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| match field {
                Some(field) => entry.get(field).cloned(),
                None => Some(entry.clone()),
            })
            .take(CHILD_LIMIT)
            .collect();
        let targets: Vec<Option<NavigationItem>> =
            items.iter().map(|raw| self.hierarchy_target(raw)).collect();
        let Some(hierarchy) = self.hierarchy.current.as_mut() else {
            return;
        };
        let depth = hierarchy.nodes.get(node).map_or(0, |entry| entry.depth) + 1;
        let mut children = Vec::new();
        for (raw, target) in items.iter().zip(targets) {
            let mut child = node_from(raw, Some(node), depth);
            child.target = target;
            children.push(hierarchy.nodes.len());
            hierarchy.nodes.push(child);
        }
        if let Some(entry) = hierarchy.nodes.get_mut(node) {
            entry.children = children;
            entry.loaded = true;
            entry.loading = false;
            entry.expanded = true;
        }
    }

    fn advance_super_search(&mut self, mut search: SuperSearch, result: Result<Value, String>) {
        let value = match result {
            Ok(value) => value,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        let items: Vec<Value> = value.as_array().cloned().unwrap_or_default();
        if search.depth == 0 {
            // The prepare answer: the type at the caret.
            let Some(item) = items.into_iter().next() else {
                self.status = "No super method found".into();
                return;
            };
            let on_type_name = item.get("name").and_then(Value::as_str) == Some(&search.word);
            search.queue.push_back((item, 0));
            if on_type_name {
                search.depth = usize::MAX; // marks "go to superclass"
            } else {
                search.depth = 1;
            }
            return self.request_next_supertypes(search);
        }
        let level = search.queue.front().map_or(0, |(_, level)| *level) + 1;
        search.queue.pop_front();
        if search.depth == usize::MAX {
            // Super Class: the first supertype (the superclass comes first).
            let Some(raw) = items.first() else {
                self.status = "No superclass found".into();
                return;
            };
            self.jump_to_item(raw, None);
            return;
        }
        for raw in &items {
            if let Some(line) = self.declaring_line(raw, &search.word) {
                self.jump_to_item(raw, Some((&search.word, line)));
                return;
            }
        }
        if level < SUPER_DEPTH {
            for raw in items {
                search.queue.push_back((raw, level));
            }
        }
        self.request_next_supertypes(search);
    }

    fn request_next_supertypes(&mut self, mut search: SuperSearch) {
        let Some((raw, _)) = search.queue.front() else {
            self.status = "No super method found".into();
            return;
        };
        let absolute = match self.hierarchy_search_path(raw) {
            Ok(path) => path,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        match self.lsp.request_with_params(
            &absolute,
            Feature::TypeHierarchySupertypes,
            json!({"item": raw}),
        ) {
            Ok(handle) => {
                search.handle = handle;
                self.hierarchy.super_search = Some(search);
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn hierarchy_search_path(&self, raw: &Value) -> Result<std::path::PathBuf, String> {
        // The active document is served by the right server in nearly every
        // case; the item's own file is the better choice for library types.
        let own = parse_locations(&item_location(raw))
            .into_iter()
            .next()
            .map(|location| location.path)
            .filter(|path| self.lsp.handles(path));
        own.or_else(|| self.active_absolute_path())
            .ok_or_else(|| "no project is open".into())
    }

    /// The line of the declaration of `method` inside the type `raw`.
    fn declaring_line(&self, raw: &Value, method: &str) -> Option<usize> {
        let location = parse_locations(&item_location(raw)).into_iter().next()?;
        let open = self
            .project
            .as_ref()
            .and_then(|project| location.path.strip_prefix(project.root()).ok())
            .and_then(|relative| self.documents.iter().find(|d| d.path == relative));
        let text = match open {
            Some(document) => document.text.text(),
            None => {
                if std::fs::metadata(&location.path).ok()?.len() > SCAN_LIMIT {
                    return None;
                }
                std::fs::read_to_string(&location.path).ok()?
            }
        };
        let range = raw.get("range")?;
        let line_of = |key: &str| range.get(key)?.get("line")?.as_u64().map(|l| l as usize);
        let (first, last) = (line_of("start")?, line_of("end")?);
        text.lines()
            .enumerate()
            .skip(first)
            .take(last.saturating_sub(first) + 1)
            .find(|(_, line)| declares_method(line, method))
            .map(|(index, _)| index)
    }

    fn jump_to_item(&mut self, raw: &Value, method: Option<(&str, usize)>) {
        let Some(mut target) = self.hierarchy_target(raw) else {
            self.status = "The target has no source to open".into();
            return;
        };
        if let Some((word, line)) = method {
            let (text, column) = self.line_and_column(&target, word, line);
            target.line = line;
            target.column = column;
            target.line_text = text;
        }
        if let Err(error) = self.navigate_to(&target) {
            self.status = error;
        }
    }

    /// The text of `line` in the item's file and the column of the method name.
    fn line_and_column(&self, target: &NavigationItem, word: &str, line: usize) -> (String, usize) {
        let text = self
            .project
            .as_ref()
            .and_then(|project| std::fs::read_to_string(project.root().join(&target.path)).ok())
            .or_else(|| {
                self.documents
                    .iter()
                    .find(|d| d.path == target.path)
                    .map(|d| d.text.text())
            })
            .and_then(|text| text.lines().nth(line).map(str::to_owned))
            .unwrap_or_default();
        let column = text
            .find(&format!("{word}("))
            .or_else(|| text.find(word))
            .map_or(0, |byte| text[..byte].chars().count());
        (text, column)
    }
}

/// Whether `line` looks like the declaration of `method`: the name followed
/// by `(` and preceded by a type, not a call, `new` or `return`.
fn declares_method(line: &str, method: &str) -> bool {
    let needle = format!("{method}(");
    let mut from = 0;
    while let Some(found) = line[from..].find(&needle) {
        let at = from + found;
        from = at + needle.len();
        let before = line[..at].trim_end();
        let boundary = line[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.' || c == '$'));
        let typed = before
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '>' | ']' | '?'));
        let keyword = before
            .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .is_some_and(|word| matches!(word, "return" | "new" | "throw" | "else" | "case"));
        let kotlin = before.ends_with("fun") || before.ends_with(' ') && before.contains("fun ");
        if boundary && ((typed && !keyword) || kotlin) {
            return true;
        }
    }
    false
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let type_ready = |state: &IdeState| {
        state
            .active_absolute_path()
            .is_some_and(|path| state.lsp.supports(&path, Feature::PrepareTypeHierarchy))
    };
    let call_ready = |state: &IdeState| {
        state
            .active_absolute_path()
            .is_some_and(|path| state.lsp.supports(&path, Feature::PrepareCallHierarchy))
    };
    registry
        .register(Command::new(
            spec(
                "hierarchy.type",
                "Type Hierarchy",
                "Navigate",
                Some("⌃H"),
                "{}",
            ),
            type_ready,
            |state: &mut IdeState, _| {
                state
                    .request_hierarchy(HierarchyKind::Type)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "hierarchy.call",
                "Call Hierarchy",
                "Navigate",
                Some("⌃⌥H"),
                "{}",
            ),
            call_ready,
            |state: &mut IdeState, _| {
                state
                    .request_hierarchy(HierarchyKind::Call)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "navigate.superMethod",
                "Go to Super Method",
                "Navigate",
                Some("⌘U"),
                "{}",
            ),
            type_ready,
            |state: &mut IdeState, _| {
                state.go_to_super_method().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "hierarchy.toggleNode",
                "Expand/Collapse Hierarchy Node",
                "Navigate",
                None,
                r#"{"node":0}"#,
            ),
            |state: &IdeState| state.hierarchy.current.is_some(),
            |state: &mut IdeState, params| {
                let node = usize_param(&params, "node")?;
                state
                    .toggle_hierarchy_node(node)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "hierarchy.openNode",
                "Open Hierarchy Node",
                "Navigate",
                None,
                r#"{"node":0}"#,
            ),
            |state: &IdeState| state.hierarchy.current.is_some(),
            |state: &mut IdeState, params| {
                let node = usize_param(&params, "node")?;
                state
                    .open_hierarchy_node(node)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "hierarchy.toggleDirection",
                "Switch Hierarchy Direction",
                "Navigate",
                None,
                "{}",
            ),
            |state: &IdeState| state.hierarchy.current.is_some(),
            |state: &mut IdeState, _| {
                state
                    .toggle_hierarchy_direction()
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("hierarchy.state", "Hierarchy State", "Navigate", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| Ok(json!({"hierarchy": state.hierarchy.current})),
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("hierarchy.close", "Close Hierarchy", "Navigate", None, "{}"),
            |state: &IdeState| state.hierarchy.current.is_some(),
            |state: &mut IdeState, _| {
                state.close_hierarchy();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_declarations_are_told_from_calls() {
        assert!(declares_method("    public void area() {", "area"));
        assert!(declares_method("    double area();", "area"));
        assert!(declares_method("    fun area(): Double", "area"));
        assert!(!declares_method("    return area();", "area"));
        assert!(!declares_method("    x.area();", "area"));
        assert!(!declares_method("    new area();", "area"));
        assert!(!declares_method("    void parea() {", "area"));
    }
}
