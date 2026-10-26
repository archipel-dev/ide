//! Spring Boot project generation for the New Project wizard (Initializr style, offline).
//!
//! [`SpringSpec::files`] turns the wizard's choices and the selected starters of the
//! [`Catalog`] into a file list: build files for Gradle (Groovy or Kotlin DSL), Maven or
//! Bazel, sources in Java, Kotlin or Groovy, `application.properties`, an
//! `@SpringBootTest` class, a sample controller when `web` is selected, `.gitignore` and
//! `HELP.md`. Nothing here touches the disk or the network.

use crate::generator::Language;
use crate::spring_catalog::{Catalog, Coordinate, Scope};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpringBuild {
    GradleGroovy,
    GradleKotlin,
    Maven,
    Bazel,
}

impl SpringBuild {
    pub fn parse(id: &str) -> Option<Self> {
        Some(match id {
            "gradle-groovy" => Self::GradleGroovy,
            "gradle-kotlin" => Self::GradleKotlin,
            "maven" => Self::Maven,
            "bazel" => Self::Bazel,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::GradleGroovy => "gradle-groovy",
            Self::GradleKotlin => "gradle-kotlin",
            Self::Maven => "maven",
            Self::Bazel => "bazel",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::GradleGroovy => "Gradle - Groovy DSL",
            Self::GradleKotlin => "Gradle - Kotlin DSL",
            Self::Maven => "Maven",
            Self::Bazel => "Bazel",
        }
    }

    pub fn is_gradle(self) -> bool {
        matches!(self, Self::GradleGroovy | Self::GradleKotlin)
    }
}

/// Why a language and build system cannot be combined, if they cannot.
pub fn unsupported_combination(
    language: Language,
    build: SpringBuild,
    war: bool,
) -> Option<&'static str> {
    if build == SpringBuild::Bazel && language != Language::Java {
        return Some(
            "Bazel projects are generated for Java only; use Gradle or Maven for Kotlin and Groovy",
        );
    }
    if build == SpringBuild::Bazel && war {
        return Some("Bazel builds a runnable jar; WAR packaging needs Gradle or Maven");
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpringSpec {
    pub language: Language,
    pub build: SpringBuild,
    pub group: String,
    pub artifact: String,
    pub package: String,
    pub description: String,
    /// Java language level: 17, 21 or 25.
    pub java_version: u32,
    pub boot_version: String,
    pub war: bool,
    /// Catalog ids of the selected starters.
    pub dependencies: Vec<String>,
}

impl SpringSpec {
    /// Package name Initializr derives from group and artifact: `com.example` and
    /// `demo-app` give `com.example.demoapp`.
    pub fn derive_package(group: &str, artifact: &str) -> String {
        let clean = |part: &str| -> String {
            let part: String = part
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_lowercase())
                .collect();
            if part.starts_with(|c: char| c.is_ascii_digit()) {
                format!("_{part}")
            } else {
                part
            }
        };
        let mut parts: Vec<String> = group
            .split('.')
            .map(clean)
            .filter(|p| !p.is_empty())
            .collect();
        let artifact = clean(artifact);
        if !artifact.is_empty() && parts.last() != Some(&artifact) {
            parts.push(artifact);
        }
        parts.join(".")
    }

    /// `demo-app` becomes `DemoAppApplication`.
    pub fn application_class(name: &str) -> String {
        let mut class = String::new();
        for word in name.split(|c: char| !c.is_ascii_alphanumeric()) {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                class.push(first.to_ascii_uppercase());
                class.extend(chars);
            }
        }
        if class.starts_with(|c: char| c.is_ascii_digit()) || class.is_empty() {
            class.insert_str(0, "Demo");
        }
        if !class.to_lowercase().ends_with("application") {
            class.push_str("Application");
        }
        class
    }

    /// Checks everything the generator relies on; messages are phrased for the wizard.
    pub fn validate(&self, catalog: &Catalog) -> Result<(), String> {
        if !crate::generator::valid_package(&self.group) {
            return Err(format!(
                "Group `{}` is not a valid Java package name",
                self.group
            ));
        }
        if self.artifact.is_empty()
            || self.artifact.len() > 80
            || !self
                .artifact
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            || !self
                .artifact
                .starts_with(|c: char| c.is_ascii_alphanumeric())
        {
            return Err(format!(
                "Artifact `{}` may only contain letters, digits, `-`, `_` and `.`",
                self.artifact
            ));
        }
        if !crate::generator::valid_package(&self.package) {
            return Err(format!(
                "Package name `{}` is not a valid Java package",
                self.package
            ));
        }
        if !crate::spring_catalog::supported_boot_version(&self.boot_version) {
            return Err(format!(
                "Spring Boot {} is not supported",
                self.boot_version
            ));
        }
        if !catalog.java_versions.contains(&self.java_version) {
            return Err(format!("Java {} is not offered", self.java_version));
        }
        if let Some(reason) = unsupported_combination(self.language, self.build, self.war) {
            return Err(reason.into());
        }
        let mut seen = BTreeSet::new();
        for id in &self.dependencies {
            if catalog.dependency(id).is_none() {
                return Err(format!("Unknown dependency `{id}`"));
            }
            seen.insert(id);
        }
        Ok(())
    }

    fn selected(&self, id: &str) -> bool {
        self.dependencies.iter().any(|dependency| dependency == id)
    }

    /// Selected coordinates in catalog order, `when` conditions applied, duplicates removed.
    fn coordinates(&self, catalog: &Catalog) -> Vec<Coordinate> {
        let mut out: Vec<Coordinate> = Vec::new();
        for dependency in &catalog.dependencies {
            if !self.selected(&dependency.id) {
                continue;
            }
            for coordinate in &dependency.coordinates {
                if coordinate
                    .when
                    .as_deref()
                    .is_some_and(|required| !self.selected(required))
                {
                    continue;
                }
                if !out.iter().any(|c| {
                    c.group == coordinate.group
                        && c.artifact == coordinate.artifact
                        && c.scopes == coordinate.scopes
                }) {
                    out.push(coordinate.clone());
                }
            }
        }
        out
    }

    fn has_web(&self) -> bool {
        self.selected("web")
    }

    fn source_dir(&self) -> &'static str {
        match self.language {
            Language::Java => "java",
            Language::Kotlin => "kotlin",
            Language::Groovy => "groovy",
        }
    }

    fn extension(&self) -> &'static str {
        match self.language {
            Language::Java => "java",
            Language::Kotlin => "kt",
            Language::Groovy => "groovy",
        }
    }

    /// The generated project's files, relative to its root. `name` is the project (directory) name.
    pub fn files(&self, name: &str, catalog: &Catalog) -> Result<Vec<(PathBuf, String)>, String> {
        self.validate(catalog)?;
        let coordinates = self.coordinates(catalog);
        let kotlin_version = catalog.kotlin_version(&self.boot_version);
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        let mut add = |path: &str, text: String| files.push((PathBuf::from(path), text));
        match self.build {
            SpringBuild::GradleGroovy | SpringBuild::GradleKotlin => {
                let kts = self.build == SpringBuild::GradleKotlin;
                let (settings, build) = if kts {
                    ("settings.gradle.kts", "build.gradle.kts")
                } else {
                    ("settings.gradle", "build.gradle")
                };
                add(settings, gradle_settings(&self.artifact, kts));
                add(
                    build,
                    gradle_build(self, &coordinates, kts, &kotlin_version),
                );
            }
            SpringBuild::Maven => add(
                "pom.xml",
                maven_pom(self, name, &coordinates, &kotlin_version),
            ),
            SpringBuild::Bazel => {
                add("MODULE.bazel", bazel_module(self, &coordinates));
                add("BUILD.bazel", bazel_build(self, &coordinates));
                add(".bazelrc", bazel_rc(self.java_version));
                add(".bazelversion", format!("{BAZEL_VERSION}\n"));
            }
        }
        let package_dir = self.package.replace('.', "/");
        let class = Self::application_class(name);
        let main_dir = format!("src/main/{}/{package_dir}", self.source_dir());
        let test_dir = format!("src/test/{}/{package_dir}", self.source_dir());
        let ext = self.extension();
        add(
            &format!("{main_dir}/{class}.{ext}"),
            self.application_source(&class),
        );
        add(
            &format!("{test_dir}/{class}Tests.{ext}"),
            self.test_source(&class),
        );
        if self.has_web() {
            add(
                &format!("{main_dir}/HelloController.{ext}"),
                self.controller_source(),
            );
        }
        if self.war {
            add(
                &format!("{main_dir}/ServletInitializer.{ext}"),
                self.servlet_initializer_source(&class),
            );
        }
        add(
            "src/main/resources/application.properties",
            self.application_properties(),
        );
        add(".gitignore", spring_gitignore().into());
        add("README.md", self.readme(name));
        add("HELP.md", self.help(name, catalog));
        Ok(files)
    }

    fn application_source(&self, class: &str) -> String {
        let package = &self.package;
        match self.language {
            Language::Java => format!(
                "package {package};\n\nimport org.springframework.boot.SpringApplication;\nimport org.springframework.boot.autoconfigure.SpringBootApplication;\n\n@SpringBootApplication\npublic class {class} {{\n\n    public static void main(String[] args) {{\n        SpringApplication.run({class}.class, args);\n    }}\n\n}}\n"
            ),
            Language::Kotlin => format!(
                "package {package}\n\nimport org.springframework.boot.autoconfigure.SpringBootApplication\nimport org.springframework.boot.runApplication\n\n@SpringBootApplication\nclass {class}\n\nfun main(args: Array<String>) {{\n    runApplication<{class}>(*args)\n}}\n"
            ),
            Language::Groovy => format!(
                "package {package}\n\nimport org.springframework.boot.SpringApplication\nimport org.springframework.boot.autoconfigure.SpringBootApplication\n\n@SpringBootApplication\nclass {class} {{\n\n    static void main(String[] args) {{\n        SpringApplication.run({class}, args)\n    }}\n\n}}\n"
            ),
        }
    }

    fn test_source(&self, class: &str) -> String {
        let package = &self.package;
        match self.language {
            Language::Java => format!(
                "package {package};\n\nimport org.junit.jupiter.api.Test;\nimport org.springframework.boot.test.context.SpringBootTest;\n\n@SpringBootTest\nclass {class}Tests {{\n\n    @Test\n    void contextLoads() {{\n    }}\n\n}}\n"
            ),
            Language::Kotlin => format!(
                "package {package}\n\nimport org.junit.jupiter.api.Test\nimport org.springframework.boot.test.context.SpringBootTest\n\n@SpringBootTest\nclass {class}Tests {{\n\n    @Test\n    fun contextLoads() {{\n    }}\n\n}}\n"
            ),
            Language::Groovy => format!(
                "package {package}\n\nimport org.junit.jupiter.api.Test\nimport org.springframework.boot.test.context.SpringBootTest\n\n@SpringBootTest\nclass {class}Tests {{\n\n    @Test\n    void contextLoads() {{\n    }}\n\n}}\n"
            ),
        }
    }

    fn controller_source(&self) -> String {
        let package = &self.package;
        match self.language {
            Language::Java => format!(
                "package {package};\n\nimport org.springframework.web.bind.annotation.GetMapping;\nimport org.springframework.web.bind.annotation.RequestParam;\nimport org.springframework.web.bind.annotation.RestController;\n\n@RestController\npublic class HelloController {{\n\n    @GetMapping(\"/hello\")\n    public String hello(@RequestParam(defaultValue = \"World\") String name) {{\n        return \"Hello, \" + name + \"!\";\n    }}\n\n}}\n"
            ),
            Language::Kotlin => format!(
                "package {package}\n\nimport org.springframework.web.bind.annotation.GetMapping\nimport org.springframework.web.bind.annotation.RequestParam\nimport org.springframework.web.bind.annotation.RestController\n\n@RestController\nclass HelloController {{\n\n    @GetMapping(\"/hello\")\n    fun hello(@RequestParam(defaultValue = \"World\") name: String): String = \"Hello, $name!\"\n\n}}\n"
            ),
            Language::Groovy => format!(
                "package {package}\n\nimport org.springframework.web.bind.annotation.GetMapping\nimport org.springframework.web.bind.annotation.RequestParam\nimport org.springframework.web.bind.annotation.RestController\n\n@RestController\nclass HelloController {{\n\n    @GetMapping('/hello')\n    String hello(@RequestParam(defaultValue = 'World') String name) {{\n        'Hello, ' + name + '!'\n    }}\n\n}}\n"
            ),
        }
    }

    fn servlet_initializer_source(&self, class: &str) -> String {
        let package = &self.package;
        match self.language {
            Language::Java => format!(
                "package {package};\n\nimport org.springframework.boot.builder.SpringApplicationBuilder;\nimport org.springframework.boot.web.servlet.support.SpringBootServletInitializer;\n\npublic class ServletInitializer extends SpringBootServletInitializer {{\n\n    @Override\n    protected SpringApplicationBuilder configure(SpringApplicationBuilder application) {{\n        return application.sources({class}.class);\n    }}\n\n}}\n"
            ),
            Language::Kotlin => format!(
                "package {package}\n\nimport org.springframework.boot.builder.SpringApplicationBuilder\nimport org.springframework.boot.web.servlet.support.SpringBootServletInitializer\n\nclass ServletInitializer : SpringBootServletInitializer() {{\n\n    override fun configure(application: SpringApplicationBuilder): SpringApplicationBuilder {{\n        return application.sources({class}::class.java)\n    }}\n\n}}\n"
            ),
            Language::Groovy => format!(
                "package {package}\n\nimport org.springframework.boot.builder.SpringApplicationBuilder\nimport org.springframework.boot.web.servlet.support.SpringBootServletInitializer\n\nclass ServletInitializer extends SpringBootServletInitializer {{\n\n    @Override\n    protected SpringApplicationBuilder configure(SpringApplicationBuilder application) {{\n        application.sources({class})\n    }}\n\n}}\n"
            ),
        }
    }

    fn application_properties(&self) -> String {
        let mut text = format!("spring.application.name={}\n", self.artifact);
        let hints: [(&str, &str); 5] = [
            ("postgresql", "jdbc:postgresql://localhost:5432/mydb"),
            ("mysql", "jdbc:mysql://localhost:3306/mydb"),
            ("mariadb", "jdbc:mariadb://localhost:3306/mydb"),
            (
                "sqlserver",
                "jdbc:sqlserver://localhost:1433;databaseName=mydb",
            ),
            ("oracle", "jdbc:oracle:thin:@//localhost:1521/FREEPDB1"),
        ];
        if (self.selected("jdbc") || self.selected("data-jpa") || self.selected("data-jdbc"))
            && let Some((_, url)) = hints.iter().find(|(id, _)| self.selected(id))
        {
            {
                text.push_str(&format!(
                    "\n# Point the application at your database before starting it.\n# spring.datasource.url={url}\n# spring.datasource.username=\n# spring.datasource.password=\n"
                ));
            }
        }
        text
    }

    fn readme(&self, name: &str) -> String {
        let mut text = format!("# {name}\n\n{}\n", self.description_line());
        match self.build {
            SpringBuild::GradleGroovy | SpringBuild::GradleKotlin => text.push_str(
                "\n## Build and run\n\n```\ngradle bootRun\ngradle test\n```\n\nThe Gradle wrapper is not included. Run `gradle wrapper` once (or let the IDE sync the project) to create it.\n",
            ),
            SpringBuild::Maven => text.push_str(
                "\n## Build and run\n\n```\nmvn spring-boot:run\nmvn test\n```\n\nThe Maven wrapper is not included. Run `mvn wrapper:wrapper` once to create it.\n",
            ),
            SpringBuild::Bazel => text.push_str(
                "\n## Build and run\n\n```\nbazel run //:app\nbazel test //:app_test\n```\n\n## Notes on Bazel\n\nSpring's Gradle and Maven plugins are not used under Bazel: there is no `bootRun`, no\n`spring-boot:run` and no repackaged fat jar. `//:app` is a plain `java_binary` that\nstarts the application from the class path. Dependencies are resolved by\n`rules_jvm_external` against the Spring Boot BOM (see `MODULE.bazel`).\n`.bazelrc` turns strict Java dependency checks off because Spring applications use many\ntypes of transitive jars; tests run through the JUnit Platform console launcher.\n",
            ),
        }
        text
    }

    fn description_line(&self) -> String {
        if self.description.trim().is_empty() {
            "Demo project for Spring Boot".into()
        } else {
            self.description.trim().replace(['\n', '\r'], " ")
        }
    }

    fn help(&self, name: &str, catalog: &Catalog) -> String {
        let mut text = format!(
            "# Getting started with {name}\n\nSpring Boot {} on Java {}, built with {}.\n\n## Run\n\n",
            self.boot_version,
            self.java_version,
            self.build.label(),
        );
        text.push_str(match self.build {
            SpringBuild::GradleGroovy | SpringBuild::GradleKotlin => {
                "```\ngradle bootRun\n```\n\nPass profiles with `gradle bootRun --args='--spring.profiles.active=dev'`.\n"
            }
            SpringBuild::Maven => {
                "```\nmvn spring-boot:run\n```\n\nPass profiles with `mvn spring-boot:run -Dspring-boot.run.profiles=dev`.\n"
            }
            SpringBuild::Bazel => {
                "```\nbazel run //:app\n```\n\nPass profiles with `bazel run //:app -- --spring.profiles.active=dev`.\n"
            }
        });
        text.push_str("\n## Dependencies\n\n");
        if self.dependencies.is_empty() {
            text.push_str("No starters were selected; only `spring-boot-starter` and the test starter are used.\n");
        }
        for id in &self.dependencies {
            if let Some(dependency) = catalog.dependency(id) {
                text.push_str(&format!(
                    "- **{}**: {}\n",
                    dependency.name, dependency.description
                ));
            }
        }
        text.push_str(
            "\n## Tests\n\n`ApplicationTests` starts the whole application context. With a database starter\nconfigure `spring.datasource.*` first (or use Testcontainers), otherwise the test cannot connect.\n",
        );
        text
    }
}

fn spring_gitignore() -> &'static str {
    ".gradle/\nbuild/\ntarget/\nout/\n!gradle/wrapper/gradle-wrapper.jar\n!.mvn/wrapper/maven-wrapper.jar\nbazel-*\n.idea/\n*.iml\n.vscode/\n.archipel/\nlocal.properties\n*.log\nHELP.md.orig\n"
}

fn quote(value: &str, kts: bool) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('\n', " ")
        .replace('\r', "");
    if kts {
        format!("\"{}\"", escaped.replace('"', "\\\"").replace('$', "\\$"))
    } else {
        format!("'{}'", escaped.replace('\'', "\\'"))
    }
}

fn gradle_settings(artifact: &str, kts: bool) -> String {
    format!("rootProject.name = {}\n", quote(artifact, kts))
}

fn gradle_notation(coordinate: &Coordinate) -> String {
    match &coordinate.version {
        Some(version) => format!("{}:{}:{version}", coordinate.group, coordinate.artifact),
        None => format!("{}:{}", coordinate.group, coordinate.artifact),
    }
}

fn gradle_configuration(scope: Scope) -> &'static str {
    match scope {
        Scope::Implementation => "implementation",
        Scope::CompileOnly => "compileOnly",
        Scope::DevelopmentOnly => "developmentOnly",
        Scope::RuntimeOnly => "runtimeOnly",
        Scope::AnnotationProcessor => "annotationProcessor",
        Scope::Test => "testImplementation",
        Scope::TestCompileOnly => "testCompileOnly",
        Scope::TestAnnotationProcessor => "testAnnotationProcessor",
    }
}

/// Every dependency of the project as `(scope, notation)` in build-file order:
/// the starters, language support, the test starters.
fn all_dependencies(
    spec: &SpringSpec,
    coordinates: &[Coordinate],
    with_language_support: bool,
) -> Vec<(Scope, String)> {
    let mut lines: Vec<(Scope, String)> = vec![(
        Scope::Implementation,
        "org.springframework.boot:spring-boot-starter".into(),
    )];
    for coordinate in coordinates {
        for scope in &coordinate.scopes {
            lines.push((*scope, gradle_notation(coordinate)));
        }
    }
    if with_language_support {
        match spec.language {
            Language::Kotlin => {
                lines.push((
                    Scope::Implementation,
                    "org.jetbrains.kotlin:kotlin-reflect".into(),
                ));
                lines.push((
                    Scope::Implementation,
                    "tools.jackson.module:jackson-module-kotlin".into(),
                ));
                lines.push((
                    Scope::Test,
                    "org.jetbrains.kotlin:kotlin-test-junit5".into(),
                ));
            }
            Language::Groovy => {
                lines.push((Scope::Implementation, "org.apache.groovy:groovy".into()));
            }
            Language::Java => {}
        }
    }
    if !has_test_starter(coordinates) {
        lines.push((
            Scope::Test,
            "org.springframework.boot:spring-boot-starter-test".into(),
        ));
    }
    lines
}

/// Boot 4 starters bring their own `-test` companion; the general test starter is only
/// needed when none of the selected starters has one.
fn has_test_starter(coordinates: &[Coordinate]) -> bool {
    coordinates.iter().any(|c| {
        c.scopes.contains(&Scope::Test)
            && c.group == "org.springframework.boot"
            && c.artifact.starts_with("spring-boot-starter-")
            && c.artifact.ends_with("-test")
    })
}

fn gradle_build(
    spec: &SpringSpec,
    coordinates: &[Coordinate],
    kts: bool,
    kotlin_version: &str,
) -> String {
    let q = |text: &str| quote(text, kts);
    let mut out = String::new();
    let boot = &spec.boot_version;
    let kotlin = spec.language == Language::Kotlin;
    out.push_str("plugins {\n");
    let id = |plugin: &str, version: Option<&str>| match (kts, version) {
        (true, Some(v)) => format!("    id(\"{plugin}\") version \"{v}\"\n"),
        (true, None) => format!("    id(\"{plugin}\")\n"),
        (false, Some(v)) => format!("    id '{plugin}' version '{v}'\n"),
        (false, None) => format!("    id '{plugin}'\n"),
    };
    match spec.language {
        Language::Java => out.push_str(&id("java", None)),
        Language::Groovy => out.push_str(&id("groovy", None)),
        Language::Kotlin => {
            out.push_str(&id("org.jetbrains.kotlin.jvm", Some(kotlin_version)));
            out.push_str(&id(
                "org.jetbrains.kotlin.plugin.spring",
                Some(kotlin_version),
            ));
        }
    }
    if spec.war {
        out.push_str(&id("war", None));
    }
    out.push_str(&id("org.springframework.boot", Some(boot)));
    out.push_str(&id("io.spring.dependency-management", Some("1.1.7")));
    out.push_str("}\n\n");
    out.push_str(&format!(
        "group = {}\nversion = {}\ndescription = {}\n\n",
        q(&spec.group),
        q("0.0.1-SNAPSHOT"),
        q(&spec.description_line())
    ));
    out.push_str(&format!(
        "java {{\n    toolchain {{\n        languageVersion = JavaLanguageVersion.of({})\n    }}\n}}\n\n",
        spec.java_version
    ));
    out.push_str("repositories {\n    mavenCentral()\n}\n\n");
    out.push_str("dependencies {\n");
    let lines = all_dependencies(spec, coordinates, true);
    let mut ordered: Vec<(&'static str, String)> = Vec::new();
    for scope in [
        Scope::Implementation,
        Scope::CompileOnly,
        Scope::DevelopmentOnly,
        Scope::RuntimeOnly,
        Scope::AnnotationProcessor,
    ] {
        for (line_scope, notation) in &lines {
            if *line_scope == scope {
                ordered.push((gradle_configuration(scope), notation.clone()));
            }
        }
    }
    if spec.war {
        ordered.push((
            "providedRuntime",
            "org.springframework.boot:spring-boot-starter-tomcat".into(),
        ));
    }
    for scope in [
        Scope::Test,
        Scope::TestCompileOnly,
        Scope::TestAnnotationProcessor,
    ] {
        for (line_scope, notation) in &lines {
            if *line_scope == scope {
                ordered.push((gradle_configuration(scope), notation.clone()));
            }
        }
        if scope == Scope::Test {
            // The launcher is not a transitive dependency of the test starters.
            ordered.push((
                "testRuntimeOnly",
                "org.junit.platform:junit-platform-launcher".into(),
            ));
        }
    }
    for (configuration, notation) in ordered {
        if kts {
            out.push_str(&format!("    {configuration}({})\n", q(&notation)));
        } else {
            out.push_str(&format!("    {configuration} {}\n", q(&notation)));
        }
    }
    out.push_str("}\n\n");
    if kotlin {
        out.push_str(if kts {
            "kotlin {\n    compilerOptions {\n        freeCompilerArgs.addAll(\"-Xjsr305=strict\", \"-Xannotation-default-target=param-property\")\n    }\n}\n\n"
        } else {
            "kotlin {\n    compilerOptions {\n        freeCompilerArgs.addAll('-Xjsr305=strict', '-Xannotation-default-target=param-property')\n    }\n}\n\n"
        });
    }
    out.push_str(if kts {
        "tasks.withType<Test> {\n    useJUnitPlatform()\n}\n"
    } else {
        "tasks.named('test') {\n    useJUnitPlatform()\n}\n"
    });
    out
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', " ")
        .replace('\r', "")
}

fn maven_dependency(coordinate: &Coordinate, scope: Option<&str>, optional: bool) -> String {
    let mut out = format!(
        "        <dependency>\n            <groupId>{}</groupId>\n            <artifactId>{}</artifactId>\n",
        xml(&coordinate.group),
        xml(&coordinate.artifact)
    );
    if let Some(version) = &coordinate.version {
        out.push_str(&format!(
            "            <version>{}</version>\n",
            xml(version)
        ));
    }
    if let Some(scope) = scope {
        out.push_str(&format!("            <scope>{scope}</scope>\n"));
    }
    if optional {
        out.push_str("            <optional>true</optional>\n");
    }
    out.push_str("        </dependency>\n");
    out
}

fn maven_pom(
    spec: &SpringSpec,
    name: &str,
    coordinates: &[Coordinate],
    kotlin_version: &str,
) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project xmlns=\"http://maven.apache.org/POM/4.0.0\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n         xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd\">\n    <modelVersion>4.0.0</modelVersion>\n",
    );
    out.push_str(&format!(
        "    <parent>\n        <groupId>org.springframework.boot</groupId>\n        <artifactId>spring-boot-starter-parent</artifactId>\n        <version>{}</version>\n        <relativePath/>\n    </parent>\n",
        xml(&spec.boot_version)
    ));
    out.push_str(&format!(
        "    <groupId>{}</groupId>\n    <artifactId>{}</artifactId>\n    <version>0.0.1-SNAPSHOT</version>\n",
        xml(&spec.group),
        xml(&spec.artifact)
    ));
    if spec.war {
        out.push_str("    <packaging>war</packaging>\n");
    }
    out.push_str(&format!(
        "    <name>{}</name>\n    <description>{}</description>\n",
        xml(name),
        xml(&spec.description_line())
    ));
    out.push_str(&format!(
        "    <properties>\n        <java.version>{}</java.version>\n",
        spec.java_version
    ));
    let kotlin = spec.language == Language::Kotlin;
    if kotlin {
        out.push_str(&format!(
            "        <kotlin.version>{}</kotlin.version>\n",
            xml(kotlin_version)
        ));
    }
    out.push_str("    </properties>\n    <dependencies>\n");
    let plain = |group: &str, artifact: &str| Coordinate {
        group: group.into(),
        artifact: artifact.into(),
        version: None,
        scopes: vec![Scope::Implementation],
        when: None,
        processor: None,
    };
    out.push_str(&maven_dependency(
        &plain("org.springframework.boot", "spring-boot-starter"),
        None,
        false,
    ));
    let mut compile_processors: Vec<&Coordinate> = Vec::new();
    let mut test_processors: Vec<&Coordinate> = Vec::new();
    for coordinate in coordinates {
        let scopes = &coordinate.scopes;
        let has = |scope| scopes.contains(&scope);
        if has(Scope::AnnotationProcessor) {
            compile_processors.push(coordinate);
        }
        if has(Scope::TestAnnotationProcessor) {
            test_processors.push(coordinate);
        }
        let rendered = if has(Scope::Test) {
            Some(maven_dependency(coordinate, Some("test"), false))
        } else if has(Scope::DevelopmentOnly) {
            Some(maven_dependency(coordinate, Some("runtime"), true))
        } else if has(Scope::RuntimeOnly) {
            Some(maven_dependency(coordinate, Some("runtime"), false))
        } else if has(Scope::CompileOnly) || has(Scope::AnnotationProcessor) {
            Some(maven_dependency(coordinate, None, true))
        } else if has(Scope::Implementation) {
            Some(maven_dependency(coordinate, None, false))
        } else {
            None
        };
        out.push_str(&rendered.unwrap_or_default());
    }
    match spec.language {
        Language::Kotlin => {
            out.push_str(&maven_dependency(
                &plain("org.jetbrains.kotlin", "kotlin-reflect"),
                None,
                false,
            ));
            out.push_str(&maven_dependency(
                &plain("org.jetbrains.kotlin", "kotlin-stdlib"),
                None,
                false,
            ));
            out.push_str(&maven_dependency(
                &plain("tools.jackson.module", "jackson-module-kotlin"),
                None,
                false,
            ));
        }
        Language::Groovy => {
            out.push_str(&maven_dependency(
                &plain("org.apache.groovy", "groovy"),
                None,
                false,
            ));
        }
        Language::Java => {}
    }
    if spec.war {
        out.push_str(&maven_dependency(
            &plain("org.springframework.boot", "spring-boot-starter-tomcat"),
            Some("provided"),
            false,
        ));
    }
    if !has_test_starter(coordinates) {
        out.push_str(&maven_dependency(
            &plain("org.springframework.boot", "spring-boot-starter-test"),
            Some("test"),
            false,
        ));
    }
    if kotlin {
        out.push_str(&maven_dependency(
            &plain("org.jetbrains.kotlin", "kotlin-test-junit5"),
            Some("test"),
            false,
        ));
    }
    out.push_str("    </dependencies>\n\n    <build>\n");
    if kotlin {
        out.push_str(
            "        <sourceDirectory>${project.basedir}/src/main/kotlin</sourceDirectory>\n        <testSourceDirectory>${project.basedir}/src/test/kotlin</testSourceDirectory>\n",
        );
    }
    out.push_str("        <plugins>\n");
    out.push_str("            <plugin>\n                <groupId>org.springframework.boot</groupId>\n                <artifactId>spring-boot-maven-plugin</artifactId>\n            </plugin>\n");
    if !compile_processors.is_empty() || !test_processors.is_empty() {
        let paths = |processors: &[&Coordinate]| -> String {
            let mut text = String::new();
            for processor in processors {
                text.push_str(&format!(
                    "                                <path>\n                                    <groupId>{}</groupId>\n                                    <artifactId>{}</artifactId>\n                                </path>\n",
                    xml(&processor.group),
                    xml(&processor.artifact)
                ));
            }
            text
        };
        out.push_str("            <plugin>\n                <groupId>org.apache.maven.plugins</groupId>\n                <artifactId>maven-compiler-plugin</artifactId>\n                <executions>\n");
        for (id, phase, goal, processors) in [
            ("default-compile", "compile", "compile", &compile_processors),
            (
                "default-testCompile",
                "test-compile",
                "testCompile",
                &test_processors,
            ),
        ] {
            if processors.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "                    <execution>\n                        <id>{id}</id>\n                        <phase>{phase}</phase>\n                        <goals>\n                            <goal>{goal}</goal>\n                        </goals>\n                        <configuration>\n                            <annotationProcessorPaths>\n{}                            </annotationProcessorPaths>\n                        </configuration>\n                    </execution>\n",
                paths(processors)
            ));
        }
        out.push_str("                </executions>\n            </plugin>\n");
    }
    match spec.language {
        Language::Kotlin => out.push_str(
            "            <plugin>\n                <groupId>org.jetbrains.kotlin</groupId>\n                <artifactId>kotlin-maven-plugin</artifactId>\n                <configuration>\n                    <args>\n                        <arg>-Xjsr305=strict</arg>\n                        <arg>-Xannotation-default-target=param-property</arg>\n                    </args>\n                    <compilerPlugins>\n                        <plugin>spring</plugin>\n                    </compilerPlugins>\n                </configuration>\n                <dependencies>\n                    <dependency>\n                        <groupId>org.jetbrains.kotlin</groupId>\n                        <artifactId>kotlin-maven-allopen</artifactId>\n                        <version>${kotlin.version}</version>\n                    </dependency>\n                </dependencies>\n            </plugin>\n",
        ),
        Language::Groovy => out.push_str(
            "            <plugin>\n                <groupId>org.codehaus.gmavenplus</groupId>\n                <artifactId>gmavenplus-plugin</artifactId>\n                <version>1.13.1</version>\n                <executions>\n                    <execution>\n                        <goals>\n                            <goal>addSources</goal>\n                            <goal>addTestSources</goal>\n                            <goal>generateStubs</goal>\n                            <goal>compile</goal>\n                            <goal>generateTestStubs</goal>\n                            <goal>compileTests</goal>\n                            <goal>removeStubs</goal>\n                            <goal>removeTestStubs</goal>\n                        </goals>\n                    </execution>\n                </executions>\n            </plugin>\n",
        ),
        Language::Java => {}
    }
    out.push_str("        </plugins>\n    </build>\n</project>\n");
    out
}

/// Bazel version pinned in `.bazelversion` (honoured by bazelisk).
pub const BAZEL_VERSION: &str = "8.3.1";
const RULES_JAVA: &str = "8.11.0";
const RULES_JVM_EXTERNAL: &str = "6.7";
const MAVEN_REPOSITORY: &str = "https://repo.maven.apache.org/maven2";

/// `org.springframework.boot:spring-boot-starter-web` becomes
/// `@maven//:org_springframework_boot_spring_boot_starter_web`.
pub fn bazel_label(group: &str, artifact: &str) -> String {
    let flat: String = format!("{group}_{artifact}")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("@maven//:{flat}")
}

/// Coordinates a Bazel build can use: development-time only starters (devtools, Docker
/// Compose) have no meaning for `bazel run` and are left out.
fn bazel_coordinates(coordinates: &[Coordinate]) -> Vec<&Coordinate> {
    coordinates
        .iter()
        .filter(|c| !c.scopes.contains(&Scope::DevelopmentOnly))
        .collect()
}

fn bazel_module(spec: &SpringSpec, coordinates: &[Coordinate]) -> String {
    let module = bazel_module_name(&spec.artifact);
    let mut out = format!(
        "# Spring Boot's Gradle and Maven plugins are not used under Bazel; see README.md.\nmodule(\n    name = \"{module}\",\n    version = \"0.0.1\",\n)\n\nbazel_dep(name = \"rules_java\", version = \"{RULES_JAVA}\")\nbazel_dep(name = \"rules_jvm_external\", version = \"{RULES_JVM_EXTERNAL}\")\n\nmaven = use_extension(\"@rules_jvm_external//:extensions.bzl\", \"maven\")\nmaven.install(\n    # Versions come from the Spring Boot BOM; entries with an explicit version override it.\n    boms = [\"org.springframework.boot:spring-boot-dependencies:{}\"],\n    artifacts = [\n",
        spec.boot_version
    );
    let mut artifacts: Vec<String> = vec![
        "org.springframework.boot:spring-boot-starter".into(),
        "org.springframework.boot:spring-boot-starter-test".into(),
        "org.junit.platform:junit-platform-console".into(),
    ];
    for coordinate in bazel_coordinates(coordinates) {
        artifacts.push(gradle_notation(coordinate));
    }
    artifacts.dedup();
    let mut seen = BTreeSet::new();
    for artifact in artifacts {
        if seen.insert(artifact.clone()) {
            out.push_str(&format!("        \"{artifact}\",\n"));
        }
    }
    out.push_str(&format!(
        "    ],\n    repositories = [\"{MAVEN_REPOSITORY}\"],\n)\nuse_repo(maven, \"maven\")\n"
    ));
    out
}

fn bazel_build(spec: &SpringSpec, coordinates: &[Coordinate]) -> String {
    let class = SpringSpec::application_class(&spec.artifact);
    let main_class = format!("{}.{class}", spec.package);
    let test_class = format!("{}.{class}Tests", spec.package);
    let coordinates = bazel_coordinates(coordinates);
    let mut deps: Vec<String> = vec![bazel_label(
        "org.springframework.boot",
        "spring-boot-starter",
    )];
    let mut plugins: Vec<(String, String, String)> = Vec::new();
    let mut test_deps: Vec<String> = vec![
        bazel_label("org.springframework.boot", "spring-boot-starter-test"),
        bazel_label("org.junit.platform", "junit-platform-console"),
    ];
    for coordinate in &coordinates {
        let label = bazel_label(&coordinate.group, &coordinate.artifact);
        for scope in &coordinate.scopes {
            match scope {
                Scope::Test => test_deps.push(label.clone()),
                Scope::AnnotationProcessor => {
                    if let Some(processor) = &coordinate.processor {
                        let name = coordinate
                            .artifact
                            .replace(|c: char| !c.is_ascii_alphanumeric(), "_");
                        plugins.push((name, processor.clone(), label.clone()));
                    }
                }
                Scope::TestCompileOnly | Scope::TestAnnotationProcessor => {}
                Scope::CompileOnly
                | Scope::Implementation
                | Scope::RuntimeOnly
                | Scope::DevelopmentOnly => {
                    if !deps.contains(&label) {
                        deps.push(label.clone());
                    }
                }
            }
        }
    }
    let mut out = String::from(
        "load(\"@rules_java//java:defs.bzl\", \"java_binary\", \"java_library\", \"java_plugin\", \"java_test\")\n\n",
    );
    plugins.sort();
    plugins.dedup();
    for (name, processor, label) in &plugins {
        out.push_str(&format!(
            "java_plugin(\n    name = \"{name}_plugin\",\n    processor_class = \"{processor}\",\n    deps = [\"{label}\"],\n)\n\n"
        ));
    }
    out.push_str("java_library(\n    name = \"app_lib\",\n    srcs = glob([\"src/main/java/**/*.java\"]),\n    resources = glob([\"src/main/resources/**\"]),\n    resource_strip_prefix = \"src/main/resources\",\n");
    if !plugins.is_empty() {
        out.push_str("    plugins = [\n");
        for (name, _, _) in &plugins {
            out.push_str(&format!("        \":{name}_plugin\",\n"));
        }
        out.push_str("    ],\n");
    }
    out.push_str("    deps = [\n");
    for dep in &deps {
        out.push_str(&format!("        \"{dep}\",\n"));
    }
    out.push_str("    ],\n)\n\n");
    out.push_str(&format!(
        "java_binary(\n    name = \"app\",\n    main_class = \"{main_class}\",\n    runtime_deps = [\":app_lib\"],\n)\n\n"
    ));
    test_deps.dedup();
    out.push_str(&format!(
        "# JUnit 5 tests run through the JUnit Platform console launcher.\njava_test(\n    name = \"app_test\",\n    srcs = glob([\"src/test/java/**/*.java\"]),\n    args = [\n        \"execute\",\n        \"--select-class={test_class}\",\n        \"--fail-if-no-tests\",\n    ],\n    main_class = \"org.junit.platform.console.ConsoleLauncher\",\n    use_testrunner = False,\n    deps = [\n        \":app_lib\",\n"
    ));
    for dep in &test_deps {
        out.push_str(&format!("        \"{dep}\",\n"));
    }
    out.push_str("    ],\n)\n");
    out
}

/// Bazel module names are lowercase letters, digits, `.`, `-` and `_`, starting with a letter.
pub(crate) fn bazel_module_name(name: &str) -> String {
    let mut module: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();
    if !module.starts_with(|c: char| c.is_ascii_lowercase()) {
        module.insert_str(0, "app-");
    }
    while module.ends_with(['.', '-', '_']) {
        module.pop();
    }
    module
}

pub(crate) fn bazel_rc(java_version: u32) -> String {
    format!(
        "# Compile and run with the JDK on JAVA_HOME; use remotejdk_{java_version} to download one instead.\nbuild --java_language_version={java_version}\nbuild --java_runtime_version=local_jdk\nbuild --tool_java_language_version={java_version}\nbuild --tool_java_runtime_version=local_jdk\n# Spring applications use many types from transitive jars.\nbuild --strict_java_deps=off\ntest --test_output=errors\n"
    )
}
