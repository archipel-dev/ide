use thiserror::Error;

/// Structured, user-presentable provisioning errors.
#[derive(Debug, Error)]
pub enum ToolchainError {
    #[error(
        "cannot reach {host}: {detail}. Check the network or proxy settings (HTTPS_PROXY), \
         or point Archipel IDE at an existing installation."
    )]
    Offline { host: String, detail: String },
    #[error("download of {url} failed with HTTP {status}")]
    Http { url: String, status: u16 },
    #[error("network error for {url}: {detail}")]
    Network { url: String, detail: String },
    #[error("checksum mismatch for {file}: expected {expected}, got {actual}")]
    Checksum {
        file: String,
        expected: String,
        actual: String,
    },
    #[error("{component} is not available for {platform}")]
    UnsupportedPlatform { component: String, platform: String },
    #[error("no {component} release matches `{request}`")]
    NoMatchingVersion { component: String, request: String },
    #[error("unknown toolchain component `{0}`")]
    UnknownComponent(String),
    #[error("{component} {version} is not installed")]
    NotInstalled { component: String, version: String },
    #[error("invalid archive {file}: {detail}")]
    Archive { file: String, detail: String },
    #[error("unexpected catalog data from {source_name}: {detail}")]
    Catalog { source_name: String, detail: String },
    #[error("invalid {file}: {detail}")]
    Settings { file: String, detail: String },
    #[error("cancelled")]
    Cancelled,
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

impl ToolchainError {
    pub(crate) fn archive(file: impl Into<String>, detail: impl ToString) -> Self {
        Self::Archive {
            file: file.into(),
            detail: detail.to_string(),
        }
    }
}
