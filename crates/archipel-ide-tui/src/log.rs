//! A scrolling list of text lines with a selection: the Run console and the Problems list.

use crate::draw::{clip, fill, put};
use archipel_kit_tui::components::RowIcon;
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::{Event, KeyCode, KeyEventKind, KeyModifiers};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::style::{Color, Modifier, Style};
use archipel_kit_tui::runtime::Runtime;

/// How a line is colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Normal,
    Muted,
    Error,
    Warning,
    Info,
    Success,
}

/// One line of a [`LogView`].
#[derive(Debug, Clone, Default)]
pub struct Line {
    pub text: String,
    pub tone: Tone,
    pub icon: Option<RowIcon>,
    /// Dim text after the line (a position).
    pub detail: String,
    /// Indentation in cells.
    pub indent: u16,
}

impl Line {
    pub fn new(text: impl Into<String>, tone: Tone) -> Self {
        Self {
            text: text.into(),
            tone,
            ..Self::default()
        }
    }
}

/// What a [`LogView`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEvent {
    None,
    /// Enter or a double click on a line.
    Activated(usize),
}

/// Scroll position and selection.
#[derive(Debug, Clone)]
pub struct LogView {
    pub scroll: usize,
    pub selected: usize,
    /// Keep the last line in view while lines are added.
    pub follow: bool,
    rows: usize,
    len: usize,
}

impl Default for LogView {
    fn default() -> Self {
        Self {
            scroll: 0,
            selected: 0,
            follow: false,
            rows: 1,
            len: 0,
        }
    }
}

const ROW: Id = Id::new("tui.log.row");

impl LogView {
    /// A view that follows the end of the output.
    pub fn following() -> Self {
        Self {
            follow: true,
            ..Self::default()
        }
    }

    fn clamp(&mut self) {
        let last = self.len.saturating_sub(1);
        self.selected = self.selected.min(last);
        let max_scroll = self.len.saturating_sub(self.rows);
        self.scroll = self.scroll.min(max_scroll);
    }

    fn reveal(&mut self) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + self.rows {
            self.scroll = self.selected + 1 - self.rows;
        }
    }

    /// Draws `lines` in `area`.
    pub fn show(
        &mut self,
        rt: &mut Runtime,
        id: Id,
        area: Rect,
        lines: &[Line],
        on: Color,
        buf: &mut Buffer,
    ) {
        let p = rt.look.palette;
        self.rows = area.height.max(1) as usize;
        self.len = lines.len();
        if self.follow {
            self.scroll = self.len.saturating_sub(self.rows);
            self.selected = self.len.saturating_sub(1);
        }
        self.clamp();
        fill(buf, area, Style::default().bg(on).fg(p.white));
        rt.widget(id, area);
        let focused = rt.focused(id);
        for (offset, index) in (self.scroll..lines.len()).take(self.rows).enumerate() {
            let line = &lines[index];
            let y = area.y + offset as u16;
            let chosen = index == self.selected && !self.follow;
            let bg = if chosen {
                if focused { p.one_bg3 } else { p.black2 }
            } else {
                on
            };
            let fg = match line.tone {
                Tone::Normal => p.white,
                Tone::Muted => p.grey_fg,
                Tone::Error => p.red,
                Tone::Warning => p.yellow,
                Tone::Info => p.nord_blue,
                Tone::Success => p.green,
            };
            let row = Rect::new(area.x, y, area.width, 1);
            fill(buf, row, Style::default().bg(bg));
            let mut x = area.x + 1 + line.indent;
            if let Some(icon) = line.icon {
                let (ch, color) = icon.char_and_color(&rt.look);
                put(
                    buf,
                    row,
                    x,
                    y,
                    &ch.to_string(),
                    Style::default().fg(color).bg(bg),
                );
                x += 2;
            }
            let room = area.right().saturating_sub(x + 1);
            let text = clip(&line.text, room);
            let style = Style::default().fg(fg).bg(bg);
            let style = if chosen {
                style.add_modifier(Modifier::BOLD)
            } else {
                style
            };
            let end = put(buf, row, x, y, &text, style);
            if !line.detail.is_empty() {
                put(
                    buf,
                    row,
                    end + 2,
                    y,
                    &clip(&line.detail, area.right().saturating_sub(end + 3)),
                    Style::default().fg(p.grey_fg).bg(bg),
                );
            }
            rt.clickable(ROW.with(index as u64), row);
        }
    }

    /// Applies `event`.
    pub fn handle(&mut self, rt: &mut Runtime, id: Id, event: &Event) -> LogEvent {
        self.clamp();
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release && rt.focused(id) => {
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    return LogEvent::None;
                }
                let before = self.selected;
                match key.code {
                    KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                    KeyCode::Down => self.selected += 1,
                    KeyCode::PageUp => self.selected = self.selected.saturating_sub(self.rows),
                    KeyCode::PageDown => self.selected += self.rows,
                    KeyCode::Home => self.selected = 0,
                    KeyCode::End => self.selected = self.len.saturating_sub(1),
                    KeyCode::Enter => return LogEvent::Activated(self.selected),
                    _ => return LogEvent::None,
                }
                self.follow = false;
                self.clamp();
                if self.selected != before {
                    self.reveal();
                }
                LogEvent::None
            }
            Event::Mouse(_) => {
                let Some(pointer) = rt.pointer().copied() else {
                    return LogEvent::None;
                };
                if pointer.wheel() != 0 {
                    // Only when the pointer is over this view's rows.
                    let over = (self.scroll..self.len.min(self.scroll + self.rows))
                        .any(|i| pointer.over(ROW.with(i as u64)))
                        || pointer.over(id);
                    if over {
                        self.follow = false;
                        let max = self.len.saturating_sub(self.rows);
                        let next = self.scroll as i64 + pointer.wheel() as i64 * 3;
                        self.scroll = next.clamp(0, max as i64) as usize;
                    }
                    return LogEvent::None;
                }
                for index in self.scroll..self.len.min(self.scroll + self.rows) {
                    let row = ROW.with(index as u64);
                    if pointer.pressed_left(row) {
                        self.selected = index;
                        self.follow = false;
                        rt.focus.focus(id);
                        if pointer.clicks >= 2 {
                            return LogEvent::Activated(index);
                        }
                    }
                }
                LogEvent::None
            }
            _ => LogEvent::None,
        }
    }
}
