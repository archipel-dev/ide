//! Small drawing helpers on ratatui's buffer (the kit's own are crate-private).

use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::ratatui::style::Style;
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
