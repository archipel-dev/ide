//! Maven: source, test and resource roots come from the POMs (`build.sourceDirectory`,
//! `testSourceDirectory`, `resources`, `testResources`, `directory`, `outputDirectory`,
//! build-helper `add-source`), the modules from `<modules>`. Dependencies come from
//! `mvn -q dependency:build-classpath` run once per scope (`compile`, `runtime`, `test`) and
//! classified from set differences; source and javadoc jars are the ones already in the local
//! repository.
//!
//! Maven is the project's `mvnw` or the managed/`PATH` `mvn`. Resolution first runs offline
//! (`-o`) and repeats online when that leaves a module without a classpath. When Maven cannot
//! run, the model still has all roots and a diagnostic says dependencies are missing.

use crate::artifacts::sibling_artifacts;
use crate::data::*;
use crate::jdk::detect_jdk;
use crate::process::run_plan;
use crate::resolver::{ImportError, ProjectResolver, ResolveRequest};
use archipel_ide_projectmodel::{Coordinates, DependencyScope, ModuleType};
use archipel_ide_runner::{TaskPlan, build_system::Maven};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct MavenProjectResolver;

const TIME_LIMIT: Duration = Duration::from_secs(20 * 60);

impl ProjectResolver for MavenProjectResolver {
    fn system_id(&self) -> ProjectSystemId {
        ProjectSystemId::Maven
    }

    fn is_applicable(&self, root: &Path) -> bool {
        root.join("pom.xml").is_file()
    }

    fn is_build_input(&self, path: &Path) -> bool {
        path.file_name().is_some_and(|n| n == "pom.xml")
            || path.ends_with(".mvn/maven.config")
            || path.ends_with(".mvn/jvm.config")
    }

    fn resolve(&self, request: &ResolveRequest) -> Result<DataNode, ImportError> {
        request.progress("Maven: reading POMs");
        let reactor = read_reactor(&request.root)?;
        let mut classpaths = Classpaths::default();
        let mut notes = Vec::new();
        if reactor
            .iter()
            .any(|p| p.packaging != "pom" || !p.dependencies.is_empty())
        {
            match resolve_classpaths(request, &reactor) {
                Ok(found) => classpaths = found,
                Err(ImportError::Cancelled) => return Err(ImportError::Cancelled),
                Err(error) => notes.push(format!(
                    "dependencies were not resolved ({error}); sources are imported without libraries"
                )),
            }
        }
        Ok(to_data_nodes(&request.root, &reactor, &classpaths, &notes))
    }
}

// ---------------------------------------------------------------------------------------
// POM model
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PomDependency {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub scope: String,
    pub classifier: String,
    pub kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pom {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub packaging: String,
    pub parent: Option<PomParent>,
    pub modules: Vec<String>,
    pub properties: HashMap<String, String>,
    pub source_directory: Option<String>,
    pub test_source_directory: Option<String>,
    pub directory: Option<String>,
    pub output_directory: Option<String>,
    pub test_output_directory: Option<String>,
    pub resources: Vec<String>,
    pub test_resources: Vec<String>,
    /// build-helper-maven-plugin `add-source` / `add-test-source` directories.
    pub added_sources: Vec<String>,
    pub added_test_sources: Vec<String>,
    pub dependencies: Vec<PomDependency>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PomParent {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub relative_path: Option<String>,
}

/// Parses the parts of a POM the importer needs. Profiles and `dependencyManagement` are
/// ignored.
pub fn parse_pom(text: &str) -> Result<Pom, String> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut pom = Pom::default();
    let mut stack: Vec<String> = Vec::new();
    let mut dep = PomDependency::default();
    let mut parent = PomParent::default();
    let mut plugin_artifact = String::new();
    let mut execution_goals: Vec<String> = Vec::new();
    let mut execution_sources: Vec<String> = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("invalid POM: {e}"))?
        {
            Event::Start(start) => {
                let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
                stack.push(name);
                match path(&stack).as_str() {
                    "project/dependencies/dependency" => dep = PomDependency::default(),
                    "project/parent" => parent = PomParent::default(),
                    "project/build/plugins/plugin" => plugin_artifact.clear(),
                    "project/build/plugins/plugin/executions/execution" => {
                        execution_goals.clear();
                        execution_sources.clear();
                    }
                    _ => {}
                }
            }
            Event::Empty(_) => {}
            Event::End(_) => {
                match path(&stack).as_str() {
                    "project/dependencies/dependency" => {
                        let mut finished = std::mem::take(&mut dep);
                        if finished.scope.is_empty() {
                            finished.scope = "compile".into();
                        }
                        if finished.kind.is_empty() {
                            finished.kind = "jar".into();
                        }
                        pom.dependencies.push(finished);
                    }
                    "project/parent" => pom.parent = Some(std::mem::take(&mut parent)),
                    "project/build/plugins/plugin/executions/execution"
                        if plugin_artifact == "build-helper-maven-plugin" =>
                    {
                        if execution_goals.iter().any(|g| g == "add-source") {
                            pom.added_sources.extend(execution_sources.iter().cloned());
                        }
                        if execution_goals.iter().any(|g| g == "add-test-source") {
                            pom.added_test_sources
                                .extend(execution_sources.iter().cloned());
                        }
                    }
                    _ => {}
                }
                stack.pop();
            }
            Event::Text(text) => {
                let value = text
                    .unescape()
                    .map_err(|e| format!("invalid POM text: {e}"))?
                    .trim()
                    .to_owned();
                if value.is_empty() {
                    continue;
                }
                let at = path(&stack);
                match at.as_str() {
                    "project/groupId" => pom.group_id = value,
                    "project/artifactId" => pom.artifact_id = value,
                    "project/version" => pom.version = value,
                    "project/packaging" => pom.packaging = value,
                    "project/parent/groupId" => parent.group_id = value,
                    "project/parent/artifactId" => parent.artifact_id = value,
                    "project/parent/version" => parent.version = value,
                    "project/parent/relativePath" => parent.relative_path = Some(value),
                    "project/modules/module" => pom.modules.push(value),
                    "project/build/sourceDirectory" => pom.source_directory = Some(value),
                    "project/build/testSourceDirectory" => pom.test_source_directory = Some(value),
                    "project/build/directory" => pom.directory = Some(value),
                    "project/build/outputDirectory" => pom.output_directory = Some(value),
                    "project/build/testOutputDirectory" => pom.test_output_directory = Some(value),
                    "project/build/resources/resource/directory" => pom.resources.push(value),
                    "project/build/testResources/testResource/directory" => {
                        pom.test_resources.push(value)
                    }
                    "project/dependencies/dependency/groupId" => dep.group_id = value,
                    "project/dependencies/dependency/artifactId" => dep.artifact_id = value,
                    "project/dependencies/dependency/version" => dep.version = value,
                    "project/dependencies/dependency/scope" => dep.scope = value,
                    "project/dependencies/dependency/classifier" => dep.classifier = value,
                    "project/dependencies/dependency/type" => dep.kind = value,
                    "project/build/plugins/plugin/artifactId" => plugin_artifact = value,
                    "project/build/plugins/plugin/executions/execution/goals/goal" => {
                        execution_goals.push(value)
                    }
                    "project/build/plugins/plugin/executions/execution/configuration/sources/source" => {
                        execution_sources.push(value)
                    }
                    other => {
                        if let Some(key) = other.strip_prefix("project/properties/") {
                            pom.properties.insert(key.to_owned(), value);
                        }
                    }
                }
            }
            Event::CData(data) => {
                // Not used by any field we read.
                let _ = data;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if pom.packaging.is_empty() {
        pom.packaging = "jar".into();
    }
    Ok(pom)
}

fn path(stack: &[String]) -> String {
    stack.join("/")
}

// ---------------------------------------------------------------------------------------
// Reactor
// ---------------------------------------------------------------------------------------

/// A POM of the reactor with everything inherited from reactor parents applied.
#[derive(Debug, Clone)]
pub struct ReactorProject {
    pub dir: PathBuf,
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub packaging: String,
    pub source_directory: PathBuf,
    pub test_source_directory: PathBuf,
    pub resources: Vec<PathBuf>,
    pub test_resources: Vec<PathBuf>,
    pub added_sources: Vec<PathBuf>,
    pub added_test_sources: Vec<PathBuf>,
    pub directory: PathBuf,
    pub output_directory: PathBuf,
    pub test_output_directory: PathBuf,
    pub language_level: Option<String>,
    pub dependencies: Vec<PomDependency>,
}

impl ReactorProject {
    pub fn id(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }
}

/// Reads the root POM and, recursively, its `<modules>`.
pub fn read_reactor(root: &Path) -> Result<Vec<ReactorProject>, ImportError> {
    let mut out = Vec::new();
    let mut seen = Vec::new();
    read_project(root, None, &mut out, &mut seen)?;
    Ok(out)
}

fn read_project(
    dir: &Path,
    inherited: Option<&Pom>,
    out: &mut Vec<ReactorProject>,
    seen: &mut Vec<PathBuf>,
) -> Result<(), ImportError> {
    let pom_file = if dir.is_file() {
        dir.to_owned()
    } else {
        dir.join("pom.xml")
    };
    let dir = pom_file.parent().unwrap_or(dir).to_owned();
    let key = std::fs::canonicalize(&pom_file).unwrap_or_else(|_| pom_file.clone());
    if seen.contains(&key) {
        return Ok(());
    }
    seen.push(key);
    let text = std::fs::read_to_string(&pom_file)
        .map_err(|e| ImportError::Io(format!("{}: {e}", pom_file.display())))?;
    let pom = parse_pom(&text)
        .map_err(|e| ImportError::Invalid(format!("{}: {e}", pom_file.display())))?;

    // Build settings and properties are inherited from the reactor parent.
    let mut properties = inherited.map(|p| p.properties.clone()).unwrap_or_default();
    properties.extend(pom.properties.clone());
    let group_id = if pom.group_id.is_empty() {
        pom.parent
            .as_ref()
            .map(|p| p.group_id.clone())
            .or_else(|| inherited.map(|p| p.group_id.clone()))
            .unwrap_or_default()
    } else {
        pom.group_id.clone()
    };
    let version = if pom.version.is_empty() {
        pom.parent
            .as_ref()
            .map(|p| p.version.clone())
            .or_else(|| inherited.map(|p| p.version.clone()))
            .unwrap_or_default()
    } else {
        pom.version.clone()
    };
    let basedir = dir.display().to_string();
    let (group_text, version_text) = (group_id.clone(), version.clone());
    let interpolate = |value: &str, directory: &str| -> String {
        let mut text = value.to_owned();
        for _ in 0..4 {
            let before = text.clone();
            text = text
                .replace("${project.basedir}", &basedir)
                .replace("${basedir}", &basedir)
                .replace("${pom.basedir}", &basedir)
                .replace("${project.build.directory}", directory)
                .replace("${project.artifactId}", &pom.artifact_id)
                .replace("${project.groupId}", &group_text)
                .replace("${project.version}", &version_text);
            for (key, replacement) in &properties {
                text = text.replace(&format!("${{{key}}}"), replacement);
            }
            if text == before {
                break;
            }
        }
        text
    };
    let pick = |own: &Option<String>, parent: Option<&Option<String>>| -> Option<String> {
        own.clone().or_else(|| parent.and_then(Clone::clone))
    };
    let parent_build = inherited.map(|p| {
        (
            &p.source_directory,
            &p.test_source_directory,
            &p.directory,
            &p.output_directory,
            &p.test_output_directory,
        )
    });
    let directory_text = pick(&pom.directory, parent_build.map(|b| b.2))
        .map(|d| interpolate(&d, "${project.basedir}/target"))
        .unwrap_or_else(|| "target".into());
    let absolute = |value: &str| -> PathBuf {
        let path = PathBuf::from(value);
        archipel_ide_projectmodel::path::normalize(&if path.is_absolute() {
            path
        } else {
            dir.join(path)
        })
    };
    let directory = absolute(&directory_text);
    let directory_str = directory.display().to_string();
    let dir_of = |own: &Option<String>, parent: Option<&Option<String>>, default: &str| {
        absolute(
            &pick(own, parent)
                .map(|v| interpolate(&v, &directory_str))
                .unwrap_or_else(|| default.to_owned()),
        )
    };
    let source_directory = dir_of(
        &pom.source_directory,
        parent_build.map(|b| b.0),
        "src/main/java",
    );
    let test_source_directory = dir_of(
        &pom.test_source_directory,
        parent_build.map(|b| b.1),
        "src/test/java",
    );
    let output_directory = dir_of(
        &pom.output_directory,
        parent_build.map(|b| b.3),
        &format!("{directory_str}/classes"),
    );
    let test_output_directory = dir_of(
        &pom.test_output_directory,
        parent_build.map(|b| b.4),
        &format!("{directory_str}/test-classes"),
    );
    let resource_dirs = |own: &[String], default: &str| -> Vec<PathBuf> {
        if own.is_empty() {
            vec![absolute(default)]
        } else {
            own.iter()
                .map(|v| absolute(&interpolate(v, &directory_str)))
                .collect()
        }
    };
    let resources = resource_dirs(&pom.resources, "src/main/resources");
    let test_resources = resource_dirs(&pom.test_resources, "src/test/resources");
    let language_level = [
        "maven.compiler.release",
        "maven.compiler.source",
        "java.version",
    ]
    .iter()
    .find_map(|key| properties.get(*key))
    .map(|v| interpolate(v, &directory_str));

    let project = ReactorProject {
        dir: dir.clone(),
        group_id,
        artifact_id: pom.artifact_id.clone(),
        version,
        packaging: pom.packaging.clone(),
        source_directory,
        test_source_directory,
        resources,
        test_resources,
        added_sources: pom
            .added_sources
            .iter()
            .map(|v| absolute(&interpolate(v, &directory_str)))
            .collect(),
        added_test_sources: pom
            .added_test_sources
            .iter()
            .map(|v| absolute(&interpolate(v, &directory_str)))
            .collect(),
        directory,
        output_directory,
        test_output_directory,
        language_level,
        dependencies: pom.dependencies.clone(),
    };
    out.push(project);

    // What children inherit: this POM's build block merged over the inherited one.
    let mut for_children = pom.clone();
    for_children.properties = properties;
    for_children.group_id = out.last().map(|p| p.group_id.clone()).unwrap_or_default();
    for_children.version = out.last().map(|p| p.version.clone()).unwrap_or_default();
    if let Some(parent) = inherited {
        for_children.source_directory = pick(&pom.source_directory, Some(&parent.source_directory));
        for_children.test_source_directory = pick(
            &pom.test_source_directory,
            Some(&parent.test_source_directory),
        );
        for_children.directory = pick(&pom.directory, Some(&parent.directory));
        for_children.output_directory = pick(&pom.output_directory, Some(&parent.output_directory));
        for_children.test_output_directory = pick(
            &pom.test_output_directory,
            Some(&parent.test_output_directory),
        );
    }
    for module in &pom.modules {
        let child = dir.join(module);
        if child.join("pom.xml").is_file() || (child.is_file() && child.extension().is_some()) {
            read_project(&child, Some(&for_children), out, seen)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Dependency resolution through Maven
// ---------------------------------------------------------------------------------------

/// Resolved jars per module (keyed by module directory) for each scope selector.
#[derive(Debug, Default, Clone)]
pub struct Classpaths {
    /// module dir -> scope -> jar paths
    pub by_module: BTreeMap<PathBuf, ScopedClasspath>,
    pub failures: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScopedClasspath {
    /// `-DincludeScope=compile`: compile, provided, system.
    pub compile: Vec<PathBuf>,
    /// `includeScope=runtime`: compile, runtime.
    pub runtime: Vec<PathBuf>,
    /// `includeScope=test`: everything.
    pub test: Vec<PathBuf>,
}

impl ScopedClasspath {
    /// Classifies every entry: compile = on both compile and runtime classpath, provided =
    /// compile only, runtime = runtime only, test = test only.
    pub fn classify(&self) -> Vec<(PathBuf, DependencyScope)> {
        let mut out: Vec<(PathBuf, DependencyScope)> = Vec::new();
        for entry in self.test.iter().chain(&self.compile).chain(&self.runtime) {
            if out.iter().any(|(p, _)| p == entry) {
                continue;
            }
            let in_compile = self.compile.contains(entry);
            let in_runtime = self.runtime.contains(entry);
            out.push((
                entry.clone(),
                DependencyScope::from_classpaths(
                    in_compile,
                    in_runtime,
                    !in_compile && !in_runtime,
                ),
            ));
        }
        // Keep Maven's classpath order (the test classpath is the complete one).
        let order = |p: &PathBuf| self.test.iter().position(|e| e == p).unwrap_or(usize::MAX);
        out.sort_by_key(|(p, _)| order(p));
        out
    }
}

const SCOPES: [&str; 3] = ["compile", "runtime", "test"];

fn classpath_file(scope: &str) -> String {
    format!("target/archipel-classpath-{scope}.txt")
}

fn maven_plan(
    request: &ResolveRequest,
    reactor_has_siblings: bool,
    scope: &str,
    offline: bool,
) -> Result<TaskPlan, ImportError> {
    let program =
        Maven::program(&request.root).map_err(|e| ImportError::ToolMissing(e.to_string()))?;
    let mut args = vec!["-q".to_owned(), "--fail-at-end".to_owned(), "-B".to_owned()];
    if offline {
        args.push("-o".into());
    }
    if reactor_has_siblings {
        // Sibling modules resolve to their class directories once the compile phase ran;
        // skipping the compiler keeps that cheap and independent of compile errors.
        args.extend([
            "-Dmaven.main.skip=true".to_owned(),
            "-Dmaven.resources.skip=true".to_owned(),
            "compile".to_owned(),
        ]);
    }
    args.extend([
        "dependency:build-classpath".to_owned(),
        format!("-Dmdep.outputFile={}", classpath_file(scope)),
        format!("-DincludeScope={scope}"),
        format!(
            "-Dmdep.pathSeparator={}",
            if cfg!(windows) { ';' } else { ':' }
        ),
    ]);
    Ok(TaskPlan::new("Maven Sync", program, args, &request.root))
}

fn resolve_classpaths(
    request: &ResolveRequest,
    reactor: &[ReactorProject],
) -> Result<Classpaths, ImportError> {
    let siblings = reactor.len() > 1;
    let mut result = Classpaths::default();
    let attempts: &[bool] = if request.settings.offline_only {
        &[true]
    } else {
        &[true, false]
    };
    for (n, &offline) in attempts.iter().enumerate() {
        result = Classpaths::default();
        let mut missing = Vec::new();
        let mut errors = Vec::new();
        for scope in SCOPES {
            request.cancel.check()?;
            request.progress(&format!(
                "Maven: resolving {scope} dependencies{}",
                if offline { " (offline)" } else { "" }
            ));
            let plan = maven_plan(request, siblings, scope, offline)?;
            for project in reactor {
                let _ = std::fs::remove_file(project.dir.join(classpath_file(scope)));
            }
            let captured = run_plan(&plan, &request.cancel, TIME_LIMIT)?;
            if !captured.success() {
                errors.push(captured.tail());
            }
            for project in reactor {
                let file = project.dir.join(classpath_file(scope));
                let Ok(text) = std::fs::read_to_string(&file) else {
                    if project.packaging != "pom" {
                        missing.push(project.artifact_id.clone());
                    }
                    continue;
                };
                let _ = std::fs::remove_file(&file);
                let entries = split_classpath(&text);
                let slot = result.by_module.entry(project.dir.clone()).or_default();
                match scope {
                    "compile" => slot.compile = entries,
                    "runtime" => slot.runtime = entries,
                    _ => slot.test = entries,
                }
            }
            // Maven removes nothing, but leave no empty `target` directories we created.
            for project in reactor {
                let target = project.dir.join("target");
                let _ = std::fs::remove_dir(target);
            }
        }
        missing.sort();
        missing.dedup();
        if missing.is_empty() {
            return Ok(result);
        }
        result.failures = missing
            .iter()
            .map(|m| format!("{m}: dependencies could not be resolved"))
            .collect();
        if n + 1 == attempts.len() {
            if result.by_module.is_empty() {
                return Err(ImportError::ToolFailed {
                    tool: "Maven".into(),
                    message: errors.into_iter().next().unwrap_or_default(),
                });
            }
            return Ok(result);
        }
    }
    Ok(result)
}

/// Splits `mvn dependency:build-classpath` output into paths.
pub fn split_classpath(text: &str) -> Vec<PathBuf> {
    let separator = if cfg!(windows) { ';' } else { ':' };
    text.trim()
        .split(separator)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

// ---------------------------------------------------------------------------------------
// Local repository coordinates
// ---------------------------------------------------------------------------------------

/// The local repository directories jars may live in.
fn local_repositories() -> Vec<PathBuf> {
    let mut repos = Vec::new();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    if let Some(home) = &home {
        if let Ok(settings) = std::fs::read_to_string(home.join(".m2/settings.xml"))
            && let Some(start) = settings.find("<localRepository>")
            && let Some(end) = settings[start..].find("</localRepository>")
        {
            let value = settings[start + "<localRepository>".len()..start + end].trim();
            if !value.is_empty() {
                repos.push(PathBuf::from(
                    value.replace("${user.home}", &home.display().to_string()),
                ));
            }
        }
        repos.push(home.join(".m2/repository"));
    }
    repos
}

/// `group:artifact:version[:classifier]` of a jar in a Maven repository layout:
/// `<repo>/<group path>/<artifact>/<version>/<artifact>-<version>[-<classifier>].jar`.
pub fn coordinates_from_repository_path(
    jar: &Path,
    repositories: &[PathBuf],
) -> Option<(Coordinates, Option<String>)> {
    let relative = repositories
        .iter()
        .find_map(|repo| jar.strip_prefix(repo).ok())
        .map(Path::to_owned)
        .or_else(|| {
            // Unknown repository location: the conventional `.m2/repository` marker.
            let text = jar.to_string_lossy();
            let at = text.find("/repository/")?;
            Some(PathBuf::from(&text[at + "/repository/".len()..]))
        })?;
    let parts: Vec<String> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.len() < 4 {
        return None;
    }
    let file = parts.last()?;
    let version = &parts[parts.len() - 2];
    let artifact = &parts[parts.len() - 3];
    let group = parts[..parts.len() - 3].join(".");
    let stem = file.rsplit_once('.')?.0;
    let prefix = format!("{artifact}-{version}");
    let classifier = stem
        .strip_prefix(&prefix)
        .and_then(|rest| rest.strip_prefix('-'))
        .map(str::to_owned);
    Some((Coordinates::new(group, artifact, version), classifier))
}

// ---------------------------------------------------------------------------------------
// Data nodes
// ---------------------------------------------------------------------------------------

pub fn to_data_nodes(
    root: &Path,
    reactor: &[ReactorProject],
    classpaths: &Classpaths,
    notes: &[String],
) -> DataNode {
    to_data_nodes_with(
        root,
        reactor,
        classpaths,
        notes,
        &local_repositories(),
        detect_jdk(),
    )
}

pub(crate) fn to_data_nodes_with(
    root: &Path,
    reactor: &[ReactorProject],
    classpaths: &Classpaths,
    notes: &[String],
    repositories: &[PathBuf],
    jdk: Option<SdkData>,
) -> DataNode {
    let root_project = reactor.first();
    let mut project = DataNode::new(NodeData::Project(ProjectData {
        system: ProjectSystemId::Maven,
        name: root_project
            .map(|p| p.artifact_id.clone())
            .unwrap_or_else(|| "project".into()),
        root: root.to_owned(),
        sdk: jdk.as_ref().map(|j| j.name.clone()),
    }));
    if let Some(jdk) = jdk {
        project.add(NodeData::Sdk(jdk));
    }
    let mut libraries: BTreeMap<String, LibraryData> = BTreeMap::new();
    let mut modules = Vec::new();
    for maven in reactor {
        let mut module = DataNode::new(NodeData::Module(ModuleData {
            id: maven.id(),
            name: maven.artifact_id.clone(),
            module_type: ModuleType::Java,
            path: maven.dir.clone(),
            language_level: maven.language_level.clone(),
            compile_output: vec![maven.output_directory.clone()],
            test_compile_output: vec![maven.test_output_directory.clone()],
        }));
        let mut content = ContentRootData::new(&maven.dir);
        if maven.packaging != "pom" {
            use ExternalSystemSourceType as T;
            content.store_path(T::Source, &maven.source_directory);
            for path in &maven.added_sources {
                content.store_path(T::Source, path);
            }
            for path in &maven.resources {
                content.store_path(T::Resource, path);
            }
            content.store_path(T::Test, &maven.test_source_directory);
            for path in &maven.added_test_sources {
                content.store_path(T::Test, path);
            }
            for path in &maven.test_resources {
                content.store_path(T::TestResource, path);
            }
            content.store_path(
                T::SourceGenerated,
                maven
                    .directory
                    .join("generated-sources")
                    .join("annotations"),
            );
            content.store_path(
                T::TestGenerated,
                maven
                    .directory
                    .join("generated-test-sources")
                    .join("test-annotations"),
            );
        }
        content.store_path(ExternalSystemSourceType::Excluded, &maven.directory);
        module.add(NodeData::ContentRoot(content));

        // Module dependencies declared in the POM (direct), then everything the classpath adds.
        let mut module_deps: Vec<(String, DependencyScope)> = Vec::new();
        for dep in &maven.dependencies {
            let Some(target) = reactor.iter().find(|p| {
                p.artifact_id == dep.artifact_id
                    && (dep.group_id.is_empty() || p.group_id == dep.group_id)
            }) else {
                continue;
            };
            let scope = DependencyScope::from_id(match dep.scope.as_str() {
                "system" | "import" => "provided",
                other => other,
            })
            .unwrap_or_default();
            if target.dir != maven.dir && !module_deps.iter().any(|(id, _)| *id == target.id()) {
                module_deps.push((target.id(), scope));
            }
        }
        let mut library_deps: Vec<(String, DependencyScope)> = Vec::new();
        if let Some(scoped) = classpaths.by_module.get(&maven.dir) {
            for (entry, scope) in scoped.classify() {
                if let Some(owner) = reactor
                    .iter()
                    .find(|p| p.dir != maven.dir && entry.starts_with(&p.directory))
                {
                    if !module_deps.iter().any(|(id, _)| *id == owner.id()) {
                        module_deps.push((owner.id(), scope));
                    }
                    continue;
                }
                if !entry.is_file() && !entry.extension().is_some_and(|e| e == "jar") {
                    continue;
                }
                let (coordinates, classifier) =
                    match coordinates_from_repository_path(&entry, repositories) {
                        Some(found) => (Some(found.0), found.1),
                        None => (None, None),
                    };
                let name = match (&coordinates, &classifier) {
                    (Some(c), Some(cl)) => format!("Maven: {c}:{cl}"),
                    (Some(c), None) => format!("Maven: {c}"),
                    (None, _) => format!(
                        "Maven: {}",
                        entry.file_name().and_then(|n| n.to_str()).unwrap_or("jar")
                    ),
                };
                libraries.entry(name.clone()).or_insert_with(|| {
                    let mut library = LibraryData::new(&name);
                    library.binary.push(entry.clone());
                    let (sources, docs) = sibling_artifacts(&entry);
                    library.source = sources;
                    library.documentation = docs;
                    library.coordinates = coordinates;
                    library
                });
                library_deps.push((name, scope));
            }
        }
        for (target, scope) in module_deps {
            module.add(NodeData::ModuleDependency(ModuleDependencyData {
                target,
                scope,
                exported: false,
            }));
        }
        for (library, scope) in library_deps {
            module.add(NodeData::LibraryDependency(LibraryDependencyData {
                library,
                scope,
                exported: false,
            }));
        }
        modules.push(module);
    }
    for library in libraries.into_values() {
        project.add(NodeData::Library(library));
    }
    project.children.extend(modules);
    for failure in &classpaths.failures {
        project.add(NodeData::Diagnostic(DiagnosticData {
            severity: Severity::Warning,
            message: failure.clone(),
        }));
    }
    for note in notes {
        project.add(NodeData::Diagnostic(DiagnosticData {
            severity: Severity::Warning,
            message: format!("Maven: {note}"),
        }));
    }
    project
}
