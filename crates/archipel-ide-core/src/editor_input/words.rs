//! Word boundaries for caret movement and word deletion (ED-CARET-010 to ED-CARET-012).
//!
//! A word is a run of identifier characters; every other non-blank character is punctuation
//! (a run of the same character is one stop). With CamelHumps, boundaries also lie inside
//! identifiers (`fooBar`, `HTMLParser`, around `_`).

use super::View;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Word,
    Space,
    Newline,
    Punct,
}

fn class(c: char) -> Class {
    if c == '\n' {
        Class::Newline
    } else if c.is_alphanumeric() || c == '_' || c == '$' {
        Class::Word
    } else if c.is_whitespace() {
        Class::Space
    } else {
        Class::Punct
    }
}

/// Whether a CamelHumps boundary lies before `chars[i]` inside the word `chars[..]`.
fn hump_before(chars: &[char], i: usize) -> bool {
    if i == 0 || i >= chars.len() {
        return false;
    }
    let (prev, cur) = (chars[i - 1], chars[i]);
    let next = chars.get(i + 1).copied();
    if cur == '_' && prev != '_' {
        return true;
    }
    if prev == '_' && cur != '_' {
        return true;
    }
    if prev == '$' && cur.is_alphanumeric() {
        return true;
    }
    if (prev.is_lowercase() || prev.is_ascii_digit()) && cur.is_uppercase() {
        return true;
    }
    prev.is_uppercase() && cur.is_uppercase() && next.is_some_and(char::is_lowercase)
}

fn is_word(c: Option<char>) -> bool {
    c.is_some_and(|c| class(c) == Class::Word)
}

/// The end of the next word-or-hump after `pos` inside a word.
fn word_end(view: &View, pos: usize, camel: bool) -> usize {
    let mut p = pos;
    if camel {
        // Skip the character at `pos` so a boundary at `pos` itself does not stop us.
        p += 1;
        while is_word(view.at(p)) && !hump_before(&view.chars, p) {
            p += 1;
        }
        return p;
    }
    while is_word(view.at(p)) {
        p += 1;
    }
    p
}

fn word_start(view: &View, pos: usize, camel: bool) -> usize {
    let mut p = pos;
    if camel {
        p -= 1;
        while p > 0 && is_word(view.at(p - 1)) && !hump_before(&view.chars, p) {
            p -= 1;
        }
        return p;
    }
    while p > 0 && is_word(view.at(p - 1)) {
        p -= 1;
    }
    p
}

/// Ctrl+Right. With the IDE caret stop policy the caret stops at the end of the word and at
/// the start of the next line; with the Windows policy it stops at the next word start.
pub(crate) fn next_word(view: &View, pos: usize, camel: bool, windows: bool) -> usize {
    let n = view.len();
    if pos >= n {
        return n;
    }
    if view.chars[pos] == '\n' {
        return pos + 1;
    }
    let mut p = pos;
    if windows {
        // Past the current lexeme, then the blanks up to the next one (or the line end).
        p = lexeme_end(view, p, camel);
        while p < n && class(view.chars[p]) == Class::Space {
            p += 1;
        }
        return p;
    }
    while p < n && class(view.chars[p]) == Class::Space {
        p += 1;
    }
    if p >= n || view.chars[p] == '\n' {
        return p;
    }
    lexeme_end(view, p, camel)
}

fn lexeme_end(view: &View, p: usize, camel: bool) -> usize {
    match view.at(p).map(class) {
        Some(Class::Word) => word_end(view, p, camel),
        Some(Class::Punct) => {
            let c = view.chars[p];
            let mut e = p;
            while view.at(e) == Some(c) {
                e += 1;
            }
            e
        }
        _ => p,
    }
}

/// Ctrl+Left: the start of the previous word; at a line start the end of the previous line.
pub(crate) fn previous_word(view: &View, pos: usize, camel: bool, windows: bool) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut p = pos.min(view.len());
    if view.chars[p - 1] == '\n' {
        return p - 1;
    }
    while p > 0 && class(view.chars[p - 1]) == Class::Space {
        p -= 1;
    }
    if p == 0 || view.chars[p - 1] == '\n' {
        return p;
    }
    let _ = windows;
    match class(view.chars[p - 1]) {
        Class::Word => word_start(view, p, camel),
        _ => {
            let c = view.chars[p - 1];
            while p > 0 && view.chars[p - 1] == c {
                p -= 1;
            }
            p
        }
    }
}

/// Ctrl+Delete: up to the end of the word, the start of the next word after blanks, or the end
/// of the line (ED-TYPE-049).
pub(crate) fn delete_word_end(view: &View, pos: usize, camel: bool) -> usize {
    let n = view.len();
    if pos >= n {
        return n;
    }
    if view.chars[pos] == '\n' {
        return pos + 1;
    }
    let mut p = pos;
    if class(view.chars[p]) == Class::Space {
        while p < n && class(view.chars[p]) == Class::Space {
            p += 1;
        }
        return p;
    }
    lexeme_end(view, p, camel)
}

/// Ctrl+Backspace: back to the start of the word, over the blanks before it; never past the
/// line start unless the caret is there.
pub(crate) fn delete_word_start(view: &View, pos: usize, camel: bool) -> usize {
    previous_word(view, pos, camel, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_input::lang::Lang;

    fn view(text: &str) -> View {
        View::of_text(text, Lang::PLAIN)
    }

    #[test]
    fn camel_humps_stop_inside_identifiers() {
        let v = view("fooBarBaz");
        assert_eq!(next_word(&v, 0, true, false), 3);
        assert_eq!(next_word(&v, 3, true, false), 6);
        assert_eq!(next_word(&v, 0, false, false), 9);
        assert_eq!(previous_word(&v, 9, true, false), 6);
        let v = view("HTMLParser");
        assert_eq!(next_word(&v, 0, true, false), 4);
        let v = view("foo_bar");
        assert_eq!(next_word(&v, 0, true, false), 3);
    }

    #[test]
    fn ide_policy_stops_at_word_ends_and_line_starts() {
        let v = view("foo bar\nbaz");
        assert_eq!(next_word(&v, 0, false, false), 3);
        assert_eq!(next_word(&v, 3, false, false), 7);
        assert_eq!(next_word(&v, 7, false, false), 8);
        assert_eq!(previous_word(&v, 8, false, false), 7);
        assert_eq!(previous_word(&v, 6, false, false), 4);
        assert_eq!(next_word(&v, 0, false, true), 4);
    }
}
