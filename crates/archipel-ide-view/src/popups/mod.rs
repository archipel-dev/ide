//! The rows and texts of the list popups: code completion, the Alt+Enter actions, the chooser
//! of several navigation targets and Search Everywhere with its siblings (Go to File, Class,
//! Symbol, Find Action). Both frontends draw these values; neither decides what a row says,
//! which part of it is secondary, which rows are groups or where an item opens
//! (`docs/specs/tui/02-shared-component-layer.md`, M13, M15, M17, M18).
//!
//! Specs: UI-POP-011, UI-POP-012, UI-POP-030..032, TUI-GUI-135, TUI-GUI-136, TUI-GUI-138,
//! TUI-GUI-139.

pub mod chooser;
pub mod completion;
pub mod intentions;
pub mod search;
