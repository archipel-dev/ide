//! The editor gutter of the terminal (Wave 1 of `docs/tui-parity.md`): breakpoint and run
//! icons in the two lanes, what a click on each part of the gutter does, the change popup and
//! the breakpoint properties dialog.
//!
//! What the gutter shows and what its popups say is the view crate's (`popups::gutter`,
//! `popups::change`, `popups::breakpoint`, shared with the GUI); this file places the glyphs
//! in the kit's lanes, routes the kit's gutter clicks and draws the change popup with the
//! GUI's colors. Specs: TUI-GUI-090, TUI-GUI-091, TUI-GUI-140, EXE-CTX-050..053.

use crate::app::{EDITOR, Msg, TuiApp};
use crate::dialogs::FormContent;
use crate::draw::{clip, fill, put, width};
use archipel_ide_view::dispatch::Popup;
use archipel_ide_view::popups::breakpoint::{self, BreakpointForm};
use archipel_ide_view::popups::change::{self, ChangePopup, Tool};
use archipel_ide_view::popups::gutter::{self, BreakpointKind, RunMarks};
use archipel_kit::Tone;
use archipel_kit::editor::{GutterArea, GutterIcon};
use archipel_kit::openapi::ui::ExitCode;
use archipel_kit_tui::Glyph;
use archipel_kit_tui::dialog::Dialog;
use archipel_kit_tui::editor::{GutterClick, GutterHover};
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton};
use archipel_kit_tui::pointer::PointerKind;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::{Position, Rect};
use archipel_kit_tui::ratatui::style::{Modifier, Style};
use archipel_kit_tui::runtime::Runtime;
use archipel_kit_tui::tooltip::Tooltip;
use std::cell::RefCell;
use std::rc::Rc;

/// The lanes of the gutter: breakpoints (and later the execution arrow) in the first, run
/// icons in the second. They are reserved, so the text never shifts when an icon appears.
pub const LANES: usize = 2;
const BREAKPOINT_LANE: usize = 0;
const RUN_LANE: usize = 1;

const CHANGE_POPUP: Id = Id::new("tui.change.popup");

/// The id the kit gives the gutter of the editor (for hit tests).
pub fn gutter_id() -> Id {
    EDITOR.with(1)
}

/// An open change popup.
pub struct ChangeOpen {
    hunk: usize,
    /// Where the bar was clicked.
    anchor: (u16, u16),
    /// Where it was drawn last.
    rect: Rect,
    buttons: Vec<(Tool, Rect)>,
}

/// The state of the gutter that is not the core's.
#[derive(Default)]
pub struct GutterUi {
    run_marks: RunMarks,
    /// The gutter cell under the pointer (the breakpoint preview follows it).
    pub hover: Option<GutterHover>,
    /// The change popup, when open.
    pub change: Option<ChangeOpen>,
    /// The breakpoint properties dialog is on screen.
    form_open: bool,
}

impl TuiApp {
    /// Sets the icons of the active editor's lanes for this frame: the breakpoints and the
    /// preview under the pointer in the first lane, the run icons in the second.
    pub(crate) fn refresh_gutter(&mut self, rt: &Runtime) {
        let Some(index) = self.active_view() else {
            return;
        };
        let (marks, _kept) = self.gutter.run_marks.marks(self.ide.state_mut(), rt.now);
        let Some(document) = self.ide.state().document(index) else {
            return;
        };
        let path = document.path.clone();
        let mut icons = Vec::new();
        let breakpoints = gutter::breakpoint_marks(self.ide.state(), &path);
        for mark in &breakpoints {
            let (tone, name) = match mark.kind {
                BreakpointKind::Line => (Tone::Error, "Breakpoint"),
                BreakpointKind::Conditional => (Tone::Warning, "Conditional breakpoint"),
                BreakpointKind::Logpoint => (Tone::Accent, "Logpoint"),
            };
            icons.push(
                GutterIcon::new(
                    mark.line,
                    Glyph::Breakpoint,
                    format!("{name} on line {}", mark.line + 1),
                )
                .lane(BREAKPOINT_LANE)
                .tone(tone),
            );
        }
        // Hovering a line number previews the breakpoint a click would set.
        if let Some(hover) = self.gutter.hover
            && matches!(
                hover.area,
                GutterArea::LineNumber | GutterArea::Icon(BREAKPOINT_LANE)
            )
            && !breakpoints.iter().any(|mark| mark.line == hover.line)
        {
            icons.push(
                GutterIcon::new(
                    hover.line,
                    Glyph::Breakpoint,
                    format!("Toggle breakpoint on line {}", hover.line + 1),
                )
                .lane(BREAKPOINT_LANE)
                .tone(Tone::Disabled),
            );
        }
        for mark in marks {
            icons.push(
                GutterIcon::new(mark.line, gutter::run_glyph(mark.kind), mark.label.clone())
                    .lane(RUN_LANE)
                    .tone(Tone::Run),
            );
        }
        self.views[index].icons = icons;
    }

    /// Takes what the kit's editor reported about the gutter for a mouse event: the hover and
    /// the clicks.
    pub(crate) fn gutter_events(
        &mut self,
        rt: &mut Runtime,
        hover: Option<GutterHover>,
        clicks: Vec<GutterClick>,
    ) {
        self.gutter.hover = hover;
        for click in clicks {
            self.gutter_click(rt, click);
        }
    }

    /// A click in the gutter (TUI-GUI-091): a line number toggles a breakpoint, a right click
    /// on it opens its properties, a run icon opens the run popup, a change bar opens the
    /// change popup.
    fn gutter_click(&mut self, rt: &mut Runtime, click: GutterClick) {
        let Some(index) = self.active_view() else {
            return;
        };
        let Some(path) = self
            .ide
            .state()
            .document(index)
            .map(|document| document.path.clone())
        else {
            return;
        };
        let at = rt
            .pointer()
            .map_or((0, 0), |pointer| (pointer.x, pointer.y));
        let left = click.button == MouseButton::Left;
        let right = click.button == MouseButton::Right;
        match click.area {
            GutterArea::LineNumber | GutterArea::Icon(BREAKPOINT_LANE) if left => {
                self.apply_effects(rt, breakpoint::toggle(&path, click.line));
            }
            GutterArea::LineNumber | GutterArea::Icon(BREAKPOINT_LANE) if right => {
                self.apply_effects(rt, breakpoint::open(&path, click.line));
            }
            GutterArea::Icon(RUN_LANE) if left || right => {
                // Only a line with a run icon has a popup.
                if self
                    .ide
                    .state()
                    .cached_gutter_marks()
                    .is_some_and(|marks| gutter::mark_at(&marks, click.line).is_some())
                {
                    self.open_chrome_popup(
                        rt,
                        Popup::Gutter(click.line),
                        Some((at.0.saturating_add(1), at.1)),
                    );
                }
            }
            GutterArea::ChangeMarker if left || right => {
                self.open_change_popup(click.line, at);
            }
            _ => {}
        }
    }

    /// Opens the change popup of the change on `line`, next to the bar at `at`.
    pub(crate) fn open_change_popup(&mut self, line: usize, at: (u16, u16)) {
        let hunk = {
            let cx = self.ide.cx();
            change::hunk_at(&cx, line)
        };
        if let Some(hunk) = hunk {
            self.gutter.change = Some(ChangeOpen {
                hunk,
                anchor: at,
                rect: Rect::default(),
                buttons: Vec::new(),
            });
        }
    }

    /// Opens the breakpoint properties dialog when the core asks for it
    /// (`debug.editBreakpoint`: a right click on a number, Edit Breakpoint).
    pub(crate) fn sync_breakpoint_form(&mut self, rt: &mut Runtime) {
        if self.gutter.form_open || !self.layers.is_empty() {
            return;
        }
        let Some((path, line)) = breakpoint::editing(self.ide.state()) else {
            return;
        };
        let form = BreakpointForm::load(self.ide.state(), &path, line);
        let shared: Rc<RefCell<Vec<String>>> = Rc::default();
        let fields: Vec<(&str, &str, &str)> = breakpoint::FIELDS
            .iter()
            .map(|field| (field.label, field.hint, form.value(field.id)))
            .collect();
        let content = FormContent::new(&fields, shared.clone());
        let dialog = Dialog::new(form.title())
            .width(64)
            .primary(breakpoint::DONE)
            .tertiary(breakpoint::REMOVE)
            .content(content)
            .on_exit(move |code| {
                let mut form = form.clone();
                for (field, value) in breakpoint::FIELDS.iter().zip(shared.borrow().iter()) {
                    form.set(field.id, value.clone());
                }
                vec![Msg::Breakpoint(code, form)]
            });
        self.gutter.form_open = true;
        self.layers.push(rt, dialog);
    }

    /// The answer of the breakpoint dialog.
    pub(crate) fn breakpoint_answer(
        &mut self,
        rt: &mut Runtime,
        code: ExitCode,
        form: BreakpointForm,
    ) {
        let effects = match code {
            ExitCode::Primary => form.done(),
            ExitCode::Tertiary => form.remove(),
            _ => vec![breakpoint::close()],
        };
        // The flag stays up while the commands run: the first of them must not open the form
        // again (the core still has it open until `debug.closeBreakpointEditor`).
        self.apply_effects(rt, effects);
        self.gutter.form_open = false;
    }

    // ------------------------------------------------------------------ change popup

    /// Keys and the mouse of the open change popup. Returns whether it took the event.
    pub(crate) fn change_event(&mut self, rt: &mut Runtime, event: &Event) -> bool {
        let Some(open) = &self.gutter.change else {
            return false;
        };
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                self.gutter.change = None;
                // Escape only closes; any other key closes it and works as usual.
                key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE
            }
            Event::Mouse(_) => {
                let Some(pointer) = rt.pointer().copied() else {
                    return true;
                };
                let position = Position::new(pointer.x, pointer.y);
                let inside = open.rect.contains(position);
                let button = open
                    .buttons
                    .iter()
                    .find(|(_, rect)| rect.contains(position))
                    .map(|(tool, _)| *tool);
                match pointer.kind {
                    PointerKind::Down(_) if !inside => {
                        self.gutter.change = None;
                        true
                    }
                    PointerKind::Up(MouseButton::Left) => {
                        if let Some(tool) = button {
                            self.change_tool(rt, tool);
                        }
                        inside
                    }
                    _ => inside,
                }
            }
            _ => false,
        }
    }

    /// Uses a tool of the change popup.
    fn change_tool(&mut self, rt: &mut Runtime, tool: Tool) {
        let Some(hunk) = self.gutter.change.as_ref().map(|open| open.hunk) else {
            return;
        };
        let step = {
            let cx = self.ide.cx();
            change::change_popup(&cx, hunk).map(|popup| popup.step(&cx, tool))
        };
        let Some(step) = step else {
            self.gutter.change = None;
            return;
        };
        if let Some(next) = step.select
            && let Some(open) = self.gutter.change.as_mut()
        {
            open.hunk = next;
        }
        if step.close {
            self.gutter.change = None;
        }
        self.apply_effects(rt, step.effects);
    }

    /// Draws the change popup (a toolbar row and the old lines) next to its bar, above the
    /// editor.
    pub(crate) fn draw_change_popup(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        let Some(open) = &self.gutter.change else {
            return;
        };
        let (hunk, anchor) = (open.hunk, open.anchor);
        let model = {
            let cx = self.ide.cx();
            change::change_popup(&cx, hunk)
        };
        let Some(model) = model else {
            self.gutter.change = None;
            return;
        };
        let (rect, buttons) = paint_change_popup(rt, &model, anchor, area, buf);
        if let Some(open) = self.gutter.change.as_mut() {
            open.rect = rect;
            open.buttons = buttons;
        }
    }
}

/// The width of a button of the toolbar row, with one cell of padding at each side.
fn button_width(tool: &change::ChangeTool) -> u16 {
    match tool.label {
        Some(label) => width(label) + 2,
        None => 3,
    }
}

/// Paints `model` and returns the popup's rectangle and the rectangle of each enabled or
/// disabled button.
fn paint_change_popup(
    rt: &mut Runtime,
    model: &ChangePopup,
    anchor: (u16, u16),
    area: Rect,
    buf: &mut Buffer,
) -> (Rect, Vec<(Tool, Rect)>) {
    let look = rt.look.clone();
    let c = look.theme.colors;
    let popup_bg = look.over(c.popup, c.popup);
    let border = look.over(c.popup_border, c.popup);
    let text = look.over(c.text, c.popup);
    let secondary = look.over(c.text_secondary, c.popup);
    let disabled = look.over(c.text_disabled, c.popup);
    let hover_bg = look.over(c.hover, c.popup);
    let code_bg = look.color(look.theme.editor.background);
    let code_fg = look.color(look.theme.editor.text);

    // Size: the toolbar row, and the old lines below it.
    let toolbar_min: u16 = model
        .tools
        .iter()
        .map(|tool| button_width(tool) + 1)
        .sum::<u16>()
        + 2;
    let code_w = model
        .old_lines
        .iter()
        .map(|line| width(line))
        .max()
        .unwrap_or(0)
        + 2;
    let inner_w = toolbar_min.max(code_w).max(24);
    let w = (inner_w + 2).min(area.width);
    let code_rows = model.old_lines.len() as u16 + u16::from(model.more > 0);
    let h = (3 + code_rows).min(area.height);
    // Right of the bar, kept inside the editor.
    let x = (anchor.0 + 2)
        .min(area.right().saturating_sub(w))
        .max(area.x);
    let mut y = anchor.1;
    if y + h > area.bottom() {
        y = area.bottom().saturating_sub(h).max(area.y);
    }
    let rect = Rect::new(x, y, w, h);
    fill(buf, rect, Style::default().bg(popup_bg).fg(text));
    rt.hits.add(rect, CHANGE_POPUP);
    // The rounded border in the popup border color.
    let edge = Style::default().fg(border).bg(popup_bg);
    for dx in 1..rect.width.saturating_sub(1) {
        put(buf, rect, rect.x + dx, rect.y, "─", edge);
        put(buf, rect, rect.x + dx, rect.bottom() - 1, "─", edge);
    }
    for dy in 1..rect.height.saturating_sub(1) {
        put(buf, rect, rect.x, rect.y + dy, "│", edge);
        put(buf, rect, rect.right() - 1, rect.y + dy, "│", edge);
    }
    put(buf, rect, rect.x, rect.y, "╭", edge);
    put(buf, rect, rect.right() - 1, rect.y, "╮", edge);
    put(buf, rect, rect.x, rect.bottom() - 1, "╰", edge);
    put(buf, rect, rect.right() - 1, rect.bottom() - 1, "╯", edge);

    // The toolbar row: groups left to right, the last group (Close) at the right edge.
    let row_y = rect.y + 1;
    let mut buttons = Vec::new();
    let mut bx = rect.x + 1;
    let mut group = 0;
    let right_edge = rect.right() - 1;
    for tool in &model.tools {
        if tool.group != group {
            group = tool.group;
            bx += 1;
        }
        let bw = button_width(tool);
        let button_x = if tool.tool == Tool::Close {
            right_edge.saturating_sub(bw)
        } else {
            bx
        };
        let button = Rect::new(button_x, row_y, bw, 1).intersection(rect);
        let id = CHANGE_POPUP.with(1 + buttons.len() as u64);
        rt.hits.add(button, id);
        let hovered = tool.enabled && rt.hovered() == Some(id);
        let background = if hovered { hover_bg } else { popup_bg };
        let color = match (tool.enabled, tool.label.is_some()) {
            (false, _) => disabled,
            (true, true) => text,
            (true, false) => secondary,
        };
        let style = Style::default().fg(color).bg(background);
        fill(buf, button, style);
        let label = match (tool.label, tool.glyph) {
            (Some(label), _) => label.to_owned(),
            (None, Some(glyph)) => look.glyph(match glyph {
                archipel_kit_model::Glyph::ChevronUp => Glyph::ChevronUp,
                archipel_kit_model::Glyph::ChevronDown => Glyph::ChevronDown,
                _ => Glyph::Close,
            }),
            (None, None) => String::new(),
        };
        put(buf, button, button.x + 1, button.y, &label, style);
        rt.tooltip
            .offer(id, button, Tooltip::new(tool.tooltip.clone()));
        buttons.push((tool.tool, button));
        if tool.tool != Tool::Close {
            bx += bw + 1;
        }
    }
    // The old lines, in the editor's colors.
    let code_style = Style::default().fg(code_fg).bg(code_bg);
    let code_area = Rect::new(
        rect.x + 1,
        rect.y + 2,
        rect.width.saturating_sub(2),
        code_rows,
    );
    if code_rows > 0 {
        fill(buf, code_area.intersection(rect), code_style);
    }
    for (row, line) in model.old_lines.iter().enumerate() {
        put(
            buf,
            code_area.intersection(rect),
            code_area.x + 1,
            code_area.y + row as u16,
            &clip(line, code_area.width.saturating_sub(2)),
            code_style,
        );
    }
    if model.more > 0 {
        put(
            buf,
            code_area.intersection(rect),
            code_area.x + 1,
            code_area.y + model.old_lines.len() as u16,
            &format!("… {} more lines", model.more),
            code_style.add_modifier(Modifier::ITALIC),
        );
    }
    (rect, buttons)
}
