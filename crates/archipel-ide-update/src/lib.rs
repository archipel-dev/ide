//! Self-update from GitHub releases ("Check for Updates" in the reference IDE).
//!
//! Release archives are produced by `cargo xtask dist` and the release workflow:
//! `archipel-ide-<version>-<os>-<arch>.tar.gz` (`.zip` on Windows) with a
//! `<archive>.sha256` checksum next to it. Updates are verified against that
//! checksum before the running executable is replaced.
//!
//! Copies installed by a native installer (`.msi`, `Archipel IDE.app` from a `.dmg`,
//! `.deb`) are owned by that installer, often in a location the user cannot
//! write to. For those, the matching installer is downloaded and verified
//! instead, and handed to the operating system after the IDE quits; see
//! [`InstallKind`] and `docs/release.md`.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use thiserror::Error;

pub const DEFAULT_REPOSITORY: &str = "archipel-dev/ide";

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected release data: {0}")]
    Format(String),
    #[error("no download for {0} in this release")]
    NoAsset(String),
    #[error("checksum mismatch for {0}")]
    Checksum(String),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

/// `major.minor.patch` with an optional pre-release suffix (`1.2.3-beta.1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

impl Version {
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim().trim_start_matches('v');
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre.to_owned())),
            None => (text, None),
        };
        let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
        let version = Version {
            major: parts.next()??,
            minor: parts.next()??,
            patch: parts.next().unwrap_or(Some(0))?,
            pre,
        };
        parts.next().is_none().then_some(version)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                // A release sorts after its pre-releases.
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub name: String,
    /// API URL; with `Accept: application/octet-stream` it serves the file (works with tokens).
    pub url: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn version(&self) -> Option<Version> {
        Version::parse(&self.tag_name)
    }

    /// The archive built for `os`/`arch` (values of `std::env::consts`).
    pub fn asset_for(&self, os: &str, arch: &str) -> Option<&Asset> {
        let suffix = format!("-{os}-{arch}.{}", archive_extension(os));
        self.assets
            .iter()
            .find(|asset| asset.name.starts_with("archipel-ide-") && asset.name.ends_with(&suffix))
    }

    /// The download that updates an installation of `kind` on `os`/`arch`.
    pub fn installer_for(&self, kind: InstallKind, os: &str, arch: &str) -> Option<&Asset> {
        let (prefix, suffix) = match kind {
            InstallKind::Archive => return self.asset_for(os, arch),
            InstallKind::Msi => ("archipel-ide-", format!("-windows-{arch}.msi")),
            InstallKind::AppBundle => ("archipel-ide-", format!("-macos-{arch}.dmg")),
            InstallKind::Deb => ("archipel-ide_", format!("_{}.deb", deb_arch(arch))),
            InstallKind::AppImage => ("archipel-ide-", format!("-linux-{arch}.AppImage")),
        };
        self.assets
            .iter()
            .find(|asset| asset.name.starts_with(prefix) && asset.name.ends_with(&suffix))
    }

    pub fn checksum_for(&self, asset: &Asset) -> Option<&Asset> {
        let name = format!("{}.sha256", asset.name);
        self.assets.iter().find(|candidate| candidate.name == name)
    }
}

pub fn archive_extension(os: &str) -> &'static str {
    if os == "windows" { "zip" } else { "tar.gz" }
}

/// Debian architecture name for a `std::env::consts::ARCH` value.
pub fn deb_arch(arch: &str) -> &str {
    match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "i386",
        "arm" => "armhf",
        other => other,
    }
}

/// File installers place next to the executable to say who owns the copy.
/// Contains `msi` or `deb`. Archives ship without it.
pub const INSTALL_MARKER: &str = "archipel-ide.install";

/// How this copy of Archipel IDE was installed, which decides how it updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKind {
    /// Extracted release archive: the executables are replaced in place.
    Archive,
    /// Windows Installer package: the new `.msi` upgrades it (MajorUpgrade).
    Msi,
    /// macOS `Archipel IDE.app`: the new `.dmg` is opened for drag-and-drop.
    AppBundle,
    /// Debian package: the new `.deb` is opened with the package installer.
    Deb,
    /// Linux AppImage (the running executable is the `.AppImage` file, from
    /// `$APPIMAGE`): the file is replaced in place by the new AppImage.
    AppImage,
}

impl InstallKind {
    /// Inspects the running executable's location.
    pub fn detect(exe: &Path) -> InstallKind {
        let mut ancestors = exe.ancestors().skip(1);
        let mut name = || {
            ancestors
                .next()
                .and_then(Path::file_name)
                .and_then(|n| n.to_str())
                .map(str::to_owned)
        };
        let in_bundle = name().as_deref() == Some("MacOS")
            && name().as_deref() == Some("Contents")
            && name().is_some_and(|bundle| bundle.ends_with(".app"));
        if in_bundle {
            return InstallKind::AppBundle;
        }
        let appimage = exe
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("appimage"));
        if appimage {
            return InstallKind::AppImage;
        }
        let marker = exe
            .parent()
            .and_then(|dir| fs::read_to_string(dir.join(INSTALL_MARKER)).ok());
        match marker.as_deref().map(str::trim) {
            Some("msi") => InstallKind::Msi,
            Some("deb") => InstallKind::Deb,
            _ => InstallKind::Archive,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            InstallKind::Archive => "archive",
            InstallKind::Msi => "MSI",
            InstallKind::AppBundle => "DMG",
            InstallKind::Deb => "Debian package",
            InstallKind::AppImage => "AppImage",
        }
    }

    /// What happens after the IDE quits, for the update notification.
    pub fn installer_hint(self) -> &'static str {
        match self {
            InstallKind::Archive | InstallKind::AppImage => "Restart the IDE to finish the update.",
            InstallKind::Msi => "Quit Archipel IDE to run the installer; it replaces this version.",
            InstallKind::AppBundle => {
                "Quit Archipel IDE; the disk image opens. Drag Archipel IDE into Applications."
            }
            InstallKind::Deb => "Quit Archipel IDE; the package opens in your software installer.",
        }
    }
}

/// Where to look for releases and how to authenticate (private repositories).
#[derive(Debug, Clone)]
pub struct UpdateSource {
    pub repository: String,
    pub token: Option<String>,
}

impl UpdateSource {
    /// Repository from `ARCHIPEL_UPDATE_REPOSITORY`, token from
    /// `ARCHIPEL_GITHUB_TOKEN` or `GITHUB_TOKEN`.
    pub fn from_env() -> Self {
        let token = ["ARCHIPEL_GITHUB_TOKEN", "GITHUB_TOKEN"]
            .iter()
            .find_map(|name| {
                archipel_ide_compat::env_var(name)
                    .ok()
                    .filter(|v| !v.is_empty())
            });
        Self {
            repository: archipel_ide_compat::env_var("ARCHIPEL_UPDATE_REPOSITORY")
                .unwrap_or_else(|_| DEFAULT_REPOSITORY.to_owned()),
            token,
        }
    }

    fn request(&self, url: &str) -> ureq::Request {
        // System trust store and HTTP(S)_PROXY, so corporate proxies work like in
        // the reference IDE.
        let agent = ureq::AgentBuilder::new()
            .try_proxy_from_env(true)
            .timeout_connect(std::time::Duration::from_secs(15))
            .timeout_read(std::time::Duration::from_secs(60))
            .build();
        let mut request = agent
            .get(url)
            .set(
                "User-Agent",
                concat!("archipel-ide/", env!("CARGO_PKG_VERSION")),
            )
            .set("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.token {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
        request
    }

    pub fn latest_release(&self) -> Result<Release, UpdateError> {
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            self.repository
        );
        let response = self
            .request(&url)
            .set("Accept", "application/vnd.github+json")
            .call()
            .map_err(|error| match error {
                ureq::Error::Status(404, _) => UpdateError::Network(format!(
                    "no published release found for {} (private repositories need \
                     ARCHIPEL_GITHUB_TOKEN)",
                    self.repository
                )),
                other => UpdateError::Network(other.to_string()),
            })?;
        let body = response
            .into_string()
            .map_err(|error| UpdateError::Network(error.to_string()))?;
        parse_release(&body)
    }

    fn download(&self, asset: &Asset, mut sink: impl Write) -> Result<(), UpdateError> {
        let request = if self.token.is_some() {
            self.request(&asset.url)
                .set("Accept", "application/octet-stream")
        } else {
            self.request(&asset.browser_download_url)
        };
        let response = request
            .call()
            .map_err(|error| UpdateError::Network(error.to_string()))?;
        let mut reader = response.into_reader().take(512 * 1024 * 1024);
        std::io::copy(&mut reader, &mut sink)?;
        Ok(())
    }
}

pub fn parse_release(json: &str) -> Result<Release, UpdateError> {
    serde_json::from_str(json).map_err(|error| UpdateError::Format(error.to_string()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Accepts `sha256sum` output (`<hex>  <name>`) or a bare hex digest.
pub fn parse_checksum(text: &str) -> Option<String> {
    let digest = text.split_whitespace().next()?.to_ascii_lowercase();
    (digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit())).then_some(digest)
}

/// Extracts the `archipel-ide` and `archipel-cli` executables from a release archive.
pub fn extract_binaries(
    archive: &[u8],
    os: &str,
    destination: &Path,
) -> Result<Vec<PathBuf>, UpdateError> {
    fs::create_dir_all(destination)?;
    let exe = if os == "windows" { ".exe" } else { "" };
    let wanted = [format!("archipel-ide{exe}"), format!("archipel-cli{exe}")];
    let mut extracted = Vec::new();
    let mut store = |name: &str, reader: &mut dyn Read| -> Result<(), UpdateError> {
        let file_name = Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if wanted.contains(&file_name) {
            let target = destination.join(&file_name);
            let mut file = fs::File::create(&target)?;
            std::io::copy(reader, &mut file)?;
            extracted.push(target);
        }
        Ok(())
    };
    if os == "windows" {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive))
            .map_err(|error| UpdateError::Format(error.to_string()))?;
        for index in 0..zip.len() {
            let mut entry = zip
                .by_index(index)
                .map_err(|error| UpdateError::Format(error.to_string()))?;
            if entry.is_file() {
                let name = entry.name().to_owned();
                store(&name, &mut entry)?;
            }
        }
    } else {
        let decoder = flate2::read::GzDecoder::new(archive);
        let mut tar = tar::Archive::new(decoder);
        for entry in tar.entries()? {
            let mut entry = entry?;
            if entry.header().entry_type().is_file() {
                let name = entry.path()?.to_string_lossy().into_owned();
                store(&name, &mut entry)?;
            }
        }
    }
    if !extracted.iter().any(|path| path.ends_with(&wanted[0])) {
        return Err(UpdateError::Format(format!(
            "{} missing from archive",
            wanted[0]
        )));
    }
    Ok(extracted)
}

/// Replaces `target` with `replacement`, keeping `<target>.old` as a backup so a
/// running executable can be swapped (also on Windows). Restores on failure.
pub fn replace_executable(replacement: &Path, target: &Path) -> Result<(), UpdateError> {
    let backup = target.with_extension("old");
    let _ = fs::remove_file(&backup);
    let had_target = target.exists();
    if had_target {
        fs::rename(target, &backup)?;
    }
    let result = fs::rename(replacement, target).or_else(|_| {
        // Different filesystem: copy instead of rename.
        fs::copy(replacement, target).map(|_| ())
    });
    if let Err(error) = result {
        if had_target {
            let _ = fs::rename(&backup, target);
        }
        return Err(error.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(target, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Result of a completed check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    UpToDate { latest: Version },
    Available { version: Version, release: Release },
}

/// Compares the latest release with `current`.
pub fn evaluate(current: &Version, release: Release) -> Result<CheckOutcome, UpdateError> {
    let version = release.version().ok_or_else(|| {
        UpdateError::Format(format!("tag `{}` is not a version", release.tag_name))
    })?;
    Ok(if version > *current && !release.draft {
        CheckOutcome::Available { version, release }
    } else {
        CheckOutcome::UpToDate { latest: version }
    })
}

/// What [`install`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// The executables next to `current_exe` were replaced.
    InPlace,
    /// A verified installer was downloaded; run it with [`launch_installer`]
    /// after the IDE quits.
    Installer { path: PathBuf, kind: InstallKind },
}

/// Downloads `asset` and checks it against its `.sha256` release asset.
fn download_verified(
    source: &UpdateSource,
    release: &Release,
    asset: &Asset,
) -> Result<Vec<u8>, UpdateError> {
    let checksum_asset = release.checksum_for(asset).ok_or_else(|| {
        UpdateError::Format(format!(
            "{} has no .sha256 checksum; refusing to install",
            asset.name
        ))
    })?;
    let mut bytes = Vec::with_capacity(asset.size.min(512 * 1024 * 1024) as usize);
    source.download(asset, &mut bytes)?;
    let mut text = Vec::new();
    source.download(checksum_asset, &mut text)?;
    let expected = parse_checksum(&String::from_utf8_lossy(&text))
        .ok_or_else(|| UpdateError::Format("unreadable checksum file".into()))?;
    if sha256_hex(&bytes) != expected {
        return Err(UpdateError::Checksum(asset.name.clone()));
    }
    Ok(bytes)
}

/// Downloads and verifies the release. Archive installations are updated in
/// place next to `current_exe`; installer-managed ones get the installer
/// saved to `downloads` (see [`InstallKind`]).
pub fn install(
    source: &UpdateSource,
    release: &Release,
    current_exe: &Path,
    downloads: &Path,
    os: &str,
    arch: &str,
) -> Result<Installed, UpdateError> {
    let kind = InstallKind::detect(current_exe);
    let asset = release
        .installer_for(kind, os, arch)
        .ok_or_else(|| UpdateError::NoAsset(format!("{} for {os}-{arch}", kind.label())))?;
    let bytes = download_verified(source, release, asset)?;
    if kind == InstallKind::AppImage {
        install_appimage(&bytes, current_exe)?;
        return Ok(Installed::InPlace);
    }
    if kind != InstallKind::Archive {
        let path = save_installer(&bytes, &asset.name, downloads)?;
        return Ok(Installed::Installer { path, kind });
    }
    install_archive(&bytes, current_exe, os)?;
    Ok(Installed::InPlace)
}

/// Writes a downloaded installer to `downloads/<file name>`. The name comes
/// from release data, so only its final component is used.
pub fn save_installer(bytes: &[u8], name: &str, downloads: &Path) -> Result<PathBuf, UpdateError> {
    let file_name = Path::new(name)
        .file_name()
        .filter(|file| !file.is_empty())
        .ok_or_else(|| UpdateError::Format(format!("invalid installer name `{name}`")))?;
    fs::create_dir_all(downloads)?;
    let path = downloads.join(file_name);
    fs::write(&path, bytes)?;
    Ok(path)
}

/// Writes the new AppImage next to the old one and swaps it in (the old file
/// stays as `<name>.old` until the next update).
fn install_appimage(bytes: &[u8], current: &Path) -> Result<(), UpdateError> {
    let staged = current.with_extension("AppImage.new");
    fs::write(&staged, bytes)?;
    replace_executable(&staged, current)
}

fn install_archive(archive: &[u8], current_exe: &Path, os: &str) -> Result<(), UpdateError> {
    let directory = current_exe
        .parent()
        .ok_or_else(|| UpdateError::Format("executable has no directory".into()))?;
    let staging = directory.join(".archipel-ide-update");
    let _ = fs::remove_dir_all(&staging);
    let binaries = extract_binaries(archive, os, &staging)?;
    for binary in binaries {
        let name = binary.file_name().unwrap_or_default();
        let target = if Path::new(name).file_stem() == current_exe.file_stem() {
            current_exe.to_owned()
        } else {
            directory.join(name)
        };
        replace_executable(&binary, &target)?;
    }
    let _ = fs::remove_dir_all(&staging);
    Ok(())
}

/// The command that opens a downloaded installer on `os`.
pub fn installer_command(installer: &Path, os: &str) -> std::process::Command {
    let mut command = match os {
        "windows" => {
            let mut command = std::process::Command::new("msiexec");
            command.arg("/i");
            command
        }
        "macos" => std::process::Command::new("open"),
        _ => std::process::Command::new("xdg-open"),
    };
    command.arg(installer);
    command
}

/// Starts the installer detached; the IDE should quit right after so the
/// installer can replace its files.
pub fn launch_installer(installer: &Path) -> Result<(), UpdateError> {
    installer_command(installer, std::env::consts::OS).spawn()?;
    Ok(())
}

/// A finished install: the new version and, for installer-managed copies,
/// the installer to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledUpdate {
    pub version: Version,
    pub installed: Installed,
}

pub enum UpdateEvent {
    Checked(Result<CheckOutcome, String>),
    Installed(Result<InstalledUpdate, String>),
}

/// Runs checks and installs on background threads; poll from the UI loop.
#[derive(Default)]
pub struct UpdateService {
    pending: Option<Receiver<UpdateEvent>>,
}

impl UpdateService {
    /// A service whose next event comes from `receiver` (tests, embedders).
    pub fn with_receiver(receiver: Receiver<UpdateEvent>) -> Self {
        Self {
            pending: Some(receiver),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    pub fn check(&mut self, source: UpdateSource, current: Version) {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = source
                .latest_release()
                .and_then(|release| evaluate(&current, release))
                .map_err(|error| error.to_string());
            let _ = sender.send(UpdateEvent::Checked(result));
        });
        self.pending = Some(receiver);
    }

    pub fn install(&mut self, source: UpdateSource, release: Release, current_exe: PathBuf) {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let os = std::env::consts::OS;
            let arch = std::env::consts::ARCH;
            let downloads = std::env::temp_dir().join("archipel-ide-update");
            let result = install(&source, &release, &current_exe, &downloads, os, arch)
                .map_err(|error| error.to_string())
                .and_then(|installed| {
                    let version = release.version().ok_or("invalid version")?;
                    Ok(InstalledUpdate { version, installed })
                });
            let _ = sender.send(UpdateEvent::Installed(result));
        });
        self.pending = Some(receiver);
    }

    pub fn poll(&mut self) -> Option<UpdateEvent> {
        let receiver = self.pending.as_ref()?;
        match receiver.try_recv() {
            Ok(event) => {
                self.pending = None;
                Some(event)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                Some(UpdateEvent::Checked(Err("update worker stopped".into())))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE: &str = r#"{
        "tag_name": "v0.0.2",
        "name": "Archipel IDE 0.0.2",
        "body": "Notes",
        "html_url": "https://github.com/archipel-dev/ide/releases/tag/v0.0.2",
        "draft": false,
        "prerelease": false,
        "assets": [
            {"name": "archipel-ide-0.0.2-linux-x86_64.tar.gz", "url": "https://api/1", "browser_download_url": "https://dl/1", "size": 10},
            {"name": "archipel-ide-0.0.2-linux-x86_64.tar.gz.sha256", "url": "https://api/2", "browser_download_url": "https://dl/2", "size": 1},
            {"name": "archipel-ide-0.0.2-windows-x86_64.zip", "url": "https://api/3", "browser_download_url": "https://dl/3", "size": 10},
            {"name": "archipel-ide-0.0.2-windows-x86_64.msi", "url": "https://api/4", "browser_download_url": "https://dl/4", "size": 10},
            {"name": "archipel-ide-0.0.2-macos-aarch64.dmg", "url": "https://api/5", "browser_download_url": "https://dl/5", "size": 10},
            {"name": "archipel-ide_0.0.2_amd64.deb", "url": "https://api/6", "browser_download_url": "https://dl/6", "size": 10},
            {"name": "archipel-ide-0.0.2-linux-x86_64.AppImage", "url": "https://api/7", "browser_download_url": "https://dl/7", "size": 10}
        ]
    }"#;

    #[test]
    fn parses_and_orders_versions() {
        let v = |s| Version::parse(s).unwrap();
        assert_eq!(
            v("v0.0.1"),
            Version {
                major: 0,
                minor: 0,
                patch: 1,
                pre: None
            }
        );
        assert!(v("0.0.2") > v("0.0.1"));
        assert!(v("0.1.0") > v("0.0.9"));
        assert!(v("1.0.0") > v("1.0.0-beta.1"));
        assert_eq!(v("1.2").to_string(), "1.2.0");
        assert_eq!(Version::parse("x.1.2"), None);
        assert_eq!(Version::parse("1.2.3.4"), None);
    }

    #[test]
    fn finds_platform_assets_and_checksums() {
        let release = parse_release(RELEASE).unwrap();
        let linux = release.asset_for("linux", "x86_64").unwrap();
        assert_eq!(linux.name, "archipel-ide-0.0.2-linux-x86_64.tar.gz");
        assert_eq!(
            release.checksum_for(linux).unwrap().name,
            "archipel-ide-0.0.2-linux-x86_64.tar.gz.sha256"
        );
        assert!(release.asset_for("windows", "x86_64").is_some());
        assert!(release.asset_for("macos", "aarch64").is_none());
    }

    #[test]
    fn finds_installers_by_install_kind() {
        let release = parse_release(RELEASE).unwrap();
        let name = |kind, os, arch| {
            release
                .installer_for(kind, os, arch)
                .map(|asset| asset.name.as_str())
        };
        assert_eq!(
            name(InstallKind::Archive, "linux", "x86_64"),
            Some("archipel-ide-0.0.2-linux-x86_64.tar.gz")
        );
        assert_eq!(
            name(InstallKind::Msi, "windows", "x86_64"),
            Some("archipel-ide-0.0.2-windows-x86_64.msi")
        );
        assert_eq!(
            name(InstallKind::AppBundle, "macos", "aarch64"),
            Some("archipel-ide-0.0.2-macos-aarch64.dmg")
        );
        assert_eq!(
            name(InstallKind::Deb, "linux", "x86_64"),
            Some("archipel-ide_0.0.2_amd64.deb")
        );
        assert_eq!(
            name(InstallKind::AppImage, "linux", "x86_64"),
            Some("archipel-ide-0.0.2-linux-x86_64.AppImage")
        );
        assert_eq!(name(InstallKind::AppBundle, "macos", "x86_64"), None);
        assert_eq!(name(InstallKind::Deb, "linux", "aarch64"), None);
    }

    #[test]
    fn detects_install_kind_from_the_executable_location() {
        let dir = temp("kind");
        let exe = dir.join("archipel");
        assert_eq!(InstallKind::detect(&exe), InstallKind::Archive);
        fs::write(dir.join(INSTALL_MARKER), "msi\r\n").unwrap();
        assert_eq!(InstallKind::detect(&exe), InstallKind::Msi);
        fs::write(dir.join(INSTALL_MARKER), "deb\n").unwrap();
        assert_eq!(InstallKind::detect(&exe), InstallKind::Deb);
        fs::write(dir.join(INSTALL_MARKER), "something else").unwrap();
        assert_eq!(InstallKind::detect(&exe), InstallKind::Archive);
        let bundle = Path::new("/Applications/Archipel IDE.app/Contents/MacOS/archipel-ide");
        assert_eq!(InstallKind::detect(bundle), InstallKind::AppBundle);
        let not_bundle = Path::new("/opt/Archipel IDE/Contents/MacOS/archipel-ide");
        assert_eq!(InstallKind::detect(not_bundle), InstallKind::Archive);
        assert_eq!(
            InstallKind::detect(Path::new("archipel-ide")),
            InstallKind::Archive
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn appimage_update_replaces_the_file_in_place() {
        let dir = temp("appimage");
        let current = dir.join("archipel-ide-0.0.1-linux-x86_64.AppImage");
        fs::write(&current, "old").unwrap();
        install_appimage(b"new", &current).unwrap();
        assert_eq!(fs::read(&current).unwrap(), b"new");
        assert_eq!(
            fs::read(dir.join("archipel-ide-0.0.1-linux-x86_64.old")).unwrap(),
            b"old"
        );
        assert!(
            !dir.join("archipel-ide-0.0.1-linux-x86_64.AppImage.new")
                .exists()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saves_installers_without_path_traversal() {
        let dir = temp("save");
        let path = save_installer(b"msi", "../../evil/archipel-ide.msi", &dir).unwrap();
        assert_eq!(path, dir.join("archipel-ide.msi"));
        assert_eq!(fs::read(&path).unwrap(), b"msi");
        assert!(save_installer(b"x", "..", &dir).is_err());
        assert!(save_installer(b"x", "", &dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn installers_open_with_the_platform_handler() {
        let program = |os| {
            installer_command(Path::new("setup"), os)
                .get_program()
                .to_string_lossy()
                .into_owned()
        };
        assert_eq!(program("windows"), "msiexec");
        assert_eq!(program("macos"), "open");
        assert_eq!(program("linux"), "xdg-open");
        let args: Vec<_> = installer_command(Path::new("setup.msi"), "windows")
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["/i", "setup.msi"]);
        assert_eq!(deb_arch("x86_64"), "amd64");
        assert_eq!(deb_arch("aarch64"), "arm64");
    }

    #[test]
    fn evaluates_against_current_version() {
        let release = parse_release(RELEASE).unwrap();
        let older = Version::parse("0.0.1").unwrap();
        assert!(matches!(
            evaluate(&older, release.clone()).unwrap(),
            CheckOutcome::Available { .. }
        ));
        let same = Version::parse("0.0.2").unwrap();
        assert!(matches!(
            evaluate(&same, release).unwrap(),
            CheckOutcome::UpToDate { .. }
        ));
    }

    #[test]
    fn checksums_parse_and_match() {
        let digest = sha256_hex(b"abc");
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            parse_checksum(&format!("{digest}  file.tar.gz\n")),
            Some(digest.clone())
        );
        assert_eq!(parse_checksum("nope"), None);
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("archipel-ide-update-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn extracts_binaries_from_tar_gz() {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        for (path, data) in [
            (
                "archipel-ide-0.0.2-linux-x86_64/archipel-ide",
                &b"new app"[..],
            ),
            ("archipel-ide-0.0.2-linux-x86_64/archipel-cli", b"new cli"),
            ("archipel-ide-0.0.2-linux-x86_64/README.md", b"readme"),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, path, data).unwrap();
        }
        let archive = builder.into_inner().unwrap().finish().unwrap();
        let dir = temp("extract");
        let files = extract_binaries(&archive, "linux", &dir).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(fs::read(dir.join("archipel-ide")).unwrap(), b"new app");
        assert!(!dir.join("README.md").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn extracts_binaries_from_zip() {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut cursor);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file(
                "archipel-ide-0.0.2-windows-x86_64/archipel-ide.exe",
                options,
            )
            .unwrap();
            zip.write_all(b"exe").unwrap();
            zip.finish().unwrap();
        }
        let dir = temp("zip");
        let files = extract_binaries(cursor.get_ref(), "windows", &dir).unwrap();
        assert_eq!(files, vec![dir.join("archipel-ide.exe")]);
        assert!(extract_binaries(cursor.get_ref(), "linux", &dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replaces_executable_and_keeps_backup() {
        let dir = temp("replace");
        let target = dir.join("archipel-ide");
        let replacement = dir.join("staged");
        fs::write(&target, "old").unwrap();
        fs::write(&replacement, "new").unwrap();
        replace_executable(&replacement, &target).unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(
            fs::read_to_string(dir.join("archipel-ide.old")).unwrap(),
            "old"
        );
        // A missing replacement restores the original.
        assert!(replace_executable(&dir.join("missing"), &target).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        fs::remove_dir_all(dir).unwrap();
    }
}
