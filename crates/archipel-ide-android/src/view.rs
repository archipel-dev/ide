//! The "Android" project view: modules grouped into manifests, sources by
//! package, resources by type and qualifier, plus a Gradle Scripts node.
//!
//! The view is computed from a flat list of project-relative paths (the
//! project tree's file index), so it needs no extra filesystem access.

use crate::AndroidProject;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Module,
    Group,
    Package,
    SourceFile,
    ResourceType,
    ResourceGroup,
    File,
    GradleScripts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewNode {
    /// Stable id for expansion state.
    pub id: String,
    pub label: String,
    /// Grey suffix, e.g. `(test)`, `(Module :app)`, `(hdpi)`, `(5)`.
    pub detail: Option<String>,
    pub kind: NodeKind,
    /// Project-relative file to open, for leaves.
    pub path: Option<PathBuf>,
    pub children: Vec<ViewNode>,
}

impl ViewNode {
    fn group(id: String, label: &str, kind: NodeKind, children: Vec<ViewNode>) -> Self {
        Self {
            id,
            label: label.to_owned(),
            detail: None,
            kind,
            path: None,
            children,
        }
    }

    fn leaf(path: &Path, label: String, detail: Option<String>, kind: NodeKind) -> Self {
        Self {
            id: format!("file:{}", path.display()),
            label,
            detail,
            kind,
            path: Some(path.to_owned()),
            children: Vec::new(),
        }
    }
}

/// Resource file name -> (qualifier, path) variants.
type ResourceNames<'a> = BTreeMap<String, Vec<(String, &'a PathBuf)>>;

const SOURCE_EXTENSIONS: [&str; 3] = ["kt", "java", "kts"];

/// Source sets in the order the reference IDE lists them.
fn source_set_rank(set: &str) -> (u8, String) {
    let rank = match set {
        "main" => 0,
        "androidTest" => 2,
        "test" => 3,
        _ => 1,
    };
    (rank, set.to_owned())
}

/// Splits `<module>/src/<set>/<rest…>` into (set, rest).
fn in_source_set<'a>(module: &Path, path: &'a Path) -> Option<(String, &'a Path)> {
    let rest = path.strip_prefix(module.join("src")).ok()?;
    let mut components = rest.components();
    let set = components.next()?.as_os_str().to_str()?.to_owned();
    Some((set, components.as_path()))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Builds the Android view for `project` from project-relative `files`
/// (directories may be included; they are ignored).
pub fn android_view(project: &AndroidProject, files: &[PathBuf]) -> Vec<ViewNode> {
    let mut modules: Vec<_> = project.modules.iter().collect();
    modules.sort_by(|a, b| a.gradle_path.cmp(&b.gradle_path));
    let mut nodes: Vec<ViewNode> = modules
        .iter()
        .map(|module| module_node(module.dir.as_path(), &module.gradle_path, files))
        .collect();
    nodes.push(gradle_scripts(project, files));
    nodes
}

fn module_node(dir: &Path, gradle_path: &str, files: &[PathBuf]) -> ViewNode {
    let name = gradle_path.trim_start_matches(':').replace(':', ".");
    let name = if name.is_empty() { "app".into() } else { name };
    let id = format!("module:{gradle_path}");
    let mut manifests = Vec::new();
    // (package, set) -> files
    let mut packages: BTreeMap<(String, (u8, String)), Vec<&PathBuf>> = BTreeMap::new();
    let mut kotlin = false;
    // type -> name -> [(qualifier, path)]
    let mut resources: BTreeMap<String, ResourceNames> = BTreeMap::new();
    for path in files {
        let Some((set, rest)) = in_source_set(dir, path) else {
            continue;
        };
        // Directories (and extension-less files) are not shown as leaves.
        if path.extension().is_none() {
            continue;
        }
        let mut parts = rest.components().map(|c| c.as_os_str().to_string_lossy());
        let Some(first) = parts.next() else { continue };
        let tail: Vec<String> = parts.map(|c| c.into_owned()).collect();
        match first.as_ref() {
            "AndroidManifest.xml" if tail.is_empty() => {
                let detail = (set != "main").then(|| format!("({set})"));
                manifests.push(ViewNode::leaf(
                    path,
                    "AndroidManifest.xml".into(),
                    detail,
                    NodeKind::File,
                ));
            }
            "java" | "kotlin" if !tail.is_empty() => {
                let extension = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or_default();
                if !SOURCE_EXTENSIONS.contains(&extension) {
                    continue;
                }
                kotlin |= extension == "kt" || first == "kotlin";
                let package = tail[..tail.len() - 1].join(".");
                packages
                    .entry((package, source_set_rank(&set)))
                    .or_default()
                    .push(path);
            }
            "res" if tail.len() == 2 => {
                let folder = &tail[0];
                let (kind, qualifier) = folder.split_once('-').unwrap_or((folder.as_str(), ""));
                resources
                    .entry(kind.to_owned())
                    .or_default()
                    .entry(tail[1].clone())
                    .or_default()
                    .push((qualifier.to_owned(), path));
            }
            _ => {}
        }
    }
    let mut children = Vec::new();
    if !manifests.is_empty() {
        manifests.sort_by(|a, b| a.detail.cmp(&b.detail));
        children.push(ViewNode::group(
            format!("{id}/manifests"),
            "manifests",
            NodeKind::Group,
            manifests,
        ));
    }
    if !packages.is_empty() {
        let label = if kotlin { "kotlin+java" } else { "java" };
        let mut package_nodes: Vec<ViewNode> = packages
            .into_iter()
            .map(|((package, (_, set)), mut files)| {
                files.sort();
                let children = files
                    .into_iter()
                    .map(|path| {
                        let name = file_name(path);
                        let label = name
                            .strip_suffix(".kt")
                            .or_else(|| name.strip_suffix(".java"))
                            .unwrap_or(&name)
                            .to_owned();
                        ViewNode::leaf(path, label, None, NodeKind::SourceFile)
                    })
                    .collect();
                let mut node = ViewNode::group(
                    format!("{id}/java/{set}/{package}"),
                    if package.is_empty() {
                        "<default package>"
                    } else {
                        &package
                    },
                    NodeKind::Package,
                    children,
                );
                node.detail = (set != "main").then(|| format!("({set})"));
                node
            })
            .collect();
        package_nodes.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
        // Keep main before other source sets for the same package.
        package_nodes.sort_by_key(|node| {
            let set = node
                .detail
                .as_deref()
                .unwrap_or("(main)")
                .trim_matches(['(', ')'])
                .to_owned();
            (node.label.clone(), source_set_rank(&set))
        });
        children.push(ViewNode::group(
            format!("{id}/java"),
            label,
            NodeKind::Group,
            package_nodes,
        ));
    }
    if !resources.is_empty() {
        let types = resources
            .into_iter()
            .map(|(kind, names)| {
                let entries = names
                    .into_iter()
                    .map(|(name, mut variants)| {
                        variants.sort();
                        if let [(qualifier, path)] = variants.as_slice() {
                            let detail = (!qualifier.is_empty()).then(|| format!("({qualifier})"));
                            return ViewNode::leaf(path, name, detail, NodeKind::File);
                        }
                        let count = variants.len();
                        let leaves = variants
                            .into_iter()
                            .map(|(qualifier, path)| {
                                let detail =
                                    (!qualifier.is_empty()).then(|| format!("({qualifier})"));
                                ViewNode::leaf(path, name.clone(), detail, NodeKind::File)
                            })
                            .collect();
                        let mut group = ViewNode::group(
                            format!("{id}/res/{kind}/{name}"),
                            &name,
                            NodeKind::ResourceGroup,
                            leaves,
                        );
                        group.detail = Some(format!("({count})"));
                        group
                    })
                    .collect();
                ViewNode::group(
                    format!("{id}/res/{kind}"),
                    &kind,
                    NodeKind::ResourceType,
                    entries,
                )
            })
            .collect();
        children.push(ViewNode::group(
            format!("{id}/res"),
            "res",
            NodeKind::Group,
            types,
        ));
    }
    ViewNode::group(id, &name, NodeKind::Module, children)
}

fn gradle_scripts(project: &AndroidProject, files: &[PathBuf]) -> ViewNode {
    let root_name = file_name(&project.root);
    let exists = |path: &Path| files.iter().any(|f| f == path);
    let mut scripts = Vec::new();
    let mut add = |path: PathBuf, detail: String| {
        if exists(&path) {
            scripts.push(ViewNode::leaf(
                &path,
                file_name(&path),
                Some(format!("({detail})")),
                NodeKind::File,
            ));
        }
    };
    for name in ["build.gradle.kts", "build.gradle"] {
        add(PathBuf::from(name), format!("Project: {root_name}"));
    }
    for module in &project.modules {
        add(
            module.build_file.clone(),
            format!("Module {}", module.gradle_path),
        );
        add(
            module.dir.join("proguard-rules.pro"),
            format!("ProGuard Rules for \"{}\"", module.gradle_path),
        );
    }
    add(
        PathBuf::from("gradle.properties"),
        "Project Properties".into(),
    );
    add(
        PathBuf::from("gradle/wrapper/gradle-wrapper.properties"),
        "Gradle Version".into(),
    );
    add(
        PathBuf::from("gradle/libs.versions.toml"),
        "Version Catalog".into(),
    );
    add(PathBuf::from("local.properties"), "SDK Location".into());
    for name in ["settings.gradle.kts", "settings.gradle"] {
        add(PathBuf::from(name), "Project Settings".into());
    }
    ViewNode::group(
        "gradle-scripts".into(),
        "Gradle Scripts",
        NodeKind::GradleScripts,
        scripts,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AndroidModule, ModuleKind};

    fn project() -> AndroidProject {
        AndroidProject {
            root: PathBuf::from("/work/MyApplication"),
            agp_version: None,
            has_gradle_wrapper: true,
            modules: vec![AndroidModule {
                gradle_path: ":app".into(),
                dir: PathBuf::from("app"),
                kind: ModuleKind::Application,
                build_file: PathBuf::from("app/build.gradle.kts"),
                namespace: Some("com.example.myapplication".into()),
                application_id: None,
                compile_sdk: None,
                min_sdk: None,
                target_sdk: None,
                manifest: Some(PathBuf::from("app/src/main/AndroidManifest.xml")),
                launcher: None,
                variants: crate::variants::VariantModel::parse(""),
            }],
            problems: Vec::new(),
        }
    }

    fn files() -> Vec<PathBuf> {
        [
            "app",
            "app/build.gradle.kts",
            "app/proguard-rules.pro",
            "app/src/main/AndroidManifest.xml",
            "app/src/debug/AndroidManifest.xml",
            "app/src/main/java/com/example/myapplication/MainActivity.kt",
            "app/src/main/java/com/example/myapplication/ui/Theme.kt",
            "app/src/test/java/com/example/myapplication/GreetingTest.kt",
            "app/src/androidTest/java/com/example/myapplication/ExampleInstrumentedTest.kt",
            "app/src/main/res/layout/activity_main.xml",
            "app/src/main/res/values/strings.xml",
            "app/src/main/res/values-night/themes.xml",
            "app/src/main/res/values/themes.xml",
            "app/src/main/res/mipmap-hdpi/ic_launcher.webp",
            "app/src/main/res/mipmap-xhdpi/ic_launcher.webp",
            "app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml",
            "app/build/generated/R.java",
            "build.gradle.kts",
            "settings.gradle.kts",
            "gradle.properties",
            "gradle/libs.versions.toml",
            "gradle/wrapper/gradle-wrapper.properties",
            "local.properties",
        ]
        .iter()
        .map(PathBuf::from)
        .collect()
    }

    fn render(nodes: &[ViewNode], depth: usize, out: &mut Vec<String>) {
        for node in nodes {
            let detail = node
                .detail
                .as_deref()
                .map(|d| format!(" {d}"))
                .unwrap_or_default();
            out.push(format!("{}{}{detail}", "  ".repeat(depth), node.label));
            render(&node.children, depth + 1, out);
        }
    }

    #[test]
    fn builds_the_android_view() {
        let nodes = android_view(&project(), &files());
        let mut lines = Vec::new();
        render(&nodes, 0, &mut lines);
        assert_eq!(
            lines,
            [
                "app",
                "  manifests",
                "    AndroidManifest.xml",
                "    AndroidManifest.xml (debug)",
                "  kotlin+java",
                "    com.example.myapplication",
                "      MainActivity",
                "    com.example.myapplication (androidTest)",
                "      ExampleInstrumentedTest",
                "    com.example.myapplication (test)",
                "      GreetingTest",
                "    com.example.myapplication.ui",
                "      Theme",
                "  res",
                "    layout",
                "      activity_main.xml",
                "    mipmap",
                "      ic_launcher.webp (2)",
                "        ic_launcher.webp (hdpi)",
                "        ic_launcher.webp (xhdpi)",
                "      ic_launcher.xml (anydpi-v26)",
                "    values",
                "      strings.xml",
                "      themes.xml (2)",
                "        themes.xml",
                "        themes.xml (night)",
                "Gradle Scripts",
                "  build.gradle.kts (Project: MyApplication)",
                "  build.gradle.kts (Module :app)",
                "  proguard-rules.pro (ProGuard Rules for \":app\")",
                "  gradle.properties (Project Properties)",
                "  gradle-wrapper.properties (Gradle Version)",
                "  libs.versions.toml (Version Catalog)",
                "  local.properties (SDK Location)",
                "  settings.gradle.kts (Project Settings)",
            ]
        );
        let main = &nodes[0].children[1].children[0].children[0];
        assert_eq!(
            main.path.as_deref(),
            Some(Path::new(
                "app/src/main/java/com/example/myapplication/MainActivity.kt"
            ))
        );
    }

    #[test]
    fn java_only_modules_and_empty_input() {
        let files = vec![PathBuf::from("app/src/main/java/a/B.java")];
        let nodes = android_view(&project(), &files);
        assert_eq!(nodes[0].children[0].label, "java");
        assert_eq!(nodes[0].children[0].children[0].children[0].label, "B");
        let nodes = android_view(&project(), &[]);
        assert!(nodes[0].children.is_empty());
        assert!(nodes[1].children.is_empty());
    }
}
