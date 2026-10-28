#![allow(dead_code)]
//! Test project builder: modules on disk, libraries from the local Maven repository, the JDK.

use archipel_ide_java::api::JavaProject;
use archipel_ide_java::index::JavaIndex;
use archipel_ide_projectmodel::{
    DependencyScope, JpsModuleSourceRootType, Library, Module, OrderEntry, ProjectModel, Sdk,
    SourceFolder,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn find_jar(group_path: &str, artifact: &str, prefer: Option<&str>) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let dir = home.join(".m2/repository").join(group_path).join(artifact);
    let mut versions: Vec<_> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    versions.sort();
    versions.reverse();
    for v in versions {
        let name = v.file_name()?.to_string_lossy().into_owned();
        if prefer.is_some_and(|p| name != p) {
            continue;
        }
        let jar = v.join(format!("{artifact}-{name}.jar"));
        if jar.exists() {
            return Some(jar);
        }
    }
    None
}

pub struct Proj {
    pub dir: tempfile::TempDir,
    pub root: PathBuf,
    pub project: JavaProject,
}

pub fn jdk_home() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.join("lib/modules").exists())
        .unwrap_or_else(|| PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64"))
}

/// One module `app` with `files` (project-relative) and libraries `(name, jar, scope)`.
pub fn project(files: &[(&str, &str)], libs: &[(&str, PathBuf, DependencyScope)]) -> Proj {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    for (p, t) in files {
        let path = root.join(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, t).unwrap();
    }
    let mut model = ProjectModel::new("demo", &root);
    let mut app = Module::new("app");
    app.content_entry_mut(&root)
        .add_source_folder(SourceFolder::new(
            root.join("src/main/java"),
            JpsModuleSourceRootType::Source,
        ))
        .add_source_folder(SourceFolder::new(
            root.join("src/test/java"),
            JpsModuleSourceRootType::TestSource,
        ));
    for (name, jar, scope) in libs {
        model.put_library(Library::new(*name).with_classes(jar));
        app.add_order_entry(OrderEntry::library(*name, *scope));
    }
    model.modules.push(app);
    model.put_sdk(Sdk::jdk_at(jdk_home(), Some("21")));
    model.project_sdk = Some("JDK 21".into());
    let model = Arc::new(model);
    let index = JavaIndex::in_memory();
    // Stubs of the JDK image and of the jars are shared by the projects of the test run.
    index.set_shared_cache(Some(
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("java-shared"),
    ));
    index.sync_blocking(model.clone());
    let project = JavaProject::new(index, model, false);
    Proj { dir, root, project }
}

impl Proj {
    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }
    pub fn analyze(&self, rel: &str, text: &str) -> Vec<(String, String)> {
        let a = self.project.analyze_text(&self.path(rel), text);
        a.problems
            .iter()
            .map(|p| {
                (
                    text[p.range.start as usize..p.range.end as usize].to_string(),
                    p.message.clone(),
                )
            })
            .collect()
    }
}

/// The hermetic fixture jar `name` (a directory of our own Java sources under `tests/fixtures`),
/// built with the JDK by `tools/java-fixtures/build.sh` into the target directory, named by the
/// hash of its sources (PLAT-IDX-034). A missing JDK fails the test; nothing is downloaded.
pub fn fixture_jar(name: &str) -> PathBuf {
    static BUILD: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("tests/fixtures").join(name);
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(&src)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no fixture sources in {}", src.display());
    // FNV-1a over names and contents.
    let mut hash: u64 = 0xcbf29ce484222325;
    for f in &files {
        let rel = f.strip_prefix(&src).unwrap().to_string_lossy().into_owned();
        for b in rel.bytes().chain(std::fs::read(f).unwrap()) {
            hash = (hash ^ u64::from(b)).wrapping_mul(0x100000001b3);
        }
    }
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("java-fixtures")
        .join(format!("{name}-{hash:016x}.jar"));
    let _guard = BUILD.lock().unwrap_or_else(|e| e.into_inner());
    if !out.exists() {
        let status = std::process::Command::new("bash")
            .arg(manifest.join("../../tools/java-fixtures/build.sh"))
            .arg(&src)
            .arg(&out)
            .status()
            .expect("bash is needed to build the fixture jars");
        assert!(
            status.success(),
            "tools/java-fixtures/build.sh failed for {name}; a JDK (javac, jar) is required"
        );
    }
    out
}

pub fn assertj() -> Option<PathBuf> {
    find_jar("org/assertj", "assertj-core", None)
}

pub fn exists(p: &Path) -> bool {
    p.exists()
}
