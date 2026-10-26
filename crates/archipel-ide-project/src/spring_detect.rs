//! Static detection of Spring Boot projects from their build files.

use std::path::{Path, PathBuf};

/// The build tool whose file references Spring Boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpringTool {
    Gradle,
    Maven,
    Bazel,
}

impl SpringTool {
    /// Value of the `build` field of a Spring Boot run configuration.
    pub fn id(self) -> &'static str {
        match self {
            Self::Gradle => "gradle",
            Self::Maven => "maven",
            Self::Bazel => "bazel",
        }
    }
}

/// A directory whose build file applies Spring Boot (plugin, parent or BOM).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpringModule {
    pub dir: PathBuf,
    pub tool: SpringTool,
}

/// Whether the build file `file_name` with `text` references the Boot plugin, parent or BOM.
pub fn references_spring_boot(file_name: &str, text: &str) -> Option<SpringTool> {
    match file_name {
        "build.gradle" | "build.gradle.kts" => {
            let version_catalog_alias = text.lines().any(|line| {
                let line = line.to_lowercase();
                line.contains("alias(libs.plugins.")
                    && line.contains("spring")
                    && line.contains("boot")
            });
            (text.contains("org.springframework.boot")
                || text.contains("spring-boot-gradle-plugin")
                || version_catalog_alias)
                .then_some(SpringTool::Gradle)
        }
        "pom.xml" => [
            "spring-boot-starter-parent",
            "spring-boot-dependencies",
            "spring-boot-maven-plugin",
        ]
        .iter()
        .any(|marker| text.contains(marker))
        .then_some(SpringTool::Maven),
        "MODULE.bazel" | "BUILD.bazel" | "WORKSPACE" | "WORKSPACE.bazel" => text
            .contains("org.springframework.boot")
            .then_some(SpringTool::Bazel),
        _ => None,
    }
}

const BUILD_FILES: [&str; 7] = [
    "build.gradle.kts",
    "build.gradle",
    "pom.xml",
    "MODULE.bazel",
    "BUILD.bazel",
    "WORKSPACE",
    "WORKSPACE.bazel",
];

/// The nearest directory at or above `file` (up to `root`) whose build file references
/// Spring Boot. Both paths are absolute.
pub fn detect(root: &Path, file: &Path) -> Option<SpringModule> {
    let mut dir = file.parent()?;
    loop {
        if !dir.starts_with(root) {
            return None;
        }
        if let Some(tool) = detect_in(dir) {
            return Some(SpringModule {
                dir: dir.to_owned(),
                tool,
            });
        }
        if dir == root {
            return None;
        }
        dir = dir.parent()?;
    }
}

/// The tool whose build file in `dir` references Spring Boot.
pub fn detect_in(dir: &Path) -> Option<SpringTool> {
    BUILD_FILES.iter().find_map(|name| {
        let text = std::fs::read_to_string(dir.join(name)).ok()?;
        references_spring_boot(name, &text)
    })
}

/// Whether the project root (or one of its direct sub-directories, for multi-module
/// builds) is a Spring Boot build.
pub fn is_spring_boot_project(root: &Path) -> bool {
    if detect_in(root).is_some() {
        return true;
    }
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            !name.starts_with('.') && !matches!(name.as_ref(), "build" | "target" | "node_modules")
        })
        .any(|entry| detect_in(&entry.path()).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_files_reference_spring_boot_through_plugin_parent_or_bom() {
        let gradle = |text| references_spring_boot("build.gradle.kts", text);
        assert_eq!(
            gradle("plugins { id(\"org.springframework.boot\") version \"4.1.2\" }"),
            Some(SpringTool::Gradle)
        );
        assert_eq!(
            gradle("plugins { alias(libs.plugins.spring.boot) }"),
            Some(SpringTool::Gradle)
        );
        assert_eq!(gradle("plugins { alias(libs.plugins.kotlin.jvm) }"), None);
        let pom = |text| references_spring_boot("pom.xml", text);
        assert_eq!(
            pom("<artifactId>spring-boot-starter-parent</artifactId>"),
            Some(SpringTool::Maven)
        );
        assert_eq!(
            pom("<artifactId>spring-boot-dependencies</artifactId>"),
            Some(SpringTool::Maven)
        );
        assert_eq!(pom("<artifactId>junit</artifactId>"), None);
        assert_eq!(
            references_spring_boot(
                "MODULE.bazel",
                "boms = [\"org.springframework.boot:spring-boot-dependencies:4.1.2\"]"
            ),
            Some(SpringTool::Bazel)
        );
        assert_eq!(
            references_spring_boot("Cargo.toml", "org.springframework.boot"),
            None
        );
    }

    #[test]
    fn detection_walks_up_to_the_nearest_boot_module() {
        let root =
            std::env::temp_dir().join(format!("archipel-spring-detect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app/src/main/java/p")).unwrap();
        std::fs::create_dir_all(root.join("lib/src")).unwrap();
        std::fs::write(root.join("build.gradle"), "plugins { id 'java' }").unwrap();
        std::fs::write(
            root.join("app/build.gradle"),
            "plugins { id 'org.springframework.boot' version '4.1.2' }",
        )
        .unwrap();
        let app = root.join("app/src/main/java/p/A.java");
        let found = detect(&root, &app).unwrap();
        assert_eq!(
            (found.dir.as_path(), found.tool),
            (root.join("app").as_path(), SpringTool::Gradle)
        );
        assert!(detect(&root, &root.join("lib/src/B.java")).is_none());
        assert!(
            is_spring_boot_project(&root),
            "a boot module below the root counts"
        );
        std::fs::remove_file(root.join("app/build.gradle")).unwrap();
        assert!(!is_spring_boot_project(&root));
        let _ = std::fs::remove_dir_all(&root);
    }
}
