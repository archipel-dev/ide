//! UI automation control protocol.
//!
//! A running workbench can listen on a local socket (opt-in through
//! [`SOCKET_ENV`]) and accept newline-delimited JSON [`Request`]s, answering each
//! with one [`Response`] line. The protocol drives parity scenes and smoke tests
//! without synthesising OS input. This crate owns the wire format, the
//! listener and a blocking client; it knows nothing about the UI toolkit — the
//! workbench drains [`Pending`] requests from [`ControlServer`] each frame.

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Environment variable naming the socket path the workbench listens on.
pub const SOCKET_ENV: &str = "ARCHIPEL_CONTROL_SOCKET";

/// Default time a `wait` request may block before it fails.
pub const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

/// One request line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Request {
    /// Runs any registered command with JSON arguments.
    Execute {
        command: String,
        #[serde(default)]
        args: Value,
    },
    /// Returns a snapshot of the workbench (project, tabs, tool windows, focus,
    /// editor text and caret, popups).
    State,
    /// Presses and releases a key chord such as `Ctrl+Shift+A` or `Escape`.
    Key { chord: String },
    /// Types text into the focused component.
    Type { text: String },
    /// Clicks at window coordinates in logical points (move, press, release),
    /// `count` times for double clicks.
    Click {
        x: f32,
        y: f32,
        #[serde(default)]
        button: PointerButton,
        #[serde(default = "one")]
        count: u8,
    },
    /// Moves the pointer, e.g. to open a submenu on hover.
    Move { x: f32, y: f32 },
    /// Writes a PNG of the window to `path`.
    Screenshot { path: PathBuf },
    /// Blocks until a condition holds or `timeout_ms` elapses.
    Wait {
        until: WaitCondition,
        /// Required for [`WaitCondition::Text`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, rename = "timeoutMs", skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
}

impl Request {
    pub fn wait_timeout(&self) -> Duration {
        match self {
            Request::Wait {
                timeout_ms: Some(ms),
                ..
            } => Duration::from_millis(*ms),
            _ => DEFAULT_WAIT_TIMEOUT,
        }
    }
}

fn one() -> u8 {
    1
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PointerButton {
    #[default]
    Primary,
    Secondary,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WaitCondition {
    /// No background search, task, or screenshot is in flight.
    Idle,
    /// A popup (Search Everywhere, a dialog, a menu, completion) is visible.
    Popup,
    /// The active editor or the status bar contains `text`.
    Text,
    /// The code analysis of the active file has settled: the daemon has analyzed the
    /// current text, its language server is ready and no markup changed for a moment
    /// (IntelliJ's `DaemonCodeAnalyzer` idle). Implies [`WaitCondition::Idle`].
    Analysis,
}

/// One response line: `{"ok":true,"result":…}` or `{"ok":false,"error":"…"}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn ok(result: Value) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: None,
            error: Some(message.into()),
        }
    }
}

/// A keyboard chord parsed from `Mod+Mod+Key` notation.
///
/// Modifiers are `Ctrl`, `Alt` (or `Option`), `Shift` and `Cmd` (or `Meta`,
/// `Super`). `Primary` means ⌘ on macOS and Ctrl elsewhere, so scripts can be
/// written once for both keymaps. The key name is passed through unchanged;
/// the UI validates it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub command: bool,
    pub key: String,
}

impl Chord {
    pub fn parse(chord: &str) -> Result<Self, ControlError> {
        let invalid = |reason: &str| ControlError::InvalidChord(format!("{chord:?}: {reason}"));
        let trimmed = chord.trim();
        if trimmed.is_empty() {
            return Err(invalid("empty chord"));
        }
        // "Ctrl++" names the plus key.
        let (modifiers, key) = match trimmed.strip_suffix("++") {
            Some(rest) => (rest, "+"),
            None => match trimmed.rsplit_once('+') {
                Some((rest, key)) => (rest, key),
                None => ("", trimmed),
            },
        };
        if key.trim().is_empty() {
            return Err(invalid("missing key"));
        }
        let mut parsed = Chord {
            key: key.trim().to_owned(),
            ..Chord::default()
        };
        for modifier in modifiers.split('+').filter(|m| !m.is_empty()) {
            match modifier.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" => parsed.ctrl = true,
                "alt" | "option" | "opt" => parsed.alt = true,
                "shift" => parsed.shift = true,
                "cmd" | "command" | "meta" | "super" => parsed.command = true,
                "primary" => {
                    if cfg!(target_os = "macos") {
                        parsed.command = true;
                    } else {
                        parsed.ctrl = true;
                    }
                }
                other => return Err(invalid(&format!("unknown modifier {other:?}"))),
            }
        }
        Ok(parsed)
    }
}

#[derive(Debug, Error)]
pub enum ControlError {
    #[error("invalid key chord {0}")]
    InvalidChord(String),
    #[error(
        "cannot reach the Archipel IDE control socket at {path}: {source}. Start the IDE with {SOCKET_ENV}={path}"
    )]
    Connect { path: PathBuf, source: io::Error },
    #[error("control socket I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("malformed control message: {0}")]
    Protocol(String),
}

/// A request received by the server, waiting for the UI to answer it.
pub struct Pending {
    pub request: Request,
    reply: Sender<Response>,
}

impl Pending {
    pub fn new(request: Request) -> (Self, Receiver<Response>) {
        let (reply, receiver) = mpsc::channel();
        (Self { request, reply }, receiver)
    }

    /// Sends the answer; a client that already hung up is ignored.
    pub fn respond(self, response: Response) {
        let _ = self.reply.send(response);
    }
}

impl fmt::Debug for Pending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pending")
            .field("request", &self.request)
            .finish()
    }
}

type Waker = std::sync::Arc<dyn Fn() + Send + Sync>;

/// Accepts connections on a local socket and forwards requests to the UI thread.
pub struct ControlServer {
    requests: Receiver<Pending>,
    path: PathBuf,
}

impl ControlServer {
    /// Listens on `path`. A stale socket file left by a crashed instance is
    /// replaced; a live one is an error. `wake` is called whenever a request
    /// arrives so an idle UI can process it.
    pub fn bind(
        path: &Path,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, ControlError> {
        let (sender, requests) = mpsc::channel();
        platform::listen(path, sender, std::sync::Arc::new(wake))?;
        Ok(Self {
            requests,
            path: path.to_owned(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Next request, if any; never blocks.
    pub fn try_recv(&self) -> Option<Pending> {
        self.requests.try_recv().ok()
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Serves one connection: each request line is answered by the UI in order.
fn serve<S: io::Read + Write>(
    stream: S,
    requests: &Sender<Pending>,
    wake: &Waker,
) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(());
        }
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Request>(&line) {
            Err(error) => Response::error(format!("invalid request: {error}")),
            Ok(request) => {
                let (pending, receiver) = Pending::new(request);
                if requests.send(pending).is_err() {
                    Response::error("the IDE is shutting down")
                } else {
                    wake();
                    receiver
                        .recv()
                        .unwrap_or_else(|_| Response::error("the IDE dropped the request"))
                }
            }
        };
        let stream = reader.get_mut();
        serde_json::to_writer(&mut *stream, &response).map_err(io::Error::other)?;
        stream.write_all(b"\n")?;
        stream.flush()?;
    }
}

/// Blocking client for one connection; requests are answered in order.
pub struct ControlClient {
    reader: BufReader<platform::ClientStream>,
}

impl ControlClient {
    pub fn connect(path: &Path) -> Result<Self, ControlError> {
        let stream = platform::connect(path).map_err(|source| ControlError::Connect {
            path: path.to_owned(),
            source,
        })?;
        Ok(Self {
            reader: BufReader::new(stream),
        })
    }

    /// Sends one request and waits for its response. A response line arrives
    /// even for failures; only transport problems are `Err`.
    pub fn send(&mut self, request: &Request) -> Result<Response, ControlError> {
        let stream = self.reader.get_mut();
        let mut line =
            serde_json::to_string(request).map_err(|e| ControlError::Protocol(e.to_string()))?;
        line.push('\n');
        stream.write_all(line.as_bytes())?;
        stream.flush()?;
        let mut answer = String::new();
        if self.reader.read_line(&mut answer)? == 0 {
            return Err(ControlError::Protocol(
                "the IDE closed the connection".into(),
            ));
        }
        serde_json::from_str(&answer).map_err(|e| ControlError::Protocol(e.to_string()))
    }
}

/// Writes tightly packed 8-bit RGBA pixels as a PNG file.
pub fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), ControlError> {
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(ControlError::Protocol(format!(
            "screenshot has {} bytes, expected {expected} for {width}×{height}",
            rgba.len()
        )));
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let file = io::BufWriter::new(std::fs::File::create(path)?);
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| ControlError::Protocol(e.to_string()))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| ControlError::Protocol(e.to_string()))?;
    writer
        .finish()
        .map_err(|e| ControlError::Protocol(e.to_string()))
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::{UnixListener, UnixStream};

    pub type ClientStream = UnixStream;

    pub fn connect(path: &Path) -> io::Result<UnixStream> {
        UnixStream::connect(path)
    }

    pub fn listen(path: &Path, requests: Sender<Pending>, wake: Waker) -> Result<(), ControlError> {
        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            if !metadata.file_type().is_socket() {
                return Err(ControlError::Io(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} exists and is not a socket", path.display()),
                )));
            }
            if UnixStream::connect(path).is_ok() {
                return Err(ControlError::Io(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!(
                        "another Archipel IDE instance is listening on {}",
                        path.display()
                    ),
                )));
            }
            std::fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)?;
        std::thread::Builder::new()
            .name("archipel-ide-control".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let requests = requests.clone();
                    let wake = wake.clone();
                    let _ = std::thread::Builder::new()
                        .name("archipel-ide-control-conn".into())
                        .spawn(move || {
                            let _ = serve(stream, &requests, &wake);
                        });
                }
            })?;
        Ok(())
    }
}

/// Windows: named pipes. A `\\.\pipe\name` path is used as given; any other
/// value names a pipe in the local namespace by its file name (so
/// `ARCHIPEL_CONTROL_SOCKET=archipel-ide` listens on `\\.\pipe\archipel-ide`).
#[cfg(windows)]
mod platform {
    use super::*;
    use interprocess::local_socket::{
        GenericFilePath, GenericNamespaced, Listener, ListenerOptions, Name, Stream, prelude::*,
    };

    pub type ClientStream = Stream;

    pub fn pipe_name(path: &Path) -> io::Result<Name<'static>> {
        if path.to_string_lossy().starts_with(r"\\.\pipe\") {
            path.as_os_str().to_owned().to_fs_name::<GenericFilePath>()
        } else {
            path.file_name()
                .unwrap_or(path.as_os_str())
                .to_owned()
                .to_ns_name::<GenericNamespaced>()
        }
    }

    pub fn connect(path: &Path) -> io::Result<Stream> {
        Stream::connect(pipe_name(path)?)
    }

    pub fn listen(path: &Path, requests: Sender<Pending>, wake: Waker) -> Result<(), ControlError> {
        let listener: Listener = ListenerOptions::new()
            .name(pipe_name(path)?)
            .create_sync()
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AddrInUse {
                    ControlError::Io(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        format!(
                            "another Archipel IDE instance is listening on {}",
                            path.display()
                        ),
                    ))
                } else {
                    ControlError::Io(error)
                }
            })?;
        std::thread::Builder::new()
            .name("archipel-ide-control".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let requests = requests.clone();
                    let wake = wake.clone();
                    let _ = std::thread::Builder::new()
                        .name("archipel-ide-control-conn".into())
                        .spawn(move || {
                            let _ = serve(stream, &requests, &wake);
                        });
                }
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn requests_use_tagged_json_lines() {
        let request: Request =
            serde_json::from_str(r#"{"op":"execute","command":"editor.save"}"#).unwrap();
        assert_eq!(
            request,
            Request::Execute {
                command: "editor.save".into(),
                args: Value::Null
            }
        );
        let wait: Request =
            serde_json::from_str(r#"{"op":"wait","until":"text","text":"fn","timeoutMs":250}"#)
                .unwrap();
        assert_eq!(wait.wait_timeout(), Duration::from_millis(250));
        assert_eq!(
            serde_json::to_value(Request::Key {
                chord: "Escape".into()
            })
            .unwrap(),
            json!({"op": "key", "chord": "Escape"})
        );
        assert!(serde_json::from_str::<Request>(r#"{"op":"explode"}"#).is_err());
        assert_eq!(
            serde_json::from_str::<Request>(r#"{"op":"click","x":18,"y":20}"#).unwrap(),
            Request::Click {
                x: 18.0,
                y: 20.0,
                button: PointerButton::Primary,
                count: 1
            }
        );
        assert!(matches!(
            serde_json::from_str::<Request>(
                r#"{"op":"click","x":1,"y":2,"button":"secondary","count":2}"#
            )
            .unwrap(),
            Request::Click {
                button: PointerButton::Secondary,
                count: 2,
                ..
            }
        ));
        assert_eq!(
            serde_json::to_value(Response::error("nope")).unwrap(),
            json!({"ok": false, "error": "nope"})
        );
    }

    #[test]
    fn chords_parse_modifiers_and_reject_garbage() {
        let chord = Chord::parse("Ctrl+Shift+A").unwrap();
        assert!(chord.ctrl && chord.shift && !chord.alt && !chord.command);
        assert_eq!(chord.key, "A");
        assert_eq!(Chord::parse("Escape").unwrap().key, "Escape");
        assert_eq!(Chord::parse("Ctrl++").unwrap().key, "+");
        assert!(Chord::parse("alt+7").unwrap().alt);
        assert!(Chord::parse("Hyper+A").is_err());
        assert!(Chord::parse("Ctrl+").is_err());
        assert!(Chord::parse("  ").is_err());
        let primary = Chord::parse("Primary+S").unwrap();
        assert!(primary.ctrl || primary.command);
    }

    #[test]
    fn png_writer_validates_buffer_size() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-png-{}", std::process::id()));
        let path = dir.join("shot.png");
        assert!(write_png(&path, 2, 2, &[0; 3]).is_err());
        write_png(&path, 2, 1, &[255; 8]).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn named_pipe_round_trips_requests() {
        let path = PathBuf::from(format!("archipel-ide-test-{}", std::process::id()));
        let server = ControlServer::bind(&path, || {}).unwrap();
        assert!(ControlServer::bind(&path, || {}).is_err(), "pipe is in use");
        let ui = std::thread::spawn(move || {
            loop {
                if let Some(pending) = server.try_recv() {
                    pending.respond(Response::ok(json!("pong")));
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let mut client = ControlClient::connect(&path).unwrap();
        assert_eq!(
            client.send(&Request::State).unwrap(),
            Response::ok(json!("pong"))
        );
        ui.join().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn server_round_trips_requests_and_reports_bad_lines() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-ctl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ctl.sock");
        let server = ControlServer::bind(&path, || {}).unwrap();
        // A second instance on the same live socket is refused.
        assert!(ControlServer::bind(&path, || {}).is_err());
        let ui = std::thread::spawn(move || {
            let mut answered = 0;
            while answered < 2 {
                if let Some(pending) = server.try_recv() {
                    let response = match &pending.request {
                        Request::State => Response::ok(json!({"tabs": []})),
                        other => Response::error(format!("unexpected {other:?}")),
                    };
                    pending.respond(response);
                    answered += 1;
                } else {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            server
        });
        let mut client = ControlClient::connect(&path).unwrap();
        assert_eq!(
            client.send(&Request::State).unwrap(),
            Response::ok(json!({"tabs": []}))
        );
        let failed = client.send(&Request::Type { text: "x".into() }).unwrap();
        assert!(!failed.ok);
        // Malformed lines are answered by the server without reaching the UI.
        let mut raw = std::os::unix::net::UnixStream::connect(&path).unwrap();
        raw.write_all(b"{not json}\n").unwrap();
        let mut answer = String::new();
        BufReader::new(raw).read_line(&mut answer).unwrap();
        assert!(answer.contains("invalid request"));
        let server = ui.join().unwrap();
        drop(server);
        assert!(!path.exists(), "socket file is removed on shutdown");
        // A stale socket file is replaced.
        {
            let _stale = std::os::unix::net::UnixListener::bind(&path).unwrap();
        }
        let again = ControlServer::bind(&path, || {}).unwrap();
        drop(again);
        let _ = std::fs::remove_dir_all(dir);
    }
}
