// Release builds are GUI apps on Windows (no console window).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod plugins;

use jetthing_ui::diagnostics;
use std::path::PathBuf;

enum Mode {
    Ide {
        project: Option<PathBuf>,
        files: Vec<PathBuf>,
    },
    /// `--app <plugin-id>`: only the plugin's app view.
    App(String),
}

/// Window chrome per platform, from the kit (`jetthing_ui::chrome::window_chrome`): macOS
/// keeps its traffic lights in a transparent full-size title area that the main toolbar
/// shares; other platforms are undecorated and the toolbar draws the window controls.
fn platform_viewport(viewport: eframe::egui::ViewportBuilder) -> eframe::egui::ViewportBuilder {
    jetthing_ui::window_chrome().viewport(viewport)
}

fn native_options(renderer: eframe::Renderer, title: &str) -> eframe::NativeOptions {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(title)
        .with_app_id("jetthing")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([800.0, 500.0]);
    // Taskbar / dock icon from the original artwork in `assets/icon`.
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon/jetthing.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    viewport = platform_viewport(viewport);
    eframe::NativeOptions {
        viewport,
        renderer,
        ..Default::default()
    }
}

fn run(renderer: eframe::Renderer, title: &str, app: jetthing_ui::IdeApp) -> eframe::Result<()> {
    eframe::run_native(
        "Jetthing",
        native_options(renderer, title),
        Box::new(move |cc| {
            cc.egui_ctx.set_theme(eframe::egui::Theme::Dark);
            jetthing_ui::install_theme(&cc.egui_ctx);
            let mut app = app;
            app.start_control_from_env(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

/// Turns winit's panic about a missing system library (e.g. `Library
/// libxkbcommon-x11.so could not be loaded.`) into install instructions.
fn missing_library_hint(message: &str) -> Option<String> {
    let rest = message.split("Library ").nth(1)?;
    let (library, _) = rest.split_once(" could not be loaded")?;
    let library = library.trim();
    // Debian/Ubuntu and Fedora package names for the libraries winit loads at runtime.
    let packages = [
        ("libxkbcommon-x11", "libxkbcommon-x11-0", "libxkbcommon-x11"),
        ("libxkbcommon", "libxkbcommon0", "libxkbcommon"),
        (
            "libwayland-client",
            "libwayland-client0",
            "libwayland-client",
        ),
        (
            "libwayland-cursor",
            "libwayland-cursor0",
            "libwayland-cursor",
        ),
        ("libX11-xcb", "libx11-xcb1", "libX11-xcb"),
        ("libX11", "libx11-6", "libX11"),
        ("libXcursor", "libxcursor1", "libXcursor"),
        ("libXi", "libxi6", "libXi"),
        ("libXrandr", "libxrandr2", "libXrandr"),
        ("libxcb", "libxcb1", "libxcb"),
    ];
    let install = packages
        .iter()
        .find(|(prefix, _, _)| library.starts_with(&format!("{prefix}.")))
        .map(|(_, apt, dnf)| {
            format!(
                "\n  Debian/Ubuntu: sudo apt install {apt}\n  Fedora:        sudo dnf install {dnf}"
            )
        })
        .unwrap_or_default();
    Some(format!(
        "jetthing: cannot start: the system library {library} is missing.\nInstall it with your package manager and start Jetthing again.{install}"
    ))
}

/// Missing desktop libraries make winit panic during startup; report them
/// as an actionable error instead of a backtrace. Any other panic leaves a
/// crash report in the log directory, offered to the user on the next start.
fn install_startup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or_default();
        match missing_library_hint(message) {
            Some(hint) => {
                diagnostics::log(diagnostics::Level::Error, &hint);
                eprintln!("{hint}");
                std::process::exit(1);
            }
            None => {
                let location = info.location().map(ToString::to_string);
                match diagnostics::write_crash_report(message, location.as_deref()) {
                    Ok(path) => eprintln!("jetthing: crash report written to {}", path.display()),
                    Err(error) => eprintln!("jetthing: could not write a crash report: {error}"),
                }
                default_hook(info)
            }
        }
    }));
}

fn parse_args() -> Result<Mode, String> {
    let mut args = std::env::args_os().skip(1);
    let mut positional = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--app" {
            let id = args
                .next()
                .and_then(|id| id.into_string().ok())
                .ok_or("--app needs a plugin id, e.g. --app dev.jetthing.hello")?;
            return Ok(Mode::App(id));
        }
        if let Some(id) = arg.to_str().and_then(|arg| arg.strip_prefix("--app=")) {
            return Ok(Mode::App(id.to_owned()));
        }
        positional.push(PathBuf::from(arg));
    }
    let mut positional = positional.into_iter();
    Ok(Mode::Ide {
        project: positional.next(),
        files: positional.collect(),
    })
}

fn build_app(mode: &Mode) -> Result<jetthing_ui::IdeApp, String> {
    match mode {
        Mode::Ide { project, files } => Ok(jetthing_ui::IdeApp::with_plugin_set(
            project.clone(),
            files.clone(),
            plugins::plugin_set(),
        )),
        Mode::App(id) => jetthing_ui::IdeApp::standalone(id, plugins::plugin_set()),
    }
}

const USAGE: &str = "usage: jetthing [PROJECT_DIR] [FILE...]
       jetthing --app <PLUGIN_ID>

Opens PROJECT_DIR (or the welcome screen) and FILEs relative to it.
  --app ID        run only the plugin's app view
  -V, --version   print version and build information
  -h, --help      print this help
Environment: JETTHING_RENDERER=glow, JETTHING_NO_UPDATE_CHECK,
JETTHING_CONTROL_SOCKET. See README.md.";

/// Release builds on Windows have no console of their own; print `--version`
/// into the terminal that started them.
#[cfg(windows)]
fn attach_parent_console() {
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    // SAFETY: no pointers are passed; failure only means there is no console.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}

fn main() -> eframe::Result<()> {
    let first = std::env::args_os().nth(1);
    if let Some(text) = jetthing_ui::build_info::cli_flag_output(
        "jetthing",
        USAGE,
        first.as_deref().and_then(|arg| arg.to_str()),
    ) {
        attach_parent_console();
        println!("{text}");
        return Ok(());
    }
    if let Err(error) = diagnostics::init() {
        eprintln!("jetthing: logging disabled: {error}");
    }
    install_startup_panic_hook();
    // Debug builds only: exercise the crash-report path end to end.
    if cfg!(debug_assertions) && std::env::var_os("JETTHING_DEBUG_PANIC").is_some() {
        panic!("JETTHING_DEBUG_PANIC requested a test crash");
    }
    // Usage: jetthing [PROJECT_DIR] [FILE...]  |  jetthing --app <PLUGIN_ID>
    let mode = match parse_args() {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("jetthing: {error}");
            std::process::exit(2);
        }
    };
    let app = match build_app(&mode) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("jetthing: {error}");
            std::process::exit(2);
        }
    };
    let project = match &mode {
        Mode::Ide { project, .. } => project.clone(),
        Mode::App(_) => None,
    };
    let title = app.app_title().unwrap_or_else(|| title_for(project));
    let forced_gl = std::env::var("JETTHING_RENDERER").is_ok_and(|value| value == "glow");
    if forced_gl {
        return run(eframe::Renderer::Glow, &title, app);
    }
    match run(eframe::Renderer::Wgpu, &title, app) {
        // No usable wgpu adapter/surface (VMs, old drivers): fall back to OpenGL.
        // The first app was consumed by the failed attempt; build a fresh one.
        Err(eframe::Error::Wgpu(error)) => {
            let message = format!("wgpu unavailable ({error}); falling back to OpenGL");
            diagnostics::log(diagnostics::Level::Warn, &message);
            eprintln!("jetthing: {message}");
            match build_app(&mode) {
                Ok(app) => run(eframe::Renderer::Glow, &title, app),
                Err(error) => {
                    eprintln!("jetthing: {error}");
                    std::process::exit(2);
                }
            }
        }
        other => other,
    }
}

fn title_for(project: Option<PathBuf>) -> String {
    project
        .as_deref()
        .and_then(|path| path.canonicalize().ok())
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .map(|name| format!("{name} – Jetthing"))
        .unwrap_or_else(|| "Jetthing".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_xkbcommon_gets_install_instructions() {
        let hint =
            missing_library_hint("Library libxkbcommon-x11.so could not be loaded.").unwrap();
        assert!(hint.contains("libxkbcommon-x11.so is missing"), "{hint}");
        assert!(hint.contains("apt install libxkbcommon-x11-0"), "{hint}");
        assert!(hint.contains("dnf install libxkbcommon-x11"), "{hint}");
        let generic = missing_library_hint("Library libfoo.so.3 could not be loaded").unwrap();
        assert!(generic.contains("libfoo.so.3 is missing"));
        assert!(!generic.contains("apt install"));
        assert_eq!(missing_library_hint("index out of bounds"), None);
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;

    #[test]
    fn the_window_chrome_comes_from_the_kit() {
        let viewport = platform_viewport(Default::default());
        if cfg!(target_os = "macos") {
            assert_eq!(viewport.fullsize_content_view, Some(true));
            assert_eq!(viewport.titlebar_shown, Some(false));
            assert_eq!(viewport.title_shown, Some(false));
        } else {
            // Undecorated: the main toolbar draws minimize, maximize and close.
            assert_eq!(viewport.decorations, Some(false));
        }
    }
}
