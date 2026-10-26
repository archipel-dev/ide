//! What was built: version, commit, date and target, for `--version`,
//! Help → About and bug reports. Filled in by `build.rs`.

use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInfo {
    /// `[workspace.package] version`.
    pub version: &'static str,
    /// First 12 hex digits of the commit, or `unknown` outside a checkout.
    pub commit: &'static str,
    /// Commit date (`YYYY-MM-DD`), or `unknown`.
    pub date: &'static str,
    /// Rust target triple.
    pub target: &'static str,
    /// Cargo profile (`release` or `debug`).
    pub profile: &'static str,
}

/// This binary's build.
pub const BUILD: BuildInfo = BuildInfo {
    version: env!("CARGO_PKG_VERSION"),
    commit: env!("ARCHIPEL_BUILD_COMMIT"),
    date: env!("ARCHIPEL_BUILD_DATE"),
    target: env!("ARCHIPEL_BUILD_TARGET"),
    profile: env!("ARCHIPEL_BUILD_PROFILE"),
};

impl BuildInfo {
    /// One line for `--version`: `archipel-ide 0.0.2 (1a2b3c4d5e6f 2026-10-10, x86_64-unknown-linux-gnu, release)`.
    pub fn version_line(&self, program: &str) -> String {
        format!(
            "{program} {} ({} {}, {}, {})",
            self.version, self.commit, self.date, self.target, self.profile
        )
    }

    /// Label/value rows for the About dialog, including the running OS.
    pub fn rows(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Version", self.version.to_owned()),
            ("Build", format!("{} ({})", self.commit, self.date)),
            ("Target", self.target.to_owned()),
            ("Profile", self.profile.to_owned()),
            (
                "Runtime",
                format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
            ),
        ]
    }

    /// Plain text to paste into a bug report.
    pub fn report(&self) -> String {
        let mut text = format!("Archipel IDE {}\n", self.version);
        for (label, value) in self.rows().into_iter().skip(1) {
            text.push_str(&format!("{label}: {value}\n"));
        }
        text
    }

    pub fn to_json(&self) -> Value {
        json!({
            "version": self.version,
            "commit": self.commit,
            "date": self.date,
            "target": self.target,
            "profile": self.profile,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        })
    }
}

/// Handles `--version`/`-V` and `--help`/`-h` before anything else starts.
/// Returns the text to print, or `None` to continue normally.
pub fn cli_flag_output(program: &str, usage: &str, first_arg: Option<&str>) -> Option<String> {
    match first_arg? {
        "--version" | "-V" => Some(BUILD.version_line(program)),
        "--help" | "-h" => Some(format!("{}\n\n{usage}", BUILD.version_line(program))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_metadata_is_embedded() {
        assert_eq!(BUILD.version, env!("CARGO_PKG_VERSION"));
        assert!(!BUILD.commit.is_empty());
        assert!(BUILD.commit == "unknown" || BUILD.commit.len() <= 12);
        assert!(BUILD.date == "unknown" || BUILD.date.len() == 10);
        assert!(BUILD.target.contains('-'), "{}", BUILD.target);
        assert!(matches!(BUILD.profile, "debug" | "release"));
    }

    #[test]
    fn version_line_and_report_name_the_build() {
        let info = BuildInfo {
            version: "1.2.3",
            commit: "0123456789ab",
            date: "2026-10-10",
            target: "x86_64-unknown-linux-gnu",
            profile: "release",
        };
        assert_eq!(
            info.version_line("archipel-ide"),
            "archipel-ide 1.2.3 (0123456789ab 2026-10-10, x86_64-unknown-linux-gnu, release)"
        );
        let report = info.report();
        assert!(report.starts_with("Archipel IDE 1.2.3\n"));
        assert!(report.contains("Build: 0123456789ab (2026-10-10)\n"));
        assert!(report.contains(&format!("Runtime: {}", std::env::consts::OS)));
        assert_eq!(info.to_json()["commit"], "0123456789ab");
    }

    #[test]
    fn version_and_help_flags_short_circuit() {
        let version = cli_flag_output("archipel-ide", "usage", Some("--version")).unwrap();
        assert!(version.starts_with(&format!("archipel-ide {}", BUILD.version)));
        assert_eq!(
            cli_flag_output("archipel-ide", "usage", Some("-V")),
            Some(version)
        );
        assert!(
            cli_flag_output("archipel-ide", "usage: x", Some("--help"))
                .unwrap()
                .ends_with("\n\nusage: x")
        );
        assert_eq!(cli_flag_output("archipel-ide", "usage", Some(".")), None);
        assert_eq!(cli_flag_output("archipel-ide", "usage", None), None);
    }
}
