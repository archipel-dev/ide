//! Local History (IJ-ED-028, IJ-TW-034): a content-addressed blob store plus a
//! per-path revision log, independent of VCS.
//!
//! No VFS hooks: the host records a revision before overwriting a file on
//! save, when an external change is applied and before a refactoring runs.
//! Revisions of unchanged content are not recorded twice. Retention is by age
//! and total size ([`Retention`]). A damaged log is dropped and recreated;
//! a damaged blob only loses that revision's content.

pub mod blob;
mod log;

pub use blob::{BlobError, BlobId, BlobStore};
pub use log::{Cause, History, HistoryError, PurgeStats, Retention, Revision};
