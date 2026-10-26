//! A minimal HTTP/1.1 file server for provisioning tests: serves fixed bodies,
//! honours `Range: bytes=n-`, can cut a response short once, and logs requests.
// Each test binary uses a different subset of the helpers.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone, Default)]
struct Route {
    body: Vec<u8>,
    /// Close the connection after this many body bytes, once.
    cut_after: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub path: String,
    pub range: Option<String>,
}

#[derive(Clone, Default)]
struct Shared {
    routes: Arc<Mutex<HashMap<String, Route>>>,
    log: Arc<Mutex<Vec<Request>>>,
}

pub struct FakeServer {
    pub base: String,
    shared: Shared,
}

impl FakeServer {
    pub fn start() -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let shared = Shared::default();
        let server = shared.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = server.clone();
                thread::spawn(move || handle(stream, &shared));
            }
        });
        FakeServer { base, shared }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    pub fn serve(&self, path: &str, body: impl Into<Vec<u8>>) {
        self.shared.routes.lock().unwrap().insert(
            path.into(),
            Route {
                body: body.into(),
                cut_after: None,
            },
        );
    }

    /// Serves `body` but drops the connection after `bytes` on the first request.
    pub fn serve_interrupted(&self, path: &str, body: impl Into<Vec<u8>>, bytes: usize) {
        self.shared.routes.lock().unwrap().insert(
            path.into(),
            Route {
                body: body.into(),
                cut_after: Some(bytes),
            },
        );
    }

    pub fn requests(&self, path: &str) -> Vec<Request> {
        self.shared
            .log
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.path == path || r.path.split('?').next() == Some(path))
            .cloned()
            .collect()
    }
}

fn handle(mut stream: TcpStream, shared: &Shared) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_owned();
    let mut range = None;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("range")
        {
            range = Some(value.trim().to_owned());
        }
    }
    shared.log.lock().unwrap().push(Request {
        path: path.clone(),
        range: range.clone(),
    });
    let route = {
        let mut routes = shared.routes.lock().unwrap();
        // Exact target first, then the path without its query string.
        let key = if routes.contains_key(&path) {
            path.clone()
        } else {
            path.split('?').next().unwrap_or(&path).to_owned()
        };
        let Some(route) = routes.get_mut(&key) else {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            return;
        };
        let current = route.clone();
        route.cut_after = None;
        current
    };
    let start = range
        .as_deref()
        .and_then(|r| r.strip_prefix("bytes="))
        .and_then(|r| r.trim_end_matches('-').parse::<usize>().ok());
    let total = route.body.len();
    let (status, body, extra) = match start {
        Some(start) if start >= total => {
            let header = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(header.as_bytes());
            return;
        }
        Some(start) => (
            "206 Partial Content",
            &route.body[start..],
            format!("Content-Range: bytes {start}-{}/{total}\r\n", total - 1),
        ),
        None => ("200 OK", &route.body[..], String::new()),
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n{extra}Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let send = route
        .cut_after
        .map_or(body.len(), |cut| cut.min(body.len()));
    let _ = stream.write_all(&body[..send]);
    let _ = stream.flush();
}

/// A `tar.gz` with one top-level directory holding `bin/<name>` (mode 755).
pub fn tool_archive(root: &str, name: &str, payload: &[u8]) -> Vec<u8> {
    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::fast(),
    ));
    let mut header = tar::Header::new_gnu();
    header.set_size(payload.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(&mut header, format!("{root}/bin/{name}"), payload)
        .unwrap();
    builder.into_inner().unwrap().finish().unwrap()
}
