//! Native code-analysis pipeline core (WP1): the frozen contracts other
//! packages build on. See `docs/analysis/architecture.md`.
//!
//! - identities and contents: [`FileId`]/[`FileTable`], the [`Vfs`] facade,
//!   [`DocumentOverlay`];
//! - the world: [`AnalysisHost`] (UI thread, `apply(Change)`), immutable
//!   [`Snapshot`]s, [`AnalysisEvent`]s;
//! - work: [`Executor`] jobs with [`CancelToken`]s returning
//!   `Result<_, Cancelled>`;
//! - caches: [`Dep`] stamps, [`Db::cached`] with interface-hash early cutoff;
//! - extension points: [`ServiceRegistry`] (language-keyed, ordered, base
//!   fallback, "any" slot) and the traits of every later package
//!   ([`index`], [`daemon`], [`inspect`], [`intention`], [`editing`],
//!   [`resolve`], [`assist`], [`refactor`], [`provider`]);
//! - shared data: [`Severity`]/[`HighlightInfo`], [`Target`]/[`Confidence`]/
//!   [`Provenance`], [`ModCommand`], [`Anchor`]/[`EditMap`].
//!
//! No UI types: this crate sits below `archipel-ide-core`.

#![forbid(unsafe_code)]

pub mod assist;
pub mod cache;
pub mod cancel;
pub mod daemon;
pub mod editing;
pub mod file;
pub mod highlight;
pub mod host;
pub mod index;
pub mod inspect;
pub mod intention;
pub mod interface;
pub mod job;
pub mod modcommand;
pub mod provider;
pub mod refactor;
pub mod resolve;
pub mod stamps;
pub mod target;
pub mod testing;
pub mod vfs;

pub use archipel_ide_language::{
    Anchor, CompletenessTier, EditMap, ElementType, FileId, FileRev, LanguageDef, LanguageId,
    Order, Registration, ServiceRegistry, SyntaxNode, SyntaxSnapshot, TokenSet,
};
pub use cache::{CacheStore, Cached, Db, DepCollector, Slot};
pub use cancel::{CancelToken, Cancelled};
pub use file::{FileMeta, FileTable};
pub use highlight::{HighlightInfo, HighlightKind, Severity, ToolId};
pub use host::{
    AnalysisEvent, AnalysisHost, Change, HostConfig, Snapshot, SyntaxUpdate, default_registry,
};
pub use index::{IndexNotReady, IndexState};
pub use intention::Priority as IntentionPriority;
pub use interface::InterfaceSummarizer;
pub use job::{Executor, JobHandle, JobSpec, Priority as JobPriority, Revoke, RevokeScope};
pub use modcommand::{ModCommand, TextEdit};
pub use stamps::{Dep, IndexId, IndexStamp, InterfaceEpoch, Stamps};
pub use target::{Confidence, Provenance, Target};
pub use vfs::{DocumentOverlay, Vfs, VfsChange, VfsError};
