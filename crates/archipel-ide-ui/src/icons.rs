//! Icons: a thin adapter over the kit's icon pack (`archipel-icons`).
//!
//! The IDE's [`Icon`] enum is kept so call sites keep compiling, but every variant now
//! names a kit [`archipel_kit::Glyph`] (see [`Icon::kit`]), file-type icons come from
//! [`archipel_kit::Icon::for_file`], and [`paint`], [`show`] and the icon buttons delegate
//! to the kit. New code should use the kit's `Glyph` and `Icon` directly (they work in
//! every kit component); this enum goes away as the areas migrate.
//!
//! | IDE icon | Kit glyph | IDE icon | Kit glyph |
//! |---|---|---|---|
//! | `MainMenu` | `Menu` | `Rollback` | `Undo` |
//! | `File` | `FileText` | `Find` | `Search` |
//! | `Logcat` | `Log` | `Variants` | `Layers` |
//! | `Ai` | `Sparkle` | `Model` | `Package` |
//! | `Task` | `Settings` | `Add` | `Plus` |
//!
//! The other variants have a kit glyph of the same name. Nothing is unmapped; these nine
//! are the ones whose kit drawing has a different name or shape than the IDE's old one.

use crate::theme;
use archipel_ide_core::project_view::DirRole;
use archipel_kit::action_system::ActionButton;
use archipel_kit::parts::Tone;
use archipel_kit::{Glyph, Theme};
use eframe::egui::{self, Color32, Rect};
#[cfg(test)]
use std::path::Path;

pub use archipel_kit::icons::{FolderKind, SymbolKind as TypeKind};

/// Neutral icon color (palette `Gray11`).
pub const ICON: theme::Token = theme::ICON;

/// An icon of the IDE; see the module documentation for the mapping onto the kit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    MainMenu,
    ChevronDown,
    ChevronUp,
    ChevronRight,
    Folder,
    File,
    /// A Java type: the kit's class, interface, enum, record or annotation symbol.
    JavaType(TypeKind),
    FileImage,
    FileArchive,
    /// A folder tinted by its role.
    FolderKind(FolderKind),
    Project,
    Commit,
    Structure,
    Branch,
    Terminal,
    Problems,
    Build,
    Find,
    Run,
    Stop,
    Search,
    Close,
    Minimize,
    Refresh,
    /// Curved arrow back (rollback / revert).
    Rollback,
    /// Two-pane compare (show diff).
    Diff,
    Error,
    Warning,
    /// Phone outline (devices, Device Manager).
    Device,
    Logcat,
    Pause,
    ScrollEnd,
    SoftWrap,
    WeakWarning,
    Pin,
    Success,
    Rerun,
    /// Stacked squares (Build Variants).
    Variants,
    /// AI Assistant: a sparkle.
    Ai,
    Plus,
    Copy,
    Send,
    /// Build-tool model (cube); used for the Gradle tool window and projects.
    Model,
    /// Single task (gear).
    Task,
    ExpandAll,
    CollapseAll,
    Add,
    Debug,
    StepOver,
    StepInto,
    StepOut,
    /// Any kit icon: what [`Icon::for_file`] returns (language tiles, symbols, folders).
    Kit(archipel_kit::Icon),
}

impl Icon {
    /// The kit icon this adapter draws.
    pub fn kit(self) -> archipel_kit::Icon {
        use archipel_kit::Icon as K;
        let glyph = match self {
            Icon::MainMenu => Glyph::Menu,
            Icon::ChevronDown => Glyph::ChevronDown,
            Icon::ChevronUp => Glyph::ChevronUp,
            Icon::ChevronRight => Glyph::ChevronRight,
            Icon::Folder => Glyph::Folder,
            Icon::File => Glyph::FileText,
            Icon::JavaType(kind) => return K::Symbol(archipel_kit::icons::Symbol::new(kind)),
            Icon::FileImage => return K::FileImage,
            Icon::FileArchive => return K::FileArchive,
            Icon::FolderKind(kind) => return K::FolderKind(kind),
            Icon::Kit(icon) => return icon,
            Icon::Project => Glyph::Project,
            Icon::Commit => Glyph::Commit,
            Icon::Structure => Glyph::Structure,
            Icon::Branch => Glyph::Branch,
            Icon::Terminal => Glyph::Terminal,
            Icon::Problems => Glyph::Problems,
            Icon::Build => Glyph::Build,
            Icon::Find | Icon::Search => Glyph::Search,
            Icon::Run => Glyph::Run,
            Icon::Stop => Glyph::Stop,
            Icon::Close => Glyph::Close,
            Icon::Minimize => Glyph::Minimize,
            Icon::Refresh => Glyph::Refresh,
            Icon::Rollback => Glyph::Undo,
            Icon::Diff => Glyph::Diff,
            Icon::Error => Glyph::Error,
            Icon::Warning => Glyph::Warning,
            Icon::Device => Glyph::Device,
            Icon::Logcat => Glyph::Log,
            Icon::Pause => Glyph::Pause,
            Icon::ScrollEnd => Glyph::ScrollEnd,
            Icon::SoftWrap => Glyph::SoftWrap,
            Icon::WeakWarning => Glyph::WeakWarning,
            Icon::Pin => Glyph::Pin,
            Icon::Success => Glyph::Success,
            Icon::Rerun => Glyph::Rerun,
            Icon::Variants => Glyph::Layers,
            Icon::Ai => Glyph::Sparkle,
            Icon::Plus | Icon::Add => Glyph::Plus,
            Icon::Copy => Glyph::Copy,
            Icon::Send => Glyph::Send,
            Icon::Model => Glyph::Package,
            Icon::Task => Glyph::Settings,
            Icon::ExpandAll => Glyph::ExpandAll,
            Icon::CollapseAll => Glyph::CollapseAll,
            Icon::Debug => Glyph::Debug,
            Icon::StepOver => Glyph::StepOver,
            Icon::StepInto => Glyph::StepInto,
            Icon::StepOut => Glyph::StepOut,
        };
        K::Glyph(glyph)
    }

    /// File icon by well-known file name, then extension (the kit's rules): language
    /// tiles, the class symbol for Java, image and archive glyphs, a text page otherwise.
    pub fn for_file(name: &str) -> Icon {
        Icon::Kit(archipel_kit::Icon::for_file(name))
    }

    /// Like [`Icon::for_file`], peeking into Java sources for the kind of their type.
    pub fn for_source(name: &str, text: Option<&str>) -> Icon {
        Icon::Kit(archipel_kit::Icon::for_source(name, text))
    }

    /// Whether this is a Java type symbol (the icon of a `.java` file).
    pub fn is_java_type(self) -> bool {
        matches!(
            self,
            Icon::JavaType(_) | Icon::Kit(archipel_kit::Icon::Symbol(_))
        )
    }

    /// Folder icon for a role computed by the Project view model.
    pub fn for_role(role: DirRole) -> Icon {
        match folder_kind(role) {
            FolderKind::Plain => Icon::Folder,
            kind => Icon::FolderKind(kind),
        }
    }

    #[cfg(test)]
    pub fn for_folder(path: &Path) -> Icon {
        Icon::for_role(archipel_ide_core::project_view::classify(
            path,
            &Default::default(),
        ))
    }
}

impl From<Icon> for archipel_kit::Icon {
    fn from(icon: Icon) -> Self {
        icon.kit()
    }
}

/// What a folder is for, from the Project view model's role.
pub fn folder_kind(role: DirRole) -> FolderKind {
    match role {
        DirRole::Plain => FolderKind::Plain,
        DirRole::Source => FolderKind::Source,
        DirRole::Test => FolderKind::Test,
        DirRole::Resources | DirRole::TestResources => FolderKind::Resources,
        DirRole::Excluded => FolderKind::Excluded,
        DirRole::Hidden => FolderKind::Hidden,
        DirRole::Package { .. } => FolderKind::Package,
    }
}

/// Paints `icon` into `rect` in `color` (status glyphs, language tiles and file icons
/// keep their own colors).
pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: impl Into<Color32>) {
    archipel_kit::icons::paint(painter, rect, icon, color);
}

/// Allocates a square icon of `size` points and paints it.
pub fn show(ui: &mut egui::Ui, icon: Icon, size: f32, color: impl Into<Color32>) -> egui::Response {
    archipel_kit::icons::show(ui, icon, size, color)
}

/// Toolbar-style icon button (the kit's `ActionButton`).
pub fn button(ui: &mut egui::Ui, icon: Icon, tooltip: &str) -> egui::Response {
    ui.add(ActionButton::new(icon, tooltip))
}

/// Icon button tinted with `color`, filled with the accent when `selected`. The kit
/// colors icon buttons by meaning, so `color` is matched to the tone with the same
/// value in the active theme (run, stop, modified, error …) and falls back to the icon
/// gray.
pub fn button_colored(
    ui: &mut egui::Ui,
    icon: Icon,
    color: impl Into<Color32>,
    tooltip: &str,
    selected: bool,
) -> egui::Response {
    sized_button_colored(ui, icon, color, tooltip, selected, false)
}

/// Main toolbar button: 30 pt hit area with the toolbar icon size (UI-CHR-003; the
/// kit's `ActionButton::large`).
pub fn toolbar_button(ui: &mut egui::Ui, icon: Icon, tooltip: &str) -> egui::Response {
    ui.add(ActionButton::new(icon, tooltip).large(true))
}

/// [`button_colored`] at the main toolbar size.
pub fn toolbar_button_colored(
    ui: &mut egui::Ui,
    icon: Icon,
    color: impl Into<Color32>,
    tooltip: &str,
    selected: bool,
) -> egui::Response {
    sized_button_colored(ui, icon, color, tooltip, selected, true)
}

fn sized_button_colored(
    ui: &mut egui::Ui,
    icon: Icon,
    color: impl Into<Color32>,
    tooltip: &str,
    selected: bool,
    large: bool,
) -> egui::Response {
    let colors = Theme::colors(ui.ctx());
    let color = color.into();
    let tone = Tone::ALL
        .into_iter()
        .find(|tone| tone.color(&colors) == color)
        .unwrap_or(Tone::Icon);
    let button = ActionButton::new(icon, tooltip).tone(tone).large(large);
    ui.add(if selected {
        button.selected(true)
    } else {
        button
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_kit::Icon as K;
    use archipel_kit::icons::Symbol;

    #[test]
    fn file_icons_come_from_the_kit() {
        assert_eq!(
            Icon::for_file("Main.java"),
            Icon::Kit(K::Symbol(Symbol::new(TypeKind::Class)))
        );
        assert!(Icon::for_file("Main.java").is_java_type());
        assert!(!Icon::for_file("lib.rs").is_java_type());
        assert_eq!(Icon::for_file("a.PNG"), Icon::Kit(K::FileImage));
        assert_eq!(Icon::for_file("a.jar"), Icon::Kit(K::FileArchive));
        assert_eq!(
            Icon::for_file("notes"),
            Icon::Kit(K::Glyph(Glyph::FileText))
        );
        assert_eq!(
            Icon::for_source("Foo.java", Some("enum Foo {}")),
            Icon::Kit(K::Symbol(Symbol::new(TypeKind::Enum)))
        );
        assert_eq!(
            Icon::for_source("a.rs", Some("enum Foo {}")),
            Icon::for_file("a.rs")
        );
    }

    #[test]
    fn folder_icons_follow_their_role() {
        let kind = |path: &str| match Icon::for_folder(Path::new(path)) {
            Icon::Folder => FolderKind::Plain,
            Icon::FolderKind(kind) => kind,
            other => panic!("{other:?}"),
        };
        assert_eq!(kind("src"), FolderKind::Plain);
        assert_eq!(kind("src/main/java"), FolderKind::Source);
        assert_eq!(kind("src/test/java"), FolderKind::Test);
        assert_eq!(kind("src/main/resources"), FolderKind::Resources);
        assert_eq!(kind("src/main/java/com"), FolderKind::Package);
        assert_eq!(kind("build"), FolderKind::Excluded);
        assert_eq!(kind(".git"), FolderKind::Hidden);
    }

    #[test]
    fn every_icon_maps_to_a_kit_icon() {
        // The adapter is total: a few representative variants, including the renamed ones.
        for (icon, glyph) in [
            (Icon::MainMenu, Glyph::Menu),
            (Icon::Find, Glyph::Search),
            (Icon::Rollback, Glyph::Undo),
            (Icon::Logcat, Glyph::Log),
            (Icon::Variants, Glyph::Layers),
            (Icon::Ai, Glyph::Sparkle),
            (Icon::Model, Glyph::Package),
            (Icon::Task, Glyph::Settings),
            (Icon::Add, Glyph::Plus),
        ] {
            assert_eq!(icon.kit(), K::Glyph(glyph), "{icon:?}");
        }
    }
}
