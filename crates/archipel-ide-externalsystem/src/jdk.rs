//! Finds the JDK a build runs on, for build systems whose tool does not report it.

use crate::data::SdkData;
use archipel_ide_projectmodel::SdkType;
use std::path::{Path, PathBuf};

/// The JDK from the managed tool environment's `JAVA_HOME`, the process `JAVA_HOME`, or the
/// `java` on `PATH`.
pub fn detect_jdk() -> Option<SdkData> {
    let from_env = archipel_ide_runner::tool_environment()
        .vars
        .into_iter()
        .find(|(name, _)| name == "JAVA_HOME")
        .map(|(_, value)| PathBuf::from(value))
        .or_else(|| std::env::var_os("JAVA_HOME").map(PathBuf::from));
    let home = from_env
        .filter(|home| home.join("bin").is_dir())
        .or_else(home_from_path)?;
    Some(sdk_at(&home))
}

fn home_from_path() -> Option<PathBuf> {
    let java = archipel_ide_runner::find_program("java")?;
    let real = std::fs::canonicalize(java).ok()?;
    // <home>/bin/java
    real.parent()?.parent().map(Path::to_owned)
}

pub fn sdk_at(home: &Path) -> SdkData {
    let version = release_version(home);
    SdkData {
        name: format!(
            "JDK {}",
            version.as_deref().unwrap_or_else(|| home
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("home"))
        ),
        kind: SdkType::Jdk,
        home: home.to_owned(),
        version,
        sources: vec![home.join("lib").join("src.zip")],
    }
}

/// `JAVA_VERSION="21.0.12"` from the JDK's `release` file.
fn release_version(home: &Path) -> Option<String> {
    let text = std::fs::read_to_string(home.join("release")).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("JAVA_VERSION="))
        .map(|v| v.trim().trim_matches('"').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_release_file() {
        let home = std::env::temp_dir().join(format!("archipel-jdk-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("release"), "IMPLEMENTOR=\"x\"\nJAVA_VERSION=\"21.0.12\"\n").unwrap();
        let sdk = sdk_at(&home);
        assert_eq!(sdk.version.as_deref(), Some("21.0.12"));
        assert_eq!(sdk.name, "JDK 21.0.12");
        std::fs::remove_dir_all(&home).unwrap();
    }
}
