//! Quick fixes, intentions and the Alt+Enter model (spec 1.9).

pub mod alert;

pub use alert::{
    ActionKind, AlertAction, AlertCache, AlertInput, AlertSet, AlertSource, Applied, ApplyInput,
    Bulb, Source, apply, assemble, fix_all_command,
};
