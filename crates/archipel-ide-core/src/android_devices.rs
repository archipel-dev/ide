//! Connected devices, the device selector and the Device Manager
//! (`android.devices.*`, `android.avd.*`, `android.emulator.*`).

use crate::{AndroidState, IdeState, spec, string_param};
use archipel_ide_android::{
    Adb, Avd, Device, DeviceTracker, EmulatorProcess, SystemEnvironment, TrackEvent, avd_home,
    find_adb, find_emulator, list_avds,
};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Devices reported by adb, AVDs on disk, and emulators started by the IDE.
#[derive(Default)]
pub struct DeviceHub {
    pub devices: Vec<Device>,
    /// Serial of the device chosen in the toolbar selector.
    pub selected: Option<String>,
    pub avds: Vec<Avd>,
    /// Last device/emulator problem, shown in the Device Manager.
    pub error: Option<String>,
    /// Overrides the AVD home (tests); `None` uses the standard locations.
    pub avd_home: Option<PathBuf>,
    tracker: Option<DeviceTracker>,
    emulators: Vec<EmulatorProcess>,
}

impl DeviceHub {
    pub fn is_tracking(&self) -> bool {
        self.tracker.is_some()
    }

    pub fn selected_device(&self) -> Option<&Device> {
        let serial = self.selected.as_deref()?;
        self.devices.iter().find(|d| d.serial == serial)
    }

    /// The online device running `avd`, if any.
    pub fn running_avd(&self, avd: &str) -> Option<&Device> {
        self.devices.iter().find(|d| {
            d.is_online()
                && d.properties
                    .as_ref()
                    .and_then(|p| p.avd_name.as_deref())
                    .is_some_and(|name| name == avd)
        })
    }

    /// Whether the IDE launched `avd` and it has not exited or come online yet.
    pub fn is_launching(&self, avd: &str) -> bool {
        self.emulators.iter().any(|e| e.avd == avd) && self.running_avd(avd).is_none()
    }

    fn set_devices(&mut self, devices: Vec<Device>) {
        self.devices = devices;
        let still_present = self
            .selected
            .as_deref()
            .is_some_and(|serial| self.devices.iter().any(|d| d.serial == serial));
        if !still_present {
            self.selected = self
                .devices
                .iter()
                .find(|d| d.is_online())
                .or(self.devices.first())
                .map(|d| d.serial.clone());
        }
    }

    /// Applies tracker events and emulator exits. Returns whether anything changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let events = self
            .tracker
            .as_ref()
            .map(DeviceTracker::poll)
            .unwrap_or_default();
        for event in events {
            changed = true;
            match event {
                TrackEvent::Devices(devices) => {
                    self.set_devices(devices);
                    self.error = None;
                }
                TrackEvent::Ended(message) => {
                    self.tracker = None;
                    self.error = Some(format!("Device tracking stopped: {message}"));
                }
            }
        }
        let mut exited = Vec::new();
        self.emulators
            .retain_mut(|emulator| match emulator.poll_exit() {
                Some((code, output)) => {
                    exited.push((emulator.avd.clone(), code, output));
                    false
                }
                None => true,
            });
        for (avd, code, output) in exited {
            changed = true;
            if code != Some(0) {
                let last = output
                    .lines()
                    .rev()
                    .find(|line| line.contains("ERROR") || line.contains("FATAL"))
                    .or_else(|| output.lines().rev().find(|l| !l.trim().is_empty()))
                    .unwrap_or("no output")
                    .trim()
                    .to_owned();
                self.error = Some(format!(
                    "Emulator {avd} exited with code {}: {last}",
                    code.map_or("?".into(), |c| c.to_string())
                ));
            }
        }
        changed
    }
}

impl Drop for DeviceHub {
    fn drop(&mut self) {
        // Emulators belong to the user's session; only stop tracking.
        self.tracker = None;
    }
}

impl AndroidState {
    fn sdk_root(&mut self) -> Option<PathBuf> {
        if self.sdk.is_none() {
            let root = self.root.clone();
            self.detect_sdk(root.as_deref());
        }
        self.sdk
            .as_ref()
            .and_then(|d| d.sdk.as_ref())
            .map(|sdk| sdk.root.clone())
    }

    pub(crate) fn adb(&mut self) -> Result<Adb, String> {
        let root = self.sdk_root();
        find_adb(root.as_deref()).map(Adb::new).ok_or_else(|| {
            "adb not found. Install Android SDK Platform-Tools (sdkmanager \"platform-tools\") \
             or set the Android SDK location."
                .into()
        })
    }

    /// Starts `adb track-devices` if it is not running.
    pub fn ensure_device_tracking(&mut self) -> Result<(), String> {
        if self.devices.is_tracking() {
            return Ok(());
        }
        let result = self
            .adb()
            .and_then(|adb| adb.track().map_err(|e| e.to_string()));
        match result {
            Ok(tracker) => {
                self.devices.tracker = Some(tracker);
                self.devices.error = None;
                Ok(())
            }
            Err(error) => {
                self.devices.error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn refresh_avds(&mut self) {
        let home = self
            .devices
            .avd_home
            .clone()
            .or_else(|| avd_home(&SystemEnvironment));
        self.devices.avds = home.as_deref().map(list_avds).unwrap_or_default();
    }

    /// `avdmanager create avd`; `image` is a `system-images;…` package path.
    pub fn create_avd(
        &mut self,
        name: &str,
        image: &str,
        device: Option<&str>,
    ) -> Result<(), String> {
        let valid = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ';' | ' '))
        };
        if !valid(name) || name.contains(' ') || name.contains(';') {
            return Err("AVD name may only contain letters, digits, '_', '-' and '.'".into());
        }
        if !valid(image) || !image.starts_with("system-images;") {
            return Err("System image must be a system-images;android-NN;… package path".into());
        }
        let avdmanager = self
            .android_sdk_tool("avdmanager")
            .ok_or("avdmanager not found. Install it with sdkmanager \"cmdline-tools;latest\".")?;
        let mut command = std::process::Command::new(&avdmanager);
        command
            .args(["create", "avd", "--force", "-n", name, "-k", image])
            .stdin(std::process::Stdio::null());
        if let Some(device) = device.filter(|d| !d.is_empty()) {
            command.args(["-d", device]);
        }
        let output = command
            .output()
            .map_err(|e| format!("cannot run {}: {e}", avdmanager.display()))?;
        if !output.status.success() {
            let text = String::from_utf8_lossy(&output.stderr);
            let text = if text.trim().is_empty() {
                String::from_utf8_lossy(&output.stdout)
            } else {
                text
            };
            return Err(format!("avdmanager failed: {}", text.trim()));
        }
        self.refresh_avds();
        Ok(())
    }

    /// `adb pair <host:port> <code>`; returns adb's confirmation line.
    pub fn pair_device(&mut self, address: &str, code: &str) -> Result<String, String> {
        let valid_address = address
            .rsplit_once(':')
            .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok());
        if !valid_address || address.chars().any(char::is_whitespace) {
            return Err("Enter the pairing address as host:port, e.g. 192.168.1.20:37123".into());
        }
        if code.is_empty() || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err("The pairing code is the 6 digits shown on the device".into());
        }
        let adb = self.adb()?;
        let output = adb
            .run(&["pair", address, code])
            .map_err(|e| e.to_string())?;
        let _ = self.ensure_device_tracking();
        Ok(output.trim().to_owned())
    }

    fn android_sdk_tool(&self, name: &str) -> Option<std::path::PathBuf> {
        self.sdk.as_ref()?.sdk.as_ref()?.tool(name)
    }

    pub fn start_emulator(&mut self, avd: &str) -> Result<(), String> {
        if self.devices.avds.is_empty() {
            self.refresh_avds();
        }
        if !self.devices.avds.iter().any(|a| a.name == avd) {
            return Err(format!("no virtual device named {avd}"));
        }
        if self.devices.running_avd(avd).is_some() || self.devices.is_launching(avd) {
            return Err(format!("{avd} is already running"));
        }
        let root = self.sdk_root();
        let program = find_emulator(root.as_deref()).ok_or_else(|| {
            "Android Emulator not found. Install it with sdkmanager \"emulator\".".to_owned()
        })?;
        let process = EmulatorProcess::launch(&program, avd)
            .map_err(|e| format!("cannot start {}: {e}", program.display()))?;
        self.devices.emulators.push(process);
        self.devices.error = None;
        let _ = self.ensure_device_tracking();
        Ok(())
    }

    pub fn stop_emulator(&mut self, avd: &str) -> Result<(), String> {
        if let Some(serial) = self.devices.running_avd(avd).map(|d| d.serial.clone()) {
            self.adb()?
                .kill_emulator(&serial)
                .map_err(|e| e.to_string())?;
        } else if let Some(index) = self.devices.emulators.iter().position(|e| e.avd == avd) {
            self.devices.emulators.remove(index).kill();
        } else {
            return Err(format!("{avd} is not running"));
        }
        Ok(())
    }
}

impl IdeState {
    pub(crate) fn device_manager_opened(&mut self) {
        // The Device Manager shares the right anchor with plugin tool windows.
        self.plugin_host.tool_window = None;
        self.android.refresh_avds();
        let _ = self.android.ensure_device_tracking();
    }

    /// Polls device tracking; called every frame by the UI.
    pub fn poll_android(&mut self) -> bool {
        let devices = self.android.devices.poll();
        let logcat = self.android.logcat.poll();
        let mirror = self.android.mirror.poll();
        let deploy = self.poll_deploy();
        let sdk_repo = self.poll_sdk_repo();
        devices || logcat || mirror || deploy || sdk_repo
    }
}

fn devices_json(state: &IdeState) -> Value {
    json!({
        "devices": state.android.devices.devices.iter().map(|d| json!({
            "serial": d.serial,
            "state": d.state.label(),
            "name": d.display_name(),
            "apiLevel": d.api_level(),
            "emulator": d.is_emulator(),
        })).collect::<Vec<_>>(),
        "selected": state.android.devices.selected,
        "tracking": state.android.devices.is_tracking(),
        "error": state.android.devices.error,
    })
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.devices.list",
                "List Android Devices",
                "Android",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                if !state.android.devices.is_tracking() {
                    // One-shot listing for headless clients.
                    let adb = state.android.adb().map_err(CommandError::Failed)?;
                    let mut devices = adb
                        .devices()
                        .map_err(|e| CommandError::Failed(e.to_string()))?;
                    for device in devices.iter_mut().filter(|d| d.is_online()) {
                        device.properties = adb.describe(&device.serial).ok();
                    }
                    state.android.devices.set_devices(devices);
                }
                Ok(devices_json(state))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.devices.track",
                "Restart Device Tracking",
                "Android",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.android.devices.tracker = None;
                state
                    .android
                    .ensure_device_tracking()
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.devices.select",
                "Select Device",
                "Android",
                None,
                r#"{"serial":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let serial = string_param(&params, "serial")?;
                if !state
                    .android
                    .devices
                    .devices
                    .iter()
                    .any(|d| d.serial == serial)
                {
                    return Err(CommandError::InvalidParams(format!(
                        "no connected device {serial}"
                    )));
                }
                state.android.devices.selected = Some(serial);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.avd.list",
                "List Virtual Devices",
                "Android",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.android.refresh_avds();
                Ok(json!({ "avds": state.android.devices.avds }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.avd.create",
                "Create Virtual Device",
                "Android",
                None,
                r#"{"name":"string","systemImage":"string","device":"string?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let name = string_param(&params, "name")?;
                let image = string_param(&params, "systemImage")?;
                let device = params.get("device").and_then(Value::as_str);
                state
                    .android
                    .create_avd(&name, &image, device)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"name": name}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.devices.pair",
                "Pair Devices Using Wi-Fi",
                "Android",
                None,
                r#"{"address":"string","code":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let address = string_param(&params, "address")?;
                let code = string_param(&params, "code")?;
                let message = state
                    .android
                    .pair_device(&address, &code)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"message": message}))
            },
        ))
        .expect("unique built-in id");
    for (id, label) in [
        ("android.emulator.start", "Start Emulator"),
        ("android.emulator.stop", "Stop Emulator"),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Android", None, r#"{"avd":"string"}"#),
                |_| true,
                move |state: &mut IdeState, params| {
                    let avd = string_param(&params, "avd")?;
                    let (result, verb) = if id == "android.emulator.start" {
                        (state.android.start_emulator(&avd), "Starting")
                    } else {
                        (state.android.stop_emulator(&avd), "Stopping")
                    };
                    result.map_err(CommandError::Failed)?;
                    state.status = format!("{verb} {avd}");
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
}

#[cfg(test)]
mod create_pair_tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;

    #[test]
    fn create_and_pair_validate_input() {
        let registry = built_in_commands();
        let mut state = IdeState::default();
        for params in [
            json!({"name": "bad name", "systemImage": "system-images;android-35;google_apis;x86_64"}),
            json!({"name": "ok", "systemImage": "platforms;android-35"}),
            json!({"name": "ok", "systemImage": "system-images;android-35;google_apis;x86_64"}),
        ] {
            let error = registry
                .execute("android.avd.create", &mut state, params)
                .unwrap_err()
                .to_string();
            assert!(!error.is_empty());
        }
        let error = registry
            .execute(
                "android.devices.pair",
                &mut state,
                json!({"address": "nonsense", "code": "123456"}),
            )
            .unwrap_err()
            .to_string();
        assert!(error.contains("host:port"), "{error}");
        let error = registry
            .execute(
                "android.devices.pair",
                &mut state,
                json!({"address": "10.0.0.2:4000", "code": "12ab"}),
            )
            .unwrap_err()
            .to_string();
        assert!(error.contains("6 digits"), "{error}");
    }
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, ToolWindow, built_in_commands};
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-core-devices-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn script(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// SDK with a fake adb that reports one emulator, then keeps tracking.
    fn fake_sdk(root: &Path) {
        let payload = "emulator-5554\tdevice product:sdk model:sdk transport_id:1\n";
        let frame = format!("{:04x}{payload}", payload.len());
        script(
            &root.join("platform-tools/adb"),
            &format!(
                r#"echo "$@" >> "{log}"
case "$*" in
  "track-devices -l") printf '%s' '{frame}'; exec sleep 30 ;;
  "devices -l") printf 'List of devices attached\n{payload}' ;;
  "-s emulator-5554 shell getprop") printf '[ro.build.version.sdk]: [35]\n' ;;
  "-s emulator-5554 emu avd name") printf 'Pixel_8_API_35\nOK\n' ;;
  "-s emulator-5554 emu kill") echo OK ;;
  *) exit 1 ;;
esac
"#,
                log = root.join("adb.log").display()
            ),
        );
        script(
            &root.join("emulator/emulator"),
            "echo 'ERROR | x86_64 emulation currently requires hardware acceleration!' >&2\nexit 1\n",
        );
    }

    fn avd(home: &Path, name: &str, display: &str) {
        let dir = home.join(format!("{name}.avd"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            home.join(format!("{name}.ini")),
            format!("path={}\ntarget=android-35\n", dir.display()),
        )
        .unwrap();
        std::fs::write(
            dir.join("config.ini"),
            format!("avd.ini.displayname={display}\nimage.sysdir.1=system-images/android-35/google_apis/x86_64/\n"),
        )
        .unwrap();
    }

    fn wait_until(state: &mut IdeState, condition: impl Fn(&IdeState) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !condition(state) {
            assert!(Instant::now() < deadline, "timed out");
            state.poll_android();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn device_manager_tracks_devices_and_controls_emulators() {
        let base = temp("hub");
        let sdk = base.join("sdk");
        fake_sdk(&sdk);
        let home = base.join("avd");
        avd(&home, "Pixel_8_API_35", "Pixel 8 API 35");
        avd(&home, "Tablet_API_34", "Medium Tablet API 34");
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.android.devices.avd_home = Some(home);
        commands
            .execute("android.sdk.setLocation", &mut state, json!({"path": sdk}))
            .unwrap();
        commands
            .execute("toolwindow.activate.deviceManager", &mut state, json!({}))
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::DeviceManager));
        assert_eq!(state.android.devices.avds.len(), 2);
        assert!(state.android.devices.is_tracking());
        wait_until(&mut state, |s| !s.android.devices.devices.is_empty());
        assert_eq!(
            state.android.devices.selected.as_deref(),
            Some("emulator-5554")
        );
        let listed = commands
            .execute("android.devices.list", &mut state, json!({}))
            .unwrap();
        assert_eq!(listed["devices"][0]["name"], "Pixel 8 API 35");
        assert_eq!(listed["devices"][0]["apiLevel"], "35");
        assert!(
            state
                .android
                .devices
                .running_avd("Pixel_8_API_35")
                .is_some()
        );

        assert!(
            commands
                .execute(
                    "android.devices.select",
                    &mut state,
                    json!({"serial": "nope"})
                )
                .is_err()
        );
        let error = commands
            .execute(
                "android.emulator.start",
                &mut state,
                json!({"avd": "Pixel_8_API_35"}),
            )
            .unwrap_err();
        assert!(error.to_string().contains("already running"), "{error}");
        commands
            .execute(
                "android.emulator.stop",
                &mut state,
                json!({"avd": "Pixel_8_API_35"}),
            )
            .unwrap();
        let log = std::fs::read_to_string(sdk.join("adb.log")).unwrap();
        assert!(log.contains("-s emulator-5554 emu kill"), "{log}");

        // Without hardware acceleration the emulator exits; the error is surfaced.
        commands
            .execute(
                "android.emulator.start",
                &mut state,
                json!({"avd": "Tablet_API_34"}),
            )
            .unwrap();
        wait_until(&mut state, |s| s.android.devices.error.is_some());
        let error = state.android.devices.error.clone().unwrap();
        assert!(error.contains("hardware acceleration"), "{error}");
        assert!(
            commands
                .execute(
                    "android.emulator.stop",
                    &mut state,
                    json!({"avd": "Tablet_API_34"})
                )
                .is_err()
        );
        drop(state);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn missing_adb_is_reported() {
        let base = temp("noadb");
        std::fs::create_dir_all(base.join("platforms")).unwrap();
        let mut state = IdeState::default();
        state.set_android_sdk(&base).unwrap();
        assert!(state.android.ensure_device_tracking().is_err() || find_adb_on_path());
        if !find_adb_on_path() {
            assert!(
                state
                    .android
                    .devices
                    .error
                    .as_deref()
                    .unwrap()
                    .contains("Install Android SDK Platform-Tools")
            );
        }
        std::fs::remove_dir_all(base).unwrap();
    }

    fn find_adb_on_path() -> bool {
        archipel_ide_android::find_adb(None).is_some()
    }
}
