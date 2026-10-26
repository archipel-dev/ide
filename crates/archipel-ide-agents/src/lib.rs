//! Engine of Agents mode: runs Claude Code and Codex as structured-protocol child processes
//! in git worktrees and turns what they print into [`events::AgentEvent`]s.
//!
//! No UI types (and no egui) here. The UI-facing contract (`AgentsBackend`) lives in
//! `archipel-ide-core`, which wraps [`engine::Engine`].
//!
//! Layout: [`claude`] and [`codex`] are the protocol adapters, [`process`] spawns and
//! supervises the CLI, [`workspace`] is the git side (worktree, changes, merge),
//! [`store`] persists sessions, [`redact`] keeps secrets off the disk, [`engine`] ties it
//! together.

pub mod adapter;
pub mod claude;
pub mod codex;
pub mod engine;
pub mod events;
pub mod process;
pub mod profile;
pub mod redact;
pub mod store;
pub mod update;
pub mod workspace;

pub use adapter::{Choice, Parsed};
pub use engine::{
    Engine, EngineConfig, News, Outcome, SessionId, SessionSnapshot, State, TaskId, TaskSnapshot,
};
pub use profile::{AgentKind, PermissionMode};
