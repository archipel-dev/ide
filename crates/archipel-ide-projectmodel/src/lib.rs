//! The project model: what the IDE knows about the structure of an open project.
//!
//! Names and semantics follow the IntelliJ Platform: [`Module`]s with [`ContentEntry`]s and
//! [`SourceFolder`]s (typed by [`JpsModuleSourceRootType`]), [`OrderEntry`]s (module SDK,
//! library, module dependency, each with a [`DependencyScope`]), [`Library`]s with classes,
//! sources and javadoc roots, [`Sdk`]s, the [`ProjectFileIndex`] and [`GlobalSearchScope`]s.
//!
//! This crate is pure data: it has no UI, no processes and no file system access (apart from
//! [`Sdk::jdk_at`], which checks two well known paths). Build tool importers live in
//! `archipel-ide-externalsystem` and produce a [`ProjectModel`]. See
//! `docs/architecture/project-model.md`.

mod file_index;
mod model;
mod module;
mod order;
pub mod path;
mod roots;
mod scope;
mod sdk;

pub use file_index::ProjectFileIndex;
pub use model::ProjectModel;
pub use module::{CompilerOutput, ExternalProject, Module, ModuleType};
pub use order::{ClasspathKind, DependencyScope, OrderEntry};
pub use roots::{ContentEntry, JpsModuleSourceRootType, SourceFolder};
pub use scope::{ContentFilter, GlobalSearchScope};
pub use sdk::{Coordinates, Library, Sdk, SdkType};

#[cfg(test)]
mod tests;
