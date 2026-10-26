# Run/Debug configurations

Captured 2026-10-10 under Xvfb (1600x1000, `ARCHIPEL_RENDERER=glow`), Gradle 8.14.3 and
JDK 21 on PATH, `java-gradle` sample (see `build-run.md`). References:
`reference/intellij-community/java-gradle/15-run-config.png` and
`x02-run-debug-configurations.png` (PR #40).

| File | Scene | Steps → result |
|---|---|---|
| `run-config/15a-run-widget-no-configurations.png` | 15 | No saved configurations: the widget shows the detected default ("Gradle Run"); popup: Edit Configurations…, Build Project, Run All Tests, Rerun Last Task |
| `run-config/x02a-add-configuration.png` | x02 | Edit Configurations… → empty dialog, **+** → "Add New Configuration" with Gradle, Cargo, Bazel, Maven, npm, Go, Shell Command |
| `run-config/x02-run-debug-configurations.png` | x02 | Gradle → form with Name, Type, Run (tasks), Options, Program arguments, Working directory, Environment variables (`KEY=value; …`, as in the reference); renamed to "App", `GREETING=hi` |
| `run-config/15-run-widget-configurations.png` | 15 + 17 | **▶ Run** in the dialog saved `.archipel/runConfigurations/App.toml` and ran it ("java-gradle [App]: successful"); the widget now reads "App" and lists it, selected |

Stored file:

```toml
name = "App"
type = "gradle"
tasks = ["run"]

[env]
GREETING = "hi"
```

Differences from the reference: no per-type option sets beyond the fields above (no JDK
selector, "Modify options", before-launch tasks, or "Store as project file" toggle; all
saved configurations are project files). Arguments are split on whitespace (no quoting).
There is no Debug or Profile button yet, and no "Current File" entry. Temporary
configurations (from running a Gradle task) are listed greyed until saved.
