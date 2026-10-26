//! Persistent file-based and stub indexes, the indexing scheduler with
//! per-index readiness (dumb mode), the TODO index and the local history
//! store (spec 1.6, 2.20, 2.25).
//!
//! - [`store`], [`redb_store`], [`overlay`]: storage (`IndexStore` backends
//!   in memory and on `redb`, open-buffer overlay);
//! - [`indexes`], [`stubs`], [`todo`]: the built-in indexes;
//! - [`service`], [`progress`]: scheduler and status;
//! - [`readers`]: typed queries (`SymbolLookup` has the old `SymbolIndex` shape);
//! - [`history`]: content-addressed blob store and revision log (Local History).
//!
//! No UI types: the app crates read [`progress::IndexStatus`].

#![forbid(unsafe_code)]

pub mod codec;
pub mod conformance;
pub mod files;
pub mod hash;
pub mod history;
pub mod indexes;
pub mod overlay;
pub mod pipeline;
pub mod progress;
pub mod readers;
pub mod redb_store;
pub mod service;
pub mod store;
pub mod stubs;
pub mod todo;

pub use indexes::ids;
pub use overlay::OverlayStore;
pub use pipeline::{ErasedIndex, IndexSet, Typed};
pub use progress::{IndexInfo, IndexStatus, Phase, SkippedFile};
pub use redb_store::RedbStore;
pub use service::{IndexConfig, IndexService};
pub use store::{DurableStore, FileRecord, MemStore, StoreError, StoreKind};
