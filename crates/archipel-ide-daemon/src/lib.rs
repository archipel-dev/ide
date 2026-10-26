//! The highlighting daemon (WP3): pass scheduler, highlight markup,
//! inspections, quick fixes and the Alt+Enter model. See
//! `docs/analysis/architecture.md` and the spec sections 1.7-1.9.
//!
//! No UI types: this crate sits below `archipel-ide-core`.

#![forbid(unsafe_code)]

pub mod clock;
pub mod markup;
pub mod scheduler;

pub use clock::{Clock, ManualClock, SystemClock};
pub use markup::{FileMarkup, HighlightId, MarkupCounts, MarkupLimits, MarkupStore, ReplaceStats};
pub use scheduler::{
    Completion, PassSpec, PassStats, Phase, RunId, RunOutcome, RunRequest, Scheduler,
    SchedulerConfig, Trigger, Triggers,
};
