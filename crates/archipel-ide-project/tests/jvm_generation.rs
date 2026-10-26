//! Plain JVM generators: Groovy as a language and Bazel as a build system.

use archipel_ide_project::generator::{JvmSpec, ProjectKind, create_project};

fn generate(
    language: &str,
    build: &str,
    dsl: &str,
    sample: bool,
    name: &str,
) -> std::path::PathBuf {
    let parent =
        std::env::temp_dir().join(format!("archipel-jvm-gen-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&parent);
    let spec = JvmSpec::parse(language, build, dsl, sample, 21).unwrap();
    create_project(&parent, "app", ProjectKind::Jvm(spec), "org.example")
        .unwrap()
        .root()
        .to_owned()
}

fn parses(path: &std::path::Path) {
    let text = std::fs::read_to_string(path).unwrap();
    let spec = archipel_ide_language::registry::detect(path, &text).unwrap();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&spec.grammar()).unwrap();
    assert!(
        !parser.parse(&text, None).unwrap().root_node().has_error(),
        "{}",
        path.display()
    );
}

#[test]
fn groovy_projects_for_gradle_maven_and_intellij() {
    let gradle = generate("groovy", "gradle", "groovy", true, "gg");
    let build = std::fs::read_to_string(gradle.join("build.gradle")).unwrap();
    assert!(build.contains("id 'groovy'") && build.contains("org.apache.groovy:groovy:4.0.26"));
    assert!(build.contains("mainClass = 'org.example.Main'"));
    let main = gradle.join("src/main/groovy/org/example/Main.groovy");
    assert!(
        std::fs::read_to_string(&main)
            .unwrap()
            .contains("static void main")
    );
    parses(&gradle.join("build.gradle"));
    parses(&main);
    let kts = generate("groovy", "gradle", "kotlin", true, "gk");
    let build = std::fs::read_to_string(kts.join("build.gradle.kts")).unwrap();
    assert!(
        build.contains("groovy\n")
            && build.contains("implementation(\"org.apache.groovy:groovy:4.0.26\")")
    );
    parses(&kts.join("build.gradle.kts"));
    let maven = generate("groovy", "maven", "kotlin", true, "gm");
    let pom = std::fs::read_to_string(maven.join("pom.xml")).unwrap();
    assert!(pom.contains("gmavenplus-plugin") && pom.contains("<artifactId>groovy</artifactId>"));
    let plain = generate("groovy", "intellij", "kotlin", true, "gi");
    assert!(plain.join("src/org/example/Main.groovy").is_file());
}

#[test]
fn bazel_projects_are_java_only_and_runnable_as_app() {
    let root = generate("java", "bazel", "kotlin", true, "bz");
    let module = std::fs::read_to_string(root.join("MODULE.bazel")).unwrap();
    assert!(module.contains("name = \"app\"") && module.contains("rules_java"));
    let build = std::fs::read_to_string(root.join("BUILD.bazel")).unwrap();
    assert!(
        build.contains("java_binary(\n    name = \"app\"")
            && build.contains("main_class = \"org.example.Main\"")
    );
    assert!(root.join(".bazelrc").is_file() && root.join(".bazelversion").is_file());
    assert!(root.join("src/main/java/org/example/Main.java").is_file());
    for file in ["MODULE.bazel", "BUILD.bazel"] {
        parses(&root.join(file));
    }
    let empty = generate("java", "bazel", "kotlin", false, "bz0");
    parses(&empty.join("BUILD.bazel"));
    assert!(JvmSpec::parse("kotlin", "bazel", "kotlin", true, 21).is_none());
    assert!(JvmSpec::parse("groovy", "bazel", "kotlin", true, 21).is_none());
}
