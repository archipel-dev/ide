//! Android Debug Bridge client: device listing, live device tracking and
//! per-device properties, all through the installed `adb` executable.

use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdbError {
    #[error(
        "cannot run {program}: {source}. Install Android SDK Platform-Tools or set the SDK location."
    )]
    Spawn {
        program: PathBuf,
        source: std::io::Error,
    },
    #[error("adb {args} failed: {message}")]
    Failed { args: String, message: String },
}

/// Connection state reported by `adb devices`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceState {
    Online,
    Offline,
    Unauthorized,
    Authorizing,
    Connecting,
    Recovery,
    Sideload,
    Bootloader,
    Other(String),
}

impl DeviceState {
    fn parse(text: &str) -> Self {
        match text {
            "device" => DeviceState::Online,
            "offline" => DeviceState::Offline,
            "unauthorized" => DeviceState::Unauthorized,
            "authorizing" => DeviceState::Authorizing,
            "connecting" => DeviceState::Connecting,
            "recovery" => DeviceState::Recovery,
            "sideload" => DeviceState::Sideload,
            "bootloader" => DeviceState::Bootloader,
            other => DeviceState::Other(other.to_owned()),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            DeviceState::Online => "Online",
            DeviceState::Offline => "Offline",
            DeviceState::Unauthorized => "Unauthorized",
            DeviceState::Authorizing => "Authorizing",
            DeviceState::Connecting => "Connecting",
            DeviceState::Recovery => "Recovery",
            DeviceState::Sideload => "Sideload",
            DeviceState::Bootloader => "Bootloader",
            DeviceState::Other(text) => text,
        }
    }
}

/// A device or emulator known to the adb server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub serial: String,
    pub state: DeviceState,
    /// `model:` from `adb devices -l` (underscores instead of spaces).
    pub model: Option<String>,
    pub product: Option<String>,
    pub transport_id: Option<String>,
    /// Filled in by [`Adb::describe`] once the device is online.
    pub properties: Option<DeviceProperties>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceProperties {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    /// `ro.build.version.sdk`, e.g. `35`.
    pub api_level: Option<String>,
    /// `ro.build.version.release`, e.g. `15`.
    pub release: Option<String>,
    /// AVD name for emulators (`adb emu avd name`).
    pub avd_name: Option<String>,
}

impl Device {
    pub fn is_emulator(&self) -> bool {
        self.serial.starts_with("emulator-")
    }

    pub fn is_online(&self) -> bool {
        self.state == DeviceState::Online
    }

    /// Name shown in the device selector, e.g. `Pixel 8 API 35` for an
    /// emulator or `Google Pixel 7` for a phone.
    pub fn display_name(&self) -> String {
        let props = self.properties.as_ref();
        if let Some(avd) = props.and_then(|p| p.avd_name.as_deref()) {
            return avd.replace('_', " ");
        }
        let model = props
            .and_then(|p| p.model.clone())
            .or_else(|| self.model.as_ref().map(|m| m.replace('_', " ")));
        match (props.and_then(|p| p.manufacturer.as_deref()), model) {
            (Some(maker), Some(model))
                if !model.to_lowercase().starts_with(&maker.to_lowercase()) =>
            {
                format!("{} {model}", capitalize(maker))
            }
            (_, Some(model)) => model,
            _ => self.serial.clone(),
        }
    }

    pub fn api_level(&self) -> Option<&str> {
        self.properties.as_ref()?.api_level.as_deref()
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Parses `adb devices -l` output (also the payload of `track-devices -l`).
pub fn parse_devices(text: &str) -> Vec<Device> {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty() && !line.starts_with("List of devices") && !line.starts_with('*')
        })
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let serial = fields.next()?.to_owned();
            let state = DeviceState::parse(fields.next()?);
            let mut device = Device {
                serial,
                state,
                model: None,
                product: None,
                transport_id: None,
                properties: None,
            };
            for field in fields {
                match field.split_once(':') {
                    Some(("model", value)) => device.model = Some(value.to_owned()),
                    Some(("product", value)) => device.product = Some(value.to_owned()),
                    Some(("transport_id", value)) => device.transport_id = Some(value.to_owned()),
                    _ => {}
                }
            }
            Some(device)
        })
        .collect()
}

/// Parses `getprop` output: `[key]: [value]` lines.
pub fn parse_getprop(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.trim().split_once("]: [")?;
            Some((
                key.strip_prefix('[')?.to_owned(),
                value.strip_suffix(']')?.to_owned(),
            ))
        })
        .collect()
}

/// Splits the `track-devices` stream into snapshots. Each message is a
/// four-digit hex length followed by that many bytes of `adb devices` text.
#[derive(Debug, Default)]
pub struct TrackParser {
    buffer: Vec<u8>,
}

impl TrackParser {
    /// Feeds bytes and returns every complete snapshot. Malformed length
    /// prefixes discard the buffer rather than failing.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Vec<Device>> {
        self.buffer.extend_from_slice(bytes);
        let mut snapshots = Vec::new();
        loop {
            if self.buffer.len() < 4 {
                break;
            }
            let Some(length) = std::str::from_utf8(&self.buffer[..4])
                .ok()
                .and_then(|hex| usize::from_str_radix(hex, 16).ok())
            else {
                self.buffer.clear();
                break;
            };
            if self.buffer.len() < 4 + length {
                break;
            }
            let payload = String::from_utf8_lossy(&self.buffer[4..4 + length]).into_owned();
            self.buffer.drain(..4 + length);
            snapshots.push(parse_devices(&payload));
        }
        snapshots
    }
}

/// Frames `payload` the way the adb server does in `track-devices`.
pub fn frame_track_message(payload: &str) -> String {
    format!("{:04x}{payload}", payload.len())
}

/// Spawns `command`, retrying briefly while the executable is busy (ETXTBSY):
/// a script that was just written can stay open in a child forked by another
/// thread for a moment.
pub(crate) fn spawn(command: &mut Command) -> std::io::Result<Child> {
    let mut attempts = 0;
    loop {
        match command.spawn() {
            Err(error)
                if error.kind() == std::io::ErrorKind::ExecutableFileBusy && attempts < 20 =>
            {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            result => return result,
        }
    }
}

/// Handle to an `adb` executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adb {
    pub program: PathBuf,
}

impl Adb {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.stdin(Stdio::null());
        command
    }

    /// Runs `adb <args>` to completion and returns stdout.
    pub fn run(&self, args: &[&str]) -> Result<String, AdbError> {
        let spawn_error = |source| AdbError::Spawn {
            program: self.program.clone(),
            source,
        };
        let output = spawn(
            self.command()
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .and_then(Child::wait_with_output)
        .map_err(spawn_error)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            return Err(AdbError::Failed {
                args: args.join(" "),
                message: if stderr.is_empty() { stdout } else { stderr },
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// `adb devices -l`, without properties.
    pub fn devices(&self) -> Result<Vec<Device>, AdbError> {
        Ok(parse_devices(&self.run(&["devices", "-l"])?))
    }

    /// Reads build properties (and the AVD name for emulators).
    pub fn describe(&self, serial: &str) -> Result<DeviceProperties, AdbError> {
        let props = parse_getprop(&self.run(&["-s", serial, "shell", "getprop"])?);
        let get = |key: &str| props.get(key).filter(|v| !v.is_empty()).cloned();
        let avd_name = if serial.starts_with("emulator-") {
            self.run(&["-s", serial, "emu", "avd", "name"])
                .ok()
                .and_then(|text| {
                    text.lines()
                        .map(str::trim)
                        .find(|line| !line.is_empty() && *line != "OK")
                        .map(str::to_owned)
                })
                .or_else(|| get("ro.boot.qemu.avd_name"))
                .or_else(|| get("ro.kernel.qemu.avd_name"))
        } else {
            None
        };
        Ok(DeviceProperties {
            manufacturer: get("ro.product.manufacturer"),
            model: get("ro.product.model"),
            api_level: get("ro.build.version.sdk"),
            release: get("ro.build.version.release"),
            avd_name,
        })
    }

    /// Asks a running emulator to shut down (`adb -s <serial> emu kill`).
    pub fn kill_emulator(&self, serial: &str) -> Result<(), AdbError> {
        self.run(&["-s", serial, "emu", "kill"]).map(|_| ())
    }

    /// Starts `adb track-devices -l` on a background thread.
    pub fn track(&self) -> Result<DeviceTracker, AdbError> {
        DeviceTracker::start(self.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackEvent {
    /// The complete current device list, with properties of online devices.
    Devices(Vec<Device>),
    /// The tracking connection ended (adb server killed, adb missing, …).
    Ended(String),
}

/// Live device list fed by `adb track-devices`. Dropping it stops tracking.
pub struct DeviceTracker {
    child: Arc<Mutex<Child>>,
    events: Receiver<TrackEvent>,
}

impl DeviceTracker {
    fn start(adb: Adb) -> Result<Self, AdbError> {
        let mut child = spawn(
            adb.command()
                .args(["track-devices", "-l"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .map_err(|source| AdbError::Spawn {
            program: adb.program.clone(),
            source,
        })?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child = Arc::new(Mutex::new(child));
        let (sender, events) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("archipel-ide-adb-track".into())
            .spawn(move || track_loop(adb, stdout, stderr, sender));
        if let Err(error) = spawned {
            if let Ok(mut child) = child.lock() {
                let _ = child.kill();
            }
            return Err(AdbError::Failed {
                args: "track-devices".into(),
                message: error.to_string(),
            });
        }
        Ok(Self { child, events })
    }

    /// Drains pending events without blocking.
    pub fn poll(&self) -> Vec<TrackEvent> {
        self.events.try_iter().collect()
    }

    /// Blocks up to `timeout` for the next event (tests and CLI).
    pub fn wait(&self, timeout: std::time::Duration) -> Option<TrackEvent> {
        self.events.recv_timeout(timeout).ok()
    }
}

impl Drop for DeviceTracker {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn track_loop(
    adb: Adb,
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
    sender: Sender<TrackEvent>,
) {
    let mut parser = TrackParser::default();
    let mut known: BTreeMap<String, DeviceProperties> = BTreeMap::new();
    if let Some(mut stdout) = stdout {
        let mut buffer = [0_u8; 4096];
        loop {
            let read = match stdout.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => read,
            };
            for mut devices in parser.push(&buffer[..read]) {
                known.retain(|serial, _| {
                    devices.iter().any(|d| &d.serial == serial && d.is_online())
                });
                for device in devices.iter_mut().filter(|d| d.is_online()) {
                    if !known.contains_key(&device.serial)
                        && let Ok(props) = adb.describe(&device.serial)
                    {
                        known.insert(device.serial.clone(), props);
                    }
                    device.properties = known.get(&device.serial).cloned();
                }
                if sender.send(TrackEvent::Devices(devices)).is_err() {
                    return;
                }
            }
        }
    }
    let mut message = String::new();
    if let Some(mut stderr) = stderr {
        let _ = stderr.read_to_string(&mut message);
    }
    let message = message.trim();
    let _ = sender.send(TrackEvent::Ended(if message.is_empty() {
        "adb device tracking stopped".into()
    } else {
        message.to_owned()
    }));
}

/// Locates `adb`: the SDK's platform-tools first, then `PATH`.
pub fn find_adb(sdk_root: Option<&Path>) -> Option<PathBuf> {
    let exe = if cfg!(windows) { "adb.exe" } else { "adb" };
    sdk_root
        .map(|root| root.join("platform-tools").join(exe))
        .filter(|path| path.is_file())
        .or_else(|| find_on_path(exe))
}

pub(crate) fn find_on_path(exe: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(exe))
            .find(|path| path.is_file())
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::time::Duration;

    const DEVICES: &str = "List of devices attached\n\
        emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:1\n\
        R5CT1234ABC            unauthorized usb:1-1 transport_id:3\n\
        192.168.1.20:5555      offline transport_id:4\n\n";

    #[test]
    fn parses_devices_long_listing() {
        let devices = parse_devices(DEVICES);
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].serial, "emulator-5554");
        assert!(devices[0].is_emulator() && devices[0].is_online());
        assert_eq!(devices[0].model.as_deref(), Some("sdk_gphone64_x86_64"));
        assert_eq!(devices[0].transport_id.as_deref(), Some("1"));
        assert_eq!(devices[1].state, DeviceState::Unauthorized);
        assert_eq!(devices[2].state, DeviceState::Offline);
        assert!(
            parse_devices("* daemon started successfully\nList of devices attached\n").is_empty()
        );
    }

    #[test]
    fn track_parser_handles_split_and_empty_frames() {
        let stream = format!(
            "{}{}",
            frame_track_message("emulator-5554\tdevice\n"),
            frame_track_message("")
        );
        let mut parser = TrackParser::default();
        let (a, b) = stream.as_bytes().split_at(7);
        assert!(parser.push(a).is_empty());
        let snapshots = parser.push(b);
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0][0].serial, "emulator-5554");
        assert!(snapshots[1].is_empty());
        assert!(parser.push(b"zzzzjunk").is_empty());
    }

    #[test]
    fn display_names() {
        let mut device = parse_devices(DEVICES).remove(0);
        assert_eq!(device.display_name(), "sdk gphone64 x86 64");
        device.properties = Some(DeviceProperties {
            avd_name: Some("Pixel_8_API_35".into()),
            ..Default::default()
        });
        assert_eq!(device.display_name(), "Pixel 8 API 35");
        device.properties = Some(DeviceProperties {
            manufacturer: Some("google".into()),
            model: Some("Pixel 7".into()),
            ..Default::default()
        });
        assert_eq!(device.display_name(), "Google Pixel 7");
        device.properties = Some(DeviceProperties {
            manufacturer: Some("samsung".into()),
            model: Some("SAMSUNG SM-S911B".into()),
            ..Default::default()
        });
        assert_eq!(device.display_name(), "SAMSUNG SM-S911B");
    }

    #[test]
    fn parses_getprop() {
        let props =
            parse_getprop("[ro.build.version.sdk]: [35]\n[ro.product.model]: [Pixel 8]\nnoise\n");
        assert_eq!(props["ro.build.version.sdk"], "35");
        assert_eq!(props["ro.product.model"], "Pixel 8");
    }

    /// Writes an executable fake `adb` that answers the commands Archipel IDE uses.
    #[cfg(unix)]
    pub(crate) fn fake_adb(dir: &Path, track_frames: &[&str]) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let frames: String = track_frames
            .iter()
            .map(|f| frame_track_message(f))
            .collect();
        let script = format!(
            r#"#!/bin/sh
echo "$@" >> "{log}"
case "$*" in
  "track-devices -l") printf '%s' '{frames}'; sleep 0.3; echo 'adb server killed' >&2; exit 1 ;;
  "devices -l") printf 'List of devices attached\nemulator-5554 device product:sdk model:sdk_phone transport_id:1\n' ;;
  "-s emulator-5554 shell getprop") printf '[ro.product.manufacturer]: [Google]\n[ro.product.model]: [sdk_gphone64_x86_64]\n[ro.build.version.sdk]: [35]\n[ro.build.version.release]: [15]\n' ;;
  "-s emulator-5554 emu avd name") printf 'Pixel_8_API_35\r\nOK\r\n' ;;
  "-s emulator-5554 emu kill") printf 'OK: killing emulator, bye bye\n' ;;
  *) echo "error: unknown command $*" >&2; exit 1 ;;
esac
"#,
            log = dir.join("adb.log").display(),
        );
        let path = dir.join("adb");
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    pub(crate) fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("archipel-ide-adb-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    fn tracks_devices_with_fake_adb() {
        let dir = temp("track");
        let adb = Adb::new(fake_adb(
            &dir,
            &[
                "",
                "emulator-5554\toffline\n",
                "emulator-5554\tdevice product:sdk model:sdk transport_id:1\n",
            ],
        ));
        let tracker = adb.track().unwrap();
        let mut snapshots = Vec::new();
        let ended = loop {
            match tracker.wait(Duration::from_secs(5)).expect("event") {
                TrackEvent::Devices(devices) => snapshots.push(devices),
                TrackEvent::Ended(message) => break message,
            }
        };
        assert_eq!(snapshots.len(), 3);
        assert!(snapshots[0].is_empty());
        assert_eq!(snapshots[1][0].state, DeviceState::Offline);
        assert_eq!(
            snapshots[1][0].properties, None,
            "offline devices are not queried"
        );
        let online = &snapshots[2][0];
        assert_eq!(online.display_name(), "Pixel 8 API 35");
        assert_eq!(online.api_level(), Some("35"));
        assert_eq!(ended, "adb server killed");
        assert!(adb.kill_emulator("emulator-5554").is_ok());
        let error = adb.run(&["bogus"]).unwrap_err().to_string();
        assert!(error.contains("unknown command bogus"), "{error}");
        assert_eq!(adb.devices().unwrap()[0].serial, "emulator-5554");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_adb_is_actionable() {
        let error = Adb::new("/nonexistent/adb")
            .devices()
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("Install Android SDK Platform-Tools"),
            "{error}"
        );
        assert!(Adb::new("/nonexistent/adb").track().is_err());
    }
}
