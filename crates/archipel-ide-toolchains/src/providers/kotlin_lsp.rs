//! JetBrains' public Kotlin language server (`Kotlin/kotlin-lsp`).
//!
//! The standalone archives live on `download.jetbrains.com/language-server/kotlin-server/<version>/`
//! with a `.sha256` next to each; the versions are the `### v<version>` headings of the
//! repository's `RELEASES.md`. The archive bundles its own JetBrains runtime, so no JDK is
//! needed. JetBrains publishes macOS builds only as `.sit` (not extractable here), and when
//! its download host or listing is unavailable, [`fwcd/kotlin-language-server`] is the fallback.
//!
//! [`fwcd/kotlin-language-server`]: https://github.com/fwcd/kotlin-language-server

use super::github::GithubReleaseProvider;
use crate::download::HttpClient;
use crate::{
    Arch, ArchiveKind, Artifact, Checksum, ComponentKind, ComponentProvider, ComponentSettings,
    ComponentSpec, Os, Platform, Release, ToolVersion, ToolchainError, VersionRequest,
};

pub const JETBRAINS_DOWNLOADS: &str =
    "https://download.jetbrains.com/language-server/kotlin-server";
pub const RELEASES_MD: &str =
    "https://raw.githubusercontent.com/Kotlin/kotlin-lsp/main/RELEASES.md";
/// Newest version known to ship standalone archives; used when the listing is unreachable.
pub const KNOWN_VERSION: &str = "263.6379.0";

pub struct KotlinLsp {
    spec: ComponentSpec,
    downloads: String,
    releases_md: String,
    fallback: GithubReleaseProvider,
}

impl Default for KotlinLsp {
    fn default() -> Self {
        Self::with_sources(JETBRAINS_DOWNLOADS, RELEASES_MD, super::github::GITHUB_API)
    }
}

/// The `kotlin-lsp` component: `bin/intellij-server --stdio` (JetBrains) or
/// `bin/kotlin-language-server` (fwcd fallback).
pub fn spec() -> ComponentSpec {
    let mut spec = ComponentSpec::new(
        "kotlin-lsp",
        "Kotlin language server",
        ComponentKind::LanguageServer,
        "intellij-server",
    );
    spec.executables.push("kotlin-language-server".into());
    spec.bin_dirs = vec!["bin".into()];
    spec.homepage = Some("https://github.com/Kotlin/kotlin-lsp".into());
    spec
}

impl KotlinLsp {
    pub fn with_sources(downloads: &str, releases_md: &str, github_api: &str) -> Self {
        let fallback =
            GithubReleaseProvider::new(spec(), "fwcd/kotlin-language-server", |_platform, _tag| {
                Some(("server.zip".into(), ArchiveKind::Zip))
            })
            .with_api(github_api);
        Self {
            spec: spec(),
            downloads: downloads.trim_end_matches('/').to_owned(),
            releases_md: releases_md.to_owned(),
            fallback,
        }
    }

    fn jetbrains(
        &self,
        request: &VersionRequest,
        platform: &Platform,
        http: &HttpClient,
    ) -> Result<Release, ToolchainError> {
        let (suffix, archive) = match (platform.os, platform.arch) {
            (Os::Linux, Arch::X86_64) => ("", ArchiveKind::TarGz),
            (Os::Linux, Arch::Aarch64) => ("-aarch64", ArchiveKind::TarGz),
            (Os::Windows, Arch::X86_64) => (".win", ArchiveKind::Zip),
            (Os::Windows, Arch::Aarch64) => ("-aarch64.win", ArchiveKind::Zip),
            (Os::Macos, _) => {
                return Err(ToolchainError::UnsupportedPlatform {
                    component: self.spec.name.clone(),
                    platform: platform.key(),
                });
            }
        };
        let mut versions = http
            .get_text(&self.releases_md)
            .map(|text| release_versions(&text))
            .unwrap_or_default();
        if versions.is_empty() {
            versions.push(ToolVersion::new(KNOWN_VERSION));
        }
        // Newest matching version that ships a standalone archive: the sidecar doubles
        // as an existence check (others answer 404 here instead of at the 370 MB download).
        let mut candidates: Vec<&ToolVersion> =
            versions.iter().filter(|v| request.matches(v)).collect();
        candidates.sort_by(|a, b| b.cmp(a));
        if candidates.is_empty() {
            return Err(ToolchainError::NoMatchingVersion {
                component: self.spec.id.clone(),
                request: request.to_string(),
            });
        }
        let mut last_error = None;
        let mut found = None;
        for version in candidates.into_iter().take(4) {
            let file = match archive {
                ArchiveKind::Zip => format!("kotlin-server-{version}{suffix}.zip"),
                _ => format!("kotlin-server-{version}{suffix}.tar.gz"),
            };
            let url = format!("{}/{version}/{file}", self.downloads);
            match http.get_text(&format!("{url}.sha256")) {
                Ok(_) => {
                    found = Some((version.clone(), url));
                    break;
                }
                Err(error) => last_error = Some(error),
            }
        }
        let Some((version, url)) = found else {
            return Err(last_error.expect("at least one candidate was tried"));
        };
        let sidecar = format!("{url}.sha256");
        Ok(Release {
            component: self.spec.id.clone(),
            version,
            artifact: Artifact {
                url,
                checksum: Checksum::Sha256Url(sidecar),
                archive,
                size: None,
            },
            license: None,
        })
    }
}

/// Versions from the `### v263.6379.0` headings of `RELEASES.md`.
fn release_versions(markdown: &str) -> Vec<ToolVersion> {
    let mut versions: Vec<ToolVersion> = markdown
        .lines()
        .filter_map(|line| line.strip_prefix("### v"))
        .map(str::trim)
        .filter(|v| {
            v.matches('.').count() == 2
                && v.split('.')
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        })
        .map(ToolVersion::new)
        .collect();
    versions.sort();
    versions.dedup();
    versions
}

impl ComponentProvider for KotlinLsp {
    fn spec(&self) -> &ComponentSpec {
        &self.spec
    }

    fn resolve(
        &self,
        request: &VersionRequest,
        platform: &Platform,
        http: &HttpClient,
        settings: Option<&ComponentSettings>,
    ) -> Result<Release, ToolchainError> {
        match self.jetbrains(request, platform, http) {
            Ok(release) => Ok(release),
            // An explicit JetBrains version is never silently swapped for another server.
            Err(error @ ToolchainError::NoMatchingVersion { .. }) => Err(error),
            Err(_) => self
                .fallback
                .resolve(&VersionRequest::Latest, platform, http, settings),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_headings() {
        let md =
            "### Kotlin LSP\n### v263.6379.0\ntext\n### v262.9593.0\n### v1.2\n### v263.6379.0\n";
        assert_eq!(
            release_versions(md),
            vec![
                ToolVersion::new("262.9593.0"),
                ToolVersion::new("263.6379.0")
            ]
        );
    }
}
