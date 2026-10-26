//! Settings | Build Tools | Gradle options reach the task plans. Kept in its own
//! test binary because the options are process-wide.

use archipel_ide_gradle::{BuildRunner, Distribution, GradleBuild, GradleOptions, set_options};
use archipel_ide_runner::{BuildSystem, TaskKind};

#[test]
fn plans_follow_the_gradle_options() {
    let root = std::env::temp_dir().join(format!("jt-gradle-options-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("settings.gradle.kts"),
        "rootProject.name = \"x\"\n",
    )
    .unwrap();
    std::fs::write(root.join("gradlew"), "").unwrap();
    std::fs::write(root.join("gradlew.bat"), "").unwrap();
    let build = GradleBuild::detect(&root).unwrap();
    let tasks = ["build".to_owned()];

    let plan = build.task_plan(TaskKind::Build, "b", &tasks).unwrap();
    assert!(!plan.args.contains(&"--offline".to_owned()));
    assert!(plan.env.is_empty());

    set_options(GradleOptions {
        build_run: BuildRunner::Gradle,
        distribution: Distribution::Wrapper,
        offline: true,
        user_home: Some("/tmp/gh".into()),
        jvm: Some("/opt/jdk".into()),
    });
    let plan = build.task_plan(TaskKind::Build, "b", &tasks).unwrap();
    assert!(plan.args.contains(&"--offline".to_owned()));
    assert!(
        plan.env
            .contains(&("GRADLE_USER_HOME".into(), "/tmp/gh".into()))
    );
    assert!(plan.env.contains(&("JAVA_HOME".into(), "/opt/jdk".into())));
    assert!(
        build
            .sync_plan()
            .unwrap()
            .args
            .contains(&"--offline".to_owned())
    );

    // "Build and run using: Archipel IDE" compiles instead of running the full build.
    let gradle = archipel_ide_gradle::Gradle;
    let plan = gradle.build_plan(&root).unwrap();
    assert_eq!(plan.args[0], "build");
    set_options(GradleOptions {
        build_run: BuildRunner::Archipel,
        ..GradleOptions::default()
    });
    let plan = gradle.build_plan(&root).unwrap();
    assert_eq!(&plan.args[..2], ["classes", "testClasses"]);

    // The local distribution ignores the wrapper (and needs `gradle` on PATH).
    set_options(GradleOptions {
        distribution: Distribution::Local,
        ..GradleOptions::default()
    });
    if let Ok(plan) = build.task_plan(TaskKind::Build, "b", &tasks) {
        assert_eq!(plan.program, "gradle");
    }
    let _ = std::fs::remove_dir_all(&root);
}
