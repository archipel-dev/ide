use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    path::Path,
    sync::{
        Mutex, PoisonError,
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
};
use thiserror::Error;
pub use vt100::{Color, Screen};

/// Exclusive access to the value of a mutex that nobody else can lock (`&mut` proves it).
fn exclusive<T>(mutex: &mut Mutex<T>) -> &mut T {
    mutex.get_mut().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Default)]
struct Replies(Vec<u8>);
impl vt100::Callbacks for Replies {
    fn unhandled_csi(
        &mut self,
        screen: &mut Screen,
        i1: Option<u8>,
        i2: Option<u8>,
        params: &[&[u16]],
        command: char,
    ) {
        if i2.is_some() {
            return;
        }
        let parameter = params
            .first()
            .and_then(|values| values.first())
            .copied()
            .unwrap_or(0);
        let reply = match (i1, command, parameter) {
            (None, 'n', 5) => "\u{1b}[0n".into(),
            (None, 'n', 6) => {
                let (row, col) = screen.cursor_position();
                format!("\u{1b}[{};{}R", row + 1, col + 1)
            }
            (None, 'c', 0) => "\u{1b}[?1;2c".into(),
            _ => return,
        };
        self.0.extend_from_slice(reply.as_bytes());
    }
}

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("failed to open pseudo-terminal: {0}")]
    Open(String),
    #[error("failed to spawn terminal process: {0}")]
    Spawn(String),
    #[error("terminal IO failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to resize terminal: {0}")]
    Resize(String),
    #[error("terminal input failed: {0}")]
    Send(String),
}

pub struct TerminalSession {
    /// The `Mutex` wrappers only make the session `Sync` (so a service can share it); every use
    /// has `&mut self` and goes through `get_mut`, so no lock is ever taken.
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: SyncSender<Vec<u8>>,
    writer_errors: Mutex<Receiver<String>>,
    child: Box<dyn Child + Send + Sync>,
    output: Mutex<Receiver<Vec<u8>>>,
    transcript: String,
    parser: vt100::Parser<Replies>,
    error: Option<String>,
    exited: bool,
}

impl TerminalSession {
    pub fn spawn_shell(cwd: &Path) -> Result<Self, TerminalError> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "powershell.exe".into()
            } else {
                "/bin/sh".into()
            }
        });
        Self::spawn(&shell, &[], cwd)
    }

    pub fn spawn(program: &str, args: &[&str], cwd: &Path) -> Result<Self, TerminalError> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| TerminalError::Open(error.to_string()))?;
        let mut command = CommandBuilder::new(program);
        command.args(args);
        command.cwd(cwd);
        command.env("TERM", "xterm-256color");
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| TerminalError::Spawn(error.to_string()))?;
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| TerminalError::Open(error.to_string()))?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|error| TerminalError::Open(error.to_string()))?;
        let (sender, output) = mpsc::sync_channel(64);
        thread::Builder::new()
            .name("archipel-ide-terminal-reader".into())
            .spawn(move || {
                let mut buffer = [0_u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => {
                            if sender.send(buffer[..read].to_vec()).is_err() {
                                break;
                            }
                        }
                    }
                }
            })
            .map_err(|error| {
                let _ = child.kill();
                TerminalError::Io(error)
            })?;
        let (input, receiver) = mpsc::sync_channel::<Vec<u8>>(64);
        let (errors, writer_errors) = mpsc::channel();
        thread::Builder::new()
            .name("archipel-ide-terminal-writer".into())
            .spawn(move || {
                while let Ok(bytes) = receiver.recv() {
                    if let Err(error) = writer.write_all(&bytes).and_then(|_| writer.flush()) {
                        let _ = errors.send(error.to_string());
                        break;
                    }
                }
            })
            .map_err(|error| {
                let _ = child.kill();
                TerminalError::Io(error)
            })?;
        Ok(Self {
            master: Mutex::new(pair.master),
            writer: input,
            writer_errors: Mutex::new(writer_errors),
            child,
            output: Mutex::new(output),
            transcript: String::new(),
            parser: vt100::Parser::new_with_callbacks(24, 100, 2000, Replies::default()),
            error: None,
            exited: false,
        })
    }

    pub fn send(&mut self, text: &str) -> Result<(), TerminalError> {
        if text.len() > 1_048_576 {
            return Err(TerminalError::Send(
                "paste is larger than 1 MiB; split it into smaller parts".into(),
            ));
        }
        if self.exited {
            return Err(TerminalError::Send(
                "process exited; restart the shell".into(),
            ));
        }
        self.parser.screen_mut().set_scrollback(0);
        self.writer
            .try_send(text.as_bytes().to_vec())
            .map_err(|error| TerminalError::Send(error.to_string()))
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), TerminalError> {
        let rows = rows.clamp(1, 200);
        let cols = cols.clamp(1, 500);
        if self.parser.screen().size() == (rows, cols) {
            return Ok(());
        }
        exclusive(&mut self.master)
            .resize(PtySize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| TerminalError::Resize(error.to_string()))?;
        self.parser.screen_mut().set_size(rows, cols);
        Ok(())
    }

    pub fn screen(&self) -> &Screen {
        self.parser.screen()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn scroll(&mut self, lines: i64) {
        let offset = self
            .parser
            .screen()
            .scrollback()
            .saturating_add_signed(lines.clamp(-2000, 2000) as isize);
        self.parser.screen_mut().set_scrollback(offset);
    }

    pub fn paste(&mut self, text: &str) -> Result<(), TerminalError> {
        let text = text.replace('\u{1b}', "").replace("\r\n", "\n");
        let text = if self.screen().bracketed_paste() {
            format!("\u{1b}[200~{text}\u{1b}[201~")
        } else {
            text.replace('\n', "\r")
        };
        self.send(&text)
    }

    pub fn poll(&mut self) -> &str {
        // A continuously producing child must not monopolize the UI frame.
        for bytes in exclusive(&mut self.output).try_iter().take(16) {
            self.parser.process(&bytes);
        }
        let replies = std::mem::take(&mut self.parser.callbacks_mut().0);
        if !replies.is_empty()
            && let Err(error) = self.writer.try_send(replies)
        {
            self.error = Some(error.to_string());
        }
        if let Ok(error) = exclusive(&mut self.writer_errors).try_recv() {
            self.error = Some(error);
        }
        self.transcript = self.parser.screen().contents();
        if !self.exited {
            self.exited = self.child.try_wait().ok().flatten().is_some();
        }
        &self.transcript
    }

    pub fn is_exited(&self) -> bool {
        self.exited
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if !self.exited {
            let _ = self.child.kill();
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn captures_output_from_a_real_pty_process() {
        let mut terminal = TerminalSession::spawn(
            "/bin/sh",
            &["-c", "printf 'archipel-ide-pty-ok\\n'"],
            Path::new("/tmp"),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if terminal.poll().contains("archipel-ide-pty-ok") {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("PTY output was not captured: {:?}", terminal.poll());
    }

    #[test]
    fn interactive_input_resizes_and_preserves_ansi_and_unicode() {
        let mut terminal = TerminalSession::spawn("/bin/sh", &["-c", r#"printf '\033[31mready\033[0m\n'; IFS= read -r answer; printf 'received:%s\n' "$answer""#], Path::new("/tmp")).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !terminal.poll().contains("ready") && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            terminal.screen().cell(0, 0).unwrap().fgcolor(),
            Color::Idx(1)
        );
        terminal.resize(12, 40).unwrap();
        assert_eq!(terminal.screen().size(), (12, 40));
        terminal.send("hello 🦀\r").unwrap();
        while Instant::now() < deadline {
            if terminal.poll().contains("received:hello 🦀") {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!(
            "interactive shell did not receive input: {}",
            terminal.poll()
        );
    }

    #[test]
    fn replies_to_cursor_position_queries() {
        let mut parser = vt100::Parser::new_with_callbacks(24, 80, 0, Replies::default());
        parser.process(b"abc\x1b[6n");
        assert_eq!(parser.callbacks().0, b"\x1b[1;4R");
    }

    #[test]
    fn blocked_child_input_is_queued_without_blocking_the_caller() {
        let mut terminal =
            TerminalSession::spawn("/bin/sh", &["-c", "sleep 30"], Path::new("/tmp")).unwrap();
        let start = Instant::now();
        terminal.send(&"x".repeat(100_000)).unwrap();
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn continuous_output_poll_yields_to_the_caller() {
        let mut terminal =
            TerminalSession::spawn("/bin/sh", &["-c", "yes"], Path::new("/tmp")).unwrap();
        thread::sleep(Duration::from_millis(30));
        let start = Instant::now();
        terminal.poll();
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
