//! Drawing and mouse/keyboard routing of the NvChad layout.

use crate::app::{
    COMMIT, DASH, EDITOR, FIND, PROBLEMS, RUN, STATUS, TAB_BUTTONS, TABS, TERMINAL, TREE,
    TreeEntry, TuiApp,
};
use archipel_ide_actions::ActionPlace;
use archipel_ide_core::ProblemCounts;
use archipel_ide_core::project_view::{RowKind, TreeRow};
use archipel_kit::wm::{ToolWindow, ToolWindowAnchor};
use archipel_kit_tui::collections::{TreeEvent, TreeNode};
use archipel_kit_tui::components::{
    Diagnostics, GitStatus, Pane, PaneKind, RowIcon, RowIconKey, StatusBar, StatusSlot,
};
use archipel_kit_tui::dashboard::{Dashboard, DashboardButton};
use archipel_kit_tui::editor::{EditorComponent, cursor_position};
use archipel_kit_tui::input::Event;
use archipel_kit_tui::overlay::draw_which_key;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::widgets::Widget;
use archipel_kit_tui::runtime::Runtime;
use archipel_kit_tui::statusbar::{StatusBarControl, StatusEvent};
use archipel_kit_tui::tabs::TabsEvent;
use archipel_kit_tui::tooltip::Tooltip;
use archipel_kit_tui::workbench::{Workbench, WorkbenchFocus};
use archipel_kit_tui::{Glyph, Language, Mode};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// The tool windows around the editor.
pub fn workbench() -> Workbench {
    let window = |id: &str, title: &str, glyph: Glyph, anchor, number: Option<u8>| {
        let mut window = ToolWindow::new(id, title, glyph, anchor);
        window.activate_number = number;
        window
    };
    Workbench::new(vec![
        window(
            "project",
            "Project",
            Glyph::Project,
            ToolWindowAnchor::Left,
            Some(1),
        ),
        window(
            "commit",
            "Commit",
            Glyph::Commit,
            ToolWindowAnchor::Left,
            None,
        ),
        window(
            "find",
            "Find",
            Glyph::Search,
            ToolWindowAnchor::Bottom,
            Some(3),
        ),
        window("run", "Run", Glyph::Run, ToolWindowAnchor::Bottom, Some(4)),
        window(
            "problems",
            "Problems",
            Glyph::Problems,
            ToolWindowAnchor::Bottom,
            Some(6),
        ),
        window(
            "terminal",
            "Terminal",
            Glyph::Terminal,
            ToolWindowAnchor::Bottom,
            None,
        ),
    ])
}

/// The status line's control with its tooltips.
pub fn status_control() -> StatusBarControl {
    StatusBarControl::new(STATUS)
        .tooltip(
            StatusSlot::Git,
            Tooltip::new("Git Branch").description("Click to open the Commit window"),
        )
        .tooltip(StatusSlot::Cursor, Tooltip::new("Caret Position"))
        .tooltip(
            StatusSlot::Cwd,
            Tooltip::new("Toggle Project").shortcut("Alt+1"),
        )
        .tooltip(
            StatusSlot::Diagnostics,
            Tooltip::new("Problems").shortcut("Alt+6"),
        )
        .tooltip(
            StatusSlot::Run,
            Tooltip::new("Run Configuration")
                .description("Click to choose; Shift+F10 runs it")
                .shortcut("Alt+Shift+F10"),
        )
}

/// The dashboard shown while no file is open.
pub fn dashboard() -> Dashboard {
    Dashboard::new(
        DASH,
        vec![
            "  ▄▀█ █▀█ █▀▀ █ █ █ █▀▀ █   ".into(),
            "  █▀█ █▀▄ █▄▄ █▀█ █ █▀▀ █▄▄ ".into(),
            "".into(),
            "       the terminal IDE".into(),
        ],
        vec![
            DashboardButton::new(Glyph::Search, "Search Everywhere", "Shift Shift", 'e'),
            DashboardButton::new(Glyph::File, "Go to File", "Ctrl Shift N", 'f'),
            DashboardButton::new(Glyph::Project, "Project", "Alt 1", 'p'),
            DashboardButton::new(Glyph::FolderOpen, "Open Folder", "", 'o'),
            DashboardButton::new(Glyph::Close, "Quit", "Ctrl K Q", 'q'),
        ],
    )
}

/// The language logo of a file, a plain file icon otherwise.
pub fn file_icon_key(path: &Path) -> RowIconKey {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    RowIconKey::Language(match extension.as_str() {
        "rs" => Language::Rust,
        "java" => Language::Java,
        "kt" => Language::Kotlin,
        "kts" => Language::GradleKotlin,
        "gradle" => Language::Gradle,
        "groovy" => Language::Groovy,
        "scala" => Language::Scala,
        "py" => Language::Python,
        "rb" => Language::Ruby,
        "php" => Language::Php,
        "lua" => Language::Lua,
        "js" | "mjs" | "cjs" => Language::JavaScript,
        "ts" => Language::TypeScript,
        "tsx" | "jsx" => Language::Tsx,
        "go" => Language::Go,
        "c" | "h" => Language::C,
        "cc" | "cpp" | "hpp" | "cxx" => Language::Cpp,
        "cs" => Language::CSharp,
        "swift" => Language::Swift,
        "sh" | "bash" | "zsh" => Language::Shell,
        "sql" => Language::Sql,
        "html" | "htm" => Language::Html,
        "css" => Language::Css,
        "xml" => Language::Xml,
        "md" | "markdown" => Language::Markdown,
        "json" => Language::Json,
        "yaml" | "yml" => Language::Yaml,
        "toml" => Language::Toml,
        "ini" | "cfg" => Language::Ini,
        "properties" => Language::Properties,
        "csv" => Language::Csv,
        _ if name == "Makefile" => Language::Makefile,
        _ if name == "Dockerfile" => Language::Dockerfile,
        _ => return RowIconKey::Glyph(Glyph::File),
    })
}

/// [`file_icon_key`] as a row icon.
pub fn file_icon(path: &Path) -> RowIcon {
    file_icon_key(path).into()
}

impl TuiApp {
    /// Rebuilds the tree nodes when the core's rows changed.
    pub(crate) fn refresh_tree(&mut self) {
        let rows = self.ide.state_mut().cached_tree_rows();
        if self
            .tree_source
            .as_ref()
            .is_some_and(|known| Arc::ptr_eq(known, &rows))
        {
            return;
        }
        let root = self
            .ide
            .state()
            .project()
            .and_then(|project| project.root().file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".to_owned());
        let (nodes, map, expanded) = build_tree(&root, &rows);
        self.tree_nodes = nodes;
        self.tree_map = map;
        self.tree.expanded = expanded;
        self.tree_source = Some(rows);
    }

    /// Draws everything.
    pub(crate) fn draw_all(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let look = rt.look.clone();
        let p = look.palette;
        Pane::new(&look).kind(PaneKind::Editor).render(area, buf);
        if area.height < 4 || area.width < 20 {
            return;
        }
        let status_row = Rect::new(area.x, area.bottom() - 1, area.width, 1);
        let work = Rect::new(area.x, area.y, area.width, area.height - 1);
        let areas = self.wb.draw(rt, work, buf);
        self.sync_focus(rt);
        if let Some(body) = areas.tool("project") {
            self.tree
                .show(rt, TREE, body, &self.tree_nodes, p.darker_black, buf);
            self.ide.data.tree_selection = self
                .tree
                .selected_path()
                .and_then(|path| self.tree_map.get(path))
                .and_then(|entry| entry.path.clone())
                .filter(|path| !path.is_absolute());
        }
        if let Some(body) = areas.tool("commit") {
            self.draw_commit(rt, body, buf);
        }
        if let Some(body) = areas.tool("run") {
            self.draw_run(rt, body, buf);
        }
        if let Some(body) = areas.tool("problems") {
            self.draw_problems(rt, body, buf);
        }
        if let Some(body) = areas.tool("find") {
            self.draw_find(rt, body, buf);
        }
        if let Some(body) = areas.tool("terminal") {
            self.draw_terminal(rt, body, buf);
        }
        self.draw_editor_area(rt, areas.editor, buf);
        self.draw_status(rt, status_row, buf);
        self.layers.draw(rt, area, buf);
        draw_which_key(rt, area, buf, &|id| {
            archipel_ide_actions::Catalog::shared().label(id)
        });
    }

    fn draw_editor_area(&mut self, rt: &mut Runtime, editor: Rect, buf: &mut Buffer) {
        if editor.height < 2 || editor.width < 5 {
            return;
        }
        let look = rt.look.clone();
        let p = look.palette;
        let bar = Rect::new(editor.x, editor.y, editor.width, 1);
        let buttons_w = 7u16.min(bar.width);
        let tabs_area = Rect::new(bar.x, bar.y, bar.width.saturating_sub(buttons_w), 1);
        self.tab_control.show(
            rt,
            &mut self.tab_state,
            &self.tabs,
            tabs_area,
            p.darker_black,
            buf,
        );
        let bx = bar.right().saturating_sub(buttons_w);
        self.buttons_rect = Rect::new(bx, bar.y, buttons_w, 1);
        for (i, glyph, color, tip) in [
            (
                0u16,
                if p.dark { Glyph::Sun } else { Glyph::Moon },
                p.yellow,
                "Switch Theme",
            ),
            (1, Glyph::Close, p.red, "Close All Tabs"),
        ] {
            let r = Rect::new(bx + i * 3 + 1, bar.y, 3, 1).intersection(bar);
            archipel_kit_tui::testing::put_text(
                buf,
                bar,
                r.x,
                r.y,
                &look.glyph(glyph),
                color,
                p.darker_black,
            );
            rt.clickable(TAB_BUTTONS.with(u64::from(i)), r);
            rt.tooltip
                .offer(TAB_BUTTONS.with(u64::from(i)), r, Tooltip::new(tip));
        }
        let body = Rect::new(editor.x, editor.y + 1, editor.width, editor.height - 1);
        match self.active_view() {
            None => {
                self.dashboard.show(rt, &look, body, buf);
                if !self.ide.state().has_project() {
                    let text = "No project is open: press O to open a folder";
                    archipel_kit_tui::testing::put_text(
                        buf,
                        body,
                        body.x + body.width.saturating_sub(text.chars().count() as u16) / 2,
                        body.bottom().saturating_sub(2),
                        text,
                        p.grey_fg,
                        p.black,
                    );
                }
            }
            Some(index) => {
                let view = &mut self.views[index];
                let crate::view::DocView {
                    editor,
                    state,
                    highlighter,
                    language,
                    highlights,
                    ..
                } = view;
                let out = EditorComponent::new(EDITOR, editor)
                    .highlighter(highlighter)
                    .language(language)
                    .highlights(highlights)
                    .show(rt, state, body, buf);
                self.popups.caret_cell = out.caret_cell;
                self.popups.editor_area = body;
                self.draw_popups(rt, buf);
            }
        }
    }

    fn draw_status(&mut self, rt: &mut Runtime, row: Rect, buf: &mut Buffer) {
        let look = rt.look.clone();
        let p = look.palette;
        let state = self.ide.state();
        let view = self.active_view().and_then(|i| self.views.get(i));
        let (cursor, file) = match (view, state.active_file()) {
            (Some(view), Some(path)) => {
                let (line, column) = cursor_position(&view.editor);
                (
                    format!("Ln {line}, Col {column}"),
                    Some((
                        file_icon(path),
                        path.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    )),
                )
            }
            _ => ("Ln -, Col -".to_owned(), None),
        };
        let git = state
            .git_status()
            .map(|status| {
                use archipel_ide_git::ChangeKind;
                let count = |wanted: &[ChangeKind]| {
                    status
                        .changes
                        .iter()
                        .filter(|c| wanted.contains(&c.kind))
                        .count() as u32
                };
                GitStatus {
                    branch: status.branch.clone(),
                    added: count(&[ChangeKind::Added, ChangeKind::Untracked]),
                    changed: count(&[
                        ChangeKind::Modified,
                        ChangeKind::Renamed,
                        ChangeKind::Conflicted,
                    ]),
                    removed: count(&[ChangeKind::Deleted]),
                }
            })
            .unwrap_or_default();
        let counts = view
            .and_then(|view| view.problems())
            .map(|problems| ProblemCounts::of(problems.iter()))
            .unwrap_or_default();
        let index_text = {
            let status = state.index_status();
            (status.phase != archipel_ide_index::Phase::Idle || status.dumb)
                .then(|| status.summary())
                .flatten()
        };
        let toolchain_text = state
            .toolchains()
            .read()
            .progress()
            .map(|(text, fraction)| match fraction {
                Some(fraction) => format!("{text} {}%", (fraction * 100.0).round() as u32),
                None => text,
            });
        let busy_text = if self.panels.running {
            Some("Running…".to_owned())
        } else {
            toolchain_text.or(index_text)
        };
        let center = rt
            .hint
            .clone()
            .or_else(|| view.and_then(|v| v.state.hint(rt.now).map(str::to_owned)))
            .or(busy_text)
            .or_else(|| (!state.status().is_empty()).then(|| state.status().to_owned()))
            .unwrap_or_default();
        let project = state
            .project()
            .and_then(|project| project.root().file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let lsp = state
            .lsp()
            .servers()
            .first()
            .map(|server| server.language.clone())
            .unwrap_or_default();
        let config = state
            .run_widget_label()
            .unwrap_or_else(|| "no configuration".to_owned());
        let mode = self.mode(rt);
        let editor_mode = (self.wb.focused_tool().is_none() && self.layers.is_empty())
            .then(|| {
                view.map(|v| {
                    if v.editor.carets.has_selection() {
                        Mode::Visual
                    } else {
                        Mode::Insert
                    }
                })
            })
            .flatten();
        let mode = editor_mode.unwrap_or(mode);
        let mut bar = StatusBar::new(&look)
            .mode(mode.label(), mode.color(&p))
            .git(&git)
            .center(&center)
            .diagnostics(Diagnostics {
                errors: counts.errors as u32,
                warnings: counts.warnings as u32,
                hints: counts.weak_warnings as u32,
                info: 0,
            })
            .lsp(&lsp)
            .run(&config, self.panels.running)
            .cwd(&project)
            .cursor(&cursor);
        if let Some((icon, name)) = &file {
            bar = bar.file(*icon, name);
        }
        self.status.show(rt, bar, row, buf);
    }

    fn mode(&self, _rt: &Runtime) -> Mode {
        if !self.layers.is_empty() {
            return Mode::Command;
        }
        match &self.wb.focus {
            WorkbenchFocus::Editor => Mode::Normal,
            WorkbenchFocus::Tool(id) if id == "run" || id == "terminal" => Mode::Terminal,
            WorkbenchFocus::Tool(_) => Mode::Normal,
        }
    }

    /// Makes the widget of the focused workbench part the focused widget.
    fn sync_focus(&self, rt: &mut Runtime) {
        if !self.layers.is_empty() {
            return;
        }
        let id = match &self.wb.focus {
            WorkbenchFocus::Editor => EDITOR,
            WorkbenchFocus::Tool(tool) => match tool.as_str() {
                "project" => TREE,
                "commit" => COMMIT,
                "run" => RUN,
                "problems" => PROBLEMS,
                "find" => FIND,
                _ => TERMINAL,
            },
        };
        if !rt.focused(id) && !rt.focused(TABS) {
            rt.focus.focus(id);
        }
    }

    /// Routes an event the actions did not take to the widgets.
    pub(crate) fn route(&mut self, rt: &mut Runtime, event: &Event) {
        self.sync_focus(rt);
        match self.status.handle(rt, event) {
            StatusEvent::Clicked(StatusSlot::Cwd, _) => {
                self.command(rt, "toolwindow.toggle", json!({ "id": "project" }))
            }
            StatusEvent::Clicked(StatusSlot::Git, _) => {
                self.command(rt, "toolwindow.toggle", json!({ "id": "commit" }))
            }
            StatusEvent::Clicked(StatusSlot::Diagnostics, _) => {
                self.command(rt, "toolwindow.toggle", json!({ "id": "problems" }))
            }
            StatusEvent::Clicked(StatusSlot::Run, _) => {
                self.open_picker(rt, crate::picker::PickerKind::Configs, 0)
            }
            _ => {}
        }
        if let Event::Mouse(_) = event
            && let Some(pointer) = rt.pointer().copied()
        {
            if pointer.clicked(TAB_BUTTONS.with(0)) {
                rt.look = next_theme(&rt.look);
            } else if pointer.clicked(TAB_BUTTONS.with(1)) {
                self.command(rt, "editor.closeAll", Value::Null);
            }
        }
        let before = self.tab_state.selected;
        match self
            .tab_control
            .handle(rt, &mut self.tab_state, &mut self.tabs, event)
        {
            TabsEvent::CloseRequested(index) => {
                self.command(rt, "editor.close", json!({ "index": index }));
            }
            TabsEvent::Selected(index) => {
                self.command(rt, "editor.activate", json!({ "index": index }));
                self.focus_editor();
            }
            _ => {
                if self.tab_state.selected != before {
                    let index = self.tab_state.selected;
                    self.command(rt, "editor.activate", json!({ "index": index }));
                }
            }
        }
        if self.views.is_empty()
            && self.wb.focused_tool().is_none()
            && let Some(button) = self.dashboard.handle(rt, event)
        {
            match button {
                0 => self.open_picker(rt, crate::picker::PickerKind::SearchEverywhere, 0),
                1 => self.open_picker(rt, crate::picker::PickerKind::Files, 0),
                2 => {
                    self.wb.toggle("project");
                    self.sync_layout();
                }
                3 => self.run_action(rt, "OpenFile", ActionPlace::Keymap),
                _ => self.run_action(rt, "Exit", ActionPlace::Keymap),
            }
        }
        // The project tree.
        match self.tree.handle(rt, TREE, event) {
            TreeEvent::Activated(path) => self.tree_activated(rt, &path, true),
            TreeEvent::Toggled(path) => self.tree_activated(rt, &path, false),
            _ => {}
        }
        if let Event::Key(key) = event
            && key.code == archipel_kit_tui::input::KeyCode::Delete
            && rt.focused(TREE)
        {
            self.confirm_delete(rt);
        }
        self.route_panels(rt, event);
        // The editor.
        if let Some(index) = self.active_view() {
            let view = &mut self.views[index];
            let crate::view::DocView {
                editor,
                state,
                highlighter,
                language,
                highlights,
                ..
            } = view;
            let out = EditorComponent::new(EDITOR, editor)
                .highlighter(highlighter)
                .language(language)
                .highlights(highlights)
                .handle(rt, state, event);
            if let Some(hint) = out.hint {
                self.ide.state_mut().set_status(hint);
            }
        }
    }

    /// An activated (Enter, double click) or toggled node of the tree.
    fn tree_activated(&mut self, rt: &mut Runtime, path: &str, activated: bool) {
        let Some(entry) = self.tree_map.get(path).cloned() else {
            return;
        };
        match entry.kind {
            RowKind::File => {
                if let Some(file) = entry.path {
                    self.command(rt, "editor.openFile", json!({ "path": file }));
                    self.focus_editor();
                }
            }
            RowKind::Directory => {
                if let Some(dir) = entry.path {
                    // Only flip the core when the tree and the core disagree.
                    let open = self.ide.state().expanded_dirs().contains(&dir);
                    let tree_open = self.tree.expanded.contains(path);
                    if activated || open != tree_open {
                        self.command(rt, "project.toggleDirectory", json!({ "path": dir }));
                    }
                }
            }
            RowKind::Group => {
                if let Some(group) = entry.group
                    && (activated
                        || self
                            .ide
                            .state()
                            .project_view()
                            .expanded_groups
                            .contains(group)
                            != self.tree.expanded.contains(path))
                {
                    self.command(rt, "project.toggleGroup", json!({ "id": group }));
                }
            }
            RowKind::Jdk | RowKind::Library => {}
        }
    }

    /// Autoscroll of a drag that rests beyond the edge of the text.
    pub(crate) fn autoscroll(&mut self, rt: &mut Runtime) {
        if let Some(index) = self.active_view() {
            let view = &mut self.views[index];
            let crate::view::DocView { editor, state, .. } = view;
            let _ = EditorComponent::new(EDITOR, editor).tick(rt, state);
        }
    }
}

pub(crate) fn next_theme(look: &archipel_kit_tui::Look) -> archipel_kit_tui::Look {
    let all = archipel_kit_tui::Theme::all();
    let current = all
        .iter()
        .position(|theme| (theme.constructor)().name == look.theme.name)
        .unwrap_or(0);
    archipel_kit_tui::Look::new((all[(current + 1) % all.len()].constructor)())
}

/// The tree nodes for the core's rows: the project root with the rows nested by depth. A
/// directory the core lists as collapsed gets a placeholder child so the tree draws a
/// chevron; the tree's expanded set follows the core's.
fn build_tree(
    root: &str,
    rows: &[TreeRow],
) -> (
    Vec<TreeNode>,
    HashMap<String, TreeEntry>,
    std::collections::BTreeSet<String>,
) {
    let mut map = HashMap::new();
    let mut expanded = std::collections::BTreeSet::new();
    let root_key = ".".to_owned();
    expanded.insert(root_key.clone());
    map.insert(
        root_key.clone(),
        TreeEntry {
            path: None,
            kind: RowKind::Group,
            group: None,
        },
    );
    // (depth, node, tree path) stack of open ancestors.
    let mut stack: Vec<(usize, TreeNode, String)> = Vec::new();
    let mut finished: Vec<TreeNode> = Vec::new();
    fn close_to(stack: &mut Vec<(usize, TreeNode, String)>, depth: usize, out: &mut Vec<TreeNode>) {
        while let Some((d, _, _)) = stack.last() {
            if *d < depth {
                break;
            }
            let (_, node, _) = stack.pop().expect("checked");
            match stack.last_mut() {
                Some((_, parent, _)) => parent.children.push(node),
                None => out.push(node),
            }
        }
    }
    for row in rows {
        close_to(&mut stack, row.depth, &mut finished);
        let parent_path = stack
            .last()
            .map(|(_, _, path)| path.clone())
            .unwrap_or_else(|| root_key.clone());
        let siblings = match stack.last() {
            Some((_, parent, _)) => &parent.children,
            None => &finished,
        };
        let mut key = row.label.clone();
        let mut n = 2;
        while siblings.iter().any(|node| node.key == key) {
            key = format!("{} ({n})", row.label);
            n += 1;
        }
        let path = format!("{parent_path}/{key}");
        let icon = match row.kind {
            RowKind::Directory => RowIcon::Folder(row.expanded),
            RowKind::File => row
                .path
                .as_deref()
                .map(file_icon)
                .unwrap_or(RowIcon::Glyph(Glyph::File)),
            RowKind::Group => RowIcon::Folder(row.expanded),
            RowKind::Jdk | RowKind::Library => RowIcon::Glyph(Glyph::Folder),
        };
        let mut node = TreeNode::new(row.label.clone()).key(key).icon(icon);
        let parent_like = matches!(row.kind, RowKind::Directory | RowKind::Group);
        if row.expanded {
            expanded.insert(path.clone());
        } else if parent_like {
            node.children.push(TreeNode::new("…").key("\u{0}"));
        }
        if row.excluded {
            node.secondary = Some("excluded".to_owned());
        }
        map.insert(
            path.clone(),
            TreeEntry {
                path: row.path.clone(),
                kind: row.kind,
                group: row.group,
            },
        );
        stack.push((row.depth, node, path));
    }
    close_to(&mut stack, 0, &mut finished);
    let mut root_node = TreeNode::new(root)
        .key(root_key)
        .icon(RowIcon::Folder(true));
    root_node.children = finished;
    (vec![root_node], map, expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_core::project_view::DirRole;
    use std::path::PathBuf;

    fn row(depth: usize, label: &str, kind: RowKind, expanded: bool, path: &str) -> TreeRow {
        TreeRow {
            depth,
            label: label.to_owned(),
            path: Some(PathBuf::from(path)),
            kind,
            expanded,
            group: None,
            role: DirRole::Plain,
            excluded: false,
        }
    }

    #[test]
    fn the_tree_nests_rows_by_depth_and_marks_collapsed_directories() {
        let rows = vec![
            row(0, "src", RowKind::Directory, true, "src"),
            row(1, "Main.java", RowKind::File, false, "src/Main.java"),
            row(1, "util", RowKind::Directory, false, "src/util"),
            row(0, "README.md", RowKind::File, false, "README.md"),
        ];
        let (nodes, map, expanded) = build_tree("demo", &rows);
        assert_eq!(nodes.len(), 1);
        let root = &nodes[0];
        assert_eq!(root.label, "demo");
        assert_eq!(root.children.len(), 2);
        let src = &root.children[0];
        assert_eq!(src.children.len(), 2);
        // A collapsed directory carries a placeholder so it shows a chevron.
        assert!(src.children[1].is_parent());
        assert!(expanded.contains("./src") && expanded.contains("."));
        assert!(!expanded.contains("./src/util"));
        let entry = &map["./src/Main.java"];
        assert_eq!(entry.path.as_deref(), Some(Path::new("src/Main.java")));
        assert_eq!(entry.kind, RowKind::File);
    }

    #[test]
    fn file_icons_follow_the_extension() {
        assert!(matches!(
            file_icon_key(Path::new("a/B.java")),
            RowIconKey::Language(Language::Java)
        ));
        assert!(matches!(
            file_icon_key(Path::new("notes.unknown")),
            RowIconKey::Glyph(Glyph::File)
        ));
    }
}
