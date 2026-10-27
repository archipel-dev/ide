//! `ProjectFileIndex`: answers "which module owns this file", "is it test code", "which
//! library provides this jar".
//!
//! Lookups walk the ancestors of a path and consult hash maps keyed by root, so they cost
//! O(path depth) and never touch the disk. The nearest registered root wins: a source folder
//! inside an excluded directory is still a source folder, a nested module owns its own files.

use crate::model::ProjectModel;
use crate::module::Module;
use crate::order::OrderEntry;
use crate::path::{normalize, split_archive_entry};
use crate::roots::{JpsModuleSourceRootType, SourceFolder};
use crate::sdk::Library;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LibraryRoot {
    Classes,
    Sources,
    Javadoc,
}

/// What the nearest registered root says about a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placement<'a> {
    Source {
        module: usize,
        folder: &'a SourceFolder,
    },
    Content {
        module: usize,
    },
    Excluded,
}

pub struct ProjectFileIndex {
    model: Arc<ProjectModel>,
    /// root path -> (module index, source folder index among `module.source_folders()`)
    source_roots: HashMap<PathBuf, (usize, usize)>,
    content_roots: HashMap<PathBuf, usize>,
    excluded: HashMap<PathBuf, ()>,
    library_roots: HashMap<PathBuf, Vec<(usize, LibraryRoot)>>,
    sdk_roots: HashMap<PathBuf, usize>,
}

impl ProjectFileIndex {
    pub fn new(model: Arc<ProjectModel>) -> Self {
        let mut source_roots = HashMap::new();
        let mut content_roots = HashMap::new();
        let mut excluded = HashMap::new();
        for (m, module) in model.modules.iter().enumerate() {
            for entry in &module.content_entries {
                content_roots.entry(entry.root.clone()).or_insert(m);
                for path in &entry.excluded_folders {
                    excluded.insert(path.clone(), ());
                }
            }
            for (s, folder) in module.source_folders().enumerate() {
                source_roots.entry(folder.path.clone()).or_insert((m, s));
            }
        }
        let mut library_roots: HashMap<PathBuf, Vec<(usize, LibraryRoot)>> = HashMap::new();
        for (l, library) in model.libraries.iter().enumerate() {
            for (roots, kind) in [
                (&library.classes, LibraryRoot::Classes),
                (&library.sources, LibraryRoot::Sources),
                (&library.javadoc, LibraryRoot::Javadoc),
            ] {
                for root in roots {
                    library_roots
                        .entry(normalize(root))
                        .or_default()
                        .push((l, kind));
                }
            }
        }
        let mut sdk_roots = HashMap::new();
        for (s, sdk) in model.sdks.iter().enumerate() {
            for root in sdk.classes.iter().chain(&sdk.sources) {
                sdk_roots.insert(normalize(root), s);
            }
        }
        Self {
            model,
            source_roots,
            content_roots,
            excluded,
            library_roots,
            sdk_roots,
        }
    }

    pub fn model(&self) -> &ProjectModel {
        &self.model
    }

    fn placement(&self, path: &Path) -> Option<Placement<'_>> {
        let path = normalize(path);
        for ancestor in path.ancestors() {
            if let Some(&(module, folder)) = self.source_roots.get(ancestor) {
                let folder = self.model.modules[module].source_folders().nth(folder)?;
                return Some(Placement::Source { module, folder });
            }
            if let Some(&module) = self.content_roots.get(ancestor) {
                return Some(Placement::Content { module });
            }
            if self.excluded.contains_key(ancestor) {
                return Some(Placement::Excluded);
            }
        }
        None
    }

    fn library_hit(&self, path: &Path) -> Option<(usize, LibraryRoot)> {
        let (archive, _) = split_archive_entry(path);
        let archive = normalize(&archive);
        for ancestor in archive.ancestors() {
            if let Some(hits) = self.library_roots.get(ancestor) {
                return hits.first().copied();
            }
        }
        None
    }

    /// `getModuleForFile(file)`: the module whose content owns the file; `None` for excluded
    /// files and files outside every content root.
    pub fn module_for_file(&self, path: &Path) -> Option<&Module> {
        self.module_for_file_ext(path, true)
    }

    /// `getModuleForFile(file, honorExclusion)`.
    pub fn module_for_file_ext(&self, path: &Path, honor_exclusion: bool) -> Option<&Module> {
        match self.placement(path)? {
            Placement::Source { module, .. } | Placement::Content { module } => {
                self.model.modules.get(module)
            }
            Placement::Excluded if honor_exclusion => None,
            Placement::Excluded => self.excluded_owner(path),
        }
    }

    fn excluded_owner(&self, path: &Path) -> Option<&Module> {
        let path = normalize(path);
        self.model.modules.iter().find(|module| {
            module
                .content_entries
                .iter()
                .any(|entry| path.starts_with(&entry.root))
        })
    }

    /// `isInContent`: under a content root and not excluded.
    pub fn is_in_content(&self, path: &Path) -> bool {
        self.module_for_file(path).is_some()
    }

    /// `isInSourceContent`: under a source folder of any type (production or test, resources
    /// and generated sources included).
    pub fn is_in_source_content(&self, path: &Path) -> bool {
        matches!(self.placement(path), Some(Placement::Source { .. }))
    }

    /// `isInTestSourceContent`.
    pub fn is_in_test_source_content(&self, path: &Path) -> bool {
        matches!(self.placement(path), Some(Placement::Source { folder, .. }) if folder.is_test_source())
    }

    /// Same as [`Self::is_in_test_source_content`] (the name used by callers asking
    /// "is this test code").
    pub fn is_in_test_source(&self, path: &Path) -> bool {
        self.is_in_test_source_content(path)
    }

    /// Source content that is not test code.
    pub fn is_in_production_source(&self, path: &Path) -> bool {
        matches!(self.placement(path), Some(Placement::Source { folder, .. }) if !folder.is_test_source())
    }

    /// `isInSource`: module source content or the sources of a library.
    pub fn is_in_source(&self, path: &Path) -> bool {
        self.is_in_source_content(path) || self.is_in_library_sources(path)
    }

    /// `isUnderSourceRootOfType`.
    pub fn is_under_source_root_of_type(
        &self,
        path: &Path,
        types: &[JpsModuleSourceRootType],
    ) -> bool {
        matches!(self.placement(path), Some(Placement::Source { folder, .. }) if types.contains(&folder.root_type))
    }

    /// `isExcluded`: excluded by a module (and not claimed by a nearer root).
    pub fn is_excluded(&self, path: &Path) -> bool {
        matches!(self.placement(path), Some(Placement::Excluded))
    }

    pub fn is_in_generated_sources(&self, path: &Path) -> bool {
        matches!(self.placement(path), Some(Placement::Source { folder, .. }) if folder.root_type.is_generated())
    }

    /// `isInLibraryClasses`.
    pub fn is_in_library_classes(&self, path: &Path) -> bool {
        matches!(self.library_hit(path), Some((_, LibraryRoot::Classes)))
    }

    /// `isInLibrarySource`.
    pub fn is_in_library_sources(&self, path: &Path) -> bool {
        matches!(self.library_hit(path), Some((_, LibraryRoot::Sources)))
    }

    /// `isInLibrary`: a classes, sources or documentation root of a library.
    pub fn is_in_library(&self, path: &Path) -> bool {
        self.library_hit(path).is_some()
    }

    /// Part of an SDK (JDK modules image, `src.zip`, Rust sysroot sources).
    pub fn is_in_sdk(&self, path: &Path) -> bool {
        self.sdk_for_path(path).is_some()
    }

    pub fn sdk_for_path(&self, path: &Path) -> Option<&crate::sdk::Sdk> {
        let (archive, _) = split_archive_entry(path);
        let archive = normalize(&archive);
        archive
            .ancestors()
            .find_map(|a| self.sdk_roots.get(a))
            .and_then(|&s| self.model.sdks.get(s))
    }

    /// `isInProject`: module content or library content.
    pub fn is_in_project(&self, path: &Path) -> bool {
        self.is_in_content(path) || self.is_in_library(path)
    }

    /// The library that has `path` (a jar, a directory root or a file inside one, also as
    /// `lib.jar!/org/A.class`) among its roots.
    pub fn library_for_path(&self, path: &Path) -> Option<&Library> {
        self.library_hit(path)
            .and_then(|(l, _)| self.model.libraries.get(l))
    }

    /// `getSourceRootForFile`.
    pub fn source_root_for_file(&self, path: &Path) -> Option<&SourceFolder> {
        match self.placement(path)? {
            Placement::Source { folder, .. } => Some(folder),
            _ => None,
        }
    }

    /// `getContentRootForFile`: the nearest content root containing `path` (also when excluded
    /// files are asked for).
    pub fn content_root_for_file(&self, path: &Path) -> Option<&Path> {
        let path = normalize(path);
        path.ancestors()
            .find_map(|a| self.content_roots.get_key_value(a))
            .map(|(root, _)| root.as_path())
    }

    /// `getPackageNameByDirectory`: the package of a directory (or of a file's directory) under a
    /// Java source root, honoring the root's package prefix.
    pub fn package_name_by_directory(&self, dir: &Path) -> Option<String> {
        let folder = self.source_root_for_file(dir)?;
        let relative = normalize(dir).strip_prefix(&folder.path).ok()?.to_owned();
        let mut parts: Vec<String> = Vec::new();
        if !folder.package_prefix.is_empty() {
            parts.push(folder.package_prefix.clone());
        }
        parts.extend(
            relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned()),
        );
        Some(parts.join("."))
    }

    /// The package of the file's directory, or of the file's own directory when `path` is one.
    pub fn package_name_for_file(&self, file: &Path) -> Option<String> {
        self.package_name_by_directory(file.parent()?)
    }

    /// `getOrderEntriesForFile`: for module files the owning module's entries; for library files
    /// every module entry that refers to the library.
    pub fn order_entries_for_file(&self, path: &Path) -> Vec<(&Module, &OrderEntry)> {
        let mut out = Vec::new();
        if let Some(module) = self.module_for_file(path) {
            out.extend(
                module
                    .order_entries
                    .iter()
                    .filter(|e| matches!(e, OrderEntry::ModuleSource))
                    .map(|e| (module, e)),
            );
        }
        if let Some(library) = self.library_for_path(path) {
            for module in &self.model.modules {
                out.extend(module.order_entries.iter().filter_map(|entry| match entry {
                    OrderEntry::Library { library: name, .. } if *name == library.name => {
                        Some((module, entry))
                    }
                    _ => None,
                }));
            }
        }
        out
    }
}
