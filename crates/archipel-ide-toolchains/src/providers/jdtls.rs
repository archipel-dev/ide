//! Eclipse JDT Language Server milestones from download.eclipse.org.
//!
//! `milestones/` lists one directory per release; each holds `latest.txt`
//! naming the tarball and a `<tarball>.sha256` checksum next to it.

use crate::download::HttpClient;
use crate::{
    ArchiveKind, Artifact, Checksum, ComponentKind, ComponentProvider, ComponentSettings,
    ComponentSpec, Platform, Release, ToolVersion, ToolchainError, VersionRequest,
};

pub const ECLIPSE_JDTLS: &str = "https://download.eclipse.org/jdtls";

pub struct EclipseJdtls {
    spec: ComponentSpec,
    base: String,
}

impl Default for EclipseJdtls {
    fn default() -> Self {
        Self::with_base(ECLIPSE_JDTLS)
    }
}

/// The `jdtls` component; it runs on a JDK (21 or newer).
pub fn spec() -> ComponentSpec {
    let mut spec = ComponentSpec::new(
        "jdtls",
        "Eclipse JDT Language Server",
        ComponentKind::LanguageServer,
        "jdtls",
    );
    spec.bin_dirs = vec!["bin".into()];
    spec.requires = vec!["jdk".into()];
    spec.homepage = Some("https://github.com/eclipse-jdtls/eclipse.jdt.ls".into());
    spec
}

impl EclipseJdtls {
    pub fn with_base(base: &str) -> Self {
        Self {
            spec: spec(),
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    fn error(&self, detail: impl Into<String>) -> ToolchainError {
        ToolchainError::Catalog {
            source_name: format!("{}/milestones/", self.base),
            detail: detail.into(),
        }
    }
}

/// Version directory names in the milestones listing (`.../milestones/1.40.0'`).
fn milestone_versions(html: &str) -> Vec<ToolVersion> {
    let mut versions: Vec<ToolVersion> = html
        .match_indices("milestones/")
        .filter_map(|(index, marker)| {
            let rest = &html[index + marker.len()..];
            let version: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let terminated = rest[version.len()..].starts_with(['\'', '"', '/']);
            (terminated && version.contains('.') && !version.ends_with('.'))
                .then(|| ToolVersion::new(version))
        })
        .collect();
    versions.sort();
    versions.dedup();
    versions
}

impl ComponentProvider for EclipseJdtls {
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
        // jdtls is platform independent (Java); one tarball serves every OS.
        let listing = http.get_text(&format!("{}/milestones/", self.base))?;
        let versions = milestone_versions(&listing);
        if versions.is_empty() {
            return Err(self.error("no milestones listed"));
        }
        let version = request
            .best(&versions)
            .ok_or_else(|| ToolchainError::NoMatchingVersion {
                component: self.spec.id.clone(),
                request: request.to_string(),
            })?
            .clone();
        let directory = format!("{}/milestones/{version}", self.base);
        let file = http.get_text(&format!("{directory}/latest.txt"))?;
        let file = file.trim();
        let valid = file.ends_with(".tar.gz")
            && file
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
        if !valid {
            return Err(self.error(format!("unexpected latest.txt for {version}: `{file}`")));
        }
        let url = format!("{directory}/{file}");
        Ok(Release {
            component: self.spec.id.clone(),
            version,
            artifact: Artifact {
                checksum: Checksum::Sha256Url(format!("{url}.sha256")),
                url,
                archive: ArchiveKind::TarGz,
                size: None,
            },
            license: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_milestone_listing() {
        let html = "<a href='/jdtls/milestones/..'> ..</a><a href='/jdtls/milestones/1.9.0'>\
                    <a href='/jdtls/milestones/1.40.0'> 1.40.0</a><a href='/jdtls/milestones/repository'>";
        let versions = milestone_versions(html);
        assert_eq!(
            versions,
            vec![ToolVersion::new("1.9.0"), ToolVersion::new("1.40.0")]
        );
    }
}
