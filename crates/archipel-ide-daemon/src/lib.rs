//! The highlighting daemon (WP3): pass scheduler, highlight markup,
//! inspections, quick fixes and the Alt+Enter model. See
//! `docs/analysis/architecture.md` and the spec sections 1.7-1.9.
//!
//! No UI types: this crate sits below `archipel-ide-core`.

#![forbid(unsafe_code)]

pub mod clock;
pub mod daemon;
pub mod external;
pub mod inspection;
pub mod intention;
pub mod markup;
pub mod passes;
pub mod problem_files;
pub mod scheduler;
pub mod service;
pub mod stripe;

pub use clock::{Clock, ManualClock, SystemClock};
pub use daemon::{Daemon, DaemonSettings};
pub use external::{ExternalDiagnostic, ExternalDiagnostics, ExternalFile};
pub use markup::{FileMarkup, HighlightId, MarkupCounts, MarkupLimits, MarkupStore, ReplaceStats};
pub use passes::{LineMarker, LineMarkerProvider, Shared};
pub use problem_files::{FileSummary, ProblemFiles};
pub use scheduler::{
    Completion, PassSpec, PassStats, Phase, RunId, RunOutcome, RunRequest, Scheduler,
    SchedulerConfig, Trigger, Triggers,
};
pub use service::DaemonService;
