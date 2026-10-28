//! Small drawing helpers on ratatui's buffer (the kit's own are crate-private).

use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthStr;

/// The display width of `text` in cells.
pub fn width(text: &str) -> u16 {
    UnicodeWidthStr::width(text).min(u16::MAX as usize) as u16
}

/// Writes `text` at (`x`, `y`) clipped to `area`; returns the x after the text.
pub fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, text: &str, style: Style) -> u16 {
    if y < area.y || y >= area.bottom() || x >= area.right() {
        return x;
    }
    let room = (area.right() - x) as usize;
    let (end, _) = buf.set_stringn(x, y, text, room, style);
    end
}

/// Fills `rect` with spaces in `style`.
pub fn fill(buf: &mut Buffer, rect: Rect, style: Style) {
    let rect = rect.intersection(buf.area);
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            buf[(x, y)].set_symbol(" ").set_style(style);
        }
    }
}

/// `text` cut to `max` cells, with an ellipsis when it was cut.
pub fn clip(text: &str, max: u16) -> String {
    if width(text) <= max {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if used + w + 1 > max {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('…');
    out
}

/// Makes the underlines of the frame safe for the terminal. The editor gives diagnostics an
/// underline with a color (SGR 58); the runtime redraws those as undercurl when the terminal
/// supports it (`undercurl`). Other terminals do not know SGR 58: xterm reads its parameters
/// as separate attributes (bright backgrounds, blinking text), so the color is dropped and
/// the plain underline takes the color of the text, which is then the severity color.
/// Only the cells inside `area` (the editor) are touched: the chrome's own underlines (the
/// selected tab) keep their color, which the kit's backend handles per terminal.
pub fn finish_underlines(buf: &mut Buffer, area: Rect, undercurl: bool) {
    if undercurl {
        return;
    }
    let area = area.intersection(buf.area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            if cell.underline_color == Color::Reset {
                continue;
            }
            if cell.modifier.contains(Modifier::UNDERLINED) {
                cell.fg = cell.underline_color;
            }
            cell.underline_color = Color::Reset;
        }
    }
}

/// The cells of `buf` that carry an underline color (the editor's diagnostics) and what they
/// show, taken before overlays are drawn.
pub fn colored_underlines(buf: &Buffer) -> Vec<(u16, u16, String, Color)> {
    let mut out = Vec::new();
    for y in buf.area.top()..buf.area.bottom() {
        for x in buf.area.left()..buf.area.right() {
            let cell = &buf[(x, y)];
            if cell.underline_color != Color::Reset {
                out.push((x, y, cell.symbol().to_owned(), cell.bg));
            }
        }
    }
    out
}

/// Overlays (popups, dialogs) fill their area with a style, which keeps the underline of the
/// editor text below. Takes the underline off the `cells` an overlay changed.
pub fn clear_covered_underlines(buf: &mut Buffer, cells: &[(u16, u16, String, Color)]) {
    for (x, y, symbol, bg) in cells {
        let cell = &mut buf[(*x, *y)];
        if cell.symbol() != symbol || cell.bg != *bg {
            cell.modifier.remove(Modifier::UNDERLINED);
            cell.underline_color = Color::Reset;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_overlay_drawn_over_a_diagnostic_does_not_keep_its_underline() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        for x in 0..2 {
            let cell = &mut buf[(x, 0)];
            cell.set_symbol("a");
            cell.modifier |= Modifier::UNDERLINED;
            cell.underline_color = Color::Rgb(255, 0, 0);
        }
        let under = colored_underlines(&buf);
        assert_eq!(under.len(), 2);
        // An overlay fills the first cell.
        buf[(0, 0)].set_symbol(" ").set_bg(Color::Rgb(1, 2, 3));
        clear_covered_underlines(&mut buf, &under);
        assert!(!buf[(0, 0)].modifier.contains(Modifier::UNDERLINED));
        assert_eq!(buf[(0, 0)].underline_color, Color::Reset);
        assert!(buf[(1, 0)].modifier.contains(Modifier::UNDERLINED));
        assert_eq!(buf[(1, 0)].underline_color, Color::Rgb(255, 0, 0));
    }
}
