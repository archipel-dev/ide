//! Android Virtual Devices: discovery from the AVD home and emulator launch.
//!
//! AVDs are read from `<avd home>/<name>.ini` and `<name>.avd/config.ini`, the
//! files `avdmanager` and Android Studio write, so listing works without
//! starting the emulator binary.

use crate::{HostEnvironment, properties};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AvdKind {
    Phone,
    Tablet,
    Wear,
    Tv,
    Automotive,
    Desktop,
}

impl AvdKind {
    pub fn label(self) -> &'static str {
        match self {
            AvdKind::Phone => "Phone",
            AvdKind::Tablet => "Tablet",
            AvdKind::Wear => "Wear OS",
            AvdKind::Tv => "TV",
            AvdKind::Automotive => "Automotive",
            AvdKind::Desktop => "Desktop",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Avd {
    /// AVD id passed to `emulator -avd`, e.g. `Pixel_8_API_35`.
    pub name: String,
    /// `avd.ini.displayname`, e.g. `Pixel 8 API 35`.
    pub display_name: String,
    pub api_level: Option<String>,
    pub abi: Option<String>,
    /// System image tag, e.g. `Google APIs`.
    pub tag: Option<String>,
    pub kind: AvdKind,
    pub path: PathBuf,
}

/// Where AVDs live: `ANDROID_AVD_HOME`, `ANDROID_USER_HOME/avd`,
/// `ANDROID_SDK_HOME/.android/avd`, else `~/.android/avd`.
pub fn avd_home(env: &dyn HostEnvironment) -> Option<PathBuf> {
    if let Some(dir) = env.var("ANDROID_AVD_HOME") {
        return Some(PathBuf::from(dir));
    }
    if let Some(dir) = env.var("ANDROID_USER_HOME") {
        return Some(PathBuf::from(dir).join("avd"));
    }
    if let Some(dir) = env.var("ANDROID_SDK_HOME") {
        return Some(PathBuf::from(dir).join(".android/avd"));
    }
    env.home_dir().map(|home| home.join(".android/avd"))
}

/// Lists AVDs in `home`, sorted by display name. Broken entries are skipped.
pub fn list_avds(home: &Path) -> Vec<Avd> {
    let Ok(entries) = std::fs::read_dir(home) else {
        return Vec::new();
    };
    let mut avds: Vec<Avd> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("ini") {
                return None;
            }
            let name = path.file_stem()?.to_str()?.to_owned();
            let ini = properties::parse(&std::fs::read_to_string(&path).ok()?);
            let dir = ini
                .get("path")
                .map(PathBuf::from)
                .filter(|p| p.is_dir())
                .or_else(|| {
                    // path.rel is relative to the parent of the AVD home (~/.android).
                    let rel = ini.get("path.rel")?;
                    home.parent().map(|parent| parent.join(rel))
                })
                .filter(|p| p.is_dir())
                .unwrap_or_else(|| home.join(format!("{name}.avd")));
            let config = properties::parse(&std::fs::read_to_string(dir.join("config.ini")).ok()?);
            Some(avd_from_config(name, &ini, &config, dir))
        })
        .collect();
    avds.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    avds
}

fn avd_from_config(
    name: String,
    ini: &std::collections::BTreeMap<String, String>,
    config: &std::collections::BTreeMap<String, String>,
    path: PathBuf,
) -> Avd {
    let sysdir = config.get("image.sysdir.1").cloned().unwrap_or_default();
    // system-images/android-35/google_apis/x86_64/
    let api_level = sysdir
        .split(['/', '\\'])
        .find_map(|part| part.strip_prefix("android-"))
        .map(str::to_owned)
        .or_else(|| {
            ini.get("target")
                .and_then(|t| t.strip_prefix("android-"))
                .map(str::to_owned)
        });
    let tag_id = config.get("tag.id").map(String::as_str).unwrap_or_default();
    let device = config
        .get("hw.device.name")
        .map(|d| d.to_lowercase())
        .unwrap_or_default();
    let kind = if tag_id.contains("wear") {
        AvdKind::Wear
    } else if tag_id.contains("tv") {
        AvdKind::Tv
    } else if tag_id.contains("automotive") {
        AvdKind::Automotive
    } else if tag_id.contains("desktop") {
        AvdKind::Desktop
    } else if device.contains("tablet") {
        AvdKind::Tablet
    } else {
        AvdKind::Phone
    };
    Avd {
        display_name: config
            .get("avd.ini.displayname")
            .filter(|d| !d.is_empty())
            .cloned()
            .unwrap_or_else(|| name.replace('_', " ")),
        abi: config.get("abi.type").cloned(),
        tag: config.get("tag.display").cloned(),
        api_level,
        kind,
        path,
        name,
    }
}

/// Locates the emulator: the SDK's `emulator/` first, then `PATH`.
pub fn find_emulator(sdk_root: Option<&Path>) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "emulator.exe"
    } else {
        "emulator"
    };
    sdk_root
        .map(|root| root.join("emulator").join(exe))
        .filter(|path| path.is_file())
        .or_else(|| crate::adb::find_on_path(exe))
}

/// An emulator started by the IDE.
pub struct EmulatorProcess {
    pub avd: String,
    child: Child,
    output: Arc<Mutex<String>>,
}

impl EmulatorProcess {
    /// Runs `emulator -avd <name>`. The emulator needs hardware acceleration
    /// (KVM, HVF or WHPX); without it, it exits and [`Self::poll_exit`] reports
    /// its own error output.
    pub fn launch(program: &Path, avd: &str) -> std::io::Result<Self> {
        let mut child = crate::adb::spawn(
            Command::new(program)
                .args(["-avd", avd])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )?;
        let output = Arc::new(Mutex::new(String::new()));
        for stream in [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let output = Arc::clone(&output);
            std::thread::Builder::new()
                .name("archipel-ide-emulator-output".into())
                .spawn(move || collect_tail(stream, output))?;
        }
        Ok(Self {
            avd: avd.to_owned(),
            child,
            output,
        })
    }

    /// `Some((exit code, last output))` once the emulator process has exited.
    pub fn poll_exit(&mut self) -> Option<(Option<i32>, String)> {
        let status = self.child.try_wait().ok().flatten()?;
        // Give the reader threads a moment to drain the pipes.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let output = self.output.lock().map(|o| o.clone()).unwrap_or_default();
        Some((status.code(), output))
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Keeps the last 8 KiB of a stream.
fn collect_tail(mut stream: Box<dyn Read + Send>, output: Arc<Mutex<String>>) {
    let mut buffer = [0_u8; 2048];
    while let Ok(read) = stream.read(&mut buffer) {
        if read == 0 {
            break;
        }
        let Ok(mut text) = output.lock() else { break };
        text.push_str(&String::from_utf8_lossy(&buffer[..read]));
        if text.len() > 8192 {
            let mut cut = text.len() - 8192;
            while !text.is_char_boundary(cut) {
                cut += 1;
            }
            text.drain(..cut);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adb::tests::temp;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn lists_avds_from_ini_files() {
        let android = temp("avd-home");
        let home = android.join("avd");
        write(
            &home.join("Pixel_8_API_35.ini"),
            &format!(
                "avd.ini.encoding=UTF-8\npath={}\npath.rel=avd/Pixel_8_API_35.avd\ntarget=android-35\n",
                home.join("Pixel_8_API_35.avd").display()
            ),
        );
        write(
            &home.join("Pixel_8_API_35.avd/config.ini"),
            "avd.ini.displayname=Pixel 8 API 35\nabi.type=x86_64\nhw.device.name=pixel_8\nimage.sysdir.1=system-images/android-35/google_apis/x86_64/\ntag.display=Google APIs\ntag.id=google_apis\n",
        );
        // Moved AVD: absolute path is stale, path.rel still resolves.
        write(
            &home.join("Wear.ini"),
            "path=/old/machine/Wear.avd\npath.rel=avd/Wear.avd\ntarget=android-34\n",
        );
        write(
            &home.join("Wear.avd/config.ini"),
            "hw.device.name=wearos_large_round\ntag.id=android-wear\n",
        );
        write(&home.join("Broken.ini"), "path=/nowhere\n");
        let avds = list_avds(&home);
        assert_eq!(avds.len(), 2);
        assert_eq!(avds[0].name, "Pixel_8_API_35");
        assert_eq!(avds[0].display_name, "Pixel 8 API 35");
        assert_eq!(avds[0].api_level.as_deref(), Some("35"));
        assert_eq!(avds[0].kind, AvdKind::Phone);
        assert_eq!(avds[0].tag.as_deref(), Some("Google APIs"));
        assert_eq!(avds[1].display_name, "Wear");
        assert_eq!(avds[1].api_level.as_deref(), Some("34"));
        assert_eq!(avds[1].kind, AvdKind::Wear);
        assert!(list_avds(&android.join("missing")).is_empty());
        std::fs::remove_dir_all(android).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn launch_reports_early_exit_output() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp("emulator");
        let program = dir.join("emulator");
        std::fs::write(
            &program,
            "#!/bin/sh\necho \"ERROR | x86_64 emulation currently requires hardware acceleration! ($2)\" >&2\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut process = EmulatorProcess::launch(&program, "Pixel_8_API_35").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let (code, output) = loop {
            if let Some(exit) = process.poll_exit() {
                break exit;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(code, Some(1));
        assert!(
            output.contains("hardware acceleration! (Pixel_8_API_35)"),
            "{output}"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
