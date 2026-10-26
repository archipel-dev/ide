//! microsoft/java-debug's jdtls plugin from Maven Central.
//!
//! The plugin is one jar that jdtls loads as an OSGi bundle (`initializationOptions.bundles`);
//! it adds the `vscode.java.*` workspace commands that start a DAP server inside jdtls.
//! `maven-metadata.xml` lists versions. Central publishes only `.sha1`/`.md5` for these jars,
//! so known releases carry a pinned SHA-256 and newer ones need a `.sha256` sidecar.

use crate::download::HttpClient;
use crate::{
    ArchiveKind, Artifact, Checksum, ComponentKind, ComponentProvider, ComponentSettings,
    ComponentSpec, Platform, Release, ToolVersion, ToolchainError, VersionRequest,
};

pub const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2";
const GROUP_PATH: &str = "com/microsoft/java";
const ARTIFACT: &str = "com.microsoft.java.debug.plugin";
/// SHA-256 of releases whose Maven Central directory publishes only SHA-1/MD5 sidecars.
/// Each was computed from a download whose SHA-1 matched Central's `.sha1`.
const PINNED_SHA256: &[(&str, &str)] = &[(
    "0.53.1",
    "4f4778d452a6a0665536f43ce4e32403a24be6593336b80dc85a322912859e24",
)];
/// Stored name of the installed jar (the component's "executable").
pub const JAR_NAME: &str = "java-debug-plugin.jar";

pub struct JavaDebugPlugin {
    spec: ComponentSpec,
    base: String,
}

impl Default for JavaDebugPlugin {
    fn default() -> Self {
        Self::with_base(MAVEN_CENTRAL)
    }
}

/// The `java-debug` component; it is loaded by `jdtls`.
pub fn spec() -> ComponentSpec {
    let mut spec = ComponentSpec::new(
        "java-debug",
        "Java debugger (java-debug)",
        ComponentKind::Debugger,
        JAR_NAME,
    );
    spec.bin_dirs = vec![String::new()];
    spec.requires = vec!["jdtls".into()];
    spec.homepage = Some("https://github.com/microsoft/java-debug".into());
    spec
}

impl JavaDebugPlugin {
    pub fn with_base(base: &str) -> Self {
        Self {
            spec: spec(),
            base: base.trim_end_matches('/').to_owned(),
        }
    }
}

/// Release versions in `maven-metadata.xml` (snapshots and qualifiers are skipped).
fn metadata_versions(xml: &str) -> Vec<ToolVersion> {
    let mut versions: Vec<ToolVersion> = xml
        .split("<version>")
        .skip(1)
        .filter_map(|rest| rest.split("</version>").next())
        .filter(|v| v.contains('.') && v.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .map(ToolVersion::new)
        .collect();
    versions.sort();
    versions.dedup();
    versions
}

impl ComponentProvider for JavaDebugPlugin {
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
        let directory = format!("{}/{GROUP_PATH}/{ARTIFACT}", self.base);
        let metadata = format!("{directory}/maven-metadata.xml");
        let versions = metadata_versions(&http.get_text(&metadata)?);
        let version = request
            .best(&versions)
            .ok_or_else(|| ToolchainError::NoMatchingVersion {
                component: self.spec.id.clone(),
                request: request.to_string(),
            })?
            .clone();
        let url = format!("{directory}/{version}/{ARTIFACT}-{version}.jar");
        let checksum = match PINNED_SHA256.iter().find(|(v, _)| *v == version.as_str()) {
            Some((_, hex)) => Checksum::Sha256((*hex).to_owned()),
            None => Checksum::Sha256Url(format!("{url}.sha256")),
        };
        Ok(Release {
            component: self.spec.id.clone(),
            version,
            artifact: Artifact {
                checksum,
                url,
                archive: ArchiveKind::Raw {
                    file_name: JAR_NAME.into(),
                },
                size: None,
            },
            license: Some("EPL-1.0".into()),
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

    #[test]
    fn parses_maven_metadata() {
        let xml = "<versioning><versions><version>0.9.0</version><version>0.53.1</version>\
                   <version>0.54.0-SNAPSHOT</version></versions></versioning>";
        assert_eq!(
            metadata_versions(xml),
            vec![ToolVersion::new("0.9.0"), ToolVersion::new("0.53.1")]
        );
    }
}
