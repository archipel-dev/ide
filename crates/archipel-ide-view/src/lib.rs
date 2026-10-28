//! The view-model layer of Archipel IDE (`docs/specs/tui/02-shared-component-layer.md`).
//!
//! Everything the chrome shows is decided here, as plain props of `archipel-kit-model`: the
//! main toolbar, the main menu, the tool window stripes and headers, the editor tab strip,
//! the status bar and the breadcrumbs, the context menus, and which action each interaction
//! starts. The egui frontend (`archipel-ide-ui`) and the terminal frontend
//! (`archipel-ide-tui`) only render these values and report events; neither contains a menu
//! list, a label, a shortcut text or an enabled rule of its own.
//!
//! The crate has no `egui`, `ratatui` or `crossterm` dependency (`tests/no_ui_deps.rs`).
//! Builders are pure functions of the [`ViewCx`] (same input, equal props); timers such as
//! the tooltip delay live in the renderers.

pub mod actions;
pub mod chrome;
pub mod conflict;
pub mod context_menu;
pub mod dispatch;

pub mod cx;

pub mod header;
pub mod icons;
pub mod keys;
pub mod main_menu;
pub mod toolbar;

pub mod status;
pub mod stripes;
pub mod tabs;

pub use cx::{Caps, UiData, ViewCx};

pub use chrome::Chrome;
pub use dispatch::{Effect, Event, Popup, Request, dispatch};
