//! The entry points the daemon pass and the code actions use: everything needed to analyze
//! one file against a project model and the Java indexes.

use crate::analyze::{Analysis, analyze};
use crate::classpath::{ClasspathSnapshot, Completeness, completeness_of};
use crate::env::{CurrentFile, Env};
use crate::index::JavaIndex;
use crate::scope::{ResolveScope, ScopeKind};
use crate::source;
use archipel_ide_projectmodel::{ProjectFileIndex, ProjectModel};
use std::path::Path;
use std::sync::Arc;

/// The immutable inputs of one analysis: indexes plus the project model of the moment.
#[derive(Clone)]
pub struct JavaProject {
    pub index: JavaIndex,
    pub model: Arc<ProjectModel>,
    pub files: Arc<ProjectFileIndex>,
    /// The model is still being (re)imported: unresolved references are pending (JAVA-ERR-003).
    pub model_loading: bool,
    /// Warnings and errors of the last import (unresolved dependencies, failed resolvers):
    /// the reason the classpath may be incomplete.
    pub import_notes: Arc<Vec<String>>,
}

impl JavaProject {
    pub fn new(index: JavaIndex, model: Arc<ProjectModel>, model_loading: bool) -> Self {
        let files = Arc::new(model.file_index());
        Self {
            index,
            model,
            files,
            model_loading,
            import_notes: Arc::default(),
        }
    }

    /// The notes of the last import (JAVA-ERR-003: why the classpath may be incomplete).
    pub fn with_import_notes(mut self, notes: Vec<String>) -> Self {
        self.import_notes = Arc::new(notes);
        self
    }

    /// Whether native Java analysis applies to `path`: Java code of some module, or of the
    /// ad-hoc module (a file outside every module, JDK classes only) once a JDK is known.
    pub fn applies_to(&self, path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str()) == Some("java")
            && (self.files.module_for_file(path).is_some() || !self.model.sdks.is_empty())
    }

    pub fn scope_for(&self, path: &Path) -> ResolveScope {
        self.classpath_for(path).scope()
    }

    /// The classpath snapshot of the module `path` belongs to, with its completeness.
    pub fn classpath_for(&self, path: &Path) -> ClasspathSnapshot {
        ResolveScope::snapshot_for_file(
            &self.index,
            &self.model,
            &self.files,
            path,
            |model, module| completeness_of(model, module, self.model_loading, &self.import_notes),
        )
    }

    /// The worst completeness over the modules of the project (status bar, control socket).
    pub fn completeness(&self) -> Completeness {
        let mut worst = Completeness::Complete;
        for module in &self.model.modules {
            let c = completeness_of(&self.model, module, self.model_loading, &self.import_notes);
            let rank = |c: &Completeness| match c {
                Completeness::Complete => 0,
                Completeness::NoLibraries => 1,
                Completeness::MissingLibraries(_) => 2,
                Completeness::Importing => 3,
                Completeness::ImportFailed(_) => 4,
            };
            if rank(&c) > rank(&worst) {
                worst = c;
            }
        }
        if worst.is_complete() && self.model_loading {
            return Completeness::Importing;
        }
        worst
    }

    /// Whether answers are complete (JAVA-STUB-026, JAVA-ERR-003).
    pub fn is_complete(&self) -> bool {
        self.index.is_ready() && !self.model_loading
    }

    /// Runs the reference analysis for the live `text` of `path`.
    pub fn analyze_text(&self, path: &Path, text: &str) -> Analysis {
        let scope = self.scope_for(path);
        let stub = source::build_file_stub(text);
        let unit = self.index.unit_for_path(path);
        let current = CurrentFile::new(unit, stub);
        let env = Env::new(&self.index, &scope, Some(&current));
        let Some(tree) = source::parse(text) else {
            return Analysis::default();
        };
        let incomplete = !self.is_complete();
        analyze(&env, text, &tree, &current, incomplete)
    }
}

impl JavaProject {
    /// Like [`Self::analyze_text`] with a tree the host already parsed.
    pub fn analyze_tree(
        &self,
        path: &Path,
        text: &str,
        tree: &tree_sitter::Tree,
    ) -> Option<(Analysis, CurrentFile, ResolveScope)> {
        let scope = self.scope_for(path);
        if scope.kind == ScopeKind::AdHoc && scope.is_empty() {
            return None;
        }
        let stub = source::build_from_tree(tree, text);
        let current = CurrentFile::new(self.index.unit_for_path(path), stub);
        let analysis = {
            let env = Env::new(&self.index, &scope, Some(&current));
            let incomplete = !self.is_complete();
            analyze(&env, text, tree, &current, incomplete)
        };
        Some((analysis, current, scope))
    }
}
