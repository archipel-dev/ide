//! Deterministic project templates. Generation never runs downloaded code.
use crate::android_template::AndroidActivity;
use crate::{Project, ProjectError};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Empty,
    RustBinary,
    RustLibrary,
    JavaGradleKotlin,
    JavaGradleGroovy,
    /// Configurable Java/Kotlin project from the New Project wizard.
    Jvm(JvmSpec),
    /// Android app (Gradle, Kotlin DSL) from the New Project wizard.
    Android(crate::android_template::AndroidActivity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java,
    Kotlin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildSystem {
    Intellij,
    Maven,
    Gradle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradleDsl {
    Kotlin,
    Groovy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JvmSpec {
    pub language: Language,
    pub build: BuildSystem,
    pub dsl: GradleDsl,
    pub sample: bool,
    /// JDK major version written into the build files.
    pub jdk: u32,
}

impl JvmSpec {
    pub fn parse(language: &str, build: &str, dsl: &str, sample: bool, jdk: u32) -> Option<Self> {
        Some(Self {
            language: match language {
                "java" => Language::Java,
                "kotlin" => Language::Kotlin,
                _ => return None,
            },
            build: match build {
                "intellij" => BuildSystem::Intellij,
                "maven" => BuildSystem::Maven,
                "gradle" => BuildSystem::Gradle,
                _ => return None,
            },
            dsl: match dsl {
                "kotlin" => GradleDsl::Kotlin,
                "groovy" => GradleDsl::Groovy,
                _ => return None,
            },
            sample,
            jdk: (8..=99).contains(&jdk).then_some(jdk)?,
        })
    }
}

impl ProjectKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "empty" => Some(Self::Empty),
            "rust-binary" => Some(Self::RustBinary),
            "rust-library" => Some(Self::RustLibrary),
            "java-gradle-kotlin" => Some(Self::JavaGradleKotlin),
            "java-gradle-groovy" => Some(Self::JavaGradleGroovy),
            "android-compose" => Some(Self::Android(AndroidActivity::Compose)),
            "android-views" => Some(Self::Android(AndroidActivity::Views)),
            "android-no-activity" => Some(Self::Android(AndroidActivity::None)),
            _ => None,
        }
    }
}

/// Creates a new child directory only; existing directories are never reused.
pub fn create_project(
    parent: &Path,
    name: &str,
    kind: ProjectKind,
    group: &str,
) -> Result<Project, ProjectError> {
    create_project_with(parent, name, kind, group, false)
}

/// Like [`create_project`]; `git` also runs `git init` in the new directory.
pub fn create_project_with(
    parent: &Path,
    name: &str,
    kind: ProjectKind,
    group: &str,
    git: bool,
) -> Result<Project, ProjectError> {
    if name.is_empty()
        || name.len() > 80
        || !name.starts_with(|c: char| c.is_ascii_alphabetic())
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ProjectError::InvalidName(name.into()));
    }
    let jvm = match kind {
        ProjectKind::JavaGradleKotlin => Some(JvmSpec {
            language: Language::Java,
            build: BuildSystem::Gradle,
            dsl: GradleDsl::Kotlin,
            sample: true,
            jdk: 21,
        }),
        ProjectKind::JavaGradleGroovy => Some(JvmSpec {
            language: Language::Java,
            build: BuildSystem::Gradle,
            dsl: GradleDsl::Groovy,
            sample: true,
            jdk: 21,
        }),
        ProjectKind::Jvm(spec) => Some(spec),
        _ => None,
    };
    let java = jvm.is_some() || matches!(kind, ProjectKind::Android(_));
    if java && !valid_package(group) {
        return Err(ProjectError::InvalidName(format!(
            "Java group/package: {group}"
        )));
    }
    // A location typed into the wizard usually does not exist yet; create it
    // like the reference IDE does instead of failing with an OS error.
    std::fs::create_dir_all(parent).map_err(|source| ProjectError::CreateLocation {
        path: parent.to_owned(),
        source,
    })?;
    let parent = parent.canonicalize()?;
    if !parent.is_dir() {
        return Err(ProjectError::NotDirectory(parent));
    }
    let root = parent.join(name);
    match std::fs::create_dir(&root) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(ProjectError::AlreadyExists(root));
        }
        Err(e) => return Err(e.into()),
    }
    let mut files: Vec<(PathBuf, String)> = vec![
        ("README.md".into(), format!("# {name}\n")),
        (
            ".gitignore".into(),
            ".archipel/\n.idea/\ntarget/\nbuild/\n.gradle/\nlocal.properties\n".into(),
        ),
    ];
    match kind {
        ProjectKind::Empty => {}
        ProjectKind::RustBinary | ProjectKind::RustLibrary => {
            files.push(("Cargo.toml".into(), format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n")));
            files.push(if kind == ProjectKind::RustBinary {
                ("src/main.rs".into(), "fn main() {\n    println!(\"Hello, world!\");\n}\n".into())
            } else {
                ("src/lib.rs".into(), "pub fn add(left: u64, right: u64) -> u64 {\n    left + right\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn adds() {\n        assert_eq!(super::add(2, 2), 4);\n    }\n}\n".into())
            });
        }
        ProjectKind::Android(activity) => {
            crate::android_template::files(&mut files, name, group, activity);
        }
        ProjectKind::JavaGradleKotlin | ProjectKind::JavaGradleGroovy | ProjectKind::Jvm(_) => {
            let spec = jvm.expect("JVM kinds carry a spec");
            jvm_files(&mut files, name, group, spec);
        }
    }
    for (path, contents) in files {
        let target = root.join(path);
        let write = || -> std::io::Result<()> {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            use std::io::Write;
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&target)?
                .write_all(contents.as_bytes())
        };
        if let Err(error) = write() {
            return Err(ProjectError::Generation {
                root,
                source: error,
            });
        }
    }
    if git {
        let output = std::process::Command::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(&root)
            .output()
            .map_err(|error| ProjectError::Generation {
                root: root.clone(),
                source: error,
            })?;
        if !output.status.success() {
            return Err(ProjectError::Generation {
                root,
                source: std::io::Error::other(format!(
                    "git init failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )),
            });
        }
    }
    Project::open(root)
}

fn jvm_files(files: &mut Vec<(PathBuf, String)>, name: &str, group: &str, spec: JvmSpec) {
    let kotlin = spec.language == Language::Kotlin;
    let groovy = spec.dsl == GradleDsl::Groovy;
    let jdk = spec.jdk;
    let package_dir = group.replace('.', "/");
    let (source_root, main_class) = match (spec.build, kotlin) {
        (BuildSystem::Intellij, _) => ("src".to_owned(), ""),
        (_, false) => ("src/main/java".to_owned(), "Main"),
        (_, true) => ("src/main/kotlin".to_owned(), "MainKt"),
    };
    match spec.build {
        BuildSystem::Intellij => {}
        BuildSystem::Gradle => {
            files.push((
                if groovy {
                    "settings.gradle"
                } else {
                    "settings.gradle.kts"
                }
                .into(),
                format!("rootProject.name = \"{name}\"\n"),
            ));
            let build = match (kotlin, groovy) {
                (false, false) => format!(
                    "plugins {{\n    application\n}}\n\ngroup = \"{group}\"\nversion = \"1.0-SNAPSHOT\"\n\nrepositories {{\n    mavenCentral()\n}}\n\njava {{\n    toolchain {{\n        languageVersion.set(JavaLanguageVersion.of({jdk}))\n    }}\n}}\n\napplication {{\n    mainClass.set(\"{group}.{main_class}\")\n}}\n"
                ),
                (false, true) => format!(
                    "plugins {{\n    id 'application'\n}}\n\ngroup = '{group}'\nversion = '1.0-SNAPSHOT'\n\nrepositories {{\n    mavenCentral()\n}}\n\njava {{\n    toolchain {{\n        languageVersion = JavaLanguageVersion.of({jdk})\n    }}\n}}\n\napplication {{\n    mainClass = '{group}.{main_class}'\n}}\n"
                ),
                (true, false) => format!(
                    "plugins {{\n    kotlin(\"jvm\") version \"2.0.21\"\n    application\n}}\n\ngroup = \"{group}\"\nversion = \"1.0-SNAPSHOT\"\n\nrepositories {{\n    mavenCentral()\n}}\n\nkotlin {{\n    jvmToolchain({jdk})\n}}\n\napplication {{\n    mainClass.set(\"{group}.{main_class}\")\n}}\n"
                ),
                (true, true) => format!(
                    "plugins {{\n    id 'org.jetbrains.kotlin.jvm' version '2.0.21'\n    id 'application'\n}}\n\ngroup = '{group}'\nversion = '1.0-SNAPSHOT'\n\nrepositories {{\n    mavenCentral()\n}}\n\nkotlin {{\n    jvmToolchain({jdk})\n}}\n\napplication {{\n    mainClass = '{group}.{main_class}'\n}}\n"
                ),
            };
            files.push((
                if groovy {
                    "build.gradle"
                } else {
                    "build.gradle.kts"
                }
                .into(),
                build,
            ));
            files[0]
                .1
                .push_str("\nRequires a JDK and Gradle on PATH. Run `gradle run` or `gradle build`.\nGenerate a pinned wrapper with `gradle wrapper` if desired.\n");
        }
        BuildSystem::Maven => {
            let (plugin, deps) = if kotlin {
                (
                    "    <sourceDirectory>src/main/kotlin</sourceDirectory>\n    <plugins>\n      <plugin>\n        <groupId>org.jetbrains.kotlin</groupId>\n        <artifactId>kotlin-maven-plugin</artifactId>\n        <version>2.0.21</version>\n        <executions>\n          <execution>\n            <id>compile</id>\n            <phase>compile</phase>\n            <goals><goal>compile</goal></goals>\n          </execution>\n        </executions>\n      </plugin>\n    </plugins>\n".to_owned(),
                    "  <dependencies>\n    <dependency>\n      <groupId>org.jetbrains.kotlin</groupId>\n      <artifactId>kotlin-stdlib</artifactId>\n      <version>2.0.21</version>\n    </dependency>\n  </dependencies>\n",
                )
            } else {
                (String::new(), "")
            };
            let build = if plugin.is_empty() {
                String::new()
            } else {
                format!("  <build>\n{plugin}  </build>\n")
            };
            files.push((
                "pom.xml".into(),
                format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n  <modelVersion>4.0.0</modelVersion>\n  <groupId>{group}</groupId>\n  <artifactId>{name}</artifactId>\n  <version>1.0-SNAPSHOT</version>\n  <properties>\n    <maven.compiler.release>{jdk}</maven.compiler.release>\n    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>\n  </properties>\n{deps}{build}</project>\n"),
            ));
        }
    }
    if spec.sample {
        let (file, text) = if kotlin {
            (
                "Main.kt",
                format!("package {group}\n\nfun main() {{\n    println(\"Hello, world!\")\n}}\n"),
            )
        } else {
            (
                "Main.java",
                format!(
                    "package {group};\n\npublic class Main {{\n    public static void main(String[] args) {{\n        System.out.println(\"Hello, world!\");\n    }}\n}}\n"
                ),
            )
        };
        files.push((format!("{source_root}/{package_dir}/{file}").into(), text));
    } else {
        files.push((format!("{source_root}/.gitkeep").into(), String::new()));
    }
}

fn valid_package(group: &str) -> bool {
    const RESERVED: &[&str] = &[
        "class",
        "public",
        "private",
        "static",
        "void",
        "int",
        "package",
        "import",
        "new",
        "return",
        "if",
        "else",
        "while",
        "for",
        "switch",
        "case",
        "default",
        "true",
        "false",
        "null",
        "enum",
        "record",
        "interface",
        "extends",
        "implements",
        "var",
        "_",
        "assert",
        "boolean",
        "break",
        "byte",
        "catch",
        "char",
        "const",
        "continue",
        "do",
        "double",
        "final",
        "finally",
        "float",
        "goto",
        "instanceof",
        "long",
        "native",
        "protected",
        "short",
        "strictfp",
        "super",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "try",
        "volatile",
        "abstract",
    ];
    !group.is_empty()
        && group.len() <= 160
        && group.split('.').all(|part| {
            !RESERVED.contains(&part)
                && part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_location_is_created_like_the_wizard_promises() {
        let parent = std::env::temp_dir()
            .join(format!(
                "archipel-ide-generators-missing-{}",
                std::process::id()
            ))
            .join("nested")
            .join("location");
        let _ = std::fs::remove_dir_all(parent.parent().unwrap().parent().unwrap());
        let project = create_project(&parent, "fresh", ProjectKind::RustBinary, "").unwrap();
        assert!(project.root().join("Cargo.toml").is_file());
        let _ = std::fs::remove_dir_all(parent.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn templates_and_collisions_are_safe() {
        let parent =
            std::env::temp_dir().join(format!("archipel-ide-generators-{}", std::process::id()));
        std::fs::create_dir_all(&parent).unwrap();
        for (name, kind, expected) in [
            ("app", ProjectKind::RustBinary, "src/main.rs"),
            ("library", ProjectKind::RustLibrary, "src/lib.rs"),
            (
                "java",
                ProjectKind::JavaGradleKotlin,
                "src/main/java/org/example/Main.java",
            ),
            ("groovy", ProjectKind::JavaGradleGroovy, "build.gradle"),
            ("empty", ProjectKind::Empty, "README.md"),
        ] {
            let root = parent.join(name);
            // Unique test-owned directory; support rerunning after a failed test.
            if root.exists() {
                std::fs::remove_dir_all(&root).unwrap();
            }
            let project = create_project(&parent, name, kind, "org.example").unwrap();
            assert!(project.root().join(expected).is_file());
            assert!(matches!(
                create_project(&parent, name, kind, "org.example"),
                Err(ProjectError::AlreadyExists(_))
            ));
        }
        assert!(create_project(&parent, "../escape", ProjectKind::Empty, "").is_err());
        assert!(
            create_project(&parent, "bad", ProjectKind::JavaGradleKotlin, "org.class").is_err()
        );
        assert!(!parent.join("bad").exists());
        let source =
            std::fs::read_to_string(parent.join("java/src/main/java/org/example/Main.java"))
                .unwrap();
        assert_eq!(
            source,
            "package org.example;\n\npublic class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, world!\");\n    }\n}\n"
        );
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn jvm_wizard_options_shape_the_project() {
        let parent = std::env::temp_dir().join(format!("archipel-ide-jvm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        std::fs::create_dir_all(&parent).unwrap();
        let spec = |language, build, dsl, sample| {
            ProjectKind::Jvm(JvmSpec::parse(language, build, dsl, sample, 17).unwrap())
        };
        let kotlin = create_project_with(
            &parent,
            "kt",
            spec("kotlin", "gradle", "groovy", true),
            "org.example",
            true,
        )
        .unwrap();
        let root = kotlin.root();
        assert!(root.join(".git").is_dir());
        assert!(root.join("src/main/kotlin/org/example/Main.kt").is_file());
        let build = std::fs::read_to_string(root.join("build.gradle")).unwrap();
        assert!(build.contains("jvmToolchain(17)") && build.contains("org.example.MainKt"));
        let maven = create_project(
            &parent,
            "mvn",
            spec("java", "maven", "kotlin", false),
            "org.example",
        )
        .unwrap();
        let pom = std::fs::read_to_string(maven.root().join("pom.xml")).unwrap();
        assert!(pom.contains("<maven.compiler.release>17<"));
        assert!(maven.root().join("src/main/java/.gitkeep").is_file());
        assert!(!maven.root().join(".git").exists());
        let plain = create_project(
            &parent,
            "plain",
            spec("java", "intellij", "kotlin", true),
            "org.example",
        )
        .unwrap();
        assert!(plain.root().join("src/org/example/Main.java").is_file());
        assert!(JvmSpec::parse("java", "ant", "kotlin", true, 17).is_none());
        assert!(JvmSpec::parse("java", "gradle", "kotlin", true, 3).is_none());
        std::fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn android_templates_generate_detectable_modules() {
        for (kind, name, has_activity, layout) in [
            ("android-compose", "Hello-App", true, false),
            ("android-views", "viewsapp", true, true),
            ("android-no-activity", "bare", false, false),
        ] {
            let dir =
                std::env::temp_dir().join(format!("jt-android-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let project =
                create_project(&dir, name, ProjectKind::parse(kind).unwrap(), "org.example")
                    .unwrap();
            let root = project.root();
            let package = crate::android_template::application_id("org.example", name);
            let manifest =
                std::fs::read_to_string(root.join("app/src/main/AndroidManifest.xml")).unwrap();
            assert_eq!(
                manifest.contains("android.intent.action.MAIN"),
                has_activity
            );
            let source = root
                .join("app/src/main/kotlin")
                .join(package.replace('.', "/"))
                .join("MainActivity.kt");
            assert_eq!(source.is_file(), has_activity);
            assert_eq!(
                root.join("app/src/main/res/layout/activity_main.xml")
                    .is_file(),
                layout
            );
            let build = std::fs::read_to_string(root.join("app/build.gradle.kts")).unwrap();
            assert!(build.contains(&format!("namespace = \"{package}\"")));
            assert_eq!(build.contains("compose = true"), kind == "android-compose");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
