//! JDKs from the foojay Disco API (<https://api.foojay.io>), the vendor-neutral
//! JDK catalog the reference IDE's *Download JDK…* uses. Temurin by default;
//! `distribution` in `toolchains.toml` picks another (`zulu`, `corretto`, ...).

use crate::download::HttpClient;
use crate::{
    ArchiveKind, Artifact, Checksum, ComponentKind, ComponentProvider, ComponentSettings,
    ComponentSpec, Os, Platform, Release, ToolVersion, ToolchainError, VersionRequest,
};
use serde::Deserialize;

pub const FOOJAY_API: &str = "https://api.foojay.io/disco/v3.0";
pub const DEFAULT_DISTRIBUTION: &str = "temurin";

#[derive(Deserialize)]
struct Envelope<T> {
    result: Vec<T>,
}

#[derive(Deserialize)]
struct MajorVersion {
    major_version: u32,
    #[serde(default)]
    term_of_support: String,
}

#[derive(Deserialize)]
struct Package {
    java_version: String,
    archive_type: String,
    #[serde(default)]
    javafx_bundled: bool,
    #[serde(default)]
    lib_c_type: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    links: Links,
}

#[derive(Deserialize)]
struct Links {
    pkg_info_uri: String,
}

#[derive(Deserialize)]
struct PackageInfo {
    filename: String,
    direct_download_uri: String,
    #[serde(default)]
    checksum: String,
    #[serde(default)]
    checksum_type: String,
    #[serde(default)]
    checksum_uri: String,
}

pub struct FoojayJdk {
    spec: ComponentSpec,
    api: String,
}

impl Default for FoojayJdk {
    fn default() -> Self {
        Self::with_api(FOOJAY_API)
    }
}

impl FoojayJdk {
    /// Uses another Disco API endpoint (mirrors, tests).
    pub fn with_api(api: &str) -> Self {
        Self {
            spec: spec(),
            api: api.trim_end_matches('/').to_owned(),
        }
    }

    fn catalog_error(&self, detail: impl Into<String>) -> ToolchainError {
        ToolchainError::Catalog {
            source_name: self.api.clone(),
            detail: detail.into(),
        }
    }

    /// The feature release for `latest` (newest GA) or `lts` (newest LTS).
    fn major_for_channel(
        &self,
        request: &VersionRequest,
        http: &HttpClient,
    ) -> Result<u32, ToolchainError> {
        let lts = match request {
            VersionRequest::Latest => false,
            VersionRequest::Channel(channel) if channel == "lts" => true,
            VersionRequest::Channel(channel) if channel == "stable" || channel == "ga" => false,
            other => {
                return Err(ToolchainError::NoMatchingVersion {
                    component: self.spec.id.clone(),
                    request: other.to_string(),
                });
            }
        };
        let url = format!(
            "{}/major_versions?ea=false&ga=true&maintained=true&include_build=false&include_versions=false",
            self.api
        );
        let majors: Envelope<MajorVersion> = http.get_json(&url)?;
        majors
            .result
            .iter()
            .filter(|major| !lts || major.term_of_support.eq_ignore_ascii_case("lts"))
            .map(|major| major.major_version)
            .max()
            .ok_or_else(|| self.catalog_error("no maintained JDK release listed"))
    }
}

/// The `jdk` component: `java`/`javac` under `bin/` (`Contents/Home/bin/` on macOS).
pub fn spec() -> ComponentSpec {
    ComponentSpec {
        id: "jdk".into(),
        name: "JDK".into(),
        kind: ComponentKind::Sdk,
        executables: vec!["java".into(), "javac".into()],
        bin_dirs: vec!["bin".into(), "Contents/Home/bin".into()],
        home_env: vec!["JAVA_HOME".into()],
        requires: Vec::new(),
        homepage: Some("https://foojay.io".into()),
    }
}

fn os_name(platform: &Platform) -> &'static str {
    match platform.os {
        Os::Linux => "linux",
        Os::Macos => "macos",
        Os::Windows => "windows",
    }
}

fn arch_name(platform: &Platform) -> &'static str {
    match platform.arch {
        crate::Arch::X86_64 => "x64",
        crate::Arch::Aarch64 => "aarch64",
    }
}

impl ComponentProvider for FoojayJdk {
    fn spec(&self) -> &ComponentSpec {
        &self.spec
    }

    /// Unpinned projects get the newest LTS release, as the reference IDE suggests.
    fn default_request(&self) -> VersionRequest {
        VersionRequest::Channel("lts".into())
    }

    fn resolve(
        &self,
        request: &VersionRequest,
        platform: &Platform,
        http: &HttpClient,
        settings: Option<&ComponentSettings>,
    ) -> Result<Release, ToolchainError> {
        let distribution = settings
            .and_then(|s| s.distribution.as_deref())
            .unwrap_or(DEFAULT_DISTRIBUTION)
            .to_ascii_lowercase();
        if distribution.is_empty()
            || !distribution
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ToolchainError::Settings {
                file: "toolchains.toml".into(),
                detail: format!("invalid JDK distribution `{distribution}`"),
            });
        }
        let (version, latest_only) = match request {
            VersionRequest::Version(prefix) => {
                let prefix = prefix.trim();
                if !prefix
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | '+'))
                {
                    return Err(ToolchainError::NoMatchingVersion {
                        component: self.spec.id.clone(),
                        request: prefix.into(),
                    });
                }
                (prefix.to_owned(), !prefix.contains(['.', '+']))
            }
            other => (self.major_for_channel(other, http)?.to_string(), true),
        };
        let archive = if platform.os == Os::Windows {
            "zip"
        } else {
            "tar.gz"
        };
        let mut url = format!(
            "{}/packages?version={}&distribution={distribution}&architecture={}&archive_type={archive}\
             &operating_system={}&package_type=jdk&release_status=ga&directly_downloadable=true",
            self.api,
            version.replace('+', "%2B"),
            arch_name(platform),
            os_name(platform),
        );
        if platform.os == Os::Linux {
            url.push_str("&lib_c_type=glibc");
        }
        if latest_only {
            url.push_str("&latest=available");
        }
        let packages: Envelope<Package> = http.get_json(&url)?;
        let best = packages
            .result
            .into_iter()
            .filter(|package| !package.javafx_bundled && package.archive_type == archive)
            .filter(|package| {
                platform.os != Os::Linux
                    || package
                        .lib_c_type
                        .as_deref()
                        .is_none_or(|libc| libc == "glibc")
            })
            .filter(|package| ToolVersion::new(&package.java_version).starts_with_version(&version))
            .max_by(|a, b| {
                ToolVersion::new(&a.java_version).cmp(&ToolVersion::new(&b.java_version))
            })
            .ok_or_else(|| ToolchainError::NoMatchingVersion {
                component: format!("JDK ({distribution}, {platform})"),
                request: request.to_string(),
            })?;
        let info: Envelope<PackageInfo> = http.get_json(&best.links.pkg_info_uri)?;
        let info = info
            .result
            .into_iter()
            .next()
            .ok_or_else(|| self.catalog_error("package info is empty"))?;
        let checksum =
            if info.checksum_type.eq_ignore_ascii_case("sha256") && info.checksum.len() == 64 {
                Checksum::Sha256(info.checksum.to_ascii_lowercase())
            } else if !info.checksum_uri.is_empty() {
                Checksum::Sha256Url(info.checksum_uri)
            } else {
                return Err(self.catalog_error(format!(
                    "{} has no SHA-256 checksum; refusing to install it",
                    info.filename
                )));
            };
        let archive_kind = ArchiveKind::from_file_name(&info.filename)
            .ok_or_else(|| self.catalog_error(format!("unknown archive type {}", info.filename)))?;
        Ok(Release {
            component: self.spec.id.clone(),
            version: ToolVersion::new(&best.java_version),
            artifact: Artifact {
                url: info.direct_download_uri,
                checksum,
                archive: archive_kind,
                size: best.size,
            },
            license: None,
        })
    }
}
