//! `cargo xtask ux-smoke`: drives the golden paths of the workbench the way a
//! person does, with real X input (`xdotool` clicks, double clicks, right clicks
//! and key presses under Xvfb). The control socket is used only to read the
//! `state` snapshot and to save screenshots; the harness never dispatches
//! commands and never injects input through the socket.
//!
//! A scene fails when a control a user needs is not where the layout promises
//! it: the click lands on nothing and the expected state never arrives. That is
//! exactly the class of bug automated command-driven checks cannot see (v0.0.7
//! shipped a New Project dialog whose Create button was off-screen).
//!
//! Coordinates are logical points in a 1440×900 window at (0, 0) with no window
//! manager; see `docs/ux/interaction-spec.md` for the layout contract they rely
//! on. Keep them in one place ([`layout`]) so a deliberate layout change is a
//! one-line update here.

use anyhow::{Context, Result, anyhow, bail};
use jetthing_control::{ControlClient, Request, Response};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::scenes::{AppProcess, Display, build_app, connect, create_fixture};

const DEFAULT_OUT: &str = "target/ux-smoke/out";
const WINDOW: (u32, u32) = (1440, 900);
/// The minimum window size the app allows (`with_min_inner_size`).
const SMALL_WINDOW: (u32, u32) = (800, 500);
const STATE_TIMEOUT: Duration = Duration::from_secs(5);

/// Points every scene clicks, for the default 1440×900 window unless noted.
mod layout {
    pub type Point = (i32, i32);
    // Welcome screen.
    pub const WELCOME_NEW_PROJECT: Point = (318, 204);
    pub const FIRST_RUN_SKIP: Point = (672, 411);
    // New Project dialog (1440×900).
    pub const NP_GENERATOR_RUST_APP: Point = (440, 587);
    pub const NP_NAME: Point = (850, 231);
    pub const NP_LOCATION: Point = (820, 265);
    pub const NP_CREATE: Point = (1006, 665);
    // New Project dialog in the 800×500 minimum window.
    pub const NP_SMALL_GENERATOR_EMPTY: Point = (77, 163);
    pub const NP_SMALL_NAME: Point = (540, 111);
    pub const NP_SMALL_LOCATION: Point = (508, 145);
    pub const NP_SMALL_CREATE: Point = (686, 475);
    // Workbench with the project tool window open (default width).
    pub const TREE_SRC_CHEVRON: Point = (80, 113);
    /// Second row under `src` of the rust-cargo fixture (`broken.rs` sorts first).
    pub const TREE_MAIN_RS: Point = (150, 161);
    /// First row under `src` of the rust-cargo fixture.
    pub const TREE_BROKEN_RS: Point = (150, 137);
    /// With `broken.rs` and `main.rs` open: centres of their tabs, and the left half of the
    /// first one (an insertion point before it).
    pub const TAB_SECOND: Point = (505, 61);
    pub const TAB_FIRST_LEFT_HALF: Point = (345, 61);
    /// The first tab once the editor is maximised (the tool window is gone).
    pub const TAB_FIRST_MAXIMISED: Point = (120, 61);
    pub const PROJECT_HIDE_BUTTON: Point = (293, 58);
    pub const PROJECT_STRIPE_BUTTON: Point = (19, 62);
    pub const FIRST_TAB_CLOSE: Point = (419, 61);
    pub const TOOLBAR_SEARCH: Point = (1287, 20);
    pub const TOOLBAR_RUN: Point = (1197, 20);
    pub const HAMBURGER: Point = (19, 20);
    pub const MENU_FILE: Point = (18, 20);
    pub const MENU_FILE_SETTINGS: Point = (46, 174);
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
                let list = args.next().ok_or_else(|| anyhow!("--only needs names"))?;
                options.only = list.split(',').map(|id| id.trim().to_owned()).collect();
            }
            "--out" => {
                options.out = args
                    .next()
                    .ok_or_else(|| anyhow!("--out needs a directory"))?
                    .into()
            }
            "--release" => options.release = true,
            other => bail!("unknown ux-smoke option {other:?}"),
        }
    }
    Ok(options)
}

type SceneFn = fn(&mut Driver) -> Result<()>;

/// The golden paths, in the order a new user hits them.
const SCENES: &[(&str, SceneFn)] = &[
    ("first-run-dialog-is-modal", first_run_dialog_is_modal),
    ("new-project-by-mouse", new_project_by_mouse),
    (
        "new-project-in-minimum-window",
        new_project_in_minimum_window,
    ),
    (
        "workbench-tree-tabs-popups-run",
        workbench_tree_tabs_popups_run,
    ),
    (
        "editor-tabs-drag-and-maximize",
        editor_tabs_drag_and_maximize,
    ),
];

pub fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let options = parse_options(args)?;
    if !cfg!(target_os = "linux") {
        bail!("ux-smoke drives X11 input with xdotool and runs on Linux only");
    }
    if Command::new("xdotool")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        bail!("xdotool is required (Debian/Ubuntu: sudo apt install xdotool)");
    }
    let scenes: Vec<_> = SCENES
        .iter()
        .filter(|(name, _)| options.only.is_empty() || options.only.iter().any(|o| o == name))
        .collect();
    if scenes.is_empty() {
        bail!(
            "no scenes selected; known: {}",
            SCENES
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let app = build_app(options.release)?;
    let display = Display::acquire()?;
    let out = std::env::current_dir()?.join(&options.out);
    fs::create_dir_all(&out)?;
    let work = std::env::current_dir()?.join("target/ux-smoke/work");
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    // One fresh home for the whole run: the first scene sees the first-run
    // dialog, the later ones start like a returning user.
    let home = work.join("home");
    fs::create_dir_all(&home)?;
    let mut failures = Vec::new();
    for (name, scene) in scenes {
        print!("ux-smoke {name} … ");
        // Stale PNGs from a previous run must not be mistaken for this run's.
        for entry in fs::read_dir(&out)?.flatten() {
            let file = entry.file_name();
            if file.to_string_lossy().starts_with(&format!("{name}-")) {
                let _ = fs::remove_file(entry.path());
            }
        }
        let project = if matches!(
            *name,
            "workbench-tree-tabs-popups-run" | "editor-tabs-drag-and-maximize"
        ) {
            Some(create_fixture("rust-cargo", &work.join("rust-cargo"))?)
        } else {
            None
        };
        let result = Driver::launch(&app, &display, &work, &home, &out, name, project.as_deref())
            .and_then(|mut driver| {
                let outcome = scene(&mut driver);
                if let Err(error) = &outcome {
                    let _ = driver.screenshot("failed");
                    eprintln!("\n  steps so far:\n    {}", driver.log.join("\n    "));
                    eprintln!("  app log: {}", driver.log_path.display());
                    return Err(anyhow!("{error:#}"));
                }
                driver.screenshot("done")?;
                Ok(())
            });
        match result {
            Ok(()) => println!("ok"),
            Err(error) => {
                println!("FAILED\n    {error:#}");
                failures.push(name.to_string());
            }
        }
    }
    println!("screenshots in {}", options.out.display());
    if !failures.is_empty() {
        bail!(
            "{} ux-smoke scene(s) failed: {}",
            failures.len(),
            failures.join(", ")
        );
    }
    Ok(())
}

/// One running IDE, driven through X input and observed through `state`.
struct Driver {
    display: String,
    window: String,
    client: ControlClient,
    _app: AppProcess,
    out: PathBuf,
    scene: String,
    work: PathBuf,
    log: Vec<String>,
    log_path: PathBuf,
}

impl Driver {
    #[allow(clippy::too_many_arguments)]
    fn launch(
        app: &Path,
        display: &Display,
        work: &Path,
        home: &Path,
        out: &Path,
        scene: &str,
        project: Option<&Path>,
    ) -> Result<Self> {
        let Some(display_name) = display.name.clone() else {
            bail!("ux-smoke needs an X display (DISPLAY or Xvfb)");
        };
        let scene_dir = work.join(scene);
        fs::create_dir_all(&scene_dir)?;
        let socket =
            std::env::temp_dir().join(format!("jt-ux-{}-{scene}.sock", std::process::id()));
        let log_path = scene_dir.join("app.log");
        let log = fs::File::create(&log_path)?;
        let mut command = Command::new(app);
        if let Some(project) = project {
            command.arg(project);
        }
        command
            .env("HOME", home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("XDG_DATA_HOME", home.join(".local/share"))
            .env("XDG_CACHE_HOME", home.join(".cache"))
            .env("XDG_STATE_HOME", home.join(".local/state"))
            .env(jetthing_control::SOCKET_ENV, &socket)
            .env("JETTHING_NO_UPDATE_CHECK", "1")
            .env("DISPLAY", &display_name)
            .stdout(log.try_clone()?)
            .stderr(log);
        if std::env::var_os("JETTHING_RENDERER").is_none() {
            command.env("JETTHING_RENDERER", "glow");
        }
        let mut process = AppProcess(
            command
                .spawn()
                .with_context(|| format!("launch {}", app.display()))?,
        );
        let client = connect(&socket, &mut process.0, &scene_dir)?;
        let mut driver = Self {
            display: display_name,
            window: String::new(),
            client,
            _app: process,
            out: out.to_owned(),
            scene: scene.to_owned(),
            work: work.to_owned(),
            log: Vec::new(),
            log_path,
        };
        driver.window = driver.find_window()?;
        // Without a window manager nothing hands the window keyboard focus.
        driver.xdotool(&["windowfocus", "--sync", &driver.window.clone()])?;
        driver.resize(WINDOW.0, WINDOW.1)?;
        std::thread::sleep(Duration::from_millis(600));
        Ok(driver)
    }

    fn find_window(&self) -> Result<String> {
        let started = Instant::now();
        loop {
            let output = Command::new("xdotool")
                .env("DISPLAY", &self.display)
                .args(["search", "--onlyvisible", "--class", "jetthing"])
                .output()
                .context("run xdotool search")?;
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(id) = text.lines().next().filter(|line| !line.trim().is_empty()) {
                return Ok(id.trim().to_owned());
            }
            if started.elapsed() > Duration::from_secs(20) {
                bail!("the IDE window did not appear on {}", self.display);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    fn xdotool(&self, args: &[&str]) -> Result<()> {
        let status = Command::new("xdotool")
            .env("DISPLAY", &self.display)
            .args(args)
            .stdout(Stdio::null())
            .status()
            .context("run xdotool")?;
        if !status.success() {
            bail!("xdotool {} failed", args.join(" "));
        }
        Ok(())
    }

    fn note(&mut self, text: String) {
        self.log.push(text);
    }

    fn settle(&self) {
        std::thread::sleep(Duration::from_millis(350));
    }

    fn click(&mut self, (x, y): layout::Point, what: &str) -> Result<()> {
        self.note(format!("click {what} at ({x}, {y})"));
        let (x, y) = (x.to_string(), y.to_string());
        self.xdotool(&["mousemove", &x, &y, "sleep", "0.12", "click", "1"])?;
        self.settle();
        Ok(())
    }

    fn double_click(&mut self, (x, y): layout::Point, what: &str) -> Result<()> {
        self.note(format!("double-click {what} at ({x}, {y})"));
        let (x, y) = (x.to_string(), y.to_string());
        self.xdotool(&[
            "mousemove",
            &x,
            &y,
            "sleep",
            "0.12",
            "click",
            "--repeat",
            "2",
            "--delay",
            "80",
            "1",
        ])?;
        self.settle();
        Ok(())
    }

    fn right_click(&mut self, (x, y): layout::Point, what: &str) -> Result<()> {
        self.note(format!("right-click {what} at ({x}, {y})"));
        let (x, y) = (x.to_string(), y.to_string());
        self.xdotool(&["mousemove", &x, &y, "sleep", "0.12", "click", "3"])?;
        self.settle();
        Ok(())
    }

    /// Presses on `from`, moves in steps to `to` and releases: a mouse drag.
    fn drag(
        &mut self,
        (x, y): layout::Point,
        (to_x, to_y): layout::Point,
        what: &str,
    ) -> Result<()> {
        self.note(format!("drag {what} from ({x}, {y}) to ({to_x}, {to_y})"));
        let (x, y) = (x.to_string(), y.to_string());
        self.xdotool(&["mousemove", &x, &y, "sleep", "0.15", "mousedown", "1"])?;
        let steps = 8;
        for step in 1..=steps {
            let px = (x.parse::<i32>()? * (steps - step) + to_x * step) / steps;
            let py = (y.parse::<i32>()? * (steps - step) + to_y * step) / steps;
            self.xdotool(&[
                "mousemove",
                &px.to_string(),
                &py.to_string(),
                "sleep",
                "0.05",
            ])?;
        }
        self.xdotool(&["sleep", "0.2", "mouseup", "1"])?;
        self.settle();
        Ok(())
    }

    fn key(&mut self, chord: &str) -> Result<()> {
        self.note(format!("key {chord}"));
        self.xdotool(&["key", "--delay", "60", chord])?;
        self.settle();
        Ok(())
    }

    fn type_text(&mut self, text: &str) -> Result<()> {
        self.note(format!("type {text:?}"));
        self.xdotool(&["type", "--delay", "30", text])?;
        self.settle();
        Ok(())
    }

    /// Selects a field's content and replaces it.
    fn replace_text(&mut self, field: layout::Point, what: &str, text: &str) -> Result<()> {
        self.click(field, what)?;
        self.key("ctrl+a")?;
        self.type_text(text)
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.note(format!("resize window to {width}×{height}"));
        let window = self.window.clone();
        self.xdotool(&["windowmove", &window, "0", "0"])?;
        self.xdotool(&[
            "windowsize",
            &window,
            &width.to_string(),
            &height.to_string(),
        ])?;
        self.wait("/window/width", &format!("window width {width}"), |value| {
            value
                .as_f64()
                .is_some_and(|w| (w - width as f64).abs() < 2.0)
        })
    }

    fn state(&mut self) -> Result<Value> {
        let Response { ok, result, error } = self.client.send(&Request::State)?;
        if !ok {
            bail!("{}", error.unwrap_or_else(|| "state request failed".into()));
        }
        Ok(result.unwrap_or(Value::Null))
    }

    /// Polls `state` until `pointer` satisfies `accept` or [`STATE_TIMEOUT`] passes.
    fn wait(&mut self, pointer: &str, what: &str, accept: impl Fn(&Value) -> bool) -> Result<()> {
        self.wait_for(pointer, what, STATE_TIMEOUT, accept)
    }

    fn wait_for(
        &mut self,
        pointer: &str,
        what: &str,
        timeout: Duration,
        accept: impl Fn(&Value) -> bool,
    ) -> Result<()> {
        let started = Instant::now();
        loop {
            let state = self.state()?;
            let current = state.pointer(pointer).cloned().unwrap_or(Value::Null);
            if accept(&current) {
                self.note(format!("ok: {what}"));
                return Ok(());
            }
            if started.elapsed() > timeout {
                bail!("expected {what}, but state{pointer} is {current} after {timeout:?}");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn expect(&mut self, pointer: &str, expected: Value, what: &str) -> Result<()> {
        self.wait(pointer, what, |value| *value == expected)
    }

    fn screenshot(&mut self, name: &str) -> Result<()> {
        let path = self.out.join(format!("{}-{name}.png", self.scene));
        let Response { ok, error, .. } = self.client.send(&Request::Screenshot { path })?;
        if !ok {
            bail!("{}", error.unwrap_or_else(|| "screenshot failed".into()));
        }
        Ok(())
    }
}

// ---- scenes -----------------------------------------------------------------

/// Scenes after the first start as a returning user; with `--only` the home is
/// still fresh, so the modal first-run dialog has to be dismissed first.
fn dismiss_first_run(d: &mut Driver) -> Result<()> {
    if d.state()?["popups"]["customize"] == Value::Bool(true) {
        d.click(layout::FIRST_RUN_SKIP, "Skip (first-run dialog)")?;
        d.expect(
            "/popups/customize",
            Value::Bool(false),
            "first-run dialog closed",
        )?;
    }
    Ok(())
}

/// First start: the Customize dialog is modal. Clicking New Project behind it
/// must not stack a second dialog; Skip dismisses it.
fn first_run_dialog_is_modal(d: &mut Driver) -> Result<()> {
    d.expect(
        "/popups/customize",
        Value::Bool(true),
        "first-run dialog shown",
    )?;
    d.screenshot("first-run")?;
    d.click(
        layout::WELCOME_NEW_PROJECT,
        "New Project tile behind the dialog",
    )?;
    d.expect(
        "/popups/dialog",
        Value::Null,
        "no New Project dialog opened behind the modal first-run dialog",
    )?;
    d.click(layout::FIRST_RUN_SKIP, "Skip")?;
    d.expect(
        "/popups/customize",
        Value::Bool(false),
        "first-run dialog closed",
    )?;
    Ok(())
}

/// Welcome → New Project → Rust Application, every field and the Create button
/// reached with the mouse; the project opens and its files exist.
fn new_project_by_mouse(d: &mut Driver) -> Result<()> {
    dismiss_first_run(d)?;
    d.click(layout::WELCOME_NEW_PROJECT, "New Project tile")?;
    d.expect(
        "/popups/dialog",
        "newProject".into(),
        "New Project dialog open",
    )?;
    d.screenshot("dialog")?;
    d.click(layout::NP_GENERATOR_RUST_APP, "Rust Application generator")?;
    d.replace_text(layout::NP_NAME, "Name field", "smoke")?;
    let location = d.work.join("projects");
    d.replace_text(
        layout::NP_LOCATION,
        "Location field",
        &location.to_string_lossy(),
    )?;
    d.click(layout::NP_CREATE, "Create button")?;
    d.wait("/project", "project created and opened", |value| {
        value.as_str().is_some_and(|root| root.ends_with("smoke"))
    })?;
    let cargo = location.join("smoke/Cargo.toml");
    if !cargo.is_file() {
        bail!("{} was not generated", cargo.display());
    }
    d.expect("/popups/dialog", Value::Null, "dialog closed after Create")?;
    Ok(())
}

/// The v0.0.7 regression: in the smallest allowed window the Create button
/// must still be on screen and clickable.
fn new_project_in_minimum_window(d: &mut Driver) -> Result<()> {
    dismiss_first_run(d)?;
    d.resize(SMALL_WINDOW.0, SMALL_WINDOW.1)?;
    d.click(layout::WELCOME_NEW_PROJECT, "New Project tile")?;
    d.expect(
        "/popups/dialog",
        "newProject".into(),
        "New Project dialog open",
    )?;
    d.screenshot("dialog-800x500")?;
    d.click(layout::NP_SMALL_GENERATOR_EMPTY, "Empty Project generator")?;
    d.replace_text(layout::NP_SMALL_NAME, "Name field", "tiny")?;
    let location = d.work.join("projects");
    d.replace_text(
        layout::NP_SMALL_LOCATION,
        "Location field",
        &location.to_string_lossy(),
    )?;
    d.click(layout::NP_SMALL_CREATE, "Create button (800×500 layout)")?;
    d.wait(
        "/project",
        "project created from the 800×500 dialog",
        |value| value.as_str().is_some_and(|root| root.ends_with("tiny")),
    )?;
    if !location.join("tiny/README.md").is_file() {
        bail!("README.md was not generated");
    }
    Ok(())
}

/// Project tree, tabs, popups, tool windows, menu bar and Run on a Cargo project.
fn workbench_tree_tabs_popups_run(d: &mut Driver) -> Result<()> {
    d.wait("/project", "project opened", |value| !value.is_null())?;
    dismiss_first_run(d)?;
    d.click(layout::TREE_SRC_CHEVRON, "src chevron")?;
    d.wait("/projectTree/expanded", "src expanded", |value| {
        value
            .as_array()
            .is_some_and(|dirs| dirs.iter().any(|dir| dir == "src"))
    })?;
    d.double_click(layout::TREE_MAIN_RS, "src/main.rs row")?;
    d.expect(
        "/activeTab",
        "src/main.rs".into(),
        "main.rs opened in a tab",
    )?;
    d.expect("/focus", "editor".into(), "editor focused after opening")?;
    d.right_click(layout::TREE_MAIN_RS, "src/main.rs row")?;
    d.expect("/popups/menu", Value::Bool(true), "tree context menu open")?;
    d.screenshot("tree-context-menu")?;
    d.key("Escape")?;
    d.expect(
        "/popups/menu",
        Value::Bool(false),
        "context menu closed by Escape",
    )?;
    d.click(layout::TOOLBAR_SEARCH, "Search Everywhere toolbar button")?;
    d.wait(
        "/popups/searchEverywhere",
        "Search Everywhere open",
        |value| !value.is_null(),
    )?;
    d.type_text("main")?;
    d.screenshot("search-everywhere")?;
    d.key("Escape")?;
    d.expect(
        "/popups/searchEverywhere",
        Value::Null,
        "Search Everywhere closed",
    )?;
    d.click(layout::FIRST_TAB_CLOSE, "close button of the main.rs tab")?;
    d.wait("/tabs", "tab closed", |value| {
        value.as_array().is_some_and(Vec::is_empty)
    })?;
    d.double_click(layout::TREE_MAIN_RS, "src/main.rs row")?;
    d.expect("/activeTab", "src/main.rs".into(), "main.rs reopened")?;
    d.click(
        layout::PROJECT_HIDE_BUTTON,
        "Project tool window hide button",
    )?;
    d.expect(
        "/toolWindows/left",
        Value::Null,
        "Project tool window hidden",
    )?;
    d.click(layout::PROJECT_STRIPE_BUTTON, "Project stripe button")?;
    d.expect(
        "/toolWindows/left",
        "project".into(),
        "Project tool window shown",
    )?;
    d.click(layout::HAMBURGER, "main menu button")?;
    d.click(layout::MENU_FILE, "File menu title")?;
    d.expect("/popups/menu", Value::Bool(true), "File menu open")?;
    d.screenshot("file-menu")?;
    d.click(layout::MENU_FILE_SETTINGS, "File › Settings…")?;
    d.expect(
        "/popups/settings",
        Value::Bool(true),
        "Settings dialog open",
    )?;
    d.screenshot("settings")?;
    d.key("Escape")?;
    d.expect(
        "/popups/settings",
        Value::Bool(false),
        "Settings closed by Escape",
    )?;
    d.click(layout::TOOLBAR_RUN, "Run toolbar button")?;
    d.wait("/run/task", "a run task started", |value| !value.is_null())?;
    d.wait_for(
        "/status",
        "cargo run finished",
        Duration::from_secs(180),
        |value| {
            value
                .as_str()
                .is_some_and(|status| status.contains("finished successfully"))
        },
    )?;
    d.screenshot("run")?;
    Ok(())
}

/// Two files open: dragging the second tab before the first reorders the bar (UX-011),
/// a double click on a tab maximises the editor and again restores it (UX-012).
fn editor_tabs_drag_and_maximize(d: &mut Driver) -> Result<()> {
    d.wait("/project", "project opened", |value| !value.is_null())?;
    dismiss_first_run(d)?;
    d.click(layout::TREE_SRC_CHEVRON, "src chevron")?;
    d.wait("/projectTree/expanded", "src expanded", |value| {
        value
            .as_array()
            .is_some_and(|dirs| dirs.iter().any(|dir| dir == "src"))
    })?;
    d.double_click(layout::TREE_BROKEN_RS, "src/broken.rs row")?;
    d.double_click(layout::TREE_MAIN_RS, "src/main.rs row")?;
    d.wait("/tabs", "two tabs open", |value| {
        value.as_array().is_some_and(|tabs| tabs.len() == 2)
    })?;
    let order = |d: &mut Driver| -> Result<Vec<String>> {
        Ok(d.state()?["tabs"]
            .as_array()
            .map(|tabs| {
                tabs.iter()
                    .filter_map(|tab| tab["path"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default())
    };
    if order(d)? != ["src/broken.rs", "src/main.rs"] {
        bail!("unexpected tab order before the drag: {:?}", order(d)?);
    }
    d.drag(
        layout::TAB_SECOND,
        layout::TAB_FIRST_LEFT_HALF,
        "the main.rs tab",
    )?;
    d.expect(
        "/tabs/0/path",
        "src/main.rs".into(),
        "main.rs is now the first tab",
    )?;
    d.expect(
        "/activeTab",
        "src/main.rs".into(),
        "the dragged tab stays the active one",
    )?;
    d.screenshot("tabs-reordered")?;
    d.double_click(layout::TAB_FIRST_LEFT_HALF, "a tab")?;
    d.expect("/toolWindows/left", Value::Null, "editor maximised")?;
    d.double_click(layout::TAB_FIRST_MAXIMISED, "a tab")?;
    d.expect("/toolWindows/left", "project".into(), "editor restored")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse_and_scene_names_are_unique() {
        let options =
            parse_options(["--only", "a,b", "--release"].map(String::from).into_iter()).unwrap();
        assert_eq!(options.only, vec!["a", "b"]);
        assert!(options.release);
        assert!(parse_options(["--bogus"].map(String::from).into_iter()).is_err());
        let mut names: Vec<&str> = SCENES.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SCENES.len());
    }

    #[test]
    fn layout_points_are_inside_their_windows() {
        for point in [
            layout::WELCOME_NEW_PROJECT,
            layout::FIRST_RUN_SKIP,
            layout::NP_CREATE,
            layout::TOOLBAR_RUN,
            layout::MENU_FILE_SETTINGS,
        ] {
            assert!(point.0 > 0 && point.0 < WINDOW.0 as i32);
            assert!(point.1 > 0 && point.1 < WINDOW.1 as i32);
        }
        for point in [layout::NP_SMALL_CREATE, layout::NP_SMALL_NAME] {
            assert!(point.0 < SMALL_WINDOW.0 as i32 && point.1 < SMALL_WINDOW.1 as i32);
        }
    }
}
