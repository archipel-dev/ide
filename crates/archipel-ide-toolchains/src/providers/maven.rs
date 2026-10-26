//! Apache Maven binary distributions from Maven Central.
//!
//! Central publishes `.sha512`/`.sha1` sidecars for `apache-maven-<v>-bin.zip`, not SHA-256,
//! so the catalog pins the SHA-256 of known releases. Each pin was computed from a download
//! whose SHA-512 matched the published `.sha512`. `latest` resolves to the newest pinned release.

use crate::download::HttpClient;
use crate::{
    ArchiveKind, Artifact, Checksum, ComponentKind, ComponentProvider, ComponentSettings,
    ComponentSpec, Platform, Release, ToolVersion, ToolchainError, VersionRequest,
};

pub const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";
const DIRECTORY: &str = "org/apache/maven/apache-maven";
/// SHA-256 of `apache-maven-<version>-bin.zip`, oldest first.
const PINNED_SHA256: &[(&str, &str)] = &[(
    "3.9.11",
    "0d7125e8c91097b36edb990ea5934e6c68b4440eef4ea96510a0f6815e7eeadb",
)];

pub struct MavenDistributions {
    spec: ComponentSpec,
    base: String,
}

impl Default for MavenDistributions {
    fn default() -> Self {
        Self::with_base(MAVEN_CENTRAL)
    }
}

/// The `maven` component: `bin/mvn` of the binary distribution; runs on a JDK.
pub fn spec() -> ComponentSpec {
    let mut spec = ComponentSpec::new("maven", "Maven", ComponentKind::BuildTool, "mvn");
    spec.bin_dirs = vec!["bin".into()];
    spec.home_env = vec!["MAVEN_HOME".into(), "M2_HOME".into()];
    spec.requires = vec!["jdk".into()];
    spec.homepage = Some("https://maven.apache.org".into());
    spec
}

impl MavenDistributions {
    pub fn with_base(base: &str) -> Self {
        Self {
            spec: spec(),
            base: base.trim_end_matches('/').to_owned(),
        }
    }
}

impl ComponentProvider for MavenDistributions {
    fn spec(&self) -> &ComponentSpec {
        &self.spec
    }

    fn resolve(
        &self,
        request: &VersionRequest,
        _platform: &Platform,
        _http: &HttpClient,
        _settings: Option<&ComponentSettings>,
    ) -> Result<Release, ToolchainError> {
        let versions: Vec<ToolVersion> = PINNED_SHA256
            .iter()
            .map(|(version, _)| ToolVersion::new(*version))
            .collect();
        let version = request
            .best(&versions)
            .ok_or_else(|| ToolchainError::NoMatchingVersion {
                component: self.spec.id.clone(),
                request: request.to_string(),
            })?
            .clone();
        let digest = PINNED_SHA256
            .iter()
            .find(|(v, _)| *v == version.as_str())
            .map(|(_, hex)| (*hex).to_owned())
            .unwrap_or_default();
        Ok(Release {
            component: self.spec.id.clone(),
            artifact: Artifact {
                url: format!(
                    "{}/{DIRECTORY}/{version}/apache-maven-{version}-bin.zip",
                    self.base
                ),
                checksum: Checksum::Sha256(digest),
                archive: ArchiveKind::Zip,
                size: None,
            },
            version,
            license: Some("Apache-2.0".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_checksums_are_hex_sha256() {
        for (version, hex) in PINNED_SHA256 {
            assert!(version.contains('.'));
            assert!(hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
