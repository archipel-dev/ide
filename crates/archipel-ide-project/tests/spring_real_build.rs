//! Builds generated projects with the real tools. Opt in with
//! `ARCHIPEL_SPRING_REAL_BUILD=gradle-groovy:java,maven:groovy,...` (build:language pairs);
//! needs network access, a JDK and Gradle or Maven on PATH. Skipped otherwise.

use archipel_ide_project::generator::{Language, ProjectKind, create_project};
use archipel_ide_project::spring::{SpringBuild, SpringSpec};
use archipel_ide_project::spring_catalog::Catalog;
use std::process::Command;

#[test]
fn generated_projects_build_with_the_real_tools() {
    let Ok(requested) = std::env::var("ARCHIPEL_SPRING_REAL_BUILD") else {
        return;
    };
    for pair in requested.split(',').filter(|pair| !pair.is_empty()) {
        let (build, language) = pair.split_once(':').expect("build:language");
        let build = SpringBuild::parse(build).expect("known build system");
        let language = match language {
            "java" => Language::Java,
            "kotlin" => Language::Kotlin,
            "groovy" => Language::Groovy,
            other => panic!("unknown language {other}"),
        };
        let parent =
            std::env::temp_dir().join(format!("archipel-real-build-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        let spec = SpringSpec {
            language,
            build,
            group: "com.example".into(),
            artifact: "demo".into(),
            package: "com.example.demo".into(),
            description: "Demo".into(),
            java_version: 21,
            boot_version: Catalog::builtin().boot_versions[0].clone(),
            war: false,
            dependencies: ["web", "validation", "actuator", "h2", "data-jpa", "lombok"]
                .map(str::to_owned)
                .to_vec(),
        };
        let project = create_project(
            &parent,
            "demo",
            ProjectKind::Spring {
                spec: Box::new(spec),
                catalog: Box::new(Catalog::builtin()),
            },
            "com.example",
        )
        .unwrap();
        let (program, args): (&str, &[&str]) = match build {
            SpringBuild::Maven => ("mvn", &["-q", "-B", "verify"]),
            SpringBuild::Bazel => ("bazel", &["test", "//..."]),
            _ => ("gradle", &["--no-daemon", "--console=plain", "build"]),
        };
        let output = Command::new(program)
            .args(args)
            .current_dir(project.root())
            .output()
            .expect("tool on PATH");
        assert!(
            output.status.success(),
            "{pair} failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let _ = std::fs::remove_dir_all(&parent);
    }
}
