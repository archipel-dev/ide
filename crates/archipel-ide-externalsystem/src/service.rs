//! Data services: apply `DataNode`s to the project model.
//!
//! IntelliJ's `ProjectDataService` imports the nodes of one key into modifiable models; the
//! `ProjectDataManager` runs the services ordered by `getOrder`. The same split is used here:
//! [`ProjectDataService`] per [`DataKey`], [`ModifiableModelsProvider`] as the model under
//! construction (it is rebuilt from scratch on every import, so there is no orphan removal),
//! [`ProjectDataManager`] as the runner. Services are an extension point: other crates can
//! register their own (for example a Spring facet) with [`ProjectDataManager::register`].

use crate::data::*;
use archipel_ide_projectmodel::{
    ContentEntry, DependencyScope, ExternalProject, JpsModuleSourceRootType, Library, Module,
    OrderEntry, ProjectModel, Sdk, SourceFolder,
};
use std::collections::HashMap;

/// `IdeModifiableModelsProvider`: the model being built plus id lookups and diagnostics.
pub struct ModifiableModelsProvider {
    pub model: ProjectModel,
    pub diagnostics: Vec<DiagnosticData>,
    /// `(system, external module id)` to the module's name in the model.
    module_names: HashMap<(ProjectSystemId, String), String>,
}

impl ModifiableModelsProvider {
    pub fn new() -> Self {
        Self {
            model: ProjectModel::default(),
            diagnostics: Vec::new(),
            module_names: HashMap::new(),
        }
    }

    pub fn module_name(&self, system: ProjectSystemId, id: &str) -> Option<&str> {
        self.module_names
            .get(&(system, id.to_owned()))
            .map(String::as_str)
    }

    pub fn report(&mut self, severity: Severity, message: impl Into<String>) {
        self.diagnostics.push(DiagnosticData {
            severity,
            message: message.into(),
        });
    }

    fn unique_module_name(&self, wanted: &str, system: ProjectSystemId) -> String {
        if self.model.module(wanted).is_none() {
            return wanted.to_owned();
        }
        let qualified = format!("{wanted} ({})", system.display_name());
        if self.model.module(&qualified).is_none() {
            return qualified;
        }
        (2..)
            .map(|n| format!("{wanted} ({n})"))
            .find(|name| self.model.module(name).is_none())
            .expect("an unused name exists")
    }
}

impl Default for ModifiableModelsProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// One node to import with its parent and the build it came from.
pub struct ImportItem<'a> {
    pub node: &'a DataNode,
    pub parent: Option<&'a DataNode>,
    pub system: ProjectSystemId,
}

/// `ProjectDataService`.
pub trait ProjectDataService: Send + Sync {
    /// `getTargetDataKey`.
    fn target_key(&self) -> DataKey;
    /// `getOrder`: lower runs first.
    fn order(&self) -> i32;
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider);
}

/// `ProjectDataManager`: runs the registered services over a project node.
pub struct ProjectDataManager {
    services: Vec<Box<dyn ProjectDataService>>,
}

impl ProjectDataManager {
    pub fn empty() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    /// A manager with the built-in services.
    pub fn new() -> Self {
        let mut manager = Self::empty();
        manager.register(Box::new(ProjectDataServiceImpl));
        manager.register(Box::new(SdkDataService));
        manager.register(Box::new(ModuleDataService));
        manager.register(Box::new(ContentRootDataService));
        manager.register(Box::new(LibraryDataService));
        manager.register(Box::new(ModuleDependencyDataService));
        manager.register(Box::new(LibraryDependencyDataService));
        manager.register(Box::new(DiagnosticDataService));
        manager
    }

    pub fn register(&mut self, service: Box<dyn ProjectDataService>) {
        self.services.push(service);
        self.services.sort_by_key(|s| s.order());
    }

    /// `importData`: applies every project tree to a fresh model.
    pub fn import_data(&self, projects: &[DataNode]) -> ModifiableModelsProvider {
        let mut models = ModifiableModelsProvider::new();
        for project in projects {
            let NodeData::Project(data) = &project.data else {
                models.report(Severity::Error, "resolver returned a node that is not a project");
                continue;
            };
            let system = data.system;
            let mut by_key: HashMap<DataKey, Vec<ImportItem<'_>>> = HashMap::new();
            project.walk(&mut |node, parent| {
                by_key.entry(node.key()).or_default().push(ImportItem {
                    node,
                    parent,
                    system,
                });
            });
            for service in &self.services {
                if let Some(items) = by_key.get(&service.target_key()) {
                    service.import_data(items, &mut models);
                }
            }
        }
        models
    }
}

impl Default for ProjectDataManager {
    fn default() -> Self {
        Self::new()
    }
}

struct ProjectDataServiceImpl;

impl ProjectDataService for ProjectDataServiceImpl {
    fn target_key(&self) -> DataKey {
        DataKey::Project
    }
    fn order(&self) -> i32 {
        0
    }
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
        for item in items {
            let NodeData::Project(data) = &item.node.data else {
                continue;
            };
            // The first project names the model; later ones (a second build system in the
            // same directory) only contribute modules.
            if models.model.name.is_empty() {
                models.model.name = data.name.clone();
                models.model.root = data.root.clone();
            }
            if models.model.project_sdk.is_none() {
                models.model.project_sdk = data.sdk.clone();
            }
        }
    }
}

struct SdkDataService;

impl ProjectDataService for SdkDataService {
    fn target_key(&self) -> DataKey {
        DataKey::Sdk
    }
    fn order(&self) -> i32 {
        10
    }
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
        for item in items {
            let NodeData::Sdk(data) = &item.node.data else {
                continue;
            };
            let mut sdk = Sdk::new(&data.name, data.kind, &data.home);
            sdk.version = data.version.clone();
            if data.kind == archipel_ide_projectmodel::SdkType::Jdk {
                sdk.classes = Sdk::jdk_at(&data.home, None).classes;
            }
            sdk.sources = data
                .sources
                .iter()
                .filter(|path| path.exists())
                .cloned()
                .collect();
            models.model.put_sdk(sdk);
        }
    }
}

struct ModuleDataService;

impl ProjectDataService for ModuleDataService {
    fn target_key(&self) -> DataKey {
        DataKey::Module
    }
    fn order(&self) -> i32 {
        20
    }
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
        for item in items {
            let NodeData::Module(data) = &item.node.data else {
                continue;
            };
            let name = models.unique_module_name(&data.name, item.system);
            let mut module = Module::new(&name);
            module.module_type = data.module_type;
            module.language_level = data.language_level.clone();
            module.compiler_output.production = data.compile_output.clone();
            module.compiler_output.test = data.test_compile_output.clone();
            module.external_project = Some(ExternalProject {
                system: item.system.id().to_owned(),
                id: data.id.clone(),
                path: data.path.clone(),
            });
            models
                .module_names
                .insert((item.system, data.id.clone()), name);
            models.model.modules.push(module);
        }
    }
}

fn owner_module<'a>(
    item: &ImportItem<'_>,
    models: &'a mut ModifiableModelsProvider,
) -> Option<&'a mut Module> {
    let NodeData::Module(parent) = &item.parent?.data else {
        return None;
    };
    let name = models.module_name(item.system, &parent.id)?.to_owned();
    models.model.module_mut(&name)
}

struct ContentRootDataService;

impl ProjectDataService for ContentRootDataService {
    fn target_key(&self) -> DataKey {
        DataKey::ContentRoot
    }
    fn order(&self) -> i32 {
        30
    }
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
        for item in items {
            let NodeData::ContentRoot(data) = &item.node.data else {
                continue;
            };
            let Some(module) = owner_module(item, models) else {
                continue;
            };
            let mut entry = ContentEntry::new(&data.root);
            for (kind, path) in &data.folders {
                let root_type = match kind {
                    ExternalSystemSourceType::Source => JpsModuleSourceRootType::Source,
                    ExternalSystemSourceType::Test => JpsModuleSourceRootType::TestSource,
                    ExternalSystemSourceType::Resource => JpsModuleSourceRootType::Resource,
                    ExternalSystemSourceType::TestResource => {
                        JpsModuleSourceRootType::TestResource
                    }
                    ExternalSystemSourceType::SourceGenerated => JpsModuleSourceRootType::Generated,
                    ExternalSystemSourceType::TestGenerated => {
                        JpsModuleSourceRootType::GeneratedTest
                    }
                    ExternalSystemSourceType::Excluded => {
                        entry.add_excluded_folder(path);
                        continue;
                    }
                };
                entry.add_source_folder(SourceFolder::new(path, root_type));
            }
            match module.content_entries.iter_mut().find(|e| e.root == entry.root) {
                Some(existing) => {
                    for folder in entry.source_folders {
                        existing.add_source_folder(folder);
                    }
                    for path in entry.excluded_folders {
                        existing.add_excluded_folder(path);
                    }
                }
                None => module.content_entries.push(entry),
            }
        }
    }
}

struct LibraryDataService;

impl ProjectDataService for LibraryDataService {
    fn target_key(&self) -> DataKey {
        DataKey::Library
    }
    fn order(&self) -> i32 {
        40
    }
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
        for item in items {
            let NodeData::Library(data) = &item.node.data else {
                continue;
            };
            let mut library = Library::new(&data.name);
            for root in &data.binary {
                library = library.with_classes(root);
            }
            for root in &data.source {
                library = library.with_sources(root);
            }
            for root in &data.documentation {
                library = library.with_javadoc(root);
            }
            library.coordinates = data.coordinates.clone();
            models.model.put_library(library);
        }
    }
}

struct ModuleDependencyDataService;

impl ProjectDataService for ModuleDependencyDataService {
    fn target_key(&self) -> DataKey {
        DataKey::ModuleDependency
    }
    fn order(&self) -> i32 {
        50
    }
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
        for item in items {
            let NodeData::ModuleDependency(data) = &item.node.data else {
                continue;
            };
            let Some(target) = models.module_name(item.system, &data.target).map(str::to_owned)
            else {
                models.report(
                    Severity::Warning,
                    format!("module dependency on unknown module {}", data.target),
                );
                continue;
            };
            let Some(module) = owner_module(item, models) else {
                continue;
            };
            if module.name == target {
                continue;
            }
            module.add_order_entry(OrderEntry::Module {
                module: target,
                scope: data.scope,
                exported: data.exported,
            });
        }
    }
}

struct LibraryDependencyDataService;

impl ProjectDataService for LibraryDependencyDataService {
    fn target_key(&self) -> DataKey {
        DataKey::LibraryDependency
    }
    fn order(&self) -> i32 {
        60
    }
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
        for item in items {
            let NodeData::LibraryDependency(data) = &item.node.data else {
                continue;
            };
            if models.model.library(&data.library).is_none() {
                models.report(
                    Severity::Warning,
                    format!("dependency on unknown library {}", data.library),
                );
                continue;
            }
            let Some(module) = owner_module(item, models) else {
                continue;
            };
            // The same library can appear on several classpaths: the widest scope wins.
            let existing = module.order_entries.iter_mut().find_map(|entry| match entry {
                OrderEntry::Library { library, scope, .. } if *library == data.library => {
                    Some(scope)
                }
                _ => None,
            });
            match existing {
                Some(scope) => *scope = widest(*scope, data.scope),
                None => {
                    module.add_order_entry(OrderEntry::Library {
                        library: data.library.clone(),
                        scope: data.scope,
                        exported: data.exported,
                    });
                }
            }
        }
    }
}

fn widest(a: DependencyScope, b: DependencyScope) -> DependencyScope {
    use DependencyScope::*;
    match (a, b) {
        (x, y) if x == y => x,
        (Test, other) | (other, Test) => other,
        (Compile, _) | (_, Compile) => Compile,
        // Runtime + provided = on every classpath.
        _ => Compile,
    }
}

struct DiagnosticDataService;

impl ProjectDataService for DiagnosticDataService {
    fn target_key(&self) -> DataKey {
        DataKey::Diagnostic
    }
    fn order(&self) -> i32 {
        100
    }
    fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
        for item in items {
            if let NodeData::Diagnostic(data) = &item.node.data {
                models.diagnostics.push(data.clone());
            }
        }
    }
}
