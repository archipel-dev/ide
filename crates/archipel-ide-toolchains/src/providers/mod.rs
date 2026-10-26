//! Built-in component providers (the first-wave catalog).
//!
//! | Component | Source |
//! |---|---|
//! | `jdk` | foojay Disco API (Temurin by default) |
//! | `jdtls` | download.eclipse.org milestones |
//! | `kotlin-lsp` | JetBrains kotlin-lsp (fwcd/kotlin-language-server as fallback) |
//! | `java-debug` | Maven Central (microsoft/java-debug jdtls plugin) |
//! | `gradle` | services.gradle.org distributions (for projects without `gradlew`) |
//! | `rust-analyzer` | GitHub releases of rust-lang/rust-analyzer |
//! | `bazelisk` | GitHub releases of bazelbuild/bazelisk |
//! | `taplo` | GitHub releases of tamasfe/taplo |

pub mod github;
pub mod gradle;
pub mod java_debug;
pub mod jdk;
pub mod jdtls;
pub mod kotlin_lsp;

use crate::{Arch, ArchiveKind, Catalog, ComponentKind, ComponentSpec, Os, Platform};
use github::GithubReleaseProvider;
use std::sync::Arc;

/// The catalog Archipel IDE ships with.
pub fn builtin_catalog() -> Catalog {
    let mut catalog = Catalog::new();
    catalog.register(Arc::new(jdk::FoojayJdk::default()));
    catalog.register(Arc::new(jdtls::EclipseJdtls::default()));
    catalog.register(Arc::new(java_debug::JavaDebugPlugin::default()));
    catalog.register(Arc::new(kotlin_lsp::KotlinLsp::default()));
    catalog.register(Arc::new(gradle::GradleDistributions::default()));
    catalog.register(Arc::new(rust_analyzer()));
    catalog.register(Arc::new(bazelisk()));
    catalog.register(Arc::new(taplo()));
    catalog
}

/// Rust target triple for a platform (`x86_64-unknown-linux-gnu`).
pub fn rust_triple(platform: &Platform) -> &'static str {
    match (platform.os, platform.arch) {
        (Os::Linux, Arch::X86_64) => "x86_64-unknown-linux-gnu",
        (Os::Linux, Arch::Aarch64) => "aarch64-unknown-linux-gnu",
        (Os::Macos, Arch::X86_64) => "x86_64-apple-darwin",
        (Os::Macos, Arch::Aarch64) => "aarch64-apple-darwin",
        (Os::Windows, Arch::X86_64) => "x86_64-pc-windows-msvc",
        (Os::Windows, Arch::Aarch64) => "aarch64-pc-windows-msvc",
    }
}

pub fn rust_analyzer() -> GithubReleaseProvider {
    let mut spec = ComponentSpec::new(
        "rust-analyzer",
        "rust-analyzer",
        ComponentKind::LanguageServer,
        "rust-analyzer",
    );
    spec.homepage = Some("https://rust-analyzer.github.io".into());
    GithubReleaseProvider::new(spec, "rust-lang/rust-analyzer", |platform, _tag| {
        let triple = rust_triple(platform);
        Some(if platform.os == Os::Windows {
            (format!("rust-analyzer-{triple}.zip"), ArchiveKind::Zip)
        } else {
            (
                format!("rust-analyzer-{triple}.gz"),
                ArchiveKind::Gz {
                    file_name: "rust-analyzer".into(),
                },
            )
        })
    })
    .with_tag_channels(&["nightly"])
}

fn go_style(platform: &Platform) -> (&'static str, &'static str) {
    let os = match platform.os {
        Os::Linux => "linux",
        Os::Macos => "darwin",
        Os::Windows => "windows",
    };
    let arch = match platform.arch {
        Arch::X86_64 => "amd64",
        Arch::Aarch64 => "arm64",
    };
    (os, arch)
}

pub fn bazelisk() -> GithubReleaseProvider {
    let mut spec = ComponentSpec::new("bazelisk", "Bazelisk", ComponentKind::BuildTool, "bazelisk");
    spec.homepage = Some("https://github.com/bazelbuild/bazelisk".into());
    GithubReleaseProvider::new(spec, "bazelbuild/bazelisk", |platform, _tag| {
        let (os, arch) = go_style(platform);
        let exe = if platform.os == Os::Windows {
            ".exe"
        } else {
            ""
        };
        Some((
            format!("bazelisk-{os}-{arch}{exe}"),
            ArchiveKind::Raw {
                file_name: format!("bazelisk{exe}"),
            },
        ))
    })
}

pub fn taplo() -> GithubReleaseProvider {
    let mut spec = ComponentSpec::new(
        "taplo",
        "Taplo (TOML)",
        ComponentKind::LanguageServer,
        "taplo",
    );
    spec.homepage = Some("https://taplo.tamasfe.dev".into());
    GithubReleaseProvider::new(spec, "tamasfe/taplo", |platform, _tag| {
        let os = match platform.os {
            Os::Linux => "linux",
            Os::Macos => "darwin",
            Os::Windows => "windows",
        };
        let arch = match platform.arch {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
        };
        Some(if platform.os == Os::Windows {
            (format!("taplo-{os}-{arch}.zip"), ArchiveKind::Zip)
        } else {
            (
                format!("taplo-{os}-{arch}.gz"),
                ArchiveKind::Gz {
                    file_name: "taplo".into(),
                },
            )
        })
    })
}

/// What a project of one language needs, detected from marker files.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ProjectRequirement {
    /// Display name ("Java", "Rust", "Bazel").
    pub language: &'static str,
    /// Component ids, in setup order.
    pub components: &'static [&'static str],
    /// File extensions whose editors show the "missing toolchain" bar.
    pub extensions: &'static [&'static str],
    /// Exact file names that also show it (`BUILD`).
    pub file_names: &'static [&'static str],
}

impl ProjectRequirement {
    /// Whether an editor for `path` belongs to this requirement's language.
    pub fn applies_to(&self, path: &std::path::Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        self.file_names.contains(&name)
            || path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
    }
}

const REQUIREMENTS: &[(&[&str], ProjectRequirement)] = &[
    (
        &[
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
            "pom.xml",
        ],
        ProjectRequirement {
            language: "Java",
            components: &["jdk", "jdtls", "java-debug"],
            extensions: &["java", "gradle"],
            file_names: &["pom.xml"],
        },
    ),
    (
        &["Cargo.toml"],
        ProjectRequirement {
            language: "Rust",
            components: &["rust-analyzer"],
            extensions: &["rs"],
            file_names: &["Cargo.toml"],
        },
    ),
    (
        &[
            "MODULE.bazel",
            "WORKSPACE",
            "WORKSPACE.bazel",
            ".bazelversion",
        ],
        ProjectRequirement {
            language: "Bazel",
            components: &["bazelisk"],
            extensions: &["bzl", "bazel"],
            file_names: &["BUILD", "WORKSPACE", "MODULE.bazel", ".bazelversion"],
        },
    ),
];

const GRADLE_MARKERS: &[&str] = &[
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
];

/// A Gradle project without the wrapper also needs a Gradle distribution.
const JAVA_GRADLE_WITHOUT_WRAPPER: ProjectRequirement = ProjectRequirement {
    language: "Java",
    components: &["jdk", "gradle", "jdtls", "java-debug"],
    extensions: &["java", "gradle"],
    file_names: &["pom.xml"],
};

const KOTLIN: ProjectRequirement = ProjectRequirement {
    language: "Kotlin",
    components: &["kotlin-lsp"],
    extensions: &["kt", "kts"],
    file_names: &[],
};

/// Whether the build at `root` uses Kotlin: Kotlin DSL scripts, a Kotlin plugin or
/// dependency in a Gradle/Maven build file, or a Kotlin source directory.
fn uses_kotlin(root: &std::path::Path) -> bool {
    let has = |name: &str| root.join(name).exists();
    if has("build.gradle.kts") || has("settings.gradle.kts") || has("src/main/kotlin") {
        return true;
    }
    ["build.gradle", "settings.gradle", "pom.xml"]
        .iter()
        .any(|name| {
            std::fs::read_to_string(root.join(name))
                .is_ok_and(|text| text.to_ascii_lowercase().contains("kotlin"))
        })
}

/// Requirements of the project at `root`, from marker files in the root.
pub fn project_requirements(root: &std::path::Path) -> Vec<ProjectRequirement> {
    let mut requirements = base_requirements(root);
    if uses_kotlin(root) {
        let at = requirements
            .iter()
            .position(|r| r.language == "Java")
            .map_or(requirements.len(), |index| index + 1);
        requirements.insert(at, KOTLIN);
    }
    requirements
}

fn base_requirements(root: &std::path::Path) -> Vec<ProjectRequirement> {
    let has = |name: &str| root.join(name).exists();
    let gradle_without_wrapper =
        GRADLE_MARKERS.iter().any(|marker| has(marker)) && !has("gradlew") && !has("gradlew.bat");
    REQUIREMENTS
        .iter()
        .filter(|(markers, _)| markers.iter().any(|marker| has(marker)))
        .map(|(_, requirement)| {
            if requirement.language == "Java" && gradle_without_wrapper {
                JAVA_GRADLE_WITHOUT_WRAPPER
            } else {
                requirement.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_project_requirements_from_markers() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-reqs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(project_requirements(&dir).is_empty());
        std::fs::write(dir.join("settings.gradle.kts"), "").unwrap();
        std::fs::write(dir.join("MODULE.bazel"), "").unwrap();
        let found = project_requirements(&dir);
        let languages: Vec<&str> = found.iter().map(|r| r.language).collect();
        // A Kotlin DSL build adds the Kotlin language server after Java's components.
        assert_eq!(languages, ["Java", "Kotlin", "Bazel"]);
        assert!(found[0].applies_to(Path::new("src/App.java")));
        assert!(!found[0].applies_to(Path::new("README.md")));
        assert!(found[1].applies_to(Path::new("src/Main.kt")));
        assert!(found[2].applies_to(Path::new("pkg/BUILD")));
        // Without a wrapper the Java project also needs Gradle; with one it does not.
        assert_eq!(
            found[0].components,
            ["jdk", "gradle", "jdtls", "java-debug"]
        );
        std::fs::write(dir.join("gradlew"), "").unwrap();
        assert_eq!(
            project_requirements(&dir)[0].components,
            ["jdk", "jdtls", "java-debug"]
        );
        // Every required component is in the built-in catalog.
        let catalog = builtin_catalog();
        let all = REQUIREMENTS
            .iter()
            .map(|(_, requirement)| requirement)
            .chain([&JAVA_GRADLE_WITHOUT_WRAPPER, &KOTLIN]);
        for requirement in all {
            for component in requirement.components {
                assert!(catalog.get(component).is_some(), "{component}");
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn kotlin_is_detected_from_build_files_and_sources() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-kt-reqs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let kotlin = |dir: &std::path::Path| {
            project_requirements(dir)
                .iter()
                .any(|r| r.language == "Kotlin")
        };
        std::fs::write(dir.join("build.gradle"), "plugins { id 'java' }").unwrap();
        assert!(!kotlin(&dir));
        std::fs::write(
            dir.join("build.gradle"),
            "plugins { id 'org.jetbrains.kotlin.jvm' version '2.0.0' }",
        )
        .unwrap();
        assert!(kotlin(&dir));
        std::fs::write(dir.join("build.gradle"), "").unwrap();
        std::fs::create_dir_all(dir.join("src/main/kotlin")).unwrap();
        assert!(kotlin(&dir));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
