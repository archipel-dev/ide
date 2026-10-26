//! Gradle distributions from services.gradle.org.
//!
//! `versions/current` names the newest release; `versions/all` lists every
//! version. Each entry has a `downloadUrl` (`gradle-<v>-bin.zip`) and the
//! `checksumUrl` of the SHA-256 published next to the zip.

use crate::download::HttpClient;
use crate::{
    ArchiveKind, Artifact, Checksum, ComponentKind, ComponentProvider, ComponentSettings,
    ComponentSpec, Platform, Release, ToolVersion, ToolchainError, VersionRequest,
};
use serde::Deserialize;

pub const GRADLE_SERVICES: &str = "https://services.gradle.org";

pub struct GradleDistributions {
    spec: ComponentSpec,
    base: String,
}

impl Default for GradleDistributions {
    fn default() -> Self {
        Self::with_base(GRADLE_SERVICES)
    }
}

/// The `gradle` component: `bin/gradle` of the binary distribution; runs on a JDK.
pub fn spec() -> ComponentSpec {
    let mut spec = ComponentSpec::new("gradle", "Gradle", ComponentKind::BuildTool, "gradle");
    spec.bin_dirs = vec!["bin".into()];
    spec.home_env = vec!["GRADLE_HOME".into()];
    spec.requires = vec!["jdk".into()];
    spec.homepage = Some("https://gradle.org".into());
    spec
}

/// One entry of `versions/current` or `versions/all`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradleVersion {
    pub version: String,
    #[serde(default)]
    pub snapshot: bool,
    #[serde(default)]
    pub nightly: bool,
    #[serde(default)]
    pub release_nightly: bool,
    #[serde(default)]
    pub broken: bool,
    #[serde(default)]
    pub rc_for: String,
    #[serde(default)]
    pub milestone_for: String,
    #[serde(default)]
    pub download_url: String,
    #[serde(default)]
    pub checksum_url: String,
}

impl GradleVersion {
    /// A final release (no snapshot, nightly, RC or milestone) that is not broken.
    pub fn is_stable(&self) -> bool {
        !(self.snapshot
            || self.nightly
            || self.release_nightly
            || self.broken
            || !self.rc_for.is_empty()
            || !self.milestone_for.is_empty())
    }
}

impl GradleDistributions {
    pub fn with_base(base: &str) -> Self {
        Self {
            spec: spec(),
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    fn error(&self, detail: impl Into<String>) -> ToolchainError {
        ToolchainError::Catalog {
            source_name: format!("{}/versions/", self.base),
            detail: detail.into(),
        }
    }

    fn release(&self, entry: &GradleVersion) -> Result<Release, ToolchainError> {
        let expected = format!("gradle-{}-bin.zip", entry.version);
        let web =
            entry.download_url.starts_with("https://") || entry.download_url.starts_with("http://");
        if !web || !entry.download_url.ends_with(&expected) {
            return Err(self.error(format!(
                "unexpected download URL for {}: `{}`",
                entry.version, entry.download_url
            )));
        }
        if entry.checksum_url.is_empty() {
            return Err(self.error(format!("no checksum published for {}", entry.version)));
        }
        Ok(Release {
            component: self.spec.id.clone(),
            version: ToolVersion::new(entry.version.clone()),
            artifact: Artifact {
                url: entry.download_url.clone(),
                checksum: Checksum::Sha256Url(entry.checksum_url.clone()),
                archive: ArchiveKind::Zip,
                size: None,
            },
            license: Some("Apache-2.0".into()),
        })
    }
}

impl ComponentProvider for GradleDistributions {
    fn spec(&self) -> &ComponentSpec {
        &self.spec
    }

    fn resolve(
        &self,
        request: &VersionRequest,
        _platform: &Platform,
        http: &HttpClient,
        _settings: Option<&ComponentSettings>,
    ) -> Result<Release, ToolchainError> {
        // The binary distribution is platform independent (it runs on the JDK).
        if matches!(request, VersionRequest::Latest | VersionRequest::Channel(_)) {
            let current: GradleVersion =
                http.get_json(&format!("{}/versions/current", self.base))?;
            if current.is_stable() {
                return self.release(&current);
            }
        }
        let all: Vec<GradleVersion> = http.get_json(&format!("{}/versions/all", self.base))?;
        let stable: Vec<&GradleVersion> = all.iter().filter(|v| v.is_stable()).collect();
        if stable.is_empty() {
            return Err(self.error("no stable Gradle release listed"));
        }
        let versions: Vec<ToolVersion> = stable
            .iter()
            .map(|v| ToolVersion::new(v.version.clone()))
            .collect();
        let best = request
            .best(&versions)
            .ok_or_else(|| ToolchainError::NoMatchingVersion {
                component: self.spec.id.clone(),
                request: request.to_string(),
            })?;
        let entry = stable
            .iter()
            .find(|v| v.version == best.to_string())
            .ok_or_else(|| self.error("version listing changed while resolving"))?;
        self.release(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_final_releases_are_stable() {
        let entry = |json: &str| serde_json::from_str::<GradleVersion>(json).unwrap();
        assert!(entry(r#"{"version":"9.1.0"}"#).is_stable());
        assert!(!entry(r#"{"version":"9.2.0-rc-1","rcFor":"9.2.0"}"#).is_stable());
        assert!(!entry(r#"{"version":"9.2.0-milestone-1","milestoneFor":"9.2.0"}"#).is_stable());
        assert!(!entry(r#"{"version":"9.3-20261010","snapshot":true,"nightly":true}"#).is_stable());
        assert!(!entry(r#"{"version":"8.0","broken":true}"#).is_stable());
    }
}
