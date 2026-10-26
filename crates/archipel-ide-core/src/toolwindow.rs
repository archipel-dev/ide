//! Tool window placement model.
//!
//! Mirrors the observable New UI behaviour: tool windows are docked to a side
//! (anchor), and at most one tool window per anchor is visible at a time.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    Left,
    Bottom,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolWindow {
    Project,
    Commit,
    Structure,
    Find,
    Build,
    Problems,
    Terminal,
    DeviceManager,
    Logcat,
    Git,
    BuildVariants,
    ResourceManager,
    RunningDevices,
    Gradle,
    Bazel,
    AiChat,
    Debug,
    Hierarchy,
    Todo,
}

impl ToolWindow {
    pub const ALL: [ToolWindow; 19] = [
        ToolWindow::Project,
        ToolWindow::Commit,
        ToolWindow::Structure,
        ToolWindow::Find,
        ToolWindow::Build,
        ToolWindow::Problems,
        ToolWindow::Terminal,
        ToolWindow::DeviceManager,
        ToolWindow::Logcat,
        ToolWindow::Git,
        ToolWindow::BuildVariants,
        ToolWindow::ResourceManager,
        ToolWindow::RunningDevices,
        ToolWindow::Gradle,
        ToolWindow::Bazel,
        ToolWindow::AiChat,
        ToolWindow::Debug,
        ToolWindow::Hierarchy,
        ToolWindow::Todo,
    ];

    pub fn id(self) -> &'static str {
        match self {
            ToolWindow::Project => "project",
            ToolWindow::Commit => "commit",
            ToolWindow::Structure => "structure",
            ToolWindow::Find => "find",
            ToolWindow::Build => "build",
            ToolWindow::Problems => "problems",
            ToolWindow::Terminal => "terminal",
            ToolWindow::DeviceManager => "deviceManager",
            ToolWindow::Logcat => "logcat",
            ToolWindow::Git => "git",
            ToolWindow::BuildVariants => "buildVariants",
            ToolWindow::ResourceManager => "resourceManager",
            ToolWindow::RunningDevices => "runningDevices",
            ToolWindow::AiChat => "aiChat",
            ToolWindow::Debug => "debug",
            ToolWindow::Hierarchy => "hierarchy",
            ToolWindow::Todo => "todo",
            ToolWindow::Gradle => "gradle",
            ToolWindow::Bazel => "bazel",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            ToolWindow::Project => "Project",
            ToolWindow::Commit => "Commit",
            ToolWindow::Structure => "Structure",
            ToolWindow::Find => "Find",
            ToolWindow::Build => "Build",
            ToolWindow::Problems => "Problems",
            ToolWindow::Terminal => "Terminal",
            ToolWindow::DeviceManager => "Device Manager",
            ToolWindow::Logcat => "Logcat",
            ToolWindow::Git => "Git",
            ToolWindow::BuildVariants => "Build Variants",
            ToolWindow::ResourceManager => "Resource Manager",
            ToolWindow::RunningDevices => "Running Devices",
            ToolWindow::AiChat => "AI Chat",
            ToolWindow::Debug => "Debug",
            ToolWindow::Hierarchy => "Hierarchy",
            ToolWindow::Todo => "TODO",
            ToolWindow::Gradle => "Gradle",
            ToolWindow::Bazel => "Bazel",
        }
    }

    /// Default activation shortcut (macOS notation, see `docs/spec/toolwindows.md`);
    /// empty when the reference assigns none.
    pub fn shortcut(self) -> &'static str {
        match self {
            ToolWindow::Project => "⌘1",
            ToolWindow::Commit => "⌘0",
            ToolWindow::Structure => "⌘7",
            ToolWindow::Find => "⌘3",
            ToolWindow::Build => "⌘4",
            ToolWindow::Problems => "⌘6",
            ToolWindow::Terminal => "⌥F12",
            // Unbound by default, like the reference IDE (which also assigns no
            // default shortcut to its AI chat window).
            ToolWindow::DeviceManager
            | ToolWindow::Logcat
            | ToolWindow::BuildVariants
            | ToolWindow::ResourceManager
            | ToolWindow::RunningDevices
            | ToolWindow::Gradle
            | ToolWindow::Bazel
            | ToolWindow::Hierarchy
            | ToolWindow::Todo
            | ToolWindow::AiChat => "",
            ToolWindow::Debug => "⌘5",
            ToolWindow::Git => "⌘9",
        }
    }

    pub fn anchor(self) -> Anchor {
        match self {
            ToolWindow::Project
            | ToolWindow::Commit
            | ToolWindow::Structure
            | ToolWindow::BuildVariants
            | ToolWindow::ResourceManager => Anchor::Left,
            ToolWindow::Find
            | ToolWindow::Build
            | ToolWindow::Problems
            | ToolWindow::Terminal
            | ToolWindow::Logcat
            | ToolWindow::Debug
            | ToolWindow::Hierarchy
            | ToolWindow::Todo
            | ToolWindow::Git => Anchor::Bottom,
            ToolWindow::DeviceManager
            | ToolWindow::RunningDevices
            | ToolWindow::Gradle
            | ToolWindow::Bazel
            | ToolWindow::AiChat => Anchor::Right,
        }
    }

    pub fn parse(id: &str) -> Option<ToolWindow> {
        ToolWindow::ALL.into_iter().find(|window| window.id() == id)
    }
}

impl fmt::Display for ToolWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolWindowLayout {
    left: Option<ToolWindow>,
    bottom: Option<ToolWindow>,
    right: Option<ToolWindow>,
    bottom_height: Option<u32>,
}

/// Smallest bottom tool window height, in points.
pub const BOTTOM_MIN_HEIGHT: f32 = 180.0;
/// Bottom tool window height used until the user drags the splitter.
pub const BOTTOM_DEFAULT_HEIGHT: f32 = 260.0;
/// Editor height that must remain when the bottom tool window grows.
pub const EDITOR_MIN_HEIGHT: f32 = 120.0;

impl ToolWindowLayout {
    /// Height the user dragged the bottom tool window to, if any.
    pub fn bottom_height(&self) -> Option<f32> {
        self.bottom_height.map(|height| height as f32)
    }

    /// Remembers a user-chosen bottom height. Non-finite values are ignored.
    pub fn set_bottom_height(&mut self, height: f32) {
        if height.is_finite() {
            self.bottom_height = Some(height.round().max(BOTTOM_MIN_HEIGHT) as u32);
        }
    }

    /// Height to lay the bottom window out with, clamped so the editor keeps
    /// at least [`EDITOR_MIN_HEIGHT`] of a window `window_height` tall.
    pub fn effective_bottom_height(&self, window_height: f32) -> f32 {
        let max = (window_height - EDITOR_MIN_HEIGHT).max(BOTTOM_MIN_HEIGHT);
        self.bottom_height()
            .unwrap_or(BOTTOM_DEFAULT_HEIGHT)
            .clamp(BOTTOM_MIN_HEIGHT, max)
    }

    fn slot(&mut self, anchor: Anchor) -> &mut Option<ToolWindow> {
        match anchor {
            Anchor::Left => &mut self.left,
            Anchor::Bottom => &mut self.bottom,
            Anchor::Right => &mut self.right,
        }
    }

    pub fn visible(&self, anchor: Anchor) -> Option<ToolWindow> {
        match anchor {
            Anchor::Left => self.left,
            Anchor::Bottom => self.bottom,
            Anchor::Right => self.right,
        }
    }

    pub fn is_visible(&self, window: ToolWindow) -> bool {
        self.visible(window.anchor()) == Some(window)
    }

    /// Shows `window`, replacing whatever was visible at the same anchor.
    pub fn show(&mut self, window: ToolWindow) {
        *self.slot(window.anchor()) = Some(window);
    }

    pub fn hide(&mut self, window: ToolWindow) {
        let slot = self.slot(window.anchor());
        if *slot == Some(window) {
            *slot = None;
        }
    }

    /// Toggles `window` and returns whether it is visible afterwards.
    pub fn toggle(&mut self, window: ToolWindow) -> bool {
        if self.is_visible(window) {
            self.hide(window);
            false
        } else {
            self.show(window);
            true
        }
    }

    pub fn hide_all(&mut self) {
        self.left = None;
        self.bottom = None;
        self.right = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_height_is_kept_across_toggles_and_clamped() {
        let mut layout = ToolWindowLayout::default();
        assert_eq!(layout.effective_bottom_height(900.0), BOTTOM_DEFAULT_HEIGHT);
        layout.set_bottom_height(500.4);
        layout.toggle(ToolWindow::Terminal);
        layout.toggle(ToolWindow::Terminal);
        assert_eq!(layout.effective_bottom_height(900.0), 500.0);
        // Never squeezes the editor out, never below the minimum.
        assert_eq!(layout.effective_bottom_height(400.0), 280.0);
        layout.set_bottom_height(f32::NAN);
        layout.set_bottom_height(10.0);
        assert_eq!(layout.bottom_height(), Some(BOTTOM_MIN_HEIGHT));
    }

    #[test]
    fn one_tool_window_per_anchor() {
        let mut layout = ToolWindowLayout::default();
        layout.show(ToolWindow::Project);
        layout.show(ToolWindow::Terminal);
        layout.show(ToolWindow::Build);
        assert!(layout.is_visible(ToolWindow::Project));
        assert!(layout.is_visible(ToolWindow::Build));
        assert!(!layout.is_visible(ToolWindow::Terminal));
        assert!(!layout.toggle(ToolWindow::Build));
        assert_eq!(layout.visible(Anchor::Bottom), None);
        assert!(layout.toggle(ToolWindow::Commit));
        assert!(!layout.is_visible(ToolWindow::Project));
        layout.show(ToolWindow::AiChat);
        assert_eq!(layout.visible(Anchor::Right), Some(ToolWindow::AiChat));
        assert!(layout.is_visible(ToolWindow::Commit));
        layout.show(ToolWindow::DeviceManager);
        assert_eq!(
            layout.visible(Anchor::Right),
            Some(ToolWindow::DeviceManager)
        );
        assert!(
            layout.is_visible(ToolWindow::Commit),
            "anchors are independent"
        );
        layout.show(ToolWindow::Gradle);
        assert_eq!(layout.visible(Anchor::Right), Some(ToolWindow::Gradle));
        assert!(layout.is_visible(ToolWindow::Commit));
        layout.hide_all();
        assert_eq!(layout.visible(Anchor::Right), None);
    }

    #[test]
    fn ids_round_trip() {
        for window in ToolWindow::ALL {
            assert_eq!(ToolWindow::parse(window.id()), Some(window));
        }
        assert_eq!(ToolWindow::parse("nope"), None);
    }
}
