# New Project

Start without arguments, click **New Project**, select a template, enter a name
and an existing parent directory, then click **Create**. File and the project
widget expose the same dialog. Successful creation opens the new project.

```sh
cargo run -p archipel-ide-app
cargo test -p archipel-ide-project templates_and_collisions
cargo test -p archipel-ide-core create_project_command
cargo test -p archipel-ide-runner detects_gradle
```

Templates: Empty, Rust application, Rust library, Java/Gradle with Kotlin DSL or
Groovy DSL. Java uses an application plugin, JDK 21 toolchain and the supplied
Java group as the sample package. Gradle build/run falls back to installed Gradle
when no wrapper exists. Rust libraries default to Cargo Test rather than Cargo Run.

Creation validates identifiers and uses an exclusive new directory. Existing
projects are never overwritten. Filesystem failures identify the retained partial
directory. The current project's files/workspace must save before creating another.

Native macOS walkthrough, 2026-10-10: created a Java/Kotlin-DSL project under `/tmp`
from the welcome button; verified the form, creation and transition to the project
workbench. Golden tests check the sample Java source. JDK/Gradle execution was not
verified on this host. No equivalence to IntelliJ's complete wizard is claimed.

References: [Gradle application plugin](https://docs.gradle.org/current/userguide/application_plugin.html)
and [JVM toolchains](https://docs.gradle.org/current/userguide/toolchains.html).

Pending: native directory chooser, JDK discovery/download, pinned Gradle wrapper,
Maven templates, dependency selection, recent-project list and trust prompts.
