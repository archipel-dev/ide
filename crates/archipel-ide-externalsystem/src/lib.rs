//! External system import: build tool resolvers produce [`DataNode`] trees, data services
//! apply them to an `archipel_ide_projectmodel::ProjectModel`.
//!
//! The pieces map to IntelliJ's external system framework: [`ProjectResolver`]
//! (`ExternalSystemProjectResolver`), [`ExternalSystemRegistry`]
//! (`ExternalSystemManager` extension point), [`DataNode`] with [`DataKey`] (`ProjectKeys`),
//! [`ProjectDataService`] and [`ProjectDataManager`] (`ProjectDataService`,
//! `ProjectDataManager`). [`import_project`] runs the whole pipeline. See
//! `docs/architecture/project-model.md`.

pub mod artifacts;
pub mod bazel;
pub mod cargo;
mod data;
pub mod gradle;
pub mod jdk;
mod import;
pub mod maven;
pub mod process;
mod resolver;
mod service;

pub use data::*;
pub use import::{ImportResult, import_project, load_cached_model, save_cached_model};
pub use resolver::{
    CancelToken, ExternalSystemRegistry, ImportError, ImportSettings, NoProgress, ProgressListener,
    ProjectResolver, ResolveRequest,
};
pub use service::{
    ImportItem, ModifiableModelsProvider, ProjectDataManager, ProjectDataService,
};
