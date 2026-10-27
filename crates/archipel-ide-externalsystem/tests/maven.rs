//! Maven resolver: POM parsing and reactor handling on fixtures (no Maven needed), plus an
//! ignored test that runs the real `mvn`.

use archipel_ide_externalsystem::maven::*;
use archipel_ide_externalsystem::*;
use archipel_ide_projectmodel::{DependencyScope, JpsModuleSourceRootType, OrderEntry};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn fixture(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("archipel-es-maven-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write(
        &root,
        "pom.xml",
        r#"<?xml version="1.0"?>
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>org.acme</groupId>
  <artifactId>parent</artifactId>
  <version>1.0</version>
  <packaging>pom</packaging>
  <properties><java.version>21</java.version><gen.dir>${project.build.directory}/gen</gen.dir></properties>
  <modules><module>core</module><module>web</module></modules>
  <dependencyManagement><dependencies><dependency>
    <groupId>ignored</groupId><artifactId>managed</artifactId><version>1</version>
  </dependency></dependencies></dependencyManagement>
</project>"#,
    );
    write(
        &root,
        "core/pom.xml",
        r#"<project>
  <parent><groupId>org.acme</groupId><artifactId>parent</artifactId><version>1.0</version></parent>
  <artifactId>core</artifactId>
  <build>
    <directory>out</directory>
    <sourceDirectory>src/java</sourceDirectory>
    <testSourceDirectory>${project.basedir}/test/java</testSourceDirectory>
    <resources><resource><directory>conf</directory></resource><resource><directory>src/main/resources</directory></resource></resources>
    <plugins>
      <plugin>
        <groupId>org.codehaus.mojo</groupId>
        <artifactId>build-helper-maven-plugin</artifactId>
        <executions>
          <execution><goals><goal>add-source</goal></goals>
            <configuration><sources><source>${gen.dir}</source><source>src/extra</source></sources></configuration>
          </execution>
          <execution><goals><goal>add-test-source</goal></goals>
            <configuration><sources><source>test/extra</source></sources></configuration>
          </execution>
        </executions>
      </plugin>
    </plugins>
  </build>
  <dependencies>
    <dependency><groupId>com.google.guava</groupId><artifactId>guava</artifactId><version>33.0.0-jre</version></dependency>
  </dependencies>
</project>"#,
    );
    write(
        &root,
        "web/pom.xml",
        r#"<project>
  <parent><groupId>org.acme</groupId><artifactId>parent</artifactId><version>1.0</version></parent>
  <artifactId>web</artifactId>
  <dependencies>
    <dependency><groupId>org.acme</groupId><artifactId>core</artifactId><version>${project.version}</version></dependency>
    <dependency><groupId>org.junit</groupId><artifactId>junit</artifactId><version>5</version><scope>test</scope></dependency>
  </dependencies>
</project>"#,
    );
    root
}

#[test]
fn parses_the_pom_parts_the_importer_needs() {
    let pom = parse_pom(
        r#"<project>
  <parent><groupId>g</groupId><artifactId>p</artifactId><version>2</version><relativePath/></parent>
  <artifactId>x</artifactId>
  <dependencies>
    <dependency><groupId>a</groupId><artifactId>b</artifactId><version>1</version><scope>provided</scope><classifier>c</classifier></dependency>
    <dependency><groupId>a</groupId><artifactId>d</artifactId></dependency>
  </dependencies>
  <profiles><profile><dependencies><dependency><artifactId>ignored</artifactId></dependency></dependencies></profile></profiles>
</project>"#,
    )
    .unwrap();
    assert_eq!(pom.artifact_id, "x");
    assert_eq!(pom.packaging, "jar");
    assert_eq!(pom.parent.as_ref().unwrap().version, "2");
    assert_eq!(pom.dependencies.len(), 2);
    assert_eq!(pom.dependencies[0].scope, "provided");
    assert_eq!(pom.dependencies[0].classifier, "c");
    assert_eq!(pom.dependencies[1].scope, "compile");
    assert!(parse_pom("<project><unclosed></project>").is_err());
}

#[test]
fn reactor_honors_build_settings_and_inheritance() {
    let root = fixture("reactor");
    let reactor = read_reactor(&root).unwrap();
    let ids: Vec<_> = reactor.iter().map(ReactorProject::id).collect();
    assert_eq!(ids, ["org.acme:parent", "org.acme:core", "org.acme:web"]);
    let core = &reactor[1];
    assert_eq!(core.group_id, "org.acme", "inherited from the parent block");
    assert_eq!(core.source_directory, root.join("core/src/java"));
    assert_eq!(core.test_source_directory, root.join("core/test/java"));
    assert_eq!(core.directory, root.join("core/out"));
    assert_eq!(core.output_directory, root.join("core/out/classes"));
    assert_eq!(
        core.test_output_directory,
        root.join("core/out/test-classes")
    );
    assert_eq!(
        core.resources,
        [root.join("core/conf"), root.join("core/src/main/resources")]
    );
    assert_eq!(
        core.added_sources,
        [root.join("core/out/gen"), root.join("core/src/extra")]
    );
    assert_eq!(core.added_test_sources, [root.join("core/test/extra")]);
    assert_eq!(
        core.language_level.as_deref(),
        Some("21"),
        "properties are inherited"
    );
    let web = &reactor[2];
    assert_eq!(
        web.source_directory,
        root.join("web/src/main/java"),
        "defaults apply"
    );
    assert_eq!(web.directory, root.join("web/target"));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn data_nodes_from_pom_roots_and_classifier_scopes() {
    let root = fixture("nodes");
    let reactor = read_reactor(&root).unwrap();
    let repo = "/home/u/.m2/repository";
    let jar = |path: &str| PathBuf::from(format!("{repo}/{path}"));
    let guava = jar("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar");
    let junit = jar("org/junit/junit/5/junit-5.jar");
    let tools = jar("org/acme/tools/1/tools-1-linux.jar");
    let mut classpaths = Classpaths::default();
    // core: guava compile. web: core's classes (reactor), guava via core, junit test only.
    classpaths.by_module.insert(
        root.join("core"),
        ScopedClasspath {
            compile: vec![guava.clone()],
            runtime: vec![guava.clone(), tools.clone()],
            test: vec![guava.clone(), tools.clone()],
        },
    );
    classpaths.by_module.insert(
        root.join("web"),
        ScopedClasspath {
            compile: vec![root.join("core/out/classes"), guava.clone()],
            runtime: vec![root.join("core/out/classes"), guava.clone()],
            test: vec![root.join("core/out/classes"), guava.clone(), junit.clone()],
        },
    );
    let tree = to_data_nodes(&root, &reactor, &classpaths, &["a note".into()]);
    let models = ProjectDataManager::new().import_data(&[tree]);
    let model = Arc::new(models.model);
    assert_eq!(model.name, "parent");
    let names: Vec<_> = model.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["parent", "core", "web"]);
    assert!(
        models
            .diagnostics
            .iter()
            .any(|d| d.message.contains("a note"))
    );

    // The pom aggregator has a content root but no source folders.
    assert_eq!(model.module("parent").unwrap().source_folders().count(), 0);
    let core = model.module("core").unwrap();
    let kinds: Vec<_> = core
        .source_folders()
        .map(|f| (f.root_type, f.path.strip_prefix(&root).unwrap().to_owned()))
        .collect();
    assert!(kinds.contains(&(JpsModuleSourceRootType::Source, "core/src/java".into())));
    assert!(kinds.contains(&(JpsModuleSourceRootType::Source, "core/src/extra".into())));
    assert!(kinds.contains(&(JpsModuleSourceRootType::Resource, "core/conf".into())));
    assert!(kinds.contains(&(
        JpsModuleSourceRootType::TestSource,
        "core/test/extra".into()
    )));
    assert!(kinds.contains(&(
        JpsModuleSourceRootType::Generated,
        "core/out/generated-sources/annotations".into()
    )));
    assert_eq!(
        core.excluded_folders().collect::<Vec<_>>(),
        [&root.join("core/out")]
    );
    assert_eq!(
        core.compiler_output.production,
        [root.join("core/out/classes")]
    );

    let scope = |module: &str, library_part: &str| {
        model
            .module(module)
            .unwrap()
            .order_entries
            .iter()
            .find_map(|e| match e {
                OrderEntry::Library { library, scope, .. } if library.contains(library_part) => {
                    Some(*scope)
                }
                _ => None,
            })
    };
    assert_eq!(scope("core", "guava"), Some(DependencyScope::Compile));
    assert_eq!(scope("core", "tools"), Some(DependencyScope::Runtime));
    assert_eq!(scope("web", "junit"), Some(DependencyScope::Test));
    assert_eq!(scope("web", "guava"), Some(DependencyScope::Compile));
    // Classifier jars get their own library.
    assert!(model.library("Maven: org.acme:tools:1:linux").is_some());
    let guava_lib = model
        .library("Maven: com.google.guava:guava:33.0.0-jre")
        .unwrap();
    assert_eq!(
        guava_lib.coordinates.as_ref().unwrap().group,
        "com.google.guava"
    );
    // Module dependencies from the pom and the reactor class directory, once.
    let web = model.module("web").unwrap();
    assert_eq!(
        web.order_entries
            .iter()
            .filter(|e| matches!(e, OrderEntry::Module { .. }))
            .count(),
        1
    );
    assert!(
        web.order_entries
            .contains(&OrderEntry::module("core", DependencyScope::Compile))
    );
    // The reactor class directory is not a library.
    assert!(model.libraries.iter().all(|l| !l.name.contains("classes")));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn classpath_scopes_are_classified_from_set_differences() {
    let p = |s: &str| PathBuf::from(s);
    let scoped = ScopedClasspath {
        compile: vec![p("/a.jar"), p("/provided.jar")],
        runtime: vec![p("/a.jar"), p("/runtime.jar")],
        test: vec![
            p("/a.jar"),
            p("/provided.jar"),
            p("/runtime.jar"),
            p("/t.jar"),
        ],
    };
    let classified = scoped.classify();
    let scope = |name: &str| {
        classified
            .iter()
            .find(|(p, _)| p.ends_with(name))
            .unwrap()
            .1
    };
    assert_eq!(scope("a.jar"), DependencyScope::Compile);
    assert_eq!(scope("provided.jar"), DependencyScope::Provided);
    assert_eq!(scope("runtime.jar"), DependencyScope::Runtime);
    assert_eq!(scope("t.jar"), DependencyScope::Test);
    assert_eq!(classified[0].0, p("/a.jar"), "Maven's order is kept");
}

#[test]
fn coordinates_come_from_the_repository_layout() {
    let repos = vec![PathBuf::from("/r/repo")];
    let (c, classifier) = coordinates_from_repository_path(
        Path::new("/r/repo/org/springframework/boot/spring-boot/3.5.0/spring-boot-3.5.0.jar"),
        &repos,
    )
    .unwrap();
    assert_eq!(c.to_string(), "org.springframework.boot:spring-boot:3.5.0");
    assert_eq!(classifier, None);
    let (c, classifier) = coordinates_from_repository_path(
        Path::new("/r/repo/io/netty/netty-x/4.1/netty-x-4.1-linux-x86_64.jar"),
        &repos,
    )
    .unwrap();
    assert_eq!(c.group, "io.netty");
    assert_eq!(classifier.as_deref(), Some("linux-x86_64"));
    assert!(coordinates_from_repository_path(Path::new("/elsewhere/a.jar"), &repos).is_none());
}

#[test]
fn splits_build_classpath_output() {
    if cfg!(unix) {
        assert_eq!(
            split_classpath("/a.jar:/b.jar\n"),
            [PathBuf::from("/a.jar"), PathBuf::from("/b.jar")]
        );
    }
    assert!(split_classpath("").is_empty());
}

/// Needs Maven (`mvn` or a wrapper), a JDK and either network access or a ~/.m2 that has the
/// `maven-dependency-plugin` and the artifacts below. Run with `cargo test -- --ignored`.
#[test]
#[ignore = "needs Maven and Maven Central (or a warm ~/.m2)"]
fn imports_a_real_multi_module_build() {
    if archipel_ide_runner::find_program("mvn").is_none() {
        eprintln!("skipping: mvn is not on PATH");
        return;
    }
    let root = fixture("real");
    // Real dependencies instead of the fake ones.
    write(
        &root,
        "core/pom.xml",
        r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <parent><groupId>org.acme</groupId><artifactId>parent</artifactId><version>1.0</version></parent>
  <artifactId>core</artifactId>
  <dependencies>
    <dependency><groupId>org.apache.commons</groupId><artifactId>commons-lang3</artifactId><version>3.14.0</version></dependency>
    <dependency><groupId>org.slf4j</groupId><artifactId>slf4j-simple</artifactId><version>2.0.13</version><scope>runtime</scope></dependency>
    <dependency><groupId>javax.servlet</groupId><artifactId>javax.servlet-api</artifactId><version>4.0.1</version><scope>provided</scope></dependency>
  </dependencies>
</project>"#,
    );
    write(
        &root,
        "web/pom.xml",
        r#"<project>
  <modelVersion>4.0.0</modelVersion>
  <parent><groupId>org.acme</groupId><artifactId>parent</artifactId><version>1.0</version></parent>
  <artifactId>web</artifactId>
  <dependencies>
    <dependency><groupId>org.acme</groupId><artifactId>core</artifactId><version>1.0</version></dependency>
    <dependency><groupId>junit</groupId><artifactId>junit</artifactId><version>4.13.2</version><scope>test</scope></dependency>
  </dependencies>
</project>"#,
    );
    let result = import_project(
        &ExternalSystemRegistry::with_defaults(),
        &ProjectDataManager::new(),
        &ResolveRequest::new(&root),
    )
    .unwrap();
    let model = &result.model;
    let scope = |module: &str, part: &str| {
        model
            .module(module)
            .unwrap()
            .order_entries
            .iter()
            .find_map(|e| match e {
                OrderEntry::Library { library, scope, .. } if library.contains(part) => {
                    Some(*scope)
                }
                _ => None,
            })
    };
    assert_eq!(
        scope("core", "commons-lang3"),
        Some(DependencyScope::Compile),
        "{:?}",
        result.diagnostics
    );
    assert_eq!(
        scope("core", "slf4j-simple"),
        Some(DependencyScope::Runtime)
    );
    assert_eq!(
        scope("core", "javax.servlet-api"),
        Some(DependencyScope::Provided)
    );
    assert_eq!(scope("web", "junit"), Some(DependencyScope::Test));
    assert_eq!(
        scope("web", "commons-lang3"),
        Some(DependencyScope::Compile),
        "transitive through core"
    );
    assert!(
        model
            .module("web")
            .unwrap()
            .order_entries
            .contains(&OrderEntry::module("core", DependencyScope::Compile))
    );
    let _ = std::fs::remove_dir_all(&root);
}
