//! A driver that runs the terminal IDE against ratatui's `TestBackend`: the same event and
//! draw calls the real loop makes, a manual clock for the timers and real time for the
//! background work of the core (the daemon, the runner, the search).
#![allow(dead_code)]

use archipel_ide_tui::{StartOptions, TuiApp};
use archipel_kit::action_system::Os;
use archipel_kit_tui::input::{self, Caps, Event, KeyCode, KeyModifiers};
use archipel_kit_tui::ratatui::Terminal;
use archipel_kit_tui::ratatui::backend::TestBackend;
use archipel_kit_tui::runtime::Runtime;
use archipel_kit_tui::{App, Look};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct Tui {
    pub app: TuiApp,
    pub rt: Runtime,
    terminal: Terminal<TestBackend>,
}

/// A fixture project in a fresh temp directory.
pub struct Project {
    pub root: PathBuf,
}

impl Project {
    pub fn new(name: &str, files: &[(&str, &str)]) -> Self {
        let root = std::env::temp_dir().join(format!("archipel-tui-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let target = root.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, text).unwrap();
        }
        Self {
            root: std::fs::canonicalize(&root).unwrap(),
        }
    }

    pub fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.root.join(path)).unwrap_or_default()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Tui {
    pub fn open(project: &Path, files: &[&str], width: u16, height: u16) -> Self {
        let app = TuiApp::new(StartOptions {
            project: Some(project.to_path_buf()),
            files: files.iter().map(PathBuf::from).collect(),
            isolated: true,
            ..StartOptions::default()
        });
        Self::with_app(app, width, height)
    }

    /// An IDE whose language servers are the scriptable fake server (`Fake` files, `.fake`).
    /// `None` when the server binary is not built (it is with `cargo test --workspace`).
    pub fn with_fake_lsp(project: &Path, files: &[&str], width: u16, height: u16) -> Option<Self> {
        use archipel_ide_lsp::{
            LanguageServerProvider, LaunchConfig, LaunchContext, LspError, LspManager,
            ServerRegistry, ServerSpec,
        };
        struct Fake(PathBuf, ServerSpec);
        impl LanguageServerProvider for Fake {
            fn spec(&self) -> &ServerSpec {
                &self.1
            }
            fn launch(&self, _: &LaunchContext<'_>) -> Result<LaunchConfig, LspError> {
                Ok(LaunchConfig::new("fake", &self.0))
            }
        }
        let exe = std::env::current_exe().ok()?;
        let server = exe
            .parent()?
            .parent()?
            .join(format!("archipel-fake-lsp{}", std::env::consts::EXE_SUFFIX));
        if !server.is_file() {
            eprintln!("archipel-fake-lsp is not built; run `cargo test --workspace`");
            return None;
        }
        let mut registry = ServerRegistry::empty();
        registry.register(std::sync::Arc::new(Fake(
            server,
            ServerSpec {
                id: "fake",
                language: "Fake",
                extensions: &[("fake", "fake")],
                file_names: &[],
                candidates: &[],
                root_markers: &[],
                install_hint: "",
                notify_missing: false,
                initialization_options: None,
            },
        )));
        let mut ide = archipel_ide_tui::Ide::start(StartOptions {
            isolated: true,
            ..StartOptions::default()
        });
        *ide.state_mut().lsp_mut() = LspManager::new(registry);
        ide.open_project(project).unwrap();
        let mut app = TuiApp::with_ide(ide);
        for file in files {
            let _ = app
                .ide_mut()
                .dispatch("editor.openFile", serde_json::json!({ "path": file }));
        }
        Some(Self::with_app(app, width, height))
    }

    pub fn with_app(app: TuiApp, width: u16, height: u16) -> Self {
        let keymap = app.keymap(Os::Linux);
        let rt = Runtime::with(Look::dark(), keymap, Caps::kitty());
        let terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut tui = Self { app, rt, terminal };
        tui.draw();
        tui
    }

    pub fn draw(&mut self) {
        let Self { app, rt, terminal } = self;
        terminal
            .draw(|frame| {
                let area = frame.area();
                rt.begin_frame(area);
                app.draw(rt, area, frame.buffer_mut());
                rt.end_frame();
            })
            .unwrap();
    }

    pub fn send(&mut self, event: Event) -> &mut Self {
        self.rt.preprocess(&event);
        self.app.event(&mut self.rt, &event);
        self.draw();
        self
    }

    /// Delivers `event` without drawing a frame after it (two strokes of a chord that arrive
    /// within one frame).
    pub fn send_undrawn(&mut self, event: Event) -> &mut Self {
        self.rt.preprocess(&event);
        self.app.event(&mut self.rt, &event);
        self
    }

    pub fn press(&mut self, code: KeyCode) -> &mut Self {
        self.send(input::key(code))
    }

    pub fn press_with(&mut self, code: KeyCode, mods: KeyModifiers) -> &mut Self {
        self.send(input::key_with(code, mods))
    }

    pub fn ctrl(&mut self, c: char) -> &mut Self {
        self.send(input::ctrl(c))
    }

    pub fn alt(&mut self, c: char) -> &mut Self {
        self.send(input::alt(c))
    }

    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for event in input::typing(text) {
            self.send(event);
        }
        self
    }

    /// A left click: press and release at (`x`, `y`).
    pub fn click(&mut self, x: u16, y: u16) -> &mut Self {
        use archipel_kit_tui::input::{MouseButton, MouseEvent, MouseEventKind};
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            self.send(Event::Mouse(MouseEvent {
                kind,
                column: x,
                row: y,
                modifiers: KeyModifiers::NONE,
            }));
        }
        self
    }

    /// Lets time pass: the manual clock advances and the app ticks (polling the core).
    pub fn advance(&mut self, seconds: f64) -> &mut Self {
        self.rt.now += seconds;
        self.rt.tick();
        self.app.tick(&mut self.rt);
        self.draw();
        self
    }

    /// The text of each screen row, trailing spaces trimmed.
    pub fn screen(&self) -> Vec<String> {
        let buffer = self.terminal.backend().buffer();
        let width = buffer.area.width;
        (0..buffer.area.height)
            .map(|y| {
                let mut line = String::new();
                for x in 0..width {
                    line.push_str(buffer[(x, y)].symbol());
                }
                line.trim_end().to_owned()
            })
            .collect()
    }

    pub fn text(&self) -> String {
        self.screen().join("\n")
    }

    /// The cell where `needle` first appears on the screen.
    pub fn find(&self, needle: &str) -> Option<(u16, u16)> {
        for (y, line) in self.screen().iter().enumerate() {
            if let Some(byte) = line.find(needle) {
                let x = unicode_width::UnicodeWidthStr::width(&line[..byte]);
                return Some((x as u16, y as u16));
            }
        }
        None
    }

    /// The foreground color of the cell at (`x`, `y`).
    pub fn fg_at(&self, x: u16, y: u16) -> archipel_kit_tui::ratatui::style::Color {
        self.terminal.backend().buffer()[(x, y)].fg
    }

    pub fn contains(&self, needle: &str) -> bool {
        self.text().contains(needle)
    }

    /// Ticks until `done` holds or `seconds` of real time pass.
    pub fn wait_for(&mut self, seconds: u64, mut done: impl FnMut(&mut Self) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(seconds);
        loop {
            self.advance(0.05);
            if done(self) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Panics with the screen when `done` does not hold in time.
    pub fn expect(&mut self, what: &str, seconds: u64, done: impl FnMut(&mut Self) -> bool) {
        if !self.wait_for(seconds, done) {
            panic!("timed out waiting for {what}\n{}", self.text());
        }
    }
}
