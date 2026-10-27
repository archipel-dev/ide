//! Native Java intelligence (Phase 4 of `docs/architecture/platform-refactor-plan.md`).
#![forbid(unsafe_code)]

pub mod analyze;
pub mod api;
pub mod classfile;
pub mod env;
pub mod index;
pub mod jimage;
pub mod scope;
pub mod source;
pub mod stub;
pub mod ty;
pub mod unit;
