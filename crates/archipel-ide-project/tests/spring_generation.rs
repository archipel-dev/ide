//! Golden tests of the Spring Boot generator: file list and key contents for every
//! build system and language, for a minimal selection and for web + JPA + PostgreSQL,
//! plus a syntax check of everything that is emitted.

use archipel_ide_project::generator::{Language, ProjectKind, create_project, create_project_with};
use archipel_ide_project::spring::{SpringBuild, SpringSpec};
use archipel_ide_project::spring_catalog::Catalog;
use std::collections::BTreeMap;

fn spec(language: Language, build: SpringBuild, dependencies: &[&str]) -> SpringSpec {
    SpringSpec {
        language,
        build,
        group: "com.example".into(),
        artifact: "demo".into(),
        package: "com.example.demo".into(),
        description: "Demo project for Spring Boot".into(),
        java_version: 21,
        boot_version: "4.1.2".into(),
        war: false,
        dependencies: dependencies.iter().map(|id| (*id).to_owned()).collect(),
    }
}

fn generate(spec: &SpringSpec) -> BTreeMap<String, String> {
    spec.files("demo", &Catalog::builtin())
        .unwrap()
        .into_iter()
        .map(|(path, text)| (path.to_string_lossy().replace('\\', "/"), text))
        .collect()
}

const WEB_JPA_PG: &[&str] = &["web", "data-jpa", "postgresql"];

fn combos() -> Vec<(Language, SpringBuild)> {
    let mut all = Vec::new();
    for build in [
        SpringBuild::GradleGroovy,
        SpringBuild::GradleKotlin,
        SpringBuild::Maven,
    ] {
        for language in [Language::Java, Language::Kotlin, Language::Groovy] {
            all.push((language, build));
        }
    }
    all.push((Language::Java, SpringBuild::Bazel));
    all
}

fn lang_dir(language: Language) -> &'static str {
    match language {
        Language::Java => "java",
        Language::Kotlin => "kotlin",
        Language::Groovy => "groovy",
    }
}

fn ext(language: Language) -> &'static str {
    match language {
        Language::Java => "java",
        Language::Kotlin => "kt",
        Language::Groovy => "groovy",
    }
}

fn build_files(build: SpringBuild) -> &'static [&'static str] {
    match build {
        SpringBuild::GradleGroovy => &["build.gradle", "settings.gradle"],
        SpringBuild::GradleKotlin => &["build.gradle.kts", "settings.gradle.kts"],
        SpringBuild::Maven => &["pom.xml"],
        SpringBuild::Bazel => &["BUILD.bazel", "MODULE.bazel", ".bazelrc", ".bazelversion"],
    }
}

#[test]
fn file_lists_match_for_every_build_system_and_language() {
    for (language, build) in combos() {
        for (selection, controller) in [(&[][..], false), (WEB_JPA_PG, true)] {
            let files = generate(&spec(language, build, selection));
            let dir = lang_dir(language);
            let ext = ext(language);
            let mut expected: Vec<String> =
                build_files(build).iter().map(|f| (*f).to_owned()).collect();
            expected.extend([
                ".gitignore".to_owned(),
                "HELP.md".to_owned(),
                "README.md".to_owned(),
                "src/main/resources/application.properties".to_owned(),
                format!("src/main/{dir}/com/example/demo/DemoApplication.{ext}"),
                format!("src/test/{dir}/com/example/demo/DemoApplicationTests.{ext}"),
            ]);
            if controller {
                expected.push(format!(
                    "src/main/{dir}/com/example/demo/HelloController.{ext}"
                ));
            }
            expected.sort();
            let actual: Vec<String> = files.keys().cloned().collect();
            assert_eq!(actual, expected, "{language:?} {build:?} {selection:?}");
        }
    }
}

#[test]
fn gradle_groovy_golden_java_web_jpa_postgres() {
    let files = generate(&spec(Language::Java, SpringBuild::GradleGroovy, WEB_JPA_PG));
    assert_eq!(files["settings.gradle"], "rootProject.name = 'demo'\n");
    assert_eq!(
        files["build.gradle"],
        "plugins {\n    id 'java'\n    id 'org.springframework.boot' version '4.1.2'\n    id 'io.spring.dependency-management' version '1.1.7'\n}\n\ngroup = 'com.example'\nversion = '0.0.1-SNAPSHOT'\ndescription = 'Demo project for Spring Boot'\n\njava {\n    toolchain {\n        languageVersion = JavaLanguageVersion.of(21)\n    }\n}\n\nrepositories {\n    mavenCentral()\n}\n\ndependencies {\n    implementation 'org.springframework.boot:spring-boot-starter'\n    implementation 'org.springframework.boot:spring-boot-starter-webmvc'\n    implementation 'org.springframework.boot:spring-boot-starter-data-jpa'\n    runtimeOnly 'org.postgresql:postgresql'\n    testImplementation 'org.springframework.boot:spring-boot-starter-webmvc-test'\n    testImplementation 'org.springframework.boot:spring-boot-starter-data-jpa-test'\n    testRuntimeOnly 'org.junit.platform:junit-platform-launcher'\n}\n\ntasks.named('test') {\n    useJUnitPlatform()\n}\n"
    );
    let minimal = generate(&spec(Language::Java, SpringBuild::GradleGroovy, &[]));
    assert!(minimal["build.gradle"].contains("    testImplementation 'org.springframework.boot:spring-boot-starter-test'\n    testRuntimeOnly 'org.junit.platform:junit-platform-launcher'\n"));
    assert!(!minimal["build.gradle"].contains("starter-webmvc"));
    assert!(
        files["src/main/resources/application.properties"]
            .starts_with("spring.application.name=demo\n")
    );
    assert!(
        files["src/main/resources/application.properties"]
            .contains("# spring.datasource.url=jdbc:postgresql://localhost:5432/mydb")
    );
}

#[test]
fn gradle_kotlin_dsl_structure() {
    let files = generate(&spec(
        Language::Kotlin,
        SpringBuild::GradleKotlin,
        WEB_JPA_PG,
    ));
    let build = &files["build.gradle.kts"];
    for needle in [
        "plugins {\n    id(\"org.jetbrains.kotlin.jvm\") version \"2.3.21\"\n    id(\"org.jetbrains.kotlin.plugin.spring\") version \"2.3.21\"\n    id(\"org.springframework.boot\") version \"4.1.2\"\n    id(\"io.spring.dependency-management\") version \"1.1.7\"\n}",
        "group = \"com.example\"",
        "languageVersion = JavaLanguageVersion.of(21)",
        "implementation(\"org.springframework.boot:spring-boot-starter-webmvc\")",
        "implementation(\"tools.jackson.module:jackson-module-kotlin\")",
        "freeCompilerArgs.addAll(\"-Xjsr305=strict\", \"-Xannotation-default-target=param-property\")",
        "implementation(\"org.jetbrains.kotlin:kotlin-reflect\")",
        "runtimeOnly(\"org.postgresql:postgresql\")",
        "testImplementation(\"org.springframework.boot:spring-boot-starter-webmvc-test\")",
        "testImplementation(\"org.jetbrains.kotlin:kotlin-test-junit5\")",
        "testRuntimeOnly(\"org.junit.platform:junit-platform-launcher\")",
        "tasks.withType<Test> {\n    useJUnitPlatform()\n}",
    ] {
        assert!(build.contains(needle), "missing {needle:?} in\n{build}");
    }
    assert!(!build.contains("kotlin.version"));
    // Structure: top-level blocks in the conventional order, braces balanced.
    let blocks: Vec<&str> = build
        .lines()
        .filter(|line| !line.starts_with(' ') && line.ends_with('{'))
        .collect();
    assert_eq!(
        blocks,
        [
            "plugins {",
            "java {",
            "repositories {",
            "dependencies {",
            "kotlin {",
            "tasks.withType<Test> {"
        ]
    );
    assert_eq!(build.matches('{').count(), build.matches('}').count());
    assert_eq!(
        files["settings.gradle.kts"],
        "rootProject.name = \"demo\"\n"
    );
}

#[test]
fn the_kotlin_version_follows_the_boot_line() {
    let mut kotlin = spec(Language::Kotlin, SpringBuild::GradleKotlin, &[]);
    kotlin.java_version = 25;
    let build = generate(&kotlin)["build.gradle.kts"].clone();
    assert!(build.contains("\"org.jetbrains.kotlin.jvm\") version \"2.3.21\""));
    assert!(build.contains("JavaLanguageVersion.of(25)"));
    kotlin.boot_version = "4.0.9".into();
    let build = generate(&kotlin)["build.gradle.kts"].clone();
    assert!(build.contains("\"org.jetbrains.kotlin.jvm\") version \"2.2.21\""));
    kotlin.build = SpringBuild::Maven;
    let pom = generate(&kotlin)["pom.xml"].clone();
    assert!(pom.contains("<kotlin.version>2.2.21</kotlin.version>"));
}

#[test]
fn maven_pom_has_parent_starters_and_boot_plugin() {
    let files = generate(&spec(Language::Java, SpringBuild::Maven, WEB_JPA_PG));
    let pom = &files["pom.xml"];
    for needle in [
        "<artifactId>spring-boot-starter-parent</artifactId>\n        <version>4.1.2</version>",
        "<java.version>21</java.version>",
        "<artifactId>spring-boot-starter-webmvc</artifactId>",
        "<artifactId>spring-boot-starter-data-jpa</artifactId>",
        "<artifactId>postgresql</artifactId>\n            <scope>runtime</scope>",
        "<artifactId>spring-boot-starter-webmvc-test</artifactId>\n            <scope>test</scope>",
        "<artifactId>spring-boot-starter-data-jpa-test</artifactId>\n            <scope>test</scope>",
        "<artifactId>spring-boot-maven-plugin</artifactId>",
    ] {
        assert!(pom.contains(needle), "missing {needle:?} in\n{pom}");
    }
    assert!(!pom.contains("<packaging>"));
    assert!(!pom.contains("<artifactId>spring-boot-starter-test</artifactId>"));
}

#[test]
fn maven_kotlin_and_groovy_get_their_compiler_plugins() {
    let kotlin = generate(&spec(Language::Kotlin, SpringBuild::Maven, &[]))["pom.xml"].clone();
    assert!(
        kotlin.contains("<sourceDirectory>${project.basedir}/src/main/kotlin</sourceDirectory>")
    );
    assert!(kotlin.contains("<plugin>spring</plugin>"));
    assert!(kotlin.contains("<artifactId>kotlin-maven-allopen</artifactId>"));
    let groovy = generate(&spec(Language::Groovy, SpringBuild::Maven, &[]))["pom.xml"].clone();
    assert!(groovy.contains("<artifactId>gmavenplus-plugin</artifactId>"));
    assert!(groovy.contains("<artifactId>groovy</artifactId>"));
}

#[test]
fn lombok_and_processors_are_wired_for_each_build_system() {
    let selection = ["lombok", "configuration-processor", "devtools"];
    let gradle =
        generate(&spec(Language::Java, SpringBuild::GradleGroovy, &selection))["build.gradle"]
            .clone();
    assert!(gradle.contains("    compileOnly 'org.projectlombok:lombok'\n"));
    assert!(gradle.contains("    testCompileOnly 'org.projectlombok:lombok'\n"));
    assert!(gradle.contains("    testAnnotationProcessor 'org.projectlombok:lombok'\n"));
    assert!(
        gradle.contains("    developmentOnly 'org.springframework.boot:spring-boot-devtools'\n")
    );
    assert!(gradle.contains("    annotationProcessor 'org.projectlombok:lombok'\n    annotationProcessor 'org.springframework.boot:spring-boot-configuration-processor'\n"));
    let pom = generate(&spec(Language::Java, SpringBuild::Maven, &selection))["pom.xml"].clone();
    assert!(
        pom.contains("<id>default-compile</id>") && pom.contains("<id>default-testCompile</id>")
    );
    assert!(pom.contains("<artifactId>spring-boot-configuration-processor</artifactId>\n                                </path>"));
    assert!(pom.contains("<optional>true</optional>"));
    let bazel =
        generate(&spec(Language::Java, SpringBuild::Bazel, &selection))["BUILD.bazel"].clone();
    assert!(bazel.contains("java_plugin(\n    name = \"lombok_plugin\""));
    assert!(bazel.contains("processor_class = \"org.springframework.boot.configurationprocessor.ConfigurationMetadataAnnotationProcessor\""));
    assert!(
        !bazel.contains("devtools"),
        "development-only starters are not part of Bazel builds"
    );
}

#[test]
fn conditional_coordinates_follow_the_selection() {
    let postgres = generate(&spec(
        Language::Java,
        SpringBuild::GradleGroovy,
        &["flyway", "postgresql"],
    ))["build.gradle"]
        .clone();
    assert!(postgres.contains("flyway-database-postgresql"));
    assert!(!postgres.contains("flyway-mysql"));
    let plain = generate(&spec(
        Language::Java,
        SpringBuild::GradleGroovy,
        &["flyway", "h2"],
    ))["build.gradle"]
        .clone();
    assert!(plain.contains("spring-boot-starter-flyway") && !plain.contains("org.flywaydb"));
}

#[test]
fn war_packaging_adds_the_plugin_tomcat_and_initializer() {
    for build in [
        SpringBuild::GradleGroovy,
        SpringBuild::GradleKotlin,
        SpringBuild::Maven,
    ] {
        for language in [Language::Java, Language::Kotlin, Language::Groovy] {
            let mut war = spec(language, build, &["web"]);
            war.war = true;
            let files = generate(&war);
            let initializer = format!(
                "src/main/{}/com/example/demo/ServletInitializer.{}",
                lang_dir(language),
                ext(language)
            );
            assert!(
                files[&initializer].contains("SpringBootServletInitializer"),
                "{build:?} {language:?}"
            );
            let build_text = files
                .get("pom.xml")
                .or_else(|| files.get("build.gradle"))
                .or_else(|| files.get("build.gradle.kts"))
                .unwrap();
            assert!(build_text.contains("spring-boot-starter-tomcat"));
            assert!(build_text.contains("war"));
        }
    }
    let mut bazel = spec(Language::Java, SpringBuild::Bazel, &[]);
    bazel.war = true;
    assert!(bazel.files("demo", &Catalog::builtin()).is_err());
}

#[test]
fn bazel_files_follow_the_boot_bom() {
    let files = generate(&spec(Language::Java, SpringBuild::Bazel, WEB_JPA_PG));
    let module = &files["MODULE.bazel"];
    assert!(module.contains("bazel_dep(name = \"rules_java\", version = "));
    assert!(module.contains("bazel_dep(name = \"rules_jvm_external\", version = "));
    assert!(
        module.contains("boms = [\"org.springframework.boot:spring-boot-dependencies:4.1.2\"]")
    );
    for artifact in [
        "org.springframework.boot:spring-boot-starter-webmvc",
        "org.springframework.boot:spring-boot-starter-data-jpa",
        "org.postgresql:postgresql",
        "org.springframework.boot:spring-boot-starter-test",
    ] {
        assert!(module.contains(&format!("\"{artifact}\",")), "{artifact}");
    }
    let build = &files["BUILD.bazel"];
    assert!(build.contains(
        "java_binary(\n    name = \"app\",\n    main_class = \"com.example.demo.DemoApplication\""
    ));
    assert!(build.contains("name = \"app_lib\""));
    assert!(build.contains("java_test("));
    assert!(build.contains("\"@maven//:org_springframework_boot_spring_boot_starter_webmvc\""));
    assert!(build.contains("--select-class=com.example.demo.DemoApplicationTests"));
    assert!(files[".bazelrc"].contains("--java_language_version=21"));
    assert_eq!(files[".bazelversion"], "8.3.1\n");
    assert!(
        files["README.md"].contains("Spring's Gradle and Maven plugins are not used under Bazel")
    );
    assert!(files["README.md"].contains("bazel run //:app"));
}

#[test]
fn sources_declare_the_application_and_a_controller_only_with_web() {
    let java = generate(&spec(Language::Java, SpringBuild::GradleGroovy, WEB_JPA_PG));
    let app = &java["src/main/java/com/example/demo/DemoApplication.java"];
    assert!(app.contains("package com.example.demo;"));
    assert!(app.contains("@SpringBootApplication\npublic class DemoApplication"));
    assert!(app.contains("public static void main(String[] args)"));
    assert!(
        java["src/test/java/com/example/demo/DemoApplicationTests.java"]
            .contains("@SpringBootTest")
    );
    assert!(
        java["src/main/java/com/example/demo/HelloController.java"].contains("@RestController")
    );
    let kotlin = generate(&spec(Language::Kotlin, SpringBuild::GradleKotlin, &[]));
    assert!(
        kotlin["src/main/kotlin/com/example/demo/DemoApplication.kt"]
            .contains("runApplication<DemoApplication>(*args)")
    );
    assert!(!kotlin.keys().any(|path| path.contains("HelloController")));
    let groovy = generate(&spec(Language::Groovy, SpringBuild::Maven, &[]));
    assert!(
        groovy["src/main/groovy/com/example/demo/DemoApplication.groovy"]
            .contains("static void main(String[] args)")
    );
}

#[test]
fn names_are_derived_like_the_initializr_does() {
    assert_eq!(
        SpringSpec::derive_package("com.example", "demo-app"),
        "com.example.demoapp"
    );
    assert_eq!(
        SpringSpec::derive_package("com.example", "demo"),
        "com.example.demo"
    );
    assert_eq!(
        SpringSpec::derive_package("Org.Acme", "9lives"),
        "org.acme._9lives"
    );
    assert_eq!(
        SpringSpec::application_class("demo-app"),
        "DemoAppApplication"
    );
    assert_eq!(
        SpringSpec::application_class("shop_application"),
        "ShopApplication"
    );
    assert_eq!(SpringSpec::application_class("1st"), "Demo1stApplication");
}

#[test]
fn invalid_specs_are_rejected_with_a_reason() {
    let catalog = Catalog::builtin();
    let mut bad = spec(Language::Java, SpringBuild::Maven, &["no-such-starter"]);
    assert!(
        bad.validate(&catalog)
            .unwrap_err()
            .contains("no-such-starter")
    );
    bad.dependencies.clear();
    bad.group = "com.class".into();
    assert!(bad.validate(&catalog).is_err());
    bad.group = "com.example".into();
    bad.package = "com..demo".into();
    assert!(bad.validate(&catalog).is_err());
    bad.package = "com.example.demo".into();
    bad.boot_version = "3.5.6".into();
    assert!(bad.validate(&catalog).is_err());
    bad.boot_version = "4.1.2".into();
    bad.java_version = 11;
    assert!(bad.validate(&catalog).is_err());
    bad.java_version = 21;
    bad.artifact = "bad artifact".into();
    assert!(bad.validate(&catalog).is_err());
    bad.artifact = "demo".into();
    bad.validate(&catalog).unwrap();
    let kotlin_bazel = spec(Language::Kotlin, SpringBuild::Bazel, &[]);
    assert!(
        kotlin_bazel
            .validate(&catalog)
            .unwrap_err()
            .contains("Java only")
    );
}

fn parse_with_grammar(path: &str, text: &str) {
    let spec = archipel_ide_language::registry::detect(std::path::Path::new(path), text)
        .unwrap_or_else(|| panic!("no grammar for {path}"));
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&spec.grammar()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "syntax error in {path}:\n{text}"
    );
}

fn parse_xml(text: &str) {
    let mut reader = quick_xml::Reader::from_str(text);
    reader.config_mut().check_end_names = true;
    loop {
        if matches!(reader.read_event().unwrap(), quick_xml::events::Event::Eof) {
            break;
        }
    }
}

#[test]
fn everything_that_is_emitted_parses() {
    for (language, build) in combos() {
        for selection in [
            &[][..],
            WEB_JPA_PG,
            &[
                "lombok",
                "configuration-processor",
                "devtools",
                "testcontainers",
                "flyway",
                "kafka",
            ][..],
        ] {
            for war in [false, true] {
                let mut spec = spec(language, build, selection);
                spec.war = war;
                if build == SpringBuild::Bazel && war {
                    continue;
                }
                for (path, text) in generate(&spec) {
                    let label = format!("{language:?} {build:?} {path}");
                    if path.ends_with("pom.xml") {
                        parse_xml(&text);
                    } else if path.ends_with(".properties")
                        || path.ends_with(".java")
                        || path.ends_with(".kt")
                        || path.ends_with(".kts")
                        || path.ends_with(".bazel")
                    {
                        parse_with_grammar(&path, &text);
                    } else if path.ends_with("HelloController.groovy") {
                        // The bundled Groovy grammar rejects annotated parameters that carry
                        // arguments; check the structure instead.
                        assert_eq!(text.matches('{').count(), text.matches('}').count());
                        assert_eq!(text.matches('(').count(), text.matches(')').count());
                    } else if path.ends_with(".gradle") || path.ends_with(".groovy") {
                        parse_with_grammar(&path, &text);
                    }
                    assert!(
                        !text.contains("{}") || path == "HELP.md",
                        "{label}: unformatted placeholder"
                    );
                }
            }
        }
    }
}

#[test]
fn creating_a_project_writes_the_files_and_opens_it() {
    let parent = std::env::temp_dir().join(format!("archipel-spring-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&parent);
    let kind = |build| ProjectKind::Spring {
        spec: Box::new(spec(Language::Java, build, &["web"])),
        catalog: Box::new(Catalog::builtin()),
    };
    let project = create_project_with(
        &parent,
        "demo",
        kind(SpringBuild::Maven),
        "com.example",
        true,
    )
    .unwrap();
    assert!(project.root().join("pom.xml").is_file());
    assert!(
        project
            .root()
            .join("src/main/java/com/example/demo/DemoApplication.java")
            .is_file()
    );
    assert!(project.root().join(".git").is_dir());
    assert!(create_project(&parent, "demo", kind(SpringBuild::Maven), "com.example").is_err());
    let mut broken = spec(Language::Java, SpringBuild::Maven, &["nope"]);
    broken.dependencies = vec!["nope".into()];
    let error = create_project(
        &parent,
        "broken",
        ProjectKind::Spring {
            spec: Box::new(broken),
            catalog: Box::new(Catalog::builtin()),
        },
        "com.example",
    )
    .unwrap_err();
    assert!(error.to_string().contains("nope"));
    assert!(
        !parent.join("broken").exists(),
        "nothing is written for an invalid selection"
    );
    std::fs::remove_dir_all(parent).unwrap();
}
