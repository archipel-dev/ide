//! Remote SDK repository: parses `sdkmanager --list` and runs installs /
//! uninstalls in the background with streamed progress.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};

/// One package of the repository listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePackage {
    /// `sdkmanager` path, e.g. `platforms;android-35`.
    pub path: String,
    pub version: String,
    pub description: String,
    pub installed: bool,
    pub obsolete: bool,
}

impl RemotePackage {
    pub fn is_system_image(&self) -> bool {
        self.path.starts_with("system-images;")
    }

    /// API level of a platform or system image package (`35`, `36.1`, `36-ext18`).
    pub fn api_level(&self) -> Option<String> {
        let segment = self.path.split(';').nth(1)?;
        segment.strip_prefix("android-").map(str::to_owned)
    }
}

/// Parses `sdkmanager --list` output (installed + available sections). A
/// package listed under both is reported once, as installed.
pub fn parse_list(text: &str) -> Vec<RemotePackage> {
    #[derive(PartialEq)]
    enum Section {
        None,
        Installed,
        Available,
        Other,
    }
    let mut section = Section::None;
    let mut packages: Vec<RemotePackage> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("installed packages") {
            section = Section::Installed;
            continue;
        }
        if lower.starts_with("available packages") {
            section = Section::Available;
            continue;
        }
        if lower.starts_with("available updates") {
            section = Section::Other;
            continue;
        }
        if section == Section::None || section == Section::Other {
            continue;
        }
        let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
        if cells.len() < 3 || cells[0] == "Path" || cells[0].starts_with("---") {
            continue;
        }
        let installed = section == Section::Installed;
        let package = RemotePackage {
            path: cells[0].to_owned(),
            version: cells[1].to_owned(),
            description: cells[2].to_owned(),
            installed,
            obsolete: cells[2].to_ascii_lowercase().contains("obsolete"),
        };
        match packages.iter_mut().find(|p| p.path == package.path) {
            Some(existing) if installed => *existing = package,
            Some(_) => {}
            None => packages.push(package),
        }
    }
    packages
}

/// Whether a system image can run on this host: x86 images need an x86 host,
/// arm images an ARM host (Apple Silicon / aarch64 Linux).
pub fn image_compatible(path: &str, host_arch: &str) -> bool {
    let abi = path.rsplit(';').next().unwrap_or_default();
    match host_arch {
        "aarch64" | "arm64" => abi.starts_with("arm64"),
        _ => abi.starts_with("x86"),
    }
}

/// The default repository channels, as shown on the SDK Update Sites tab.
pub const DEFAULT_UPDATE_SITES: &[(&str, &str)] = &[
    (
        "Android SDK Updates",
        "https://dl.google.com/android/repository/repository2-3.xml",
    ),
    (
        "Android SDK Add-on List",
        "https://dl.google.com/android/repository/addons_list-5.xml",
    ),
    (
        "Google API add-ons and system images",
        "https://dl.google.com/android/repository/sys-img/google_apis/sys-img2-3.xml",
    ),
    (
        "Google Play system images",
        "https://dl.google.com/android/repository/sys-img/google_apis_playstore/sys-img2-3.xml",
    ),
    (
        "Android TV system images",
        "https://dl.google.com/android/repository/sys-img/android-tv/sys-img2-3.xml",
    ),
];

/// Events from a background `sdkmanager` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    /// Progress or output line.
    Line(String),
    Done(Result<String, String>),
}

/// A running `sdkmanager` process.
pub struct SdkJob {
    rx: Receiver<JobEvent>,
}

impl SdkJob {
    pub fn poll(&self) -> Vec<JobEvent> {
        self.rx.try_iter().collect()
    }

    /// Runs `sdkmanager --sdk_root=<root> <args>` on a worker thread.
    pub fn spawn(sdkmanager: &Path, sdk_root: &Path, args: &[String]) -> Self {
        let (tx, rx) = mpsc::channel();
        let program = sdkmanager.to_owned();
        let mut full = vec![format!("--sdk_root={}", sdk_root.display())];
        full.extend(args.iter().cloned());
        std::thread::spawn(move || {
            let spawned = Command::new(&program)
                .args(&full)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            let mut child = match spawned {
                Ok(child) => child,
                Err(error) => {
                    let _ = tx.send(JobEvent::Done(Err(format!(
                        "cannot run {}: {error}",
                        program.display()
                    ))));
                    return;
                }
            };
            let stderr = child.stderr.take();
            let err_tx = tx.clone();
            let err_thread = std::thread::spawn(move || {
                let mut text = String::new();
                if let Some(mut stderr) = stderr {
                    let _ = stderr.read_to_string(&mut text);
                }
                let _ = err_tx;
                text
            });
            let mut all = String::new();
            if let Some(mut stdout) = child.stdout.take() {
                let mut buffer = [0u8; 4096];
                let mut line = String::new();
                while let Ok(n) = stdout.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    for ch in String::from_utf8_lossy(&buffer[..n]).chars() {
                        all.push(ch);
                        // sdkmanager redraws progress with `\r`.
                        if ch == '\n' || ch == '\r' {
                            if !line.trim().is_empty() {
                                let _ = tx.send(JobEvent::Line(line.trim().to_owned()));
                            }
                            line.clear();
                        } else {
                            line.push(ch);
                        }
                    }
                }
            }
            let status = child.wait();
            let stderr = err_thread.join().unwrap_or_default();
            let result = match status {
                Ok(status) if status.success() => Ok(all),
                Ok(_) => {
                    let message = if stderr.trim().is_empty() {
                        all.lines().last().unwrap_or_default().to_owned()
                    } else {
                        stderr.trim().to_owned()
                    };
                    Err(format!("sdkmanager failed: {message}"))
                }
                Err(error) => Err(format!("sdkmanager failed: {error}")),
            };
            let _ = tx.send(JobEvent::Done(result));
        });
        Self { rx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = "\
Installed packages:
  Path                 | Version | Description                | Location
  -------              | ------- | -------                    | -------
  build-tools;35.0.0   | 35.0.0  | Android SDK Build-Tools 35 | build-tools/35.0.0
  platforms;android-35 | 2       | Android SDK Platform 35    | platforms/android-35

Available Packages:
  Path                                      | Version | Description
  -------                                   | ------- | -------
  build-tools;35.0.0                        | 35.0.0  | Android SDK Build-Tools 35
  platforms;android-36                      | 2       | Android SDK Platform 36
  platforms;android-10                      | 2       | Android SDK Platform 10 (Obsolete)
  system-images;android-36;google_apis;x86_64 | 3     | Google APIs Intel x86_64 Atom System Image
  system-images;android-36;google_apis;arm64-v8a | 3  | Google APIs ARM 64 v8a System Image

Available Updates:
  ID                   | Installed | Available
  -------              | -------   | -------
  platforms;android-35 | 2         | 3
";

    #[test]
    fn parses_installed_and_available() {
        let packages = parse_list(LISTING);
        assert_eq!(packages.len(), 6);
        let build_tools = packages
            .iter()
            .find(|p| p.path == "build-tools;35.0.0")
            .unwrap();
        assert!(build_tools.installed, "installed wins over available");
        let p36 = packages
            .iter()
            .find(|p| p.path == "platforms;android-36")
            .unwrap();
        assert!(!p36.installed);
        assert_eq!(p36.api_level().as_deref(), Some("36"));
        assert!(
            packages
                .iter()
                .find(|p| p.path.ends_with("android-10"))
                .unwrap()
                .obsolete
        );
        assert!(
            !packages.iter().any(|p| p.version == "Installed"),
            "updates are skipped"
        );
    }

    #[test]
    fn image_compatibility_follows_host_arch() {
        let x86 = "system-images;android-36;google_apis;x86_64";
        let arm = "system-images;android-36;google_apis;arm64-v8a";
        assert!(image_compatible(x86, "x86_64") && !image_compatible(arm, "x86_64"));
        assert!(image_compatible(arm, "aarch64") && !image_compatible(x86, "aarch64"));
    }

    #[cfg(unix)]
    #[test]
    fn job_streams_progress_and_reports_failure() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("archipel-ide-sdkrepo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("sdkmanager");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '[===] 10%% Downloading\\r[====] 100%% Done\\n'\n[ \"$2\" = fail ] && { echo nope >&2; exit 3; }\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let run = |arg: &str| {
            let job = SdkJob::spawn(&script, &dir, &[arg.to_owned()]);
            let mut lines = Vec::new();
            loop {
                for event in job.poll() {
                    match event {
                        JobEvent::Line(line) => lines.push(line),
                        JobEvent::Done(result) => return (lines, result),
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };
        let (lines, result) = run("ok");
        assert_eq!(lines.len(), 2);
        assert!(result.is_ok());
        let (_, result) = run("fail");
        assert!(result.unwrap_err().contains("nope"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
