//! Components published as GitHub release assets (rust-analyzer, bazelisk,
//! taplo, ...). The expected SHA-256 comes from the asset's `digest` field,
//! else from a `<asset>.sha256` or checksum-list asset in the same release.

use crate::download::HttpClient;
use crate::{
    ArchiveKind, Artifact, Checksum, ComponentProvider, ComponentSettings, ComponentSpec, Platform,
    Release, ToolVersion, ToolchainError, VersionRequest,
};
use serde::Deserialize;

pub const GITHUB_API: &str = "https://api.github.com";

/// Picks the asset name and archive kind for a platform and release tag.
pub type AssetPicker = dyn Fn(&Platform, &str) -> Option<(String, ArchiveKind)> + Send + Sync;

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: Option<u64>,
    /// `sha256:<hex>`, set by GitHub for assets uploaded since mid-2025.
    #[serde(default)]
    digest: Option<String>,
}

pub struct GithubReleaseProvider {
    spec: ComponentSpec,
    repository: String,
    api: String,
    asset: Box<AssetPicker>,
    /// Channels that are release tags (`nightly` for rust-analyzer).
    tag_channels: &'static [&'static str],
}

impl GithubReleaseProvider {
    pub fn new(
        spec: ComponentSpec,
        repository: &str,
        asset: impl Fn(&Platform, &str) -> Option<(String, ArchiveKind)> + Send + Sync + 'static,
    ) -> Self {
        Self {
            spec,
            repository: repository.into(),
            api: GITHUB_API.into(),
            asset: Box::new(asset),
            tag_channels: &[],
        }
    }

    /// Uses another API endpoint (GitHub Enterprise mirrors, tests).
    pub fn with_api(mut self, api: &str) -> Self {
        self.api = api.trim_end_matches('/').to_owned();
        self
    }

    pub fn with_tag_channels(mut self, channels: &'static [&'static str]) -> Self {
        self.tag_channels = channels;
        self
    }

    fn release(
        &self,
        request: &VersionRequest,
        http: &HttpClient,
    ) -> Result<GithubRelease, ToolchainError> {
        let base = format!("{}/repos/{}/releases", self.api, self.repository);
        let url = match request {
            VersionRequest::Latest => format!("{base}/latest"),
            VersionRequest::Channel(channel) if self.tag_channels.contains(&channel.as_str()) => {
                format!("{base}/tags/{channel}")
            }
            VersionRequest::Channel(_) => {
                return Err(ToolchainError::NoMatchingVersion {
                    component: self.spec.id.clone(),
                    request: request.to_string(),
                });
            }
            VersionRequest::Version(version) => {
                // Try the tag as given, then with the conventional `v` prefix.
                let exact: Result<GithubRelease, _> =
                    http.get_json(&format!("{base}/tags/{version}"));
                return match exact {
                    Err(ToolchainError::Http { status: 404, .. }) => http
                        .get_json(&format!("{base}/tags/v{version}"))
                        .map_err(|error| match error {
                            ToolchainError::Http { status: 404, .. } => {
                                ToolchainError::NoMatchingVersion {
                                    component: self.spec.id.clone(),
                                    request: version.clone(),
                                }
                            }
                            other => other,
                        }),
                    other => other,
                };
            }
        };
        http.get_json(&url)
    }
}

impl ComponentProvider for GithubReleaseProvider {
    fn spec(&self) -> &ComponentSpec {
        &self.spec
    }

    fn resolve(
        &self,
        request: &VersionRequest,
        platform: &Platform,
        http: &HttpClient,
        _settings: Option<&ComponentSettings>,
    ) -> Result<Release, ToolchainError> {
        let release = self.release(request, http)?;
        if release.draft {
            return Err(ToolchainError::NoMatchingVersion {
                component: self.spec.id.clone(),
                request: request.to_string(),
            });
        }
        let (name, archive) = (self.asset)(platform, &release.tag_name).ok_or_else(|| {
            ToolchainError::UnsupportedPlatform {
                component: self.spec.name.clone(),
                platform: platform.key(),
            }
        })?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .ok_or_else(|| ToolchainError::UnsupportedPlatform {
                component: format!("{} {}", self.spec.name, release.tag_name),
                platform: platform.key(),
            })?;
        let digest = asset
            .digest
            .as_deref()
            .and_then(|digest| digest.strip_prefix("sha256:"))
            .filter(|hex| hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()));
        let checksum = match digest {
            Some(hex) => Checksum::Sha256(hex.to_ascii_lowercase()),
            None => {
                let sidecar = format!("{name}.sha256");
                let list = release.assets.iter().find(|candidate| {
                    let lower = candidate.name.to_ascii_lowercase();
                    candidate.name == sidecar
                        || lower == "sha256sums"
                        || lower == "sha256sums.txt"
                        || lower.ends_with("checksums.txt")
                });
                let list = list.ok_or_else(|| ToolchainError::Catalog {
                    source_name: format!("github.com/{}", self.repository),
                    detail: format!(
                        "{name} has no published SHA-256 checksum; refusing to install it"
                    ),
                })?;
                Checksum::Sha256Url(list.browser_download_url.clone())
            }
        };
        // Tags are versions (`v0.9.3`) or dates (`2026-10-06` for rust-analyzer).
        Ok(Release {
            component: self.spec.id.clone(),
            version: ToolVersion::new(&release.tag_name),
            artifact: Artifact {
                url: asset.browser_download_url.clone(),
                checksum,
                archive,
                size: asset.size,
            },
            license: None,
        })
    }
}
