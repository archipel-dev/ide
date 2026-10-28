//! Icon keys of the chrome: which glyph stands for a tool window.

use archipel_ide_core::ToolWindow;
use archipel_kit_model::{Glyph, IconKey};

/// The glyph of a tool window's stripe button.
pub fn tool_window_glyph(window: ToolWindow) -> Glyph {
    match window {
        ToolWindow::Project => Glyph::Project,
        ToolWindow::Commit => Glyph::Commit,
        ToolWindow::Structure | ToolWindow::Hierarchy => Glyph::Structure,
        ToolWindow::Agents => Glyph::Agent,
        ToolWindow::ResourceManager | ToolWindow::BuildVariants => Glyph::Layers,
        ToolWindow::Git => Glyph::Branch,
        ToolWindow::Logcat => Glyph::Log,
        ToolWindow::Problems => Glyph::Problems,
        ToolWindow::Todo => Glyph::Success,
        ToolWindow::Terminal => Glyph::Terminal,
        ToolWindow::Build => Glyph::Build,
        ToolWindow::Debug => Glyph::Debug,
        ToolWindow::Find => Glyph::Search,
        ToolWindow::Gradle | ToolWindow::Bazel => Glyph::Package,
        ToolWindow::DeviceManager | ToolWindow::RunningDevices => Glyph::Device,
        ToolWindow::AiChat => Glyph::Sparkle,
    }
}

/// [`tool_window_glyph`] as an icon key.
pub fn tool_window_icon(window: ToolWindow) -> IconKey {
    IconKey::Glyph(tool_window_glyph(window))
}

/// The action that activates `window` in the keymaps, when the reference IDE has one.
pub fn activation_action(window: ToolWindow) -> Option<&'static str> {
    Some(match window {
        ToolWindow::Project => "ActivateProjectToolWindow",
        ToolWindow::Commit => "ActivateCommitToolWindow",
        ToolWindow::Structure => "ActivateStructureToolWindow",
        ToolWindow::Find => "ActivateFindToolWindow",
        ToolWindow::Build => "ActivateRunToolWindow",
        ToolWindow::Problems => "ActivateProblemsViewToolWindow",
        ToolWindow::Terminal => "ActivateTerminalToolWindow",
        ToolWindow::Git => "ActivateVersionControlToolWindow",
        ToolWindow::Debug => "ActivateDebugToolWindow",
        _ => return None,
    })
}
