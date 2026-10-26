//! Spawning and supervising an agent CLI.
//!
//! The process is started from an argument vector, never through a shell, with a cleared
//! environment plus an explicit list. stdout and stderr are read on their own threads and
//! delivered through a callback (the engine puts them on a channel the UI polls), stdin is
//! fed by a writer thread, so nothing here can block the UI thread. On Unix the agent leads
//! its own process group, so stopping it also stops the tools it started.

use std::ffi::OsString;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Longest stdout/stderr line kept; the rest of an over-long line is dropped.
pub const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;
/// How long to wait for the output readers after the process ended.
const READER_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct SpawnSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    /// The complete environment of the process (it is cleared first).
    pub env: Vec<(OsString, OsString)>,
}

/// Something the process produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Line(String),
    Stderr(String),
    /// The process ended and its output is drained. `None`: killed by a signal.
    Exited(Option<i32>),
}

/// A running agent process.
pub struct Proc {
    child: Arc<Mutex<Child>>,
    stdin: Mutex<Option<Sender<String>>>,
    pid: u32,
    exited: Arc<AtomicBool>,
}

impl std::fmt::Debug for Proc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Proc").field("pid", &self.pid).finish()
    }
}

/// Starts the process. `sink` receives every line, stderr line and the exit, from other
/// threads.
pub fn spawn(spec: &SpawnSpec, sink: Arc<dyn Fn(Body) + Send + Sync>) -> io::Result<Proc> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .env_clear()
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn()?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take();

    let mut readers = Vec::new();
    if let Some(stdout) = stdout {
        let sink = sink.clone();
        readers.push(thread::spawn(move || {
            read_lines(stdout, |line| sink(Body::Line(line)));
        }));
    }
    if let Some(stderr) = stderr {
        let sink = sink.clone();
        readers.push(thread::spawn(move || {
            read_lines(stderr, |line| sink(Body::Stderr(line)));
        }));
    }
    let stdin_tx = stdin.map(|mut stdin| {
        let (tx, rx) = mpsc::channel::<String>();
        thread::spawn(move || {
            for line in rx {
                if stdin
                    .write_all(line.as_bytes())
                    .and_then(|()| stdin.write_all(b"\n"))
                    .and_then(|()| stdin.flush())
                    .is_err()
                {
                    break;
                }
            }
            // Dropping `stdin` closes the pipe: the agent sees end of input.
        });
        tx
    });

    let child = Arc::new(Mutex::new(child));
    let exited = Arc::new(AtomicBool::new(false));
    {
        let child = child.clone();
        let exited = exited.clone();
        thread::spawn(move || {
            let code = loop {
                let status = match child.lock() {
                    Ok(mut child) => child.try_wait(),
                    Err(_) => break None,
                };
                match status {
                    Ok(Some(status)) => break status.code(),
                    Ok(None) => thread::sleep(Duration::from_millis(15)),
                    Err(_) => break None,
                }
            };
            exited.store(true, Ordering::SeqCst);
            // Deliver the rest of the output first. A background process that inherited
            // the pipes must not hold the session hostage.
            let deadline = Instant::now() + READER_GRACE;
            while readers.iter().any(|r| !r.is_finished()) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            sink(Body::Exited(code));
        });
    }
    Ok(Proc {
        child,
        stdin: Mutex::new(stdin_tx),
        pid,
        exited,
    })
}

/// Reads `\n`-separated lines without ever failing on invalid UTF-8 or growing without
/// bound.
fn read_lines(reader: impl io::Read, mut on_line: impl FnMut(String)) {
    let mut reader = io::BufReader::with_capacity(64 * 1024, reader);
    let mut line: Vec<u8> = Vec::new();
    let mut overflow = false;
    loop {
        let (consumed, newline) = match reader.fill_buf() {
            Ok([]) => break,
            Ok(buf) => match buf.iter().position(|b| *b == b'\n') {
                Some(at) => {
                    if !overflow {
                        push_capped(&mut line, &buf[..at], &mut overflow);
                    }
                    (at + 1, true)
                }
                None => {
                    if !overflow {
                        push_capped(&mut line, buf, &mut overflow);
                    }
                    (buf.len(), false)
                }
            },
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        reader.consume(consumed);
        if newline {
            if !overflow {
                let text = String::from_utf8_lossy(&line);
                on_line(text.trim_end_matches('\r').to_owned());
            }
            line.clear();
            overflow = false;
        }
    }
    if !line.is_empty() && !overflow {
        on_line(String::from_utf8_lossy(&line).into_owned());
    }
}

fn push_capped(line: &mut Vec<u8>, bytes: &[u8], overflow: &mut bool) {
    if line.len() + bytes.len() > MAX_LINE_BYTES {
        *overflow = true;
        line.clear();
    } else {
        line.extend_from_slice(bytes);
    }
}

impl Proc {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    /// Queues one line for the agent's stdin. Never blocks; `Err` when stdin is closed.
    pub fn write_line(&self, line: &str) -> io::Result<()> {
        let guard = self
            .stdin
            .lock()
            .map_err(|_| io::Error::other("stdin lock poisoned"))?;
        match guard.as_ref() {
            Some(tx) => tx
                .send(line.to_owned())
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "the agent closed its input")),
            None => Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdin is closed")),
        }
    }

    /// Closes stdin (end of input), asks the process group to terminate and kills it after
    /// `grace`. Returns immediately.
    pub fn stop(&self, grace: Duration) {
        if let Ok(mut stdin) = self.stdin.lock() {
            stdin.take();
        }
        if self.is_exited() {
            return;
        }
        if grace.is_zero() {
            self.kill();
            return;
        }
        signal_group(self.pid, "TERM");
        let child = self.child.clone();
        let exited = self.exited.clone();
        let pid = self.pid;
        thread::spawn(move || {
            let deadline = Instant::now() + grace;
            while !exited.load(Ordering::SeqCst) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            if !exited.load(Ordering::SeqCst) {
                kill_now(pid, &child);
            }
        });
    }

    /// Kills the process group at once.
    pub fn kill(&self) {
        if let Ok(mut stdin) = self.stdin.lock() {
            stdin.take();
        }
        if !self.is_exited() {
            kill_now(self.pid, &self.child);
        }
    }

    /// Waits until the process is gone; `false` on timeout.
    pub fn wait_exit(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while !self.is_exited() {
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(5));
        }
        true
    }
}

fn kill_now(pid: u32, child: &Mutex<Child>) {
    signal_group(pid, "KILL");
    if let Ok(mut child) = child.lock() {
        let _ = child.kill();
    }
}

#[cfg(unix)]
fn signal_group(pid: u32, signal: &str) {
    // `kill` is run without a shell; a negative pid addresses the whole process group.
    let _ = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg("--")
        .arg(format!("-{pid}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(unix))]
fn signal_group(_pid: u32, _signal: &str) {
    // No graceful signal off Unix; `kill_now` and `stop` fall back to killing the child.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_survive_invalid_utf8_and_missing_final_newline() {
        let mut lines = Vec::new();
        read_lines(&b"one\r\ntw\xffo\nlast"[..], |l| lines.push(l));
        assert_eq!(lines, ["one", "tw\u{fffd}o", "last"]);
    }

    #[test]
    fn overlong_lines_are_dropped_not_buffered() {
        let mut data = vec![b'x'; MAX_LINE_BYTES + 10];
        data.extend_from_slice(b"\nok\n");
        let mut lines = Vec::new();
        read_lines(&data[..], |l| lines.push(l));
        assert_eq!(lines, ["ok"]);
    }

    #[cfg(unix)]
    #[test]
    fn echo_round_trip_and_exit_code() {
        let (tx, rx) = mpsc::channel();
        let sink: Arc<dyn Fn(Body) + Send + Sync> = {
            let tx = Mutex::new(tx);
            Arc::new(move |b| {
                let _ = tx.lock().unwrap().send(b);
            })
        };
        let spec = SpawnSpec {
            program: "sh".into(),
            args: vec!["-c".into(), "read a; echo got:$a; echo err >&2; exit 3".into()],
            cwd: std::env::temp_dir(),
            env: vec![("PATH".into(), std::env::var_os("PATH").unwrap_or_default())],
        };
        let proc = spawn(&spec, sink).unwrap();
        proc.write_line("hello").unwrap();
        let mut seen = Vec::new();
        loop {
            let body = rx.recv_timeout(Duration::from_secs(10)).unwrap();
            let done = matches!(body, Body::Exited(_));
            seen.push(body);
            if done {
                break;
            }
        }
        assert!(seen.contains(&Body::Line("got:hello".into())));
        assert!(seen.contains(&Body::Stderr("err".into())));
        assert_eq!(seen.last(), Some(&Body::Exited(Some(3))));
        assert!(proc.write_line("late").is_err() || proc.is_exited());
    }

    #[cfg(unix)]
    #[test]
    fn stop_terminates_and_the_environment_is_cleared() {
        let (tx, rx) = mpsc::channel();
        let sink: Arc<dyn Fn(Body) + Send + Sync> = {
            let tx = Mutex::new(tx);
            Arc::new(move |b| {
                let _ = tx.lock().unwrap().send(b);
            })
        };
        let spec = SpawnSpec {
            program: "sh".into(),
            args: vec!["-c".into(), "echo [${HOME}]; sleep 30".into()],
            cwd: std::env::temp_dir(),
            env: vec![("PATH".into(), std::env::var_os("PATH").unwrap_or_default())],
        };
        // HOME is set in this process but not passed on: the child must not see it.
        let proc = spawn(&spec, sink).unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)).unwrap(),
            Body::Line("[]".into())
        );
        proc.stop(Duration::from_secs(5));
        assert!(proc.wait_exit(Duration::from_secs(10)));
    }
}
