//! A Telescope-style picker: a centered float with a prompt, optional tabs, ranked rows and a
//! footer. Search Everywhere, Go to File, Find Action, Recent Files and the run configuration
//! chooser are pickers; the rows come from the core (`archipel_ide_core::quick_nav`), so the
//! ranking is the GUI's.

use crate::draw::{clip, fill, put, width};
use archipel_ide_core::quick_nav::{Hit, SearchTab};
use archipel_ide_view::popups::search as model;
use archipel_kit_model::{IconKey, Tone};
use archipel_kit_tui::chrome::{Tokens, icon_cell};
use archipel_kit_tui::components::RowIcon;
use archipel_kit_tui::controls::{FieldEvent, TextFieldControl, TextFieldState};
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::{Event, KeyCode, KeyEventKind, KeyModifiers};
use archipel_kit_tui::overlay::{Handled, Overlay};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::style::{Modifier, Style};
use archipel_kit_tui::runtime::Runtime;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

const PROMPT: Id = Id::new("tui.picker.prompt");
const ROW: Id = Id::new("tui.picker.row");
const TAB: Id = Id::new("tui.picker.tab");
const CHECK: Id = Id::new("tui.picker.check");

/// What choosing a row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Open a file (project-relative), optionally at a line and column.
    File {
        path: PathBuf,
        /// 1-based, as `editor.openFile` takes it.
        line: Option<usize>,
        column: Option<usize>,
        length: Option<usize>,
    },
    /// Start an action or command by id.
    Action(String),
    /// Select and run a run configuration.
    Config(String),
    /// Start a command of the core (a detected Run, Build or Test target).
    Command(&'static str),
    /// A Search Everywhere hit: opened through `Hit::open_commands`, an action through the
    /// registry.
    Hit(Hit),
    /// A line of a file in a recent locations list.
    Nothing,
}

/// One row of a picker.
#[derive(Debug, Clone)]
pub struct PickerRow {
    pub text: String,
    /// Dim text after the label.
    pub detail: String,
    /// Right-aligned text (a shortcut, a file name).
    pub trailing: String,
    pub icon: Option<RowIcon>,
    /// The icon as the shared model names it (Search Everywhere rows).
    pub icon_key: Option<IconKey>,
    /// The title of the group this row starts (a separator row above it).
    pub group: Option<&'static str>,
    /// Character positions of `text` to emphasize.
    pub positions: Vec<usize>,
    pub enabled: bool,
    pub target: Target,
    /// Shown in the footer while the row is selected.
    pub footer: String,
}

impl PickerRow {
    pub fn new(text: impl Into<String>, target: Target) -> Self {
        Self {
            text: text.into(),
            detail: String::new(),
            trailing: String::new(),
            icon: None,
            icon_key: None,
            group: None,
            positions: Vec::new(),
            enabled: true,
            target,
            footer: String::new(),
        }
    }
}

/// What the picker is for; the app computes its rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    SearchEverywhere,
    Files,
    Actions,
    Recent,
    Configs,
}

/// The state the overlay draws and the app fills.
pub struct PickerShared {
    pub kind: PickerKind,
    pub title: String,
    pub tabs: Vec<&'static str>,
    pub tab: usize,
    pub query: TextFieldState,
    pub rows: Vec<PickerRow>,
    pub selected: usize,
    /// The (query revision, tab) the rows were computed for.
    pub computed_for: Option<(u64, usize)>,
    /// A background search is running; rows may change without input.
    pub busy: bool,
    /// "Include non-project items" of the header band.
    pub include_non_project: bool,
    /// "Include disabled actions" of the Actions tab.
    pub include_disabled: bool,
    scroll: usize,
}

impl PickerShared {
    pub fn new(kind: PickerKind, title: &str, tabs: Vec<&'static str>, tab: usize) -> Self {
        Self {
            kind,
            title: title.to_owned(),
            tabs,
            tab,
            query: TextFieldState::new(),
            rows: Vec::new(),
            selected: 0,
            computed_for: None,
            busy: false,
            include_non_project: false,
            include_disabled: false,
            scroll: 0,
        }
    }

    /// Whether the rows are out of date.
    pub fn stale(&self) -> bool {
        self.computed_for != Some((self.query.revision, self.tab)) || self.busy
    }

    /// Stores freshly computed rows, keeping the selection near where it was.
    pub fn set_rows(&mut self, rows: Vec<PickerRow>) {
        let fresh = self.computed_for != Some((self.query.revision, self.tab));
        self.rows = rows;
        if fresh {
            self.selected = 0;
            self.scroll = 0;
        }
        self.selected = self.selected.min(self.rows.len().saturating_sub(1));
        self.computed_for = Some((self.query.revision, self.tab));
    }

    /// The tab as the shared model names it (Search Everywhere only).
    pub fn search_tab(&self) -> SearchTab {
        SearchTab::ALL[self.tab.min(SearchTab::ALL.len() - 1)]
    }

    /// Whether this is Search Everywhere (or a tab of it: Go to File, Find Action).
    pub fn is_search(&self) -> bool {
        matches!(
            self.kind,
            PickerKind::SearchEverywhere | PickerKind::Files | PickerKind::Actions
        )
    }

    /// Flips the header checkbox of the tab; the rows are computed again.
    pub fn toggle_include(&mut self) {
        match model::checkbox(
            self.search_tab(),
            self.include_non_project,
            self.include_disabled,
        ) {
            Some(check) if check.label == "Include disabled actions" => {
                self.include_disabled = !self.include_disabled
            }
            Some(_) => self.include_non_project = !self.include_non_project,
            None => return,
        }
        self.computed_for = None;
    }

    /// The selected row.
    pub fn current(&self) -> Option<&PickerRow> {
        self.rows.get(self.selected)
    }
}

/// A shared handle the app keeps to fill the rows of the open picker.
pub type Shared = Rc<RefCell<PickerShared>>;

type Chooser<M> = Box<dyn FnMut(&PickerShared, Target, bool) -> Vec<M>>;

/// The overlay: draws the shared state and handles the keys.
pub struct PickerOverlay<M> {
    shared: Shared,
    choose: Chooser<M>,
    page: usize,
}

impl<M> PickerOverlay<M> {
    pub fn new(
        shared: Shared,
        choose: impl FnMut(&PickerShared, Target, bool) -> Vec<M> + 'static,
    ) -> Self {
        Self {
            shared,
            choose: Box::new(choose),
            page: 10,
        }
    }

    fn pick(&mut self, index: usize, alt: bool) -> Handled<M> {
        let (target, shared_ref) = {
            let shared = self.shared.borrow();
            let Some(row) = shared.rows.get(index) else {
                return Handled::consumed();
            };
            if !row.enabled {
                return Handled::consumed();
            }
            (row.target.clone(), self.shared.clone())
        };
        let messages = (self.choose)(&shared_ref.borrow(), target, alt);
        Handled::closed().with_all(messages)
    }
}

fn float(area: Rect) -> Rect {
    let w = area.width.saturating_sub(4).clamp(20, 110).min(area.width);
    let h = area.height.saturating_sub(4).clamp(6, 26).min(area.height);
    let x = area.x + (area.width - w) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 3;
    Rect::new(x, y, w, h)
}

impl<M> Overlay<M> for PickerOverlay<M> {
    fn event(&mut self, rt: &mut Runtime, event: &Event) -> Handled<M> {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let count = self.shared.borrow().rows.len();
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let alt = key
                    .modifiers
                    .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL);
                match key.code {
                    KeyCode::Esc => return Handled::closed(),
                    KeyCode::Down => self.step(1, count),
                    KeyCode::Up => self.step(-1, count),
                    KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::ALT) => {
                        self.shared.borrow_mut().toggle_include();
                    }
                    KeyCode::Char('n') if ctrl => self.step(1, count),
                    KeyCode::Char('p') if ctrl => self.step(-1, count),
                    KeyCode::PageDown => self.step(self.page as isize, count),
                    KeyCode::PageUp => self.step(-(self.page as isize), count),
                    KeyCode::Tab | KeyCode::BackTab => {
                        let mut shared = self.shared.borrow_mut();
                        let tabs = shared.tabs.len();
                        if tabs > 1 {
                            let back = key.code == KeyCode::BackTab
                                || key.modifiers.contains(KeyModifiers::SHIFT);
                            shared.tab = (shared.tab + if back { tabs - 1 } else { 1 }) % tabs;
                        }
                    }
                    KeyCode::Enter => {
                        let index = self.shared.borrow().selected;
                        return self.pick(index, alt);
                    }
                    _ => {
                        let mut shared = self.shared.borrow_mut();
                        let result =
                            TextFieldControl::search(PROMPT).handle(rt, &mut shared.query, event);
                        if result == FieldEvent::Cancelled {
                            return Handled::closed();
                        }
                    }
                }
                Handled::consumed()
            }
            Event::Paste(_) => {
                let mut shared = self.shared.borrow_mut();
                TextFieldControl::search(PROMPT).handle(rt, &mut shared.query, event);
                Handled::consumed()
            }
            Event::Mouse(_) => {
                if let Some(pointer) = rt.pointer().copied() {
                    let tabs = self.shared.borrow().tabs.len();
                    for index in 0..tabs {
                        if pointer.clicked(TAB.with(index as u64)) {
                            self.shared.borrow_mut().tab = index;
                            return Handled::consumed();
                        }
                    }
                    if pointer.clicked(CHECK) {
                        self.shared.borrow_mut().toggle_include();
                        return Handled::consumed();
                    }
                    let count = self.shared.borrow().rows.len();
                    for index in 0..count {
                        if pointer.clicked(ROW.with(index as u64)) {
                            return self.pick(index, false);
                        }
                    }
                    if pointer.wheel() != 0 {
                        self.step(pointer.wheel() as isize * 3, count);
                    }
                    let mut shared = self.shared.borrow_mut();
                    TextFieldControl::search(PROMPT).handle(rt, &mut shared.query, event);
                }
                Handled::consumed()
            }
            _ => Handled::ignored(),
        }
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        if self.shared.borrow().is_search() {
            self.draw_search(rt, area, buf);
        } else {
            self.draw_plain(rt, area, buf);
        }
    }
}

impl<M> PickerOverlay<M> {
    /// Recent Files and the run configurations: a prompt, tabs and rows.
    fn draw_plain(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let p = rt.look.palette;
        let rect = float(area);
        let mut shared = self.shared.borrow_mut();
        fill(buf, rect, Style::default().bg(p.darker_black).fg(p.white));
        // Prompt.
        let prompt_area = Rect::new(rect.x, rect.y, rect.width, 1);
        let title = shared.title.clone();
        let query = shared.query.clone();
        TextFieldControl::search(PROMPT)
            .label(&title)
            .on(p.black2)
            .show(rt, &query, prompt_area, buf);
        if !rt.focused(PROMPT) {
            rt.focus.focus(PROMPT);
        }
        let mut y = rect.y + 1;
        // Tabs.
        if shared.tabs.len() > 1 {
            let mut x = rect.x + 1;
            for (index, tab) in shared.tabs.iter().enumerate() {
                let active = index == shared.tab;
                let style = if active {
                    Style::default()
                        .fg(p.nord_blue)
                        .bg(p.darker_black)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(p.grey_fg).bg(p.darker_black)
                };
                x = put(buf, rect, x, y, tab, style) + 2;
            }
            put(
                buf,
                rect,
                rect.right().saturating_sub(10),
                y,
                "Tab: next",
                Style::default().fg(p.grey).bg(p.darker_black),
            );
            y += 1;
        }
        let footer_y = rect.bottom() - 1;
        let body = footer_y.saturating_sub(y) as usize;
        self.page = body.max(1);
        // Keep the selection visible.
        if shared.selected < shared.scroll {
            shared.scroll = shared.selected;
        } else if body > 0 && shared.selected >= shared.scroll + body {
            shared.scroll = shared.selected + 1 - body;
        }
        let scroll = shared.scroll;
        let selected = shared.selected;
        for (offset, index) in (scroll..shared.rows.len()).take(body).enumerate() {
            let row = &shared.rows[index];
            let cy = y + offset as u16;
            let row_rect = Rect::new(rect.x, cy, rect.width, 1);
            let chosen = index == selected;
            let (bg, fg) = if chosen {
                (p.one_bg3, p.white)
            } else if row.enabled {
                (p.darker_black, p.white)
            } else {
                (p.darker_black, p.grey)
            };
            let base = Style::default().bg(bg).fg(fg);
            fill(buf, row_rect, base);
            let mut x = rect.x + 1;
            if let Some(icon) = row.icon {
                let (ch, color) = icon.char_and_color(&rt.look);
                put(
                    buf,
                    row_rect,
                    x,
                    cy,
                    &ch.to_string(),
                    base.fg(if chosen { fg } else { color }),
                );
            }
            x += 3;
            let trailing_w = width(&row.trailing);
            let limit = rect.right().saturating_sub(trailing_w + 3);
            let text_area = Rect::new(rect.x, cy, limit.saturating_sub(rect.x), 1);
            for (i, ch) in row.text.chars().enumerate() {
                let style = if row.positions.contains(&i) {
                    base.fg(p.orange).add_modifier(Modifier::BOLD)
                } else if chosen {
                    base.add_modifier(Modifier::BOLD)
                } else {
                    base
                };
                x = put(buf, text_area, x, cy, ch.encode_utf8(&mut [0; 4]), style);
            }
            if !row.detail.is_empty() && x + 2 < limit {
                put(
                    buf,
                    text_area,
                    x + 2,
                    cy,
                    &clip(&row.detail, limit.saturating_sub(x + 2)),
                    base.fg(p.grey_fg),
                );
            }
            if trailing_w > 0 {
                put(
                    buf,
                    row_rect,
                    rect.right().saturating_sub(trailing_w + 1),
                    cy,
                    &row.trailing,
                    base.fg(p.grey_fg),
                );
            }
            rt.clickable(ROW.with(index as u64), row_rect);
        }
        if shared.rows.is_empty() {
            let message = if shared.busy {
                "Searching…"
            } else {
                "No matches"
            };
            put(
                buf,
                rect,
                rect.x + 1,
                y,
                message,
                Style::default().fg(p.grey_fg).bg(p.darker_black),
            );
        }
        // Footer.
        let footer_rect = Rect::new(rect.x, footer_y, rect.width, 1);
        fill(buf, footer_rect, Style::default().bg(p.black2));
        let text = shared
            .current()
            .map(|row| row.footer.clone())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "↑↓ move · Enter open · Esc close".to_owned());
        put(
            buf,
            footer_rect,
            rect.x + 1,
            footer_y,
            &clip(&text, rect.width.saturating_sub(2)),
            Style::default().fg(p.grey_fg).bg(p.black2),
        );
    }
}

impl<M> PickerOverlay<M> {
    fn step(&mut self, delta: isize, count: usize) {
        let mut shared = self.shared.borrow_mut();
        if count == 0 {
            return;
        }
        let next = (shared.selected as isize + delta).clamp(0, count as isize - 1);
        shared.selected = next as usize;
    }
}

/// The float of Search Everywhere: 110 cells at most, centered, a third from the top.
fn search_float(area: Rect) -> Rect {
    let w = area.width.saturating_sub(2).clamp(20, 110).min(area.width);
    let h = area.height.saturating_sub(2).clamp(8, 30).min(area.height);
    let x = area.x + (area.width - w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 4;
    Rect::new(x, y, w, h)
}

impl<M> PickerOverlay<M> {
    /// The anatomy of TUI-GUI-135: the header band with the tabs and the checkbox, the
    /// accent underline row, the search row with its hint, a divider, the list with group
    /// separators, and the footer with the location and the link.
    fn draw_search(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let look = rt.look.clone();
        let c = look.theme.colors;
        let body = Tokens::new(&look, c.popup);
        let band = Tokens::new(&look, c.group_separator);
        let rect = search_float(area);
        let mut shared = self.shared.borrow_mut();
        let body_style = Style::default().fg(body.of(&look, c.text)).bg(body.bg);
        let band_bg = band.bg;
        let secondary = |tokens: &Tokens| {
            Style::default()
                .fg(tokens.of(&look, c.text_secondary))
                .bg(tokens.bg)
        };
        fill(buf, rect, body_style);

        // Header band: two rows.
        fill(
            buf,
            Rect::new(rect.x, rect.y, rect.width, 2),
            Style::default().bg(band_bg),
        );
        let tab_row = rect.y;
        let mut x = rect.x + 1;
        let current = shared.tab;
        let labels: Vec<&'static str> = shared.tabs.clone();
        for (index, label) in labels.iter().enumerate() {
            let selected = index == current;
            let w = width(label) + 2;
            let cell = Rect::new(x, tab_row, w, 1).intersection(rect);
            let style = if selected {
                Style::default()
                    .fg(band.of(&look, c.text_strong))
                    .bg(band_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                secondary(&band)
            };
            put(buf, rect, x + 1, tab_row, label, style);
            if selected {
                // The accent underline row of the selected tab.
                for ux in cell.x..cell.right() {
                    put(
                        buf,
                        rect,
                        ux,
                        tab_row + 1,
                        "▔",
                        Style::default().fg(band.of(&look, c.accent)).bg(band_bg),
                    );
                }
            }
            rt.clickable(
                TAB.with(index as u64),
                Rect::new(x, tab_row, w, 2).intersection(rect),
            );
            x += w;
        }
        // The checkbox at the right of the band.
        let tab = shared.search_tab();
        if let Some(check) =
            model::checkbox(tab, shared.include_non_project, shared.include_disabled)
        {
            let text = format!(
                "{} {}",
                if check.checked { "[x]" } else { "[ ]" },
                check.label
            );
            let at = rect.right().saturating_sub(width(&text) + 2);
            if at > x + 1 {
                put(buf, rect, at, tab_row, &text, secondary(&band));
                rt.clickable(CHECK, Rect::new(at, tab_row, width(&text), 1));
            }
        }

        // Search row.
        let field_row = Rect::new(rect.x + 1, rect.y + 2, rect.width.saturating_sub(2), 1);
        let hint_w = width(model::HINT);
        let placeholder = if field_row.width > width(model::PLACEHOLDER) + hint_w + 8 {
            model::PLACEHOLDER
        } else {
            ""
        };
        let query = shared.query.clone();
        TextFieldControl::search(PROMPT)
            .placeholder(placeholder)
            .on(body.bg)
            .show(rt, &query, field_row, buf);
        if !rt.focused(PROMPT) {
            rt.focus.focus(PROMPT);
        }
        if query.is_empty() && field_row.width > hint_w + 12 {
            put(
                buf,
                rect,
                field_row.right().saturating_sub(hint_w + 1),
                field_row.y,
                model::HINT,
                Style::default()
                    .fg(body.of(&look, c.text_placeholder))
                    .bg(body.bg),
            );
        }
        // Divider.
        let divider = rect.y + 3;
        for dx in rect.x..rect.right() {
            put(
                buf,
                rect,
                dx,
                divider,
                "─",
                Style::default()
                    .fg(body.of(&look, c.popup_border))
                    .bg(body.bg),
            );
        }

        // List.
        let list_top = divider + 1;
        let footer_y = rect.bottom() - 1;
        let body_rows = footer_y.saturating_sub(list_top) as usize;
        self.page = body_rows.max(1);
        // Visual rows: a group title row before the first row of a group.
        let mut visual: Vec<(Option<&'static str>, usize)> = Vec::new();
        for (index, row) in shared.rows.iter().enumerate() {
            if let Some(group) = row.group {
                visual.push((Some(group), index));
            }
            visual.push((None, index));
        }
        let selected_visual = visual
            .iter()
            .position(|(group, index)| group.is_none() && *index == shared.selected)
            .unwrap_or(0);
        // Keep the selection (and its group title when it is the first row) in view.
        let mut scroll = shared.scroll.min(visual.len().saturating_sub(1));
        let first_of_group = selected_visual > 0 && visual[selected_visual - 1].0.is_some();
        let want_top = selected_visual - usize::from(first_of_group);
        if want_top < scroll {
            scroll = want_top;
        } else if body_rows > 0 && selected_visual >= scroll + body_rows {
            scroll = selected_visual + 1 - body_rows;
        }
        shared.scroll = scroll;
        let selected = shared.selected;
        for (offset, (group, index)) in visual.iter().skip(scroll).take(body_rows).enumerate() {
            let cy = list_top + offset as u16;
            let line = Rect::new(rect.x, cy, rect.width, 1);
            if let Some(group) = group {
                put(buf, line, rect.x + 2, cy, group, secondary(&body));
                continue;
            }
            let row = &shared.rows[*index];
            let chosen = *index == selected;
            let row_tokens = if chosen {
                Tokens::new(&look, c.selection)
            } else {
                Tokens::new(&look, c.popup)
            };
            let base = Style::default()
                .fg(if row.enabled {
                    row_tokens.of(&look, c.text)
                } else {
                    row_tokens.of(&look, c.text_disabled)
                })
                .bg(row_tokens.bg);
            // The selection is inset by one cell from the border.
            fill(
                buf,
                Rect::new(rect.x + 1, cy, rect.width.saturating_sub(2), 1),
                base,
            );
            let dim = Style::default()
                .fg(row_tokens.of(&look, c.text_secondary))
                .bg(row_tokens.bg);
            if let Some(key) = &row.icon_key {
                let (glyph, color) = icon_cell(&look, &row_tokens, key, Tone::Default);
                put(buf, line, rect.x + 2, cy, &glyph, base.fg(color));
            }
            let trailing_w = width(&row.trailing);
            let limit = rect.right().saturating_sub(trailing_w + 3);
            let text_area = Rect::new(rect.x, cy, limit.saturating_sub(rect.x), 1);
            let mut x = rect.x + 4;
            for (i, ch) in row.text.chars().enumerate() {
                let style = if row.positions.contains(&i) {
                    Style::default()
                        .fg(row_tokens.of(&look, c.search_match_text))
                        .bg(row_tokens.of(&look, c.search_match))
                } else {
                    base
                };
                x = put(buf, text_area, x, cy, ch.encode_utf8(&mut [0; 4]), style);
            }
            if !row.detail.is_empty() && x + 2 < limit {
                put(
                    buf,
                    text_area,
                    x + 1,
                    cy,
                    &clip(&row.detail, limit.saturating_sub(x + 1)),
                    dim,
                );
            }
            if trailing_w > 0 {
                put(
                    buf,
                    line,
                    rect.right().saturating_sub(trailing_w + 2),
                    cy,
                    &row.trailing,
                    dim,
                );
            }
            rt.clickable(
                ROW.with(*index as u64),
                Rect::new(rect.x + 1, cy, rect.width.saturating_sub(2), 1),
            );
        }
        if shared.rows.is_empty() {
            let message = if shared.busy {
                model::empty_text(true)
            } else if shared.query.is_empty() {
                ""
            } else {
                model::empty_text(false)
            };
            put(
                buf,
                rect,
                rect.x + 2,
                list_top + 1,
                message,
                secondary(&body),
            );
        }

        // Footer: the location of the selected item and the link.
        let footer_rect = Rect::new(rect.x, footer_y, rect.width, 1);
        fill(buf, footer_rect, Style::default().bg(band_bg));
        let selected_row = shared.current();
        let left = selected_row
            .map(|row| row.footer.clone())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "↑↓ move · Enter open · Esc close".to_owned());
        // The terminal cannot assign shortcuts: the Actions tab has no link.
        let link = if tab == SearchTab::Actions {
            ""
        } else {
            model::link(tab, selected_row.is_some())
        };
        let link_w = width(link);
        if link_w > 0 {
            put(
                buf,
                footer_rect,
                rect.right().saturating_sub(link_w + 2),
                footer_y,
                link,
                Style::default().fg(band.of(&look, c.link)).bg(band_bg),
            );
        }
        put(
            buf,
            footer_rect,
            rect.x + 2,
            footer_y,
            &clip(&left, rect.width.saturating_sub(link_w + 6)),
            secondary(&band),
        );
    }
}
