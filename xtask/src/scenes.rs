//! `cargo xtask scenes`: replays the parity scene scripts in `docs/parity/scenes/`
//! against a real workbench over the control socket and writes PNGs.
//!
//! Each scene starts a fresh IDE process on a freshly generated fixture project,
//! so scenes are independent and reproducible. Scenes assert workbench state
//! (via the `state` request), never pixels; the PNGs are evidence for review.

use anyhow::{Context, Result, anyhow, bail};
use archipel_ide_control::{ControlClient, Request, Response};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const SCENES_DIR: &str = "docs/parity/scenes";
const DEFAULT_OUT: &str = "docs/parity/evidence/archipel-ide/scenes";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    /// Scene number shared with `docs/parity/evidence/reference/`.
    pub id: String,
    pub title: String,
    /// Reference capture this scene corresponds to (documentation only).
    #[serde(default)]
    pub reference: Option<String>,
    /// Fixture project to generate and open; none shows the welcome screen.
    #[serde(default)]
    pub fixture: Option<String>,
    /// Project-relative files passed on the command line.
    #[serde(default)]
    pub open: Vec<String>,
    /// Toolchain components the scene needs (`jdtls`); it is skipped, not
    /// failed, when one is not installed on this machine.
    #[serde(default)]
    pub requires: Vec<String>,
    /// Extra environment variables of the IDE process (e.g. to slow indexing down).
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    pub steps: Vec<Value>,
}

/// One scene step: a control request, or a runner-side action.
#[derive(Debug, PartialEq)]
pub enum Step {
    Control(Request),
    /// Saves a screenshot as `<out>/<name>`.
    Screenshot {
        name: String,
    },
    /// Compares the JSON value at `pointer` in the `state` snapshot.
    Assert {
        pointer: String,
        equals: Option<Value>,
        contains: Option<String>,
    },
    Sleep(Duration),
    /// Polls the `state` snapshot until the value at `pointer` matches or
    /// `timeout` passes (slow background work: a project import).
    WaitFor {
        pointer: String,
        equals: Option<Value>,
        contains: Option<String>,
        timeout: Duration,
    },
    /// Writes (or appends to) a file in the fixture, e.g. to create VCS changes.
    WriteFile {
        path: String,
        contents: String,
        append: bool,
    },
}

impl Step {
    pub fn parse(value: &Value) -> Result<Self> {
        let op = value
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("step without \"op\": {value}"))?;
        let string = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| anyhow!("{op} step needs string \"{key}\": {value}"))
        };
        Ok(match op {
            "screenshot" => Step::Screenshot {
                name: string("name")?,
            },
            "assert" => {
                let equals = value.get("equals").cloned();
                let contains = value
                    .get("contains")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                if equals.is_none() && contains.is_none() {
                    bail!("assert step needs \"equals\" or \"contains\": {value}");
                }
                Step::Assert {
                    pointer: string("pointer")?,
                    equals,
                    contains,
                }
            }
            "waitFor" => {
                let equals = value.get("equals").cloned();
                let contains = value
                    .get("contains")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                if equals.is_none() && contains.is_none() {
                    bail!("waitFor step needs \"equals\" or \"contains\": {value}");
                }
                Step::WaitFor {
                    pointer: string("pointer")?,
                    equals,
                    contains,
                    timeout: Duration::from_millis(
                        value
                            .get("timeoutMs")
                            .and_then(Value::as_u64)
                            .unwrap_or(10_000),
                    ),
                }
            }
            "sleep" => Step::Sleep(Duration::from_millis(
                value
                    .get("ms")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| anyhow!("sleep step needs \"ms\": {value}"))?,
            )),
            "writeFile" | "appendFile" => Step::WriteFile {
                path: string("path")?,
                contents: string("contents")?,
                append: op == "appendFile",
            },
            _ => Step::Control(
                serde_json::from_value(value.clone())
                    .with_context(|| format!("invalid control step {value}"))?,
            ),
        })
    }
}

pub fn load_scenes(dir: &Path) -> Result<Vec<(Scene, Vec<Step>)>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)?;
            let scene: Scene =
                serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
            let steps = scene
                .steps
                .iter()
                .map(Step::parse)
                .collect::<Result<Vec<_>>>()
                .with_context(|| format!("in {}", path.display()))?;
            if let Some(fixture) = &scene.fixture {
                fixture_files(fixture)?;
            }
            Ok((scene, steps))
        })
        .collect()
}

struct Options {
    only: Vec<String>,
    out: PathBuf,
    release: bool,
}

fn parse_options(args: impl Iterator<Item = String>) -> Result<Options> {
    let mut options = Options {
        only: Vec::new(),
        out: PathBuf::from(DEFAULT_OUT),
        release: false,
    };
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--only" => {
                let list = args.next().ok_or_else(|| anyhow!("--only needs ids"))?;
                options.only = list.split(',').map(|id| id.trim().to_owned()).collect();
            }
            "--out" => {
                options.out = args
                    .next()
                    .ok_or_else(|| anyhow!("--out needs a directory"))?
                    .into()
            }
            "--release" => options.release = true,
            other => bail!("unknown scenes option {other:?}"),
        }
    }
    Ok(options)
}

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let options = parse_options(args)?;
    let scenes: Vec<_> = load_scenes(Path::new(SCENES_DIR))?
        .into_iter()
        .filter(|(scene, _)| options.only.is_empty() || options.only.contains(&scene.id))
        .collect();
    if scenes.is_empty() {
        bail!("no scenes selected");
    }
    let app = build_app(options.release)?;
    let display = Display::acquire()?;
    let out = std::env::current_dir()?.join(&options.out);
    fs::create_dir_all(&out)?;
    let work = std::env::current_dir()?.join("target/scenes");
    let mut failures = Vec::new();
    for (scene, steps) in &scenes {
        match &scene.reference {
            Some(reference) => print!(
                "scene {} {} (reference {reference}) … ",
                scene.id, scene.title
            ),
            None => print!("scene {} {} … ", scene.id, scene.title),
        }
        match run_scene(&app, &display, &work, &out, scene, steps) {
            Ok(()) => println!("ok"),
            Err(error) if error.downcast_ref::<Skipped>().is_some() => {
                println!("skipped ({error})")
            }
            Err(error) => {
                println!("FAILED\n    {error:#}");
                failures.push(scene.id.clone());
            }
        }
    }
    println!("screenshots in {}", options.out.display());
    if !failures.is_empty() {
        bail!(
            "{} scene(s) failed: {}",
            failures.len(),
            failures.join(", ")
        );
    }
    Ok(())
}

/// A scene that cannot run on this machine (a required tool is missing).
#[derive(Debug)]
struct Skipped(String);

impl std::fmt::Display for Skipped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Skipped {}

pub(crate) fn build_app(release: bool) -> Result<PathBuf> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut command = Command::new(cargo);
    command.args(["build", "-p", "archipel-ide-app"]);
    if release {
        command.arg("--release");
    }
    if !command.status().context("run cargo build")?.success() {
        bail!("building archipel-ide-app failed");
    }
    let profile = if release { "release" } else { "debug" };
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"));
    let exe = target
        .join(profile)
        .join(format!("archipel-ide{}", std::env::consts::EXE_SUFFIX));
    std::path::absolute(&exe).context("resolve app path")
}

/// The X display scenes render on. Uses `$DISPLAY` when set; otherwise starts
/// Xvfb on Linux. Other platforms use their native window system.
pub(crate) struct Display {
    pub(crate) name: Option<String>,
    xvfb: Option<Child>,
}

impl Display {
    pub(crate) fn acquire() -> Result<Self> {
        if let Some(name) = std::env::var("DISPLAY").ok().filter(|d| !d.is_empty()) {
            return Ok(Self {
                name: Some(name),
                xvfb: None,
            });
        }
        if !cfg!(target_os = "linux") {
            return Ok(Self {
                name: None,
                xvfb: None,
            });
        }
        let number = (90..140)
            .find(|n| !Path::new(&format!("/tmp/.X11-unix/X{n}")).exists())
            .ok_or_else(|| anyhow!("no free X display number"))?;
        let name = format!(":{number}");
        let child = Command::new("Xvfb")
            .args([
                name.as_str(),
                "-screen",
                "0",
                "1600x1000x24",
                "-nolisten",
                "tcp",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context(
                "DISPLAY is unset and Xvfb could not be started; install xvfb or set DISPLAY",
            )?;
        let socket = PathBuf::from(format!("/tmp/.X11-unix/X{number}"));
        let started = Instant::now();
        while !socket.exists() {
            if started.elapsed() > Duration::from_secs(10) {
                bail!("Xvfb {name} did not start");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(Self {
            name: Some(name),
            xvfb: Some(child),
        })
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        if let Some(child) = self.xvfb.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Kills the IDE when a scene ends, however it ends.
pub(crate) struct AppProcess(pub(crate) Child);

impl Drop for AppProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run_scene(
    app: &Path,
    display: &Display,
    work: &Path,
    out: &Path,
    scene: &Scene,
    steps: &[Step],
) -> Result<()> {
    let scene_dir = work.join(&scene.id);
    if scene_dir.exists() {
        fs::remove_dir_all(&scene_dir)?;
    }
    fs::create_dir_all(&scene_dir)?;
    let project = match &scene.fixture {
        Some(name) => Some(create_fixture(name, &scene_dir.join(name))?),
        None => None,
    };
    // Unix socket paths are limited to ~100 bytes, so keep them short.
    let socket =
        std::env::temp_dir().join(format!("jt-scene-{}-{}.sock", std::process::id(), scene.id));
    let log = fs::File::create(scene_dir.join("app.log"))?;
    let mut command = Command::new(app);
    if let Some(project) = &project {
        command.arg(project);
        command.args(&scene.open);
    }
    command
        .env(archipel_ide_control::SOCKET_ENV, &socket)
        .env("ARCHIPEL_NO_UPDATE_CHECK", "1")
        .env("ARCHIPEL_NO_TOOLCHAIN_SETUP", "1")
        // Indexes and history of a scene never touch the user's cache.
        .env("ARCHIPEL_INDEX_DIR", scene_dir.join("cache"))
        .envs(&scene.env)
        .stdout(log.try_clone()?)
        .stderr(log);
    if archipel_ide_compat::env_var_os("ARCHIPEL_RENDERER").is_none() && cfg!(target_os = "linux") {
        command.env("ARCHIPEL_RENDERER", "glow");
    }
    if let Some(display) = &display.name {
        command.env("DISPLAY", display);
    }
    let mut process = AppProcess(
        command
            .spawn()
            .with_context(|| format!("launch {}", app.display()))?,
    );
    let mut client = connect(&socket, &mut process.0, &scene_dir)?;
    for component in &scene.requires {
        let located = send(
            &mut client,
            &Request::Execute {
                command: "toolchain.locate".into(),
                args: serde_json::json!({"component": component}),
            },
        )?;
        if located.is_null() {
            return Err(Skipped(format!("{component} is not installed")).into());
        }
    }
    for (index, step) in steps.iter().enumerate() {
        run_step(&mut client, step, project.as_deref(), out)
            .with_context(|| format!("step {} ({:?})", index + 1, step))?;
    }
    Ok(())
}

pub(crate) fn connect(socket: &Path, app: &mut Child, scene_dir: &Path) -> Result<ControlClient> {
    let started = Instant::now();
    loop {
        if let Ok(client) = ControlClient::connect(socket) {
            return Ok(client);
        }
        if let Some(status) = app.try_wait()? {
            let log = fs::read_to_string(scene_dir.join("app.log")).unwrap_or_default();
            let tail: Vec<&str> = log.lines().take(5).collect();
            bail!(
                "the IDE exited ({status}) before opening its control socket:\n{}",
                tail.join("\n")
            );
        }
        if started.elapsed() > STARTUP_TIMEOUT {
            bail!(
                "the IDE did not open {} within {STARTUP_TIMEOUT:?}",
                socket.display()
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn send(client: &mut ControlClient, request: &Request) -> Result<Value> {
    let Response { ok, result, error } = client.send(request)?;
    if !ok {
        bail!("{}", error.unwrap_or_else(|| "request failed".into()));
    }
    Ok(result.unwrap_or(Value::Null))
}

fn run_step(
    client: &mut ControlClient,
    step: &Step,
    project: Option<&Path>,
    out: &Path,
) -> Result<()> {
    match step {
        Step::Control(request) => {
            send(client, request)?;
        }
        Step::Screenshot { name } => {
            send(
                client,
                &Request::Screenshot {
                    path: out.join(name),
                },
            )?;
        }
        Step::Assert {
            pointer,
            equals,
            contains,
        } => {
            let state = send(client, &Request::State)?;
            check_assertion(&state, pointer, equals.as_ref(), contains.as_deref())?;
        }
        Step::Sleep(duration) => std::thread::sleep(*duration),
        Step::WaitFor {
            pointer,
            equals,
            contains,
            timeout,
        } => {
            let started = Instant::now();
            loop {
                let state = send(client, &Request::State)?;
                match check_assertion(&state, pointer, equals.as_ref(), contains.as_deref()) {
                    Ok(()) => break,
                    Err(error) if started.elapsed() > *timeout => {
                        bail!("timed out after {timeout:?}: {error}")
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(250)),
                }
            }
        }
        Step::WriteFile {
            path,
            contents,
            append,
        } => {
            let project = project.ok_or_else(|| anyhow!("file steps need a fixture"))?;
            let target = project.join(path);
            if *append {
                let mut text = fs::read_to_string(&target).unwrap_or_default();
                text.push_str(contents);
                fs::write(&target, text)?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target, contents)?;
            }
        }
    }
    Ok(())
}

pub fn check_assertion(
    state: &Value,
    pointer: &str,
    equals: Option<&Value>,
    contains: Option<&str>,
) -> Result<()> {
    let actual = state.pointer(pointer).unwrap_or(&Value::Null);
    if let Some(expected) = equals
        && actual != expected
    {
        bail!("state{pointer} is {actual}, expected {expected}");
    }
    if let Some(needle) = contains {
        let haystack = match actual {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        if !haystack.contains(needle) {
            bail!("state{pointer} = {actual} does not contain {needle:?}");
        }
    }
    Ok(())
}

/// Files of a fixture project, as (relative path, contents).
pub fn fixture_files(name: &str) -> Result<Vec<(&'static str, &'static str)>> {
    match name {
        "java-gradle" => Ok(vec![
            (
                "settings.gradle.kts",
                "rootProject.name = \"java-gradle\"\n",
            ),
            (
                "build.gradle.kts",
                "plugins {\n    application\n}\n\nrepositories {\n    mavenCentral()\n}\n\ndependencies {\n    testImplementation(\"org.junit.jupiter:junit-jupiter:5.11.0\")\n}\n\napplication {\n    mainClass = \"com.example.App\"\n}\n",
            ),
            (
                "src/main/java/com/example/App.java",
                "package com.example;\n\npublic class App {\n    public static String greeting(String name) {\n        return \"Hello, \" + name + \"!\";\n    }\n\n    public static void main(String[] args) {\n        System.out.println(greeting(\"Gradle\"));\n        System.out.println(greeting(\"Java\"));\n    }\n}\n",
            ),
            (
                "src/test/java/com/example/AppTest.java",
                "package com.example;\n\nimport static org.junit.jupiter.api.Assertions.assertEquals;\n\nimport org.junit.jupiter.api.Test;\n\nclass AppTest {\n    @Test\n    void greets() {\n        assertEquals(\"Hello, Java!\", App.greeting(\"Java\"));\n    }\n}\n",
            ),
            (".gitignore", ".gradle/\nbuild/\n.archipel/\n"),
            (
                "README.md",
                "# java-gradle\n\nSample project for Archipel IDE parity scenes.\n",
            ),
        ]),
        // A test that calls AssertJ's `assertThat` without the static import:
        // the language server marks it and offers the import.
        "java-gradle-assertj" => Ok(vec![
            (
                "settings.gradle",
                "rootProject.name = 'java-gradle-assertj'\n",
            ),
            (
                "build.gradle",
                "plugins {\n    id 'java'\n}\n\nrepositories {\n    mavenCentral()\n}\n\ndependencies {\n    testImplementation 'org.junit.jupiter:junit-jupiter:5.11.0'\n    testRuntimeOnly 'org.junit.platform:junit-platform-launcher:1.11.0'\n    testImplementation 'org.assertj:assertj-core:3.26.3'\n}\n\ntest {\n    useJUnitPlatform()\n}\n",
            ),
            (
                "src/main/java/com/example/App.java",
                "package com.example;\n\npublic class App {\n    public static String greeting(String name) {\n        return \"Hello, \" + name + \"!\";\n    }\n}\n",
            ),
            (
                "src/test/java/com/example/AppTest.java",
                "package com.example;\n\nimport org.junit.jupiter.api.Test;\n\nclass AppTest {\n    @Test\n    void greets() {\n        assertThat(App.greeting(\"Java\")).isEqualTo(\"Hello, Java!\");\n    }\n}\n",
            ),
            (".gitignore", ".gradle/\nbuild/\nbin/\n.archipel/\n"),
        ]),
        "rust-cargo" => Ok(vec![
            (
                "Cargo.toml",
                "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n# Standalone even when generated inside another Cargo workspace.\n[workspace]\n",
            ),
            (
                "src/main.rs",
                "fn greeting(name: &str) -> String {\n    format!(\"Hello, {name}!\")\n}\n\nfn main() {\n    println!(\"{}\", greeting(\"Cargo\"));\n    println!(\"{}\", greeting(\"Rust\"));\n}\n",
            ),
            // Not part of the crate (no `mod`), so builds stay green; it carries a
            // syntax error for the Problems scene.
            ("src/broken.rs", "fn broken( {\n    let x = ;\n}\n"),
            (".gitignore", "target/\n.archipel/\n"),
        ]),
        "spring-maven" => Ok(vec![
            (
                "pom.xml",
                "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n  <modelVersion>4.0.0</modelVersion>\n  <parent>\n    <groupId>org.springframework.boot</groupId>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>4.1.2</version>\n  </parent>\n  <groupId>com.example</groupId>\n  <artifactId>demo</artifactId>\n  <version>0.0.1</version>\n</project>\n",
            ),
            (
                "src/main/java/com/example/demo/DemoApplication.java",
                "package com.example.demo;\n\nimport org.springframework.boot.SpringApplication;\nimport org.springframework.boot.autoconfigure.SpringBootApplication;\n\n@SpringBootApplication\npublic class DemoApplication {\n\n    public static void main(String[] args) {\n        SpringApplication.run(DemoApplication.class, args);\n    }\n}\n",
            ),
            (".gitignore", "target/\n.archipel/\n"),
        ]),
        other => bail!(
            "unknown fixture {other:?} (known: java-gradle, java-gradle-assertj, rust-cargo, spring-maven)"
        ),
    }
}

/// Writes the fixture and commits it to a fresh Git repository on `master`.
pub(crate) fn create_fixture(name: &str, dir: &Path) -> Result<PathBuf> {
    for (path, contents) in fixture_files(name)? {
        let target = dir.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, contents)?;
    }
    let git = |args: &[&str]| -> Result<()> {
        let status = Command::new("git")
            .args([
                "-c",
                "user.name=Archipel IDE Scenes",
                "-c",
                "user.email=scenes@archipel-ide.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("run git (required for fixtures)")?;
        if !status.success() {
            bail!("git {} failed in {}", args.join(" "), dir.display());
        }
        Ok(())
    };
    git(&["init", "-q", "-b", "master"])?;
    git(&["add", "-A"])?;
    git(&["commit", "-q", "-m", "Initial commit"])?;
    git(&["commit", "-q", "--allow-empty", "-m", "Add greeting"])?;
    Ok(fs::canonicalize(dir)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn steps_parse_control_requests_and_runner_actions() {
        assert_eq!(
            Step::parse(&json!({"op": "key", "chord": "Escape"})).unwrap(),
            Step::Control(Request::Key {
                chord: "Escape".into()
            })
        );
        assert_eq!(
            Step::parse(&json!({"op": "sleep", "ms": 5})).unwrap(),
            Step::Sleep(Duration::from_millis(5))
        );
        assert!(matches!(
            Step::parse(&json!({"op": "appendFile", "path": "a", "contents": "b"})).unwrap(),
            Step::WriteFile { append: true, .. }
        ));
        assert!(Step::parse(&json!({"op": "assert", "pointer": "/x"})).is_err());
        assert!(Step::parse(&json!({"op": "screenshot"})).is_err());
        assert!(Step::parse(&json!({"op": "teleport"})).is_err());
        assert!(Step::parse(&json!({"chord": "A"})).is_err());
    }

    #[test]
    fn assertions_compare_state_values() {
        let state = json!({"toolWindows": {"bottom": "terminal"}, "status": "Opened App.java"});
        assert!(
            check_assertion(
                &state,
                "/toolWindows/bottom",
                Some(&json!("terminal")),
                None
            )
            .is_ok()
        );
        assert!(check_assertion(&state, "/toolWindows/left", Some(&Value::Null), None).is_ok());
        assert!(
            check_assertion(&state, "/toolWindows/bottom", Some(&json!("find")), None).is_err()
        );
        assert!(check_assertion(&state, "/status", None, Some("App.java")).is_ok());
        assert!(check_assertion(&state, "/status", None, Some("Main.kt")).is_err());
    }

    #[test]
    fn checked_in_scenes_are_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(SCENES_DIR);
        let scenes = load_scenes(&root).unwrap();
        let ids: Vec<&str> = scenes.iter().map(|(scene, _)| scene.id.as_str()).collect();
        for required in ["01", "04", "05", "06", "11", "12"] {
            assert!(ids.contains(&required), "scene {required} missing: {ids:?}");
        }
        for (scene, steps) in &scenes {
            assert!(
                steps
                    .iter()
                    .any(|step| matches!(step, Step::Screenshot { .. })),
                "scene {} takes no screenshot",
                scene.id
            );
            assert!(
                steps.iter().any(|step| matches!(step, Step::Assert { .. })),
                "scene {} asserts nothing",
                scene.id
            );
        }
    }
}
