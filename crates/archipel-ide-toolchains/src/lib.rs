//! Toolchain provisioning: language servers, SDKs/JDKs, build tools, debuggers
//! and formatters that a language integration needs.
//!
//! The crate is headless (no UI dependencies). It provides:
//!
//! - a component model ([`ComponentSpec`], [`Release`], [`Artifact`]) and the
//!   [`ComponentProvider`] trait language bundles implement to describe where a
//!   component comes from ([`catalog`]);
//! - version requests and ordering ([`version`]);
//! - resumable, SHA-256 verified, proxy-aware downloads ([`download`]);
//! - safe archive extraction for `tar.gz`, `tar.xz`, `zip`, `gz` and raw
//!   executables ([`extract`]);
//! - atomic installs under `<data dir>/archipel/toolchains/<component>/<version>/`
//!   with uninstall ([`store`]);
//! - PATH-first discovery with manual paths and managed installs ([`discovery`]);
//! - project pinning from `.archipel/toolchains.toml`, `rust-toolchain.toml`,
//!   `.bazelversion`, Gradle toolchains, `.nvmrc`, `.python-version` and `go.mod`
//!   ([`pinning`]);
//! - a background install service with progress and cancellation ([`service`]);
//! - the built-in catalog: JDK (foojay), jdtls, rust-analyzer, bazelisk, taplo
//!   ([`providers`]).
//!
//! Nothing here downloads on its own: installs only start from an explicit
//! [`service::ToolchainService::install`] call, which the IDE issues from a user
//! action (the `toolchain.install` command).

pub mod catalog;
pub mod discovery;
pub mod download;
pub mod extract;
pub mod pinning;
pub mod platform;
pub mod providers;
pub mod service;
pub mod settings;
pub mod store;
pub mod version;

mod error;
mod model;

pub use catalog::{Catalog, ComponentProvider, IndexProvider, StaticProvider};
pub use discovery::{LocateSource, Located, Toolchains};
pub use error::ToolchainError;
pub use model::{ArchiveKind, Artifact, Checksum, ComponentKind, ComponentSpec, Release};
pub use pinning::{Pin, PinSource, ProjectPins};
pub use platform::{Arch, Os, Platform};
pub use settings::{ComponentSettings, ToolchainSettings};
pub use store::{Installed, ToolchainStore};
pub use version::{ToolVersion, VersionRequest};

use std::path::PathBuf;

/// Root of managed installs: `$ARCHIPEL_TOOLCHAINS_DIR`, else
/// `<data dir>/archipel/toolchains` (`$XDG_DATA_HOME` or `~/.local/share` on
/// Linux, `~/Library/Application Support` on macOS, `%LOCALAPPDATA%` on Windows).
pub fn default_root() -> PathBuf {
    let env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if let Some(dir) =
        archipel_ide_compat::env_var_os("ARCHIPEL_TOOLCHAINS_DIR").filter(|value| !value.is_empty())
    {
        return PathBuf::from(dir);
    }
    let base = if cfg!(windows) {
        env("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".local/share")))
    };
    let root = base
        .unwrap_or_else(std::env::temp_dir)
        .join("archipel")
        .join("toolchains");
    // Installs made by the Jetthing versions (`.../jetthing/toolchains`) move to the new
    // parent directory on first use; their receipts are still read.
    if !root.exists()
        && let Some(parent) = root.parent()
    {
        archipel_ide_compat::adopt_legacy_dir(parent.to_path_buf());
    }
    root
}

/// Per-user configuration directory (`$XDG_CONFIG_HOME/archipel`, ...).
pub fn config_dir() -> Option<PathBuf> {
    let env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let base = if cfg!(windows) {
        env("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".config")))
    };
    base.map(|base| archipel_ide_compat::adopt_legacy_dir(base.join("archipel")))
}
