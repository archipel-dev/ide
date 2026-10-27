//! Screen mirroring by polling `adb exec-out screencap -p` on a background
//! thread. Each successful capture is delivered as PNG bytes.

use crate::adb::Adb;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorEvent {
    /// A complete PNG capture.
    Frame(Vec<u8>),
    /// Capturing failed and the session ended.
    Ended(String),
}

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Pause between captures.
const INTERVAL: Duration = Duration::from_millis(150);

/// A running mirror session; dropping it stops the polling thread.
pub struct ScreenMirror {
    pub serial: String,
    events: crate::Inbox<MirrorEvent>,
    stop: Arc<AtomicBool>,
}

/// One `screencap` capture of `serial`.
pub fn capture(adb: &Adb, serial: &str) -> Result<Vec<u8>, String> {
    let output = Command::new(&adb.program)
        .args(["-s", serial, "exec-out", "screencap", "-p"])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("could not run {}: {e}", adb.program.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if stderr.is_empty() {
            format!("adb screencap exited with {}", output.status)
        } else {
            stderr
        });
    }
    if !output.stdout.starts_with(&PNG_MAGIC) {
        return Err("adb screencap did not return a PNG image".into());
    }
    Ok(output.stdout)
}

impl ScreenMirror {
    pub fn start(adb: Adb, serial: &str) -> Self {
        let (sender, events) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let device = serial.to_owned();
        std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                let message = match capture(&adb, &device) {
                    Ok(png) => MirrorEvent::Frame(png),
                    Err(error) => MirrorEvent::Ended(error),
                };
                let ended = matches!(message, MirrorEvent::Ended(_));
                if sender.send(message).is_err() || ended {
                    return;
                }
                std::thread::sleep(INTERVAL);
            }
        });
        Self {
            serial: serial.to_owned(),
            events: crate::Inbox::new(events),
            stop,
        }
    }

    /// Pending events; consecutive frames collapse to the newest one.
    pub fn poll(&self) -> Vec<MirrorEvent> {
        let mut out: Vec<MirrorEvent> = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            if matches!(event, MirrorEvent::Frame(_))
                && matches!(out.last(), Some(MirrorEvent::Frame(_)))
            {
                out.pop();
            }
            out.push(event);
        }
        out
    }
}

impl Drop for ScreenMirror {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn fake_adb(dir: &std::path::Path, body: &str) -> Adb {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join("adb");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        Adb::new(path)
    }

    #[test]
    fn captures_png_and_reports_failure() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-mirror-{}", std::process::id()));
        let ok = fake_adb(&dir.join("ok"), "printf '\\211PNG\\r\\n\\032\\nrest'");
        let png = capture(&ok, "emulator-5554").unwrap();
        assert!(png.starts_with(&PNG_MAGIC));
        let bad = fake_adb(&dir.join("bad"), "echo 'device offline' >&2; exit 1");
        assert_eq!(capture(&bad, "x").unwrap_err(), "device offline");
        let junk = fake_adb(&dir.join("junk"), "echo hello");
        assert!(capture(&junk, "x").unwrap_err().contains("PNG"));
        let mirror = ScreenMirror::start(ok, "emulator-5554");
        let mut frames = 0;
        for _ in 0..50 {
            frames += mirror
                .poll()
                .iter()
                .filter(|e| matches!(e, MirrorEvent::Frame(_)))
                .count();
            if frames > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(frames > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
