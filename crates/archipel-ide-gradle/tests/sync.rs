//! Runs a real Gradle sync against a multi-module fixture.
//!
//! Skipped (with a message) when Gradle or a JDK is not installed, or when
//! `ARCHIPEL_SKIP_GRADLE_IT` is set.

use archipel_ide_gradle::{GradleBuild, GradleModel, model::model_path};
use archipel_ide_runner::{TaskService, TaskState, find_program};
use std::time::{Duration, Instant};

fn write(root: &std::path::Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

#[test]
fn syncs_multi_module_fixture_with_real_gradle() {
    if archipel_ide_compat::env_var_os("ARCHIPEL_SKIP_GRADLE_IT").is_some() {
        eprintln!("skipping: ARCHIPEL_SKIP_GRADLE_IT is set");
        return;
    }
    if find_program("gradle").is_none() || find_program("java").is_none() {
        eprintln!("skipping: Gradle and a JDK must be on PATH for the sync integration test");
        return;
    }
    let root =
        std::env::temp_dir().join(format!("archipel-ide-gradle-sync-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write(
        &root,
        "settings.gradle.kts",
        "rootProject.name = \"fixture\"\ninclude(\":app\", \":lib\")\n",
    );
    write(
        &root,
        "app/build.gradle.kts",
        "plugins { application }\napplication { mainClass = \"demo.App\" }\ndependencies { implementation(project(\":lib\")) }\n",
    );
    write(
        &root,
        "lib/build.gradle.kts",
        "plugins { `java-library` }\n",
    );
    write(
        &root,
        "app/src/main/java/demo/App.java",
        "package demo;\npublic class App { public static void main(String[] a) {} }\n",
    );

    let build = GradleBuild::detect(&root).unwrap();
    let plan = build.sync_plan().unwrap();
    let mut service = TaskService::default();
    service.start(plan).unwrap();
    let deadline = Instant::now() + Duration::from_secs(600);
    while service.is_running() && Instant::now() < deadline {
        service.poll().unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }
    service.poll().unwrap();
    let console: Vec<_> = service
        .console()
        .lines()
        .iter()
        .map(|l| l.text.clone())
        .collect();
    assert_eq!(service.state(), &TaskState::Exited(0), "{console:#?}");

    let model = GradleModel::parse(&std::fs::read_to_string(model_path(&root)).unwrap()).unwrap();
    let paths: Vec<_> = model.projects.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(paths, [":", ":app", ":lib"]);
    let app = model.project(":app").unwrap();
    assert!(app.plugins.application && app.plugins.java);
    assert!(
        app.task_groups()["application"]
            .iter()
            .any(|task| task.path == ":app:run")
    );
    assert!(app.source_sets.iter().any(|set| set.name == "test"));
    assert!(!model.project(":lib").unwrap().plugins.application);
    assert!(
        model.task(":archipelModel").is_none(),
        "helper task must be hidden"
    );
    assert!(model.task(":app:archipelRunMain").is_none());

    // The init script's JavaExec task runs any main class (gutter run icons).
    let script = build.init_script().unwrap();
    std::fs::write(
        root.join("app/src/main/java/demo/App.java"),
        "package demo;\npublic class App { public static void main(String[] a) { System.out.println(\"ran \" + a.length); } }\n",
    )
    .unwrap();
    let plan = build
        .task_plan(
            archipel_ide_runner::TaskKind::Run,
            "App",
            &[
                ":app:archipelRunMain".into(),
                "--init-script".into(),
                script.display().to_string(),
                "-Parchipel.mainClass=demo.App".into(),
                "-q".into(),
            ],
        )
        .unwrap();
    service.start(plan).unwrap();
    let deadline = Instant::now() + Duration::from_secs(600);
    while service.is_running() && Instant::now() < deadline {
        service.poll().unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }
    service.poll().unwrap();
    let console: Vec<_> = service
        .console()
        .lines()
        .iter()
        .map(|l| l.text.clone())
        .collect();
    assert_eq!(service.state(), &TaskState::Exited(0), "{console:#?}");
    assert!(console.iter().any(|line| line == "ran 0"), "{console:#?}");
    let _ = std::fs::remove_dir_all(&root);
}
