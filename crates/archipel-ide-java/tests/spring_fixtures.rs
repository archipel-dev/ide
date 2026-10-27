//! The headline scenario on real Spring Boot projects made by the IDE's generator and imported
//! by the real build tools (Gradle and Maven). Needs the tools and warm dependency caches, so the
//! tests are ignored by default:
//! `cargo test -p archipel-ide-java --test spring_fixtures -- --ignored --test-threads=1 --nocapture`.

use archipel_ide_analysis::{FileId, FileRev, ModCommand, Severity};
use archipel_ide_externalsystem::{
    ExternalSystemRegistry, ProjectDataManager, ResolveRequest, import_project,
};
use archipel_ide_java::api::JavaProject;
use archipel_ide_java::diagnostics::{JavaSettings, apply_command, collect_detailed, first_choice};
use archipel_ide_java::index::JavaIndex;
use archipel_ide_java::source;
use archipel_ide_project::generator::Language;
use archipel_ide_project::spring::{SpringBuild, SpringSpec};
use archipel_ide_project::spring_catalog::Catalog;
use std::path::PathBuf;
use std::sync::Arc;

const CALC_TEST: &str = r#"package com.example.demo;

import org.junit.jupiter.api.Test;

public class CalcTest {
    @Test
    public void adds() {
        String x = "a";
        assertThat(x).isEqualTo("a");
    }
}
"#;

fn generate(build: SpringBuild, name: &str) -> PathBuf {
    let catalog = Catalog::builtin();
    let spec = SpringSpec {
        language: Language::Java,
        build,
        group: "com.example".into(),
        artifact: name.into(),
        package: "com.example.demo".into(),
        description: "Demo".into(),
        java_version: 21,
        boot_version: catalog
            .boot_versions
            .first()
            .cloned()
            .unwrap_or_else(|| "3.5.0".into()),
        war: false,
        dependencies: ["web", "data-jpa", "lombok", "h2", "validation"]
            .map(String::from)
            .to_vec(),
    };
    let root = std::env::temp_dir().join(format!(
        "archipel-java-spring-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in spec.files(name, &catalog).expect("valid spec") {
        let target = root.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, text).unwrap();
    }
    std::fs::canonicalize(&root).unwrap()
}

fn headline(build: SpringBuild, name: &str) {
    let root = generate(build, name);
    let result = import_project(
        &ExternalSystemRegistry::with_defaults(),
        &ProjectDataManager::new(),
        &ResolveRequest::new(&root),
    )
    .unwrap_or_else(|e| panic!("import failed: {e}"));
    let model = Arc::new(result.model);
    let index = JavaIndex::in_memory();
    let started = std::time::Instant::now();
    index.sync_blocking(model.clone());
    eprintln!(
        "indexed {} libraries in {:.1}s",
        model.libraries.len(),
        started.elapsed().as_secs_f32()
    );
    let project = JavaProject::new(index, model.clone(), false);
    let test = root.join("src/test/java/com/example/demo/CalcTest.java");
    let main = root.join("src/main/java/com/example/demo/CalcMain.java");
    std::fs::write(&test, CALC_TEST).unwrap();
    // Every generated source file analyzes without a single error (no false "unresolved").
    for entry in walkdir::WalkDir::new(root.join("src"))
        .into_iter()
        .filter_map(Result::ok)
    {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("java") || p == test {
            continue;
        }
        let text = std::fs::read_to_string(p).unwrap();
        let tree = source::parse(&text).unwrap();
        let c = collect_detailed(&project, &JavaSettings::default(), p, &text, &tree);
        let errs: Vec<_> = c
            .infos
            .iter()
            .filter(|i| i.severity >= Severity::ERROR)
            .map(|i| i.message.clone())
            .collect();
        assert!(errs.is_empty(), "{}: {errs:?}", p.display());
    }
    // The headline: the native error and the static import fix, AssertJ first.
    let tree = source::parse(CALC_TEST).unwrap();
    let c = collect_detailed(&project, &JavaSettings::default(), &test, CALC_TEST, &tree);
    let errs: Vec<_> = c
        .infos
        .iter()
        .filter(|i| i.severity >= Severity::ERROR)
        .collect();
    assert_eq!(
        errs.len(),
        1,
        "{:?}",
        errs.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
    );
    assert_eq!(
        errs[0].message.as_deref(),
        Some("Cannot resolve method 'assertThat(String)'")
    );
    assert_eq!(errs[0].fixes[0].text.as_ref(), "Import static method...");
    let (_, fix) = &c.static_fixes[0];
    let names: Vec<_> = fix.candidates.iter().map(|c| c.display()).collect();
    eprintln!("candidates: {names:?}");
    assert_eq!(names[0], "Assertions.assertThat");
    assert!(
        fix.candidates
            .iter()
            .all(|c| c.package() == "org.assertj.core.api"),
        "{names:?}"
    );
    let cmd = first_choice(ModCommand::ChooseFrom {
        title: "Method to Import".into(),
        rows: (0..fix.candidates.len())
            .map(|i| archipel_ide_analysis::ChoiceRow {
                label: fix.candidates[i].display(),
                detail: None,
                command: fix.apply(FileId(1), FileRev(1), CALC_TEST, i),
            })
            .collect(),
    });
    let after = apply_command(&cmd, FileId(1), CALC_TEST).unwrap();
    assert_eq!(
        after,
        CALC_TEST.replace("import org.junit.jupiter.api.Test;\n", "import org.junit.jupiter.api.Test;\n\nimport static org.assertj.core.api.Assertions.assertThat;\n")
    );
    // Production code is not offered the test-scope import (JAVA-IMP-004).
    let prod_text = CALC_TEST
        .replace("CalcTest", "CalcMain")
        .replace("import org.junit.jupiter.api.Test;\n\n", "")
        .replace("    @Test\n", "");
    let tree = source::parse(&prod_text).unwrap();
    let c = collect_detailed(&project, &JavaSettings::default(), &main, &prod_text, &tree);
    let e: Vec<_> = c
        .infos
        .iter()
        .filter(|i| i.severity >= Severity::ERROR)
        .collect();
    assert_eq!(e.len(), 1);
    assert!(
        c.static_fixes.is_empty(),
        "production file offered {:?}",
        c.static_fixes
            .iter()
            .map(|(_, f)| f.candidates.len())
            .collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "needs Gradle and warm caches"]
fn gradle_spring_boot_headline() {
    headline(SpringBuild::GradleKotlin, "demo-gradle");
}

#[test]
#[ignore = "needs Maven and warm caches"]
fn maven_spring_boot_headline() {
    headline(SpringBuild::Maven, "demo-maven");
}
