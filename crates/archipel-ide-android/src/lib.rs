//! Android tooling: SDK discovery, package inventory and Android Gradle project
//! detection.
//!
//! Everything here reads files and environment only through [`HostEnvironment`]
//! so the IDE core can swap in a fake host in tests. Nothing in this crate
//! depends on Android Studio; it works with an SDK installed by `sdkmanager`.

pub mod adb;
pub mod avd;
pub mod deploy;
pub mod layout;
pub mod logcat;
pub mod merged_manifest;
pub mod mirror;
pub mod project;
mod properties;
pub mod resource_manager;
pub mod resources;
pub mod sdk;
pub mod sdk_repo;
pub mod variants;
pub mod vector;
pub mod view;

pub use adb::{
    Adb, AdbError, Device, DeviceProperties, DeviceState, DeviceTracker, TrackEvent, find_adb,
};
pub use avd::{Avd, AvdKind, EmulatorProcess, avd_home, find_emulator, list_avds};
pub use logcat::{FilterContext, LogEntry, LogFilter, LogLevel, LogcatEvent, LogcatSession};
pub use project::{AndroidModule, AndroidProject, LauncherActivity, ModuleKind, ProjectError};
pub use sdk::{
    AndroidSdk, PackageCategory, SdkDetection, SdkPackage, SdkProbe, SdkSource, api_release_name,
    validate as validate_sdk, write_local_properties_sdk_dir,
};

use std::ffi::OsString;
use std::path::PathBuf;

/// Host facts needed for SDK discovery.
pub trait HostEnvironment {
    fn var(&self, name: &str) -> Option<OsString>;
    fn home_dir(&self) -> Option<PathBuf>;
    /// `std::env::consts::OS` style name: `linux`, `macos`, `windows`.
    fn os(&self) -> &str;
}

/// The real process environment.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemEnvironment;

impl HostEnvironment for SystemEnvironment {
    fn var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name).filter(|value| !value.is_empty())
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.var("HOME")
            .or_else(|| self.var("USERPROFILE"))
            .map(PathBuf::from)
    }

    fn os(&self) -> &str {
        std::env::consts::OS
    }
}
