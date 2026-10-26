//! Line-oriented console model for process output.
//!
//! Child processes write bytes in arbitrary chunks. Build tools additionally emit
//! carriage-return progress lines and ANSI escape sequences even when they are not
//! attached to a terminal. [`ConsoleBuffer`] turns that stream into stable, complete
//! lines: partial lines are held until their newline arrives, a lone `\r` rewinds the
//! current line, escape sequences are removed and blank progress remnants are dropped.

use crate::OutputStream;

/// Upper bound on retained lines; the oldest lines are discarded first.
pub const MAX_CONSOLE_LINES: usize = 50_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleLine {
    pub stream: OutputStream,
    pub text: String,
}

#[derive(Debug, Default, Clone)]
pub struct ConsoleBuffer {
    lines: Vec<ConsoleLine>,
    pending: [Pending; 2],
    dropped: usize,
}

#[derive(Debug, Default, Clone)]
struct Pending {
    text: String,
    /// Inside an escape sequence that started in a previous chunk.
    escape: EscapeState,
    /// A `\r` was seen; the next printable character starts the line over.
    rewind: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum EscapeState {
    #[default]
    None,
    /// Saw ESC.
    Esc,
    /// Inside `ESC [` … final byte.
    Csi,
    /// Inside `ESC ]` … BEL or ST.
    Osc,
    /// Saw ESC inside an OSC (possible ST).
    OscEsc,
}

fn slot(stream: OutputStream) -> usize {
    match stream {
        OutputStream::Stdout => 0,
        OutputStream::Stderr => 1,
    }
}

impl ConsoleBuffer {
    /// Appends a chunk and returns the number of lines completed by it.
    pub fn push(&mut self, stream: OutputStream, text: &str) -> usize {
        let before = self.lines.len() + self.dropped;
        let mut completed = Vec::new();
        let pending = &mut self.pending[slot(stream)];
        for ch in text.chars() {
            match pending.escape {
                EscapeState::Esc => {
                    pending.escape = match ch {
                        '[' => EscapeState::Csi,
                        ']' => EscapeState::Osc,
                        _ => EscapeState::None,
                    };
                    continue;
                }
                EscapeState::Csi => {
                    if ('@'..='~').contains(&ch) {
                        pending.escape = EscapeState::None;
                    }
                    continue;
                }
                EscapeState::Osc => {
                    match ch {
                        '\u{7}' => pending.escape = EscapeState::None,
                        '\u{1b}' => pending.escape = EscapeState::OscEsc,
                        _ => {}
                    }
                    continue;
                }
                EscapeState::OscEsc => {
                    pending.escape = if ch == '\\' {
                        EscapeState::None
                    } else {
                        EscapeState::Osc
                    };
                    continue;
                }
                EscapeState::None => {}
            }
            match ch {
                '\u{1b}' => pending.escape = EscapeState::Esc,
                '\n' => {
                    pending.rewind = false;
                    completed.push(std::mem::take(&mut pending.text));
                }
                '\r' => pending.rewind = true,
                '\t' => {
                    pending.rewind_if_needed();
                    pending.text.push_str("    ");
                }
                ch if ch.is_control() => {}
                ch => {
                    pending.rewind_if_needed();
                    pending.text.push(ch);
                }
            }
        }
        for text in completed {
            self.commit(stream, text);
        }
        self.lines.len() + self.dropped - before
    }

    fn commit(&mut self, stream: OutputStream, text: String) {
        let text = text.trim_end().to_owned();
        // Progress redraws leave blank lines behind; keep a single blank line at most.
        if text.is_empty()
            && self
                .lines
                .last()
                .is_none_or(|previous| previous.text.is_empty())
        {
            return;
        }
        self.lines.push(ConsoleLine { stream, text });
        if self.lines.len() > MAX_CONSOLE_LINES {
            let excess = self.lines.len() - MAX_CONSOLE_LINES;
            self.lines.drain(..excess);
            self.dropped += excess;
        }
    }

    /// Flushes unterminated lines (called when the process exits).
    pub fn finish(&mut self) -> usize {
        let mut count = 0;
        for stream in [OutputStream::Stdout, OutputStream::Stderr] {
            let text = std::mem::take(&mut self.pending[slot(stream)].text);
            if !text.trim().is_empty() {
                self.commit(stream, text);
                count += 1;
            }
            self.pending[slot(stream)] = Pending::default();
        }
        count
    }

    /// Completed lines, oldest first.
    pub fn lines(&self) -> &[ConsoleLine] {
        &self.lines
    }

    /// Text of the unterminated line on `stream` (for live progress display).
    pub fn partial(&self, stream: OutputStream) -> &str {
        &self.pending[slot(stream)].text
    }

    /// Lines discarded because of [`MAX_CONSOLE_LINES`].
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Pending {
    fn rewind_if_needed(&mut self) {
        if self.rewind {
            self.text.clear();
            self.rewind = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(buffer: &ConsoleBuffer) -> Vec<&str> {
        buffer
            .lines()
            .iter()
            .map(|line| line.text.as_str())
            .collect()
    }

    #[test]
    fn joins_lines_split_across_chunks() {
        let mut buffer = ConsoleBuffer::default();
        assert_eq!(buffer.push(OutputStream::Stdout, "Hello, Gr"), 0);
        assert_eq!(buffer.partial(OutputStream::Stdout), "Hello, Gr");
        assert_eq!(buffer.push(OutputStream::Stdout, "adle!\nnext"), 1);
        buffer.finish();
        assert_eq!(texts(&buffer), ["Hello, Gradle!", "next"]);
    }

    #[test]
    fn carriage_return_overwrites_progress_and_crlf_is_a_newline() {
        let mut buffer = ConsoleBuffer::default();
        buffer.push(
            OutputStream::Stdout,
            "<====> 10%\r<=====> 50%\r> Task :compileJava\r\nok\r\n",
        );
        assert_eq!(texts(&buffer), ["> Task :compileJava", "ok"]);
    }

    #[test]
    fn strips_ansi_sequences_even_when_split() {
        let mut buffer = ConsoleBuffer::default();
        buffer.push(OutputStream::Stderr, "\u{1b}[1;3");
        buffer.push(OutputStream::Stderr, "1merror\u{1b}[0m: boom\n");
        buffer.push(
            OutputStream::Stdout,
            "\u{1b}]8;;http://x\u{7}link\u{1b}]8;;\u{1b}\\\n",
        );
        assert_eq!(texts(&buffer), ["error: boom", "link"]);
        assert_eq!(buffer.lines()[0].stream, OutputStream::Stderr);
    }

    #[test]
    fn collapses_runs_of_blank_lines() {
        let mut buffer = ConsoleBuffer::default();
        buffer.push(OutputStream::Stdout, "\n\na\n\n\n\nb\n   \n\n");
        assert_eq!(texts(&buffer), ["a", "", "b", ""]);
    }

    #[test]
    fn keeps_streams_independent() {
        let mut buffer = ConsoleBuffer::default();
        buffer.push(OutputStream::Stdout, "out-");
        buffer.push(OutputStream::Stderr, "err\n");
        buffer.push(OutputStream::Stdout, "line\n");
        assert_eq!(texts(&buffer), ["err", "out-line"]);
    }

    #[test]
    fn bounds_retained_lines() {
        let mut buffer = ConsoleBuffer::default();
        let text: String = (0..MAX_CONSOLE_LINES + 10)
            .map(|index| format!("{index}\n"))
            .collect();
        buffer.push(OutputStream::Stdout, &text);
        assert_eq!(buffer.lines().len(), MAX_CONSOLE_LINES);
        assert_eq!(buffer.dropped(), 10);
        assert_eq!(buffer.lines()[0].text, "10");
    }
}
