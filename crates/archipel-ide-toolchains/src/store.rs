//! Managed installs: `<root>/<component>/<version>/`.
//!
//! Installs are atomic: archives are extracted into `<root>/.staging/`, a
//! receipt is written, and the directory is renamed into place in one step, so
//! a crash or cancellation never leaves a half-installed version visible.
//! Uninstall renames the version away before deleting it for the same reason.

use crate::download::{DownloadProgress, HttpClient, parse_checksum_file};
use crate::{Checksum, ComponentSpec, Platform, Release, ToolVersion, ToolchainError, extract};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

/// Marker written into every managed install.
pub const RECEIPT: &str = ".archipel-install.json";
/// Receipt name of installs made before the rename to Archipel IDE; still read.
const LEGACY_RECEIPT: &str = ".jetthing-install.json";

/// The receipt of the install in `dir`: the current name, else the legacy one.
fn receipt_path(dir: &Path) -> PathBuf {
    let current = dir.join(RECEIPT);
    let legacy = dir.join(LEGACY_RECEIPT);
    if !current.exists() && legacy.exists() {
        legacy
    } else {
        current
    }
}

/// A managed installation of one component version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installed {
    pub component: String,
    pub version: ToolVersion,
    pub source_url: String,
    pub sha256: String,
    /// Seconds since the Unix epoch.
    pub installed_at: u64,
    /// Install directory; filled in when read, not stored.
    #[serde(skip)]
    pub path: PathBuf,
}

impl Installed {
    /// The first of `spec`'s executables found in its bin dirs.
    pub fn executable(&self, spec: &ComponentSpec, platform: &Platform) -> Option<PathBuf> {
        find_executable(&self.path, spec, platform)
    }
}

/// Finds `spec`'s primary executable below `home` (`<home>/<bin_dir>/<exe>`).
pub fn find_executable(home: &Path, spec: &ComponentSpec, platform: &Platform) -> Option<PathBuf> {
    let primary = spec.primary_executable();
    spec.bin_dirs.iter().find_map(|bin| {
        platform
            .executable_names(primary)
            .into_iter()
            .map(|name| home.join(bin).join(name))
            .find(|path| path.is_file())
    })
}

/// Install progress reported to the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallPhase {
    Resolving,
    Downloading(DownloadProgress),
    Verifying,
    Extracting,
}

#[derive(Debug, Clone)]
pub struct ToolchainStore {
    root: PathBuf,
}

impl ToolchainStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn downloads_dir(&self) -> PathBuf {
        self.root.join(".downloads")
    }

    /// `<root>/<component>/<version>`, refusing names that are not plain path segments.
    pub fn install_dir(
        &self,
        component: &str,
        version: &ToolVersion,
    ) -> Result<PathBuf, ToolchainError> {
        for (what, name) in [("component", component), ("version", version.as_str())] {
            let valid = !name.is_empty()
                && !name.starts_with('.')
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'));
            if !valid {
                return Err(ToolchainError::Catalog {
                    source_name: component.into(),
                    detail: format!("invalid {what} name `{name}`"),
                });
            }
        }
        Ok(self.root.join(component).join(version.as_str()))
    }

    /// Every managed install, newest version first per component.
    pub fn list(&self) -> Vec<Installed> {
        let Ok(components) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut all = Vec::new();
        for component in components.flatten() {
            let name = component.file_name().to_string_lossy().into_owned();
            if !name.starts_with('.') {
                all.extend(self.installed(&name));
            }
        }
        all
    }

    /// Managed installs of `component`, newest first.
    pub fn installed(&self, component: &str) -> Vec<Installed> {
        let Ok(versions) = fs::read_dir(self.root.join(component)) else {
            return Vec::new();
        };
        let mut found: Vec<Installed> = versions
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let text = fs::read_to_string(receipt_path(&path)).ok()?;
                let mut installed: Installed = serde_json::from_str(&text).ok()?;
                (installed.component == component).then(|| {
                    installed.path = path;
                    installed
                })
            })
            .collect();
        found.sort_by(|a, b| b.version.cmp(&a.version));
        found
    }

    pub fn get(&self, component: &str, version: &ToolVersion) -> Option<Installed> {
        self.installed(component)
            .into_iter()
            .find(|installed| &installed.version == version)
    }

    /// Downloads, verifies and installs `release`. Already-installed versions
    /// are returned as they are.
    pub fn install(
        &self,
        release: &Release,
        http: &HttpClient,
        progress: &mut dyn FnMut(InstallPhase),
        cancel: &AtomicBool,
    ) -> Result<Installed, ToolchainError> {
        let target = self.install_dir(&release.component, &release.version)?;
        if let Some(existing) = self.get(&release.component, &release.version) {
            return Ok(existing);
        }
        progress(InstallPhase::Resolving);
        let file_name = release.file_name();
        let expected = match &release.artifact.checksum {
            Checksum::Sha256(digest) => digest.to_ascii_lowercase(),
            Checksum::Sha256Url(url) => {
                let text = http.get_text(url)?;
                parse_checksum_file(&text, &file_name).ok_or_else(|| ToolchainError::Catalog {
                    source_name: url.clone(),
                    detail: "no SHA-256 digest in checksum file".into(),
                })?
            }
        };
        let partial = self.downloads_dir().join(format!(
            "{}-{}-{file_name}.part",
            release.component, release.version
        ));
        http.download(
            &release.artifact.url,
            &partial,
            &expected,
            &mut |p| progress(InstallPhase::Downloading(p)),
            cancel,
        )?;
        progress(InstallPhase::Verifying);
        progress(InstallPhase::Extracting);
        let installed = self.install_archive(release, &partial, &expected);
        let _ = fs::remove_file(&partial);
        let installed = installed?;
        debug_assert_eq!(installed.path, target);
        Ok(installed)
    }

    /// Installs an already verified archive atomically.
    pub fn install_archive(
        &self,
        release: &Release,
        archive: &Path,
        sha256: &str,
    ) -> Result<Installed, ToolchainError> {
        let target = self.install_dir(&release.component, &release.version)?;
        let staging = self.root.join(".staging").join(format!(
            "{}-{}-{}-{}",
            release.component,
            release.version,
            std::process::id(),
            unix_now_nanos()
        ));
        let _ = fs::remove_dir_all(&staging);
        let result = (|| {
            extract::extract(archive, &release.artifact.archive, &staging)?;
            let installed = Installed {
                component: release.component.clone(),
                version: release.version.clone(),
                source_url: release.artifact.url.clone(),
                sha256: sha256.to_owned(),
                installed_at: unix_now_nanos() / 1_000_000_000,
                path: target.clone(),
            };
            let receipt = serde_json::to_string_pretty(&installed)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            fs::write(staging.join(RECEIPT), receipt)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            if target.exists() {
                // A directory without receipt (interrupted manual copy); replace it.
                self.remove_atomically(&target)?;
            }
            fs::rename(&staging, &target)?;
            Ok(installed)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    pub fn uninstall(&self, component: &str, version: &ToolVersion) -> Result<(), ToolchainError> {
        let target = self.install_dir(component, version)?;
        if !receipt_path(&target).is_file() {
            return Err(ToolchainError::NotInstalled {
                component: component.into(),
                version: version.to_string(),
            });
        }
        self.remove_atomically(&target)?;
        // Drop the component directory when its last version is gone.
        if let Some(parent) = target.parent() {
            let _ = fs::remove_dir(parent);
        }
        Ok(())
    }

    fn remove_atomically(&self, target: &Path) -> Result<(), ToolchainError> {
        let trash = self.root.join(".trash");
        fs::create_dir_all(&trash)?;
        let doomed = trash.join(format!("{}-{}", std::process::id(), unix_now_nanos()));
        fs::rename(target, &doomed)?;
        fs::remove_dir_all(&doomed)?;
        Ok(())
    }

    /// Removes leftovers of interrupted installs older than a day (younger ones
    /// may belong to an install running in another Archipel IDE window).
    pub fn clean(&self) {
        let day = std::time::Duration::from_secs(24 * 60 * 60);
        for dir in [".staging", ".trash"] {
            let Ok(entries) = fs::read_dir(self.root.join(dir)) else {
                continue;
            };
            for entry in entries.flatten() {
                let stale = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|age| age > day);
                if stale {
                    let _ = fs::remove_dir_all(entry.path());
                }
            }
        }
    }
}

fn unix_now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or_default()
}
