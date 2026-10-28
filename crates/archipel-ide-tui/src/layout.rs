//! Drawing and mouse routing of the GUI-faithful layout: the chrome from the view crate, the
//! tool window bodies, the editor column and the welcome screen.

use crate::app::{COMMIT, EDITOR, FIND, PROBLEMS, RUN, TERMINAL, TREE, TreeEntry, TuiApp};
use crate::shell::Focus;
use archipel_ide_core::project_view::{RowKind, TreeRow};
use archipel_ide_core::{Anchor, ToolWindow};
use archipel_ide_view::{Caps, Chrome};
use archipel_kit_tui::collections::{TreeEvent, TreeNode};
use archipel_kit_tui::components::{RowIcon, RowIconKey};
use archipel_kit_tui::editor::EditorComponent;
use archipel_kit_tui::input::Event;
use archipel_kit_tui::overlay::draw_which_key;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::style::Style;
use archipel_kit_tui::runtime::Runtime;
use archipel_kit_tui::{Glyph, Language};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

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

/// The view crate's keyboard capabilities of the runtime.
pub(crate) fn view_caps(rt: &Runtime) -> Caps {
    Caps {
        kitty: rt.caps.kitty,
        mac: rt.caps.os.is_mac(),
        super_reported: rt.caps.super_reported,
        terminal: true,
    }
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

    /// The theme and glyph mode follow the settings, live.
    fn sync_theme(&mut self, rt: &mut Runtime) {
        let name = crate::theme::setting(self.ide.state());
        if name != self.theme_name || self.ascii != rt.look.ascii {
            self.theme_name = name.clone();
            rt.look = crate::theme::look(&name, self.ascii);
        }
    }

    /// The facts only this frontend knows, for the view builders.
    fn sync_ui_data(&mut self, rt: &Runtime) {
        let caps = view_caps(rt);
        self.ide.caps = caps;
        self.ide.data.focused_window = self.shell.focused_tool();
        // Read-only is probed at most once a second: the builders do no I/O.
        let path = self
            .ide
            .state()
            .active_doc()
            .map(|document| self.ide.state().absolute_path(&document.path));
        match &path {
            Some(path) => {
                let stale = self
                    .read_only
                    .as_ref()
                    .is_none_or(|(known, at, _)| known != path || rt.now - at >= 1.0);
                if stale {
                    let value = std::fs::metadata(path).is_ok_and(|m| m.permissions().readonly());
                    self.read_only = Some((path.clone(), rt.now, value));
                }
                self.ide.data.read_only = self.read_only.as_ref().is_some_and(|(_, _, v)| *v);
            }
            None => self.ide.data.read_only = false,
        }
        // The declarations around the caret, from the structure of the active document.
        let crumbs = match self.ide.state().active_doc() {
            Some(document) => {
                let revision = document.text.revision();
                let fresh = self
                    .crumb_cache
                    .as_ref()
                    .is_some_and(|(p, r, _)| *p == document.path && *r == revision);
                if !fresh {
                    let items = Arc::new(document.analysis.structure());
                    self.crumb_cache = Some((document.path.clone(), revision, items));
                }
                self.crumb_cache
                    .as_ref()
                    .map(|(_, _, items)| archipel_ide_view::status::code_crumbs(document, items))
                    .unwrap_or_default()
            }
            None => Vec::new(),
        };
        self.ide.data.code_crumbs = crumbs;
    }

    /// Draws everything.
    pub(crate) fn draw_all(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        self.sync_theme(rt);
        let look = rt.look.clone();
        let colors = look.theme.colors;
        let island = look.over(colors.island, colors.island);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_symbol(" ").set_style(
                    Style::default()
                        .bg(island)
                        .fg(look.over(colors.text, colors.island)),
                );
            }
        }
        let (left, right, bottom) = {
            let layout = self.ide.state().tool_windows();
            (
                layout.visible(Anchor::Left),
                layout.visible(Anchor::Right),
                layout.visible(Anchor::Bottom),
            )
        };
        let geo = self.shell.compute(area, left, right, bottom).clone();
        if geo.too_small {
            let text = format!(
                "Terminal too small: need {}x{}, have {}x{}",
                crate::shell::MIN_SIZE.0,
                crate::shell::MIN_SIZE.1,
                area.width,
                area.height
            );
            crate::draw::put(
                buf,
                area,
                area.x,
                area.y,
                &text,
                Style::default()
                    .fg(look.over(colors.text, colors.island))
                    .bg(island),
            );
            return;
        }
        self.sync_ui_data(rt);
        let title = {
            let state = self.ide.state();
            let project = archipel_ide_view::toolbar::project_name(&self.ide.cx());
            archipel_ide_view::status::window_title(
                &project,
                state.active_doc().map(|doc| doc.path.as_path()),
            )
        };
        rt.title = Some(title);
        let built = Chrome::build_with(&self.ide.cx(), self.chrome.menu_expanded());
        self.chrome.built = Some(built.clone());

        // Row 0 and the stripes.
        self.chrome
            .draw_toolbar(rt, &built.toolbar, &built.menu, geo.toolbar, buf);
        self.chrome.left.draw(rt, &built.left, geo.left_stripe, buf);
        self.chrome
            .right
            .draw(rt, &built.right, geo.right_stripe, buf);

        // Tool windows: header and body.
        let mut tools: Vec<(ToolWindow, Rect)> = Vec::new();
        for pane in [geo.left, geo.right, geo.bottom].into_iter().flatten() {
            let focused = self.shell.focus == Focus::Tool(pane.window);
            if let Some((_, header)) = built.headers.iter().find(|(w, _)| *w == pane.window) {
                let mut header = header.clone();
                header.focused = focused;
                match pane.window.anchor() {
                    Anchor::Left => self.chrome.header_left.draw(rt, &header, pane.header, buf),
                    Anchor::Right => self.chrome.header_right.draw(rt, &header, pane.header, buf),
                    Anchor::Bottom => self
                        .chrome
                        .header_bottom
                        .draw(rt, &header, pane.header, buf),
                }
            }
            tools.push((pane.window, pane.body));
        }
        self.shell.register(rt);
        // The splitter columns.
        let sep = look.over(colors.separator, colors.island);
        for (_, rect) in &geo.splitters {
            if rect.width == 1 {
                for y in rect.top()..rect.bottom() {
                    crate::draw::put(
                        buf,
                        area,
                        rect.x,
                        y,
                        "│",
                        Style::default().fg(sep).bg(island),
                    );
                }
            }
        }
        for (window, body) in tools {
            self.draw_tool_body(rt, window, body, buf);
        }
        self.sync_focus(rt);

        // The editor column.
        self.chrome.tabs.draw(rt, &built.tabs, geo.tabs, buf);
        self.draw_editor_area(rt, geo.editor, buf);
        // The status bar.
        self.chrome.status.draw(rt, &built.status, geo.status, buf);

        self.layers.draw(rt, area, buf);
        self.chrome.draw_popups(rt, buf);
        draw_which_key(rt, area, buf, &|id| {
            archipel_ide_actions::Catalog::shared().label(id)
        });
    }

    fn draw_tool_body(
        &mut self,
        rt: &mut Runtime,
        window: ToolWindow,
        body: Rect,
        buf: &mut Buffer,
    ) {
        let p = rt.look.palette;
        match window {
            ToolWindow::Project => {
                self.tree
                    .show(rt, TREE, body, &self.tree_nodes, p.black, buf);
                self.ide.data.tree_selection = self
                    .tree
                    .selected_path()
                    .and_then(|path| self.tree_map.get(path))
                    .and_then(|entry| entry.path.clone())
                    .filter(|path| !path.is_absolute());
            }
            ToolWindow::Commit => self.draw_commit(rt, body, buf),
            ToolWindow::Build => self.draw_run(rt, body, buf),
            ToolWindow::Problems => self.draw_problems(rt, body, buf),
            ToolWindow::Find => self.draw_find(rt, body, buf),
            ToolWindow::Terminal => self.draw_terminal(rt, body, buf),
            other => self.draw_unavailable(rt, other, body, buf),
        }
    }

    /// The body of a window without a terminal rendering yet: says so and names the fallback
    /// (TUI-GUI-076).
    fn draw_unavailable(
        &mut self,
        rt: &mut Runtime,
        window: ToolWindow,
        body: Rect,
        buf: &mut Buffer,
    ) {
        let look = rt.look.clone();
        let c = look.theme.colors;
        let style = Style::default()
            .fg(look.over(c.text_secondary, c.island))
            .bg(look.over(c.island, c.island));
        let lines = [
            format!("{} is not available in the terminal yet.", window.title()),
            "Open it in the window: archipel-ide without --tui.".to_owned(),
        ];
        for (i, line) in lines.iter().enumerate() {
            crate::draw::put(buf, body, body.x + 1, body.y + 1 + i as u16, line, style);
        }
    }

    /// Makes the widget of the focused part the focused widget.
    fn sync_focus(&self, rt: &mut Runtime) {
        if !self.layers.is_empty() {
            return;
        }
        let id = match self.shell.focus {
            Focus::Editor => EDITOR,
            Focus::Tool(window) => match window {
                ToolWindow::Project => TREE,
                ToolWindow::Commit => COMMIT,
                ToolWindow::Build => RUN,
                ToolWindow::Problems => PROBLEMS,
                ToolWindow::Find => FIND,
                ToolWindow::Terminal => TERMINAL,
                _ => EDITOR,
            },
        };
        if !rt.focused(id) {
            rt.focus.focus(id);
        }
    }

    fn draw_editor_area(&mut self, rt: &mut Runtime, editor: Rect, buf: &mut Buffer) {
        if editor.height < 2 || editor.width < 5 {
            return;
        }
        match self.active_view() {
            None => self.draw_welcome(rt, editor, buf),
            Some(index) => {
                // The last column is the error stripe.
                let body = Rect::new(editor.x, editor.y, editor.width - 1, editor.height);
                let view = &mut self.views[index];
                let crate::view::DocView {
                    editor: model,
                    state,
                    highlighter,
                    language,
                    highlights,
                    follow,
                    ..
                } = view;
                let mut out = EditorComponent::new(EDITOR, model)
                    .highlighter(highlighter)
                    .language(language)
                    .highlights(highlights)
                    .bracket_folding(false)
                    .wrap(true)
                    .show(rt, state, body, buf);
                // The core moved the caret (typing, a caret key, undo): scroll it into view.
                if *follow {
                    *follow = false;
                    if out.caret_cell.is_none() {
                        let line = model.document.line_number(model.carets.primary().offset);
                        state.scroll_to_line(line);
                        out = EditorComponent::new(EDITOR, model)
                            .highlighter(highlighter)
                            .language(language)
                            .highlights(highlights)
                            .bracket_folding(false)
                            .wrap(true)
                            .show(rt, state, body, buf);
                    }
                }
                self.popups.caret_cell = out.caret_cell;
                self.popups.editor_area = body;
                let (page, top) = (out.visible_lines.len().max(1), out.visible_lines.start);
                if let Some(document) = self.ide.state_mut().document_mut(index) {
                    document.input.page_lines = page;
                    document.input.top_line = top;
                }
                self.draw_error_stripe(
                    rt,
                    index,
                    Rect::new(editor.right() - 1, editor.y, 1, editor.height),
                    buf,
                );
                self.draw_popups(rt, buf);
            }
        }
    }

    /// The error stripe (TUI-GUI-093): the inspection widget in its top cell and a tick per
    /// row bucket, error beating warning; a click moves the caret to the first problem of the
    /// bucket.
    fn draw_error_stripe(&mut self, rt: &mut Runtime, index: usize, rect: Rect, buf: &mut Buffer) {
        let look = rt.look.clone();
        let c = look.theme.colors;
        let e = look.theme.editor;
        let bg = look.over(c.island, c.island);
        let problems: Vec<archipel_ide_core::Problem> = self
            .views
            .get(index)
            .and_then(|view| view.problems())
            .map(<[_]>::to_vec)
            .unwrap_or_default();
        self.stripe_buckets.clear();
        let total_lines = self
            .ide
            .state()
            .document(index)
            .map_or(1, |document| document.text.len_lines().max(1));
        let worst = problems.iter().map(|p| rank(p.severity)).max();
        let (glyph, color) = match worst {
            None => (Glyph::Success, look.over(c.success, c.island)),
            Some(2) => (Glyph::Error, look.over(c.error, c.island)),
            Some(1) => (Glyph::Warning, look.over(c.warning, c.island)),
            Some(_) => (Glyph::WeakWarning, look.over(c.text_placeholder, c.island)),
        };
        crate::draw::put(
            buf,
            rect,
            rect.x,
            rect.y,
            &look.glyph(glyph),
            Style::default().fg(color).bg(bg),
        );
        let rows = rect.height.saturating_sub(1) as usize;
        if rows == 0 {
            return;
        }
        let mut buckets: Vec<Option<(u8, usize)>> = vec![None; rows];
        for problem in &problems {
            let row = (problem.line * rows / total_lines).min(rows - 1);
            let rank = rank(problem.severity);
            match &mut buckets[row] {
                Some((best, line)) if *best >= rank => {
                    let _ = line;
                }
                slot => *slot = Some((rank, problem.line)),
            }
        }
        for (i, bucket) in buckets.iter().enumerate() {
            if let Some((rank, line)) = bucket {
                let color = match rank {
                    2 => look.over(e.error_stripe, c.island),
                    1 => look.over(e.warning_stripe, c.island),
                    _ => look.over(c.text_placeholder, c.island),
                };
                let y = rect.y + 1 + i as u16;
                crate::draw::put(buf, rect, rect.x, y, "▪", Style::default().fg(color).bg(bg));
                rt.clickable(
                    crate::app::STRIPE_TICK.with(i as u64),
                    Rect::new(rect.x, y, 1, 1),
                );
                self.stripe_buckets.push((i as u64, *line));
            }
        }
    }

    /// The Welcome screen (no project open): Open, New Project, the recent projects as rows.
    fn draw_welcome(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let look = rt.look.clone();
        let c = look.theme.colors;
        let bg = look.over(c.island, c.island);
        let strong = Style::default()
            .fg(look.over(c.text_strong, c.island))
            .bg(bg)
            .add_modifier(archipel_kit_tui::ratatui::style::Modifier::BOLD);
        let normal = Style::default().fg(look.over(c.text, c.island)).bg(bg);
        let dim = Style::default()
            .fg(look.over(c.text_secondary, c.island))
            .bg(bg);
        self.welcome_rows.clear();
        let mut y = area.y + 2;
        let x = area.x + 3;
        crate::draw::put(buf, area, x, y, "Welcome to Archipel IDE", strong);
        y += 2;
        let has_project = self.ide.state().has_project();
        let mut rows: Vec<(String, String, Option<archipel_ide_view::Effect>)> = Vec::new();
        if !has_project {
            rows.push((
                "Open…".into(),
                "Open a project folder".into(),
                Some(archipel_ide_view::Effect::Action {
                    id: "OpenFile".into(),
                    place: archipel_ide_actions::ActionPlace::MainMenu,
                }),
            ));
            rows.push((
                "New Project…".into(),
                "Create a project".into(),
                Some(archipel_ide_view::Effect::Action {
                    id: "NewProject".into(),
                    place: archipel_ide_actions::ActionPlace::MainMenu,
                }),
            ));
            let recent: Vec<_> = self.ide.state().recent_projects().read().entries().to_vec();
            for entry in recent.iter().take(8) {
                rows.push((
                    entry.name.clone(),
                    entry.path.display().to_string(),
                    Some(archipel_ide_view::Effect::Request(
                        archipel_ide_view::Request::OpenProject(entry.path.display().to_string()),
                    )),
                ));
            }
        } else {
            let lines = [
                ("Search Everywhere", "SearchEverywhere"),
                ("Project View", "ActivateProjectToolWindow"),
                ("Find in Files", "FindInPath"),
                ("Terminal", "ActivateTerminalToolWindow"),
            ];
            for (label, action) in lines {
                let shortcut = self.ide.shortcut_label(action).unwrap_or_default();
                crate::draw::put(buf, area, x, y, label, normal);
                crate::draw::put(buf, area, x + 24, y, &shortcut, dim);
                y += 1;
            }
            return;
        }
        for (i, (label, detail, effect)) in rows.into_iter().enumerate() {
            let id = crate::app::WELCOME.with(i as u64);
            let rect = Rect::new(x, y, area.width.saturating_sub(6).min(70), 1);
            let hovered = rt.hovered() == Some(id);
            let row_bg = if hovered {
                look.over(c.row_hover, c.island)
            } else {
                bg
            };
            crate::draw::fill(buf, rect, Style::default().bg(row_bg));
            crate::draw::put(
                buf,
                rect,
                rect.x + 1,
                y,
                &label,
                Style::default().fg(look.over(c.link, c.island)).bg(row_bg),
            );
            let at = rect.x + 3 + crate::draw::width(&label).max(16);
            crate::draw::put(
                buf,
                rect,
                at,
                y,
                &crate::draw::clip(&detail, rect.right().saturating_sub(at + 1)),
                Style::default()
                    .fg(look.over(c.text_secondary, c.island))
                    .bg(row_bg),
            );
            rt.clickable(id, rect);
            if let Some(effect) = effect {
                self.welcome_rows.push((id, effect));
            }
            y += 1;
        }
    }

    /// Routes an event the actions did not take to the widgets.
    pub(crate) fn route(&mut self, rt: &mut Runtime, event: &Event) {
        self.sync_focus(rt);
        // A press focuses the part under the pointer.
        if let Event::Mouse(_) = event
            && let Some(pointer) = rt.pointer().copied()
            && pointer.kind
                == archipel_kit_tui::pointer::PointerKind::Down(
                    archipel_kit_tui::input::MouseButton::Left,
                )
        {
            let at = archipel_kit_tui::ratatui::layout::Position::new(pointer.x, pointer.y);
            let geo = self.shell.geo.clone();
            let hit = [geo.left, geo.right, geo.bottom]
                .into_iter()
                .flatten()
                .find(|pane| pane.rect.contains(at));
            if let Some(pane) = hit {
                self.shell.focus_tool(pane.window);
                if pane.window == ToolWindow::Terminal {
                    self.ensure_terminal();
                }
            } else if geo.editor.contains(at) || geo.tabs.contains(at) {
                self.shell.focus_editor();
            }
            for (id, line) in self.stripe_buckets.clone() {
                if pointer.over(crate::app::STRIPE_TICK.with(id)) {
                    self.command(
                        rt,
                        "editor.gotoLine",
                        json!({ "line": line + 1, "column": 1 }),
                    );
                    self.shell.focus_editor();
                }
            }
            for (id, effect) in self.welcome_rows.clone() {
                if pointer.over(id) {
                    self.apply_effects(rt, vec![effect]);
                }
            }
        }
        if let Event::Mouse(_) = event
            && let Some(pointer) = rt.pointer().copied()
            && self.shell.handle_pointer(&pointer)
        {
            return;
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
        // The editor widget: the mouse, and the keys of its find bar. Editing keys went to the
        // core before (see `TuiApp::event`).
        let forward = match event {
            Event::Key(key) => {
                key.kind != archipel_kit_tui::input::KeyEventKind::Release
                    && (self.find_open() || is_find_key(key))
            }
            Event::Paste(_) => self.find_open(),
            _ => true,
        };
        if forward {
            self.forward_to_editor_widget(rt, event);
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
            let _ = EditorComponent::new(EDITOR, editor)
                .bracket_folding(false)
                .wrap(true)
                .tick(rt, state);
        }
    }
}

/// The keys that open or drive the find bar of the editor widget (Find, Replace, Find Next,
/// Find Previous): the core has no find bar, so the widget's is used.
fn is_find_key(key: &archipel_kit_tui::input::KeyEvent) -> bool {
    use archipel_kit_tui::input::{KeyCode, KeyModifiers};
    match key.code {
        KeyCode::F(3) => true,
        KeyCode::Char(c) => {
            key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
                && matches!(c.to_ascii_lowercase(), 'f' | 'r' | 'l')
        }
        _ => false,
    }
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

fn rank(severity: archipel_ide_core::Severity) -> u8 {
    use archipel_ide_core::Severity;
    match severity {
        Severity::Error => 2,
        Severity::Warning => 1,
        Severity::WeakWarning | Severity::Hint => 0,
    }
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
