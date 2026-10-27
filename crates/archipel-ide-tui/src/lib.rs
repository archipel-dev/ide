//! Terminal frontend of Archipel IDE, built on `archipel-kit-tui`.
//!
//! The IDE core (`archipel-ide-core`, the platform kernel, the action registry) is
//! UI-independent; this crate is a second frontend next to `archipel-ide-ui`. It owns no
//! project, document or run state: it asks the core through commands and actions
//! ([`ide::Ide`]), mirrors the editor buffers ([`view::DocView`]) and draws what the core's
//! services answer. See `docs/tui.md`.

pub mod actions;
pub mod app;
pub mod dialogs;
pub mod draw;
pub mod ide;
pub mod keys;
pub mod layout;
pub mod log;
pub mod nav;
pub mod panels;
pub mod picker;
pub mod popups;
pub mod terminal;
pub mod view;

pub use app::TuiApp;
pub use ide::{Ide, StartOptions};

use archipel_kit::action_system::Os;
use archipel_kit_tui::Look;
use archipel_kit_tui::runtime::{self, RunOptions};

/// Runs the IDE in the terminal until the user quits.
///
/// # Errors
///
/// When the terminal cannot be set up or fails.
pub fn run(options: StartOptions) -> std::io::Result<()> {
    let mut app = TuiApp::new(options);
    let keymap = app.keymap(Os::current());
    let run_options = RunOptions {
        report_modifier_keys: true,
        ..RunOptions::default()
    };
    let result = runtime::run_app_with_keymap(Look::dark(), &mut app, run_options, move |_| keymap);
    app.ide_mut().shutdown();
    result
}
