//! The component catalog: providers that know where each component comes from.
//!
//! Language bundles implement [`ComponentProvider`] and register it in the
//! [`Catalog`]. [`StaticProvider`] (releases known at build time) and
//! [`IndexProvider`] (a JSON index on a server, e.g. a corporate mirror) cover
//! simple sources; API-backed sources (foojay Disco, GitHub releases) get
//! their own provider.

use crate::download::HttpClient;
use crate::{
    Artifact, ComponentSettings, ComponentSpec, Platform, Release, ToolVersion, ToolchainError,
    VersionRequest,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A source of releases for one component.
pub trait ComponentProvider: Send + Sync {
    fn spec(&self) -> &ComponentSpec;

    /// Resolves `request` to a concrete, verifiable release for `platform`.
    /// May use the network through `http`; never downloads the artifact itself.
    fn resolve(
        &self,
        request: &VersionRequest,
        platform: &Platform,
        http: &HttpClient,
        settings: Option<&ComponentSettings>,
    ) -> Result<Release, ToolchainError>;

    /// What to install when the project pins nothing (`lts` for JDKs).
    fn default_request(&self) -> VersionRequest {
        VersionRequest::Latest
    }
}

/// One release in a static list or JSON index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedRelease {
    pub version: ToolVersion,
    /// Channels this release belongs to (`stable`, `lts`).
    #[serde(default)]
    pub channels: Vec<String>,
    /// Artifacts keyed by platform (`linux-x86_64`, `macos-aarch64`, ...).
    pub artifacts: BTreeMap<String, Artifact>,
    #[serde(default)]
    pub license: Option<String>,
}

/// JSON index format served by [`IndexProvider`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseIndex {
    pub releases: Vec<IndexedRelease>,
}

impl ReleaseIndex {
    /// Picks the newest release matching `request` with an artifact for `platform`.
    pub fn resolve(
        &self,
        component: &str,
        request: &VersionRequest,
        platform: &Platform,
    ) -> Result<Release, ToolchainError> {
        let key = platform.key();
        let matching: Vec<&IndexedRelease> = self
            .releases
            .iter()
            .filter(|release| match request {
                VersionRequest::Channel(channel) => release
                    .channels
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(channel)),
                other => other.matches(&release.version),
            })
            .collect();
        if matching.is_empty() {
            return Err(ToolchainError::NoMatchingVersion {
                component: component.into(),
                request: request.to_string(),
            });
        }
        let best = matching
            .into_iter()
            .filter(|release| release.artifacts.contains_key(&key))
            .max_by(|a, b| a.version.cmp(&b.version))
            .ok_or_else(|| ToolchainError::UnsupportedPlatform {
                component: component.into(),
                platform: key.clone(),
            })?;
        Ok(Release {
            component: component.into(),
            version: best.version.clone(),
            artifact: best.artifacts[&key].clone(),
            license: best.license.clone(),
        })
    }
}

/// Releases known up front.
pub struct StaticProvider {
    pub spec: ComponentSpec,
    pub index: ReleaseIndex,
}

impl ComponentProvider for StaticProvider {
    fn spec(&self) -> &ComponentSpec {
        &self.spec
    }

    fn resolve(
        &self,
        request: &VersionRequest,
        platform: &Platform,
        _http: &HttpClient,
        _settings: Option<&ComponentSettings>,
    ) -> Result<Release, ToolchainError> {
        self.index.resolve(&self.spec.id, request, platform)
    }
}

/// Releases listed in a JSON [`ReleaseIndex`] at `url`.
pub struct IndexProvider {
    pub spec: ComponentSpec,
    pub url: String,
}

impl ComponentProvider for IndexProvider {
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
        let index: ReleaseIndex = http.get_json(&self.url)?;
        index.resolve(&self.spec.id, request, platform)
    }
}

/// Registry of component providers.
#[derive(Clone, Default)]
pub struct Catalog {
    providers: Vec<Arc<dyn ComponentProvider>>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a provider; a later registration replaces one with the same id.
    pub fn register(&mut self, provider: Arc<dyn ComponentProvider>) {
        let id = provider.spec().id.clone();
        self.providers.retain(|existing| existing.spec().id != id);
        self.providers.push(provider);
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn ComponentProvider>> {
        self.providers.iter().find(|p| p.spec().id == id)
    }

    pub fn providers(&self) -> &[Arc<dyn ComponentProvider>] {
        &self.providers
    }

    pub fn specs(&self) -> impl Iterator<Item = &ComponentSpec> {
        self.providers.iter().map(|p| p.spec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArchiveKind, Checksum, ComponentKind};

    fn artifact(url: &str) -> Artifact {
        Artifact {
            url: url.into(),
            checksum: Checksum::Sha256("0".repeat(64)),
            archive: ArchiveKind::TarGz,
            size: None,
        }
    }

    fn index() -> ReleaseIndex {
        let linux = |url: &str| BTreeMap::from([("linux-x86_64".to_string(), artifact(url))]);
        ReleaseIndex {
            releases: vec![
                IndexedRelease {
                    version: ToolVersion::new("17.0.12"),
                    channels: vec!["lts".into()],
                    artifacts: linux("17"),
                    license: None,
                },
                IndexedRelease {
                    version: ToolVersion::new("21.0.4"),
                    channels: vec!["lts".into()],
                    artifacts: linux("21"),
                    license: None,
                },
                IndexedRelease {
                    version: ToolVersion::new("23.0.1"),
                    channels: vec![],
                    artifacts: BTreeMap::new(),
                    license: None,
                },
            ],
        }
    }

    #[test]
    fn resolves_versions_channels_and_platforms() {
        let linux = Platform::parse("linux", "x86_64").unwrap();
        let mac = Platform::parse("macos", "aarch64").unwrap();
        let index = index();
        let resolve = |request: &str, platform| {
            index.resolve("jdk", &VersionRequest::parse(request), platform)
        };
        assert_eq!(resolve("17", &linux).unwrap().artifact.url, "17");
        assert_eq!(resolve("lts", &linux).unwrap().version.as_str(), "21.0.4");
        // Latest skips releases without an artifact for this platform.
        assert_eq!(
            resolve("latest", &linux).unwrap().version.as_str(),
            "21.0.4"
        );
        assert!(matches!(
            resolve("23", &linux),
            Err(ToolchainError::UnsupportedPlatform { .. })
        ));
        assert!(matches!(
            resolve("17", &mac),
            Err(ToolchainError::UnsupportedPlatform { .. })
        ));
        assert!(matches!(
            resolve("11", &linux),
            Err(ToolchainError::NoMatchingVersion { .. })
        ));
    }

    #[test]
    fn later_registrations_replace_earlier_ones() {
        let mut catalog = Catalog::new();
        for url in ["a", "b"] {
            let mut spec = ComponentSpec::new("jdk", "JDK", ComponentKind::Sdk, "java");
            spec.homepage = Some(url.into());
            catalog.register(Arc::new(StaticProvider {
                spec,
                index: ReleaseIndex::default(),
            }));
        }
        assert_eq!(catalog.providers().len(), 1);
        assert_eq!(
            catalog.get("jdk").unwrap().spec().homepage.as_deref(),
            Some("b")
        );
    }
}
