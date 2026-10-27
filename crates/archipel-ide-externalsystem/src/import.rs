//! The import pipeline: resolve every applicable build system, apply the data nodes.

use crate::data::{DataNode, DiagnosticData, ProjectSystemId, Severity};
use crate::resolver::{ExternalSystemRegistry, ImportError, ResolveRequest};
use crate::service::ProjectDataManager;
use archipel_ide_projectmodel::ProjectModel;
use std::path::{Path, PathBuf};

/// Where the last imported model is cached, so reopening shows it before the re-import ends.
pub const CACHE_FILE: &str = ".archipel/externalsystem/project-model.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportResult {
    pub model: ProjectModel,
    /// Build systems that contributed.
    pub systems: Vec<ProjectSystemId>,
    pub diagnostics: Vec<DiagnosticData>,
    /// Number of data nodes that were applied.
    pub nodes: usize,
}

impl ImportResult {
    pub fn errors(&self) -> impl Iterator<Item = &DiagnosticData> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }
}

/// Runs all resolvers that apply to `request.root` and builds the model.
///
/// A resolver that fails does not fail the import when another one succeeded; its error is
/// reported as a diagnostic. When every resolver fails the first error is returned.
/// Returns `Ok` with an empty model when no build system applies.
pub fn import_project(
    registry: &ExternalSystemRegistry,
    manager: &ProjectDataManager,
    request: &ResolveRequest,
) -> Result<ImportResult, ImportError> {
    let resolvers = registry.detect(&request.root);
    let mut trees: Vec<DataNode> = Vec::new();
    let mut systems = Vec::new();
    let mut diagnostics = Vec::new();
    let mut first_error = None;
    for resolver in &resolvers {
        request.cancel.check()?;
        request.progress(&format!(
            "{}: importing",
            resolver.system_id().display_name()
        ));
        match resolver.resolve(request) {
            Ok(tree) => {
                systems.push(resolver.system_id());
                trees.push(tree);
            }
            Err(ImportError::Cancelled) => return Err(ImportError::Cancelled),
            Err(error) => {
                diagnostics.push(DiagnosticData {
                    severity: Severity::Error,
                    message: format!("{}: {error}", resolver.system_id().display_name()),
                });
                first_error.get_or_insert(error);
            }
        }
    }
    if trees.is_empty()
        && let Some(error) = first_error
    {
        return Err(error);
    }
    let nodes = trees.iter().map(DataNode::size).sum();
    let mut models = manager.import_data(&trees);
    if models.model.name.is_empty() {
        models.model.name = request
            .root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        models.model.root = request.root.clone();
    }
    diagnostics.extend(models.diagnostics);
    Ok(ImportResult {
        model: models.model,
        systems,
        diagnostics,
        nodes,
    })
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(CACHE_FILE)
}

/// The model of the last successful import, if any.
pub fn load_cached_model(root: &Path) -> Option<ProjectModel> {
    let path = archipel_ide_compat::project_read_path(cache_path(root));
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_cached_model(root: &Path, model: &ProjectModel) -> Result<(), String> {
    let path = archipel_ide_compat::project_write_path(cache_path(root));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string(model).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}
