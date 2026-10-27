//! The Terminal tool window: the screen of a pseudo-terminal session drawn into cells.

use crate::draw::fill;
use archipel_ide_terminal::{Color as TermColor, Screen};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::style::{Color, Modifier, Style};
use archipel_kit_tui::runtime::{CursorShape, Runtime};

fn color(color: TermColor, default: Color) -> Color {
    match color {
        TermColor::Default => default,
        TermColor::Idx(index) => Color::Indexed(index),
        TermColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Draws `screen` in `area` on the background `on`; the cursor shows when `focused`.
pub fn draw(
    rt: &mut Runtime,
    area: Rect,
    on: Color,
    screen: &Screen,
    focused: bool,
    buf: &mut Buffer,
) {
    let p = rt.look.palette;
    fill(buf, area, Style::default().bg(on).fg(p.white));
    let (rows, cols) = screen.size();
    for row in 0..rows.min(area.height) {
        let mut col = 0;
        while col < cols.min(area.width) {
            let Some(cell) = screen.cell(row, col) else {
                col += 1;
                continue;
            };
            if cell.is_wide_continuation() {
                col += 1;
                continue;
            }
            let mut fg = color(cell.fgcolor(), p.white);
            let mut bg = color(cell.bgcolor(), on);
            if cell.inverse() {
                std::mem::swap(&mut fg, &mut bg);
            }
            let mut modifier = Modifier::empty();
            if cell.bold() {
                modifier |= Modifier::BOLD;
            }
            if cell.italic() {
                modifier |= Modifier::ITALIC;
            }
            if cell.underline() {
                modifier |= Modifier::UNDERLINED;
            }
            let text = cell.contents();
            let x = area.x + col;
            let y = area.y + row;
            let target = &mut buf[(x, y)];
            target
                .set_symbol(if text.is_empty() { " " } else { text })
                .set_style(Style::default().fg(fg).bg(bg).add_modifier(modifier));
            col += if cell.is_wide() { 2 } else { 1 };
        }
    }
    if focused && !screen.hide_cursor() {
        let (row, col) = screen.cursor_position();
        if row < area.height && col < area.width {
            rt.set_cursor(area.x + col, area.y + row, CursorShape::Bar);
        }
    }
}
