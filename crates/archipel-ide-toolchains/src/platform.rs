//! Host operating system and CPU architecture.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Linux,
    Macos,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    /// The platform Archipel IDE runs on, if it is one toolchains are published for.
    pub fn current() -> Option<Platform> {
        Self::parse(std::env::consts::OS, std::env::consts::ARCH)
    }

    /// From `std::env::consts::{OS, ARCH}`-style names (and common aliases).
    pub fn parse(os: &str, arch: &str) -> Option<Platform> {
        let os = match os {
            "linux" => Os::Linux,
            "macos" | "darwin" | "mac" => Os::Macos,
            "windows" | "win" => Os::Windows,
            _ => return None,
        };
        let arch = match arch {
            "x86_64" | "amd64" | "x64" => Arch::X86_64,
            "aarch64" | "arm64" => Arch::Aarch64,
            _ => return None,
        };
        Some(Platform { os, arch })
    }

    /// `linux-x86_64`, the key used in catalog indexes.
    pub fn key(&self) -> String {
        self.to_string()
    }

    /// Executable file names to try for `name` (`java` → `java.exe` on Windows).
    pub fn executable_names(&self, name: &str) -> Vec<String> {
        if self.os == Os::Windows && !name.contains('.') {
            ["exe", "cmd", "bat"]
                .iter()
                .map(|ext| format!("{name}.{ext}"))
                .collect()
        } else {
            vec![name.to_owned()]
        }
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Os::Linux => "linux",
            Os::Macos => "macos",
            Os::Windows => "windows",
        })
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
        })
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.os, self.arch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aliases_and_formats_keys() {
        let platform = Platform::parse("darwin", "arm64").unwrap();
        assert_eq!(platform.key(), "macos-aarch64");
        assert_eq!(Platform::parse("freebsd", "x86_64"), None);
        let windows = Platform::parse("windows", "x64").unwrap();
        assert_eq!(windows.executable_names("java")[0], "java.exe");
        assert_eq!(windows.executable_names("jdtls.py"), vec!["jdtls.py"]);
    }
}
