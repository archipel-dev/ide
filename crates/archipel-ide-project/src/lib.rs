use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
use walkdir::WalkDir;

pub mod android_template;
pub mod generator;
pub mod recent;
mod watcher;

pub use watcher::{FileChanges, ProjectWatcher};

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project path is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("there is no directory at {0}")]
    NotFound(PathBuf),
    #[error("cannot create the project location {path}: {source}")]
    CreateLocation {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("path is outside the project: {0}")]
    OutsideRoot(PathBuf),
    #[error("file is not valid UTF-8: {0}")]
    NonUtf8(PathBuf),
    #[error("already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("could not finish creating project at {root}; partial files were kept: {source}")]
    Generation {
        root: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid name `{0}`: use a relative path without `..`")]
    InvalidName(String),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub relative_path: PathBuf,
    /// A directory, or a symbolic link to one.
    pub is_dir: bool,
    /// The entry itself is a symbolic link. Linked directories are listed lazily
    /// when expanded rather than walked (they often point at large build trees).
    #[serde(default)]
    pub is_symlink: bool,
    /// A build-output directory (`target`, `node_modules`) that is shown but not
    /// walked; its children are listed when it is expanded.
    #[serde(default)]
    pub lazy: bool,
}

/// Names never shown in the Project tree. `.jetthing` is the data directory of versions
/// before the rename to Archipel IDE.
const HIDDEN: [&str; 3] = [".git", ".archipel", ".jetthing"];
/// Huge generated directories: listed (as excluded folders) but only walked on demand.
const LAZY: [&str; 2] = ["target", "node_modules"];

fn is_lazy_dir(name: &std::ffi::OsStr, is_dir: bool) -> bool {
    is_dir && name.to_str().is_some_and(|name| LAZY.contains(&name))
}

/// Eclipse metadata that jdtls / Buildship generate next to a Gradle or Maven build.
/// (`jetthing-model.json` is what versions before the rename wrote.)
const GENERATED_METADATA: [&str; 5] = [
    ".classpath",
    ".project",
    ".settings",
    "archipel-model.json",
    "jetthing-model.json",
];

/// Whether the project-root entry `name` is an IDE-generated artefact (jdtls metadata or
/// its `bin/` class output) rather than a user file. Only applies to Gradle/Maven
/// projects, where Archipel IDE's language server creates them; `bin/` is hidden only when
/// it holds nothing but `.class` files.
pub fn is_generated_artefact(root: &Path, name: &str) -> bool {
    let built = [
        "build.gradle",
        "build.gradle.kts",
        "settings.gradle",
        "settings.gradle.kts",
        "pom.xml",
    ]
    .iter()
    .any(|file| root.join(file).is_file());
    if !built {
        return false;
    }
    if GENERATED_METADATA.contains(&name) {
        return true;
    }
    name == "bin" && only_class_files(&root.join("bin"))
}

fn only_class_files(dir: &Path) -> bool {
    WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .all(|e| e.path().extension().is_some_and(|ext| ext == "class"))
}

fn file_entry(path: &Path, root: &Path, file_type: fs::FileType) -> Option<FileEntry> {
    let is_symlink = file_type.is_symlink();
    Some(FileEntry {
        relative_path: path.strip_prefix(root).ok()?.to_owned(),
        is_dir: file_type.is_dir() || (is_symlink && path.is_dir()),
        is_symlink,
        lazy: false,
    })
}

#[derive(Debug, Clone)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let path = path.as_ref();
        if !path.exists() {
            // Name the path: the bare OS error ("No such file or directory")
            // leaves the user guessing which field was wrong.
            return Err(ProjectError::NotFound(path.to_owned()));
        }
        let root = path.canonicalize()?;
        if !root.is_dir() {
            return Err(ProjectError::NotDirectory(root));
        }
        Ok(Self { root })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn entries(&self, max: usize) -> Vec<FileEntry> {
        let mut walk = WalkDir::new(&self.root)
            .follow_links(false)
            .sort_by(|a, b| {
                // Project view order: directories first, then case-insensitive names.
                b.file_type()
                    .is_dir()
                    .cmp(&a.file_type().is_dir())
                    .then_with(|| {
                        let a = a.file_name().to_string_lossy().to_lowercase();
                        let b = b.file_name().to_string_lossy().to_lowercase();
                        a.cmp(&b)
                    })
            })
            .into_iter()
            .filter_entry(|e| {
                let Some(name) = e.file_name().to_str() else {
                    return true;
                };
                !HIDDEN.contains(&name)
                    && !(e.depth() == 1 && is_generated_artefact(&self.root, name))
            });
        let mut entries = Vec::new();
        while let Some(entry) = walk.next() {
            let Ok(entry) = entry else { continue };
            if entry.depth() == 0 {
                continue;
            }
            if entries.len() >= max {
                break;
            }
            let lazy = is_lazy_dir(entry.file_name(), entry.file_type().is_dir());
            if lazy {
                walk.skip_current_dir();
            }
            if let Some(mut file) = file_entry(entry.path(), &self.root, entry.file_type()) {
                file.lazy = lazy;
                entries.push(file);
            }
        }
        entries
    }

    /// Direct children of a directory (following symbolic links), in Project
    /// view order. Used to expand linked directories that `entries` does not walk.
    pub fn children(&self, relative: &Path) -> Result<Vec<FileEntry>, ProjectError> {
        let directory = self.checked_follow(relative)?;
        let mut children: Vec<FileEntry> = fs::read_dir(&directory)?
            .filter_map(Result::ok)
            .filter(|entry| {
                !entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| HIDDEN.contains(&name))
            })
            .filter(|entry| {
                !(relative.as_os_str().is_empty()
                    && entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| is_generated_artefact(&self.root, name)))
            })
            .filter_map(|entry| {
                let logical = self.root.join(relative).join(entry.file_name());
                file_entry(&logical, &self.root, entry.file_type().ok()?)
            })
            .collect();
        children.sort_by(|a, b| {
            b.is_dir.cmp(&a.is_dir).then_with(|| {
                let name = |entry: &FileEntry| {
                    entry
                        .relative_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default()
                };
                name(a).cmp(&name(b))
            })
        });
        Ok(children)
    }

    /// Resolves an existing project path for reading. Unlike [`Self::checked`]
    /// it may leave the root, but only through a symbolic link that lives in the
    /// project (e.g. Bazel's `bazel-bin`); `..` and absolute paths are rejected.
    fn checked_follow(&self, relative: &Path) -> Result<PathBuf, ProjectError> {
        use std::path::Component;
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return Err(ProjectError::InvalidName(relative.display().to_string()));
        }
        let candidate = self.root.join(relative);
        let canonical = candidate.canonicalize()?;
        if canonical.starts_with(&self.root) {
            return Ok(canonical);
        }
        let mut logical = self.root.clone();
        for component in relative.components() {
            logical.push(component);
            if logical
                .symlink_metadata()
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                return Ok(canonical);
            }
        }
        Err(ProjectError::OutsideRoot(candidate))
    }
    fn checked(&self, relative: &Path) -> Result<PathBuf, ProjectError> {
        let candidate = self.root.join(relative);
        let canonical = candidate.canonicalize()?;
        if !canonical.starts_with(&self.root) {
            return Err(ProjectError::OutsideRoot(candidate));
        }
        Ok(canonical)
    }
    pub fn read_text(&self, relative: &Path) -> Result<String, ProjectError> {
        let path = self.checked_follow(relative)?;
        String::from_utf8(fs::read(&path)?).map_err(|_| ProjectError::NonUtf8(path))
    }
    /// Validates a project-relative path that may not exist yet.
    fn checked_new(&self, relative: &Path) -> Result<PathBuf, ProjectError> {
        use std::path::Component;
        let invalid = || ProjectError::InvalidName(relative.display().to_string());
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(invalid());
        }
        let candidate = self.root.join(relative);
        // The nearest existing ancestor must resolve inside the root (no symlink escape).
        let mut ancestor = candidate.parent();
        while let Some(dir) = ancestor {
            if dir.exists() {
                if !dir.canonicalize()?.starts_with(&self.root) {
                    return Err(ProjectError::OutsideRoot(candidate));
                }
                break;
            }
            ancestor = dir.parent();
        }
        if candidate.symlink_metadata().is_ok() {
            return Err(ProjectError::AlreadyExists(relative.to_owned()));
        }
        Ok(candidate)
    }

    /// Creates an empty file, including missing parent directories.
    pub fn create_file(&self, relative: &Path) -> Result<(), ProjectError> {
        let path = self.checked_new(relative)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        Ok(())
    }

    pub fn create_dir(&self, relative: &Path) -> Result<(), ProjectError> {
        let path = self.checked_new(relative)?;
        fs::create_dir_all(path)?;
        Ok(())
    }

    /// Renames a file or directory in place; `new_name` is a single path segment.
    /// Returns the new project-relative path.
    pub fn rename(&self, relative: &Path, new_name: &str) -> Result<PathBuf, ProjectError> {
        if new_name.is_empty()
            || new_name.contains(['/', '\\'])
            || new_name == "."
            || new_name == ".."
        {
            return Err(ProjectError::InvalidName(new_name.to_owned()));
        }
        let from = self.checked(relative)?;
        if from == self.root {
            return Err(ProjectError::InvalidName(relative.display().to_string()));
        }
        let target = relative
            .parent()
            .map(|parent| parent.join(new_name))
            .unwrap_or_else(|| PathBuf::from(new_name));
        let to = self.checked_new(&target)?;
        fs::rename(from, to)?;
        Ok(target)
    }

    /// Copies a file or directory into the directory `dest_dir` (project-relative,
    /// empty for the root). A name clash gets a ` copy` suffix like the reference
    /// IDE's paste. Returns the new project-relative path.
    pub fn copy_into(&self, relative: &Path, dest_dir: &Path) -> Result<PathBuf, ProjectError> {
        let from = self.checked(relative)?;
        let dir = self.checked(dest_dir)?;
        if !dir.is_dir() {
            return Err(ProjectError::NotDirectory(dest_dir.to_owned()));
        }
        if from == self.root {
            return Err(ProjectError::InvalidName(relative.display().to_string()));
        }
        if from.is_dir() && dir.starts_with(&from) {
            return Err(ProjectError::InvalidName(format!(
                "cannot copy {} into itself",
                relative.display()
            )));
        }
        let target = self.free_name(dest_dir, relative)?;
        copy_tree(&from, &self.root.join(&target))?;
        Ok(target)
    }

    /// Moves a file or directory into `dest_dir`. Returns the new relative path.
    pub fn move_into(&self, relative: &Path, dest_dir: &Path) -> Result<PathBuf, ProjectError> {
        let from = self.checked(relative)?;
        let dir = self.checked(dest_dir)?;
        if !dir.is_dir() {
            return Err(ProjectError::NotDirectory(dest_dir.to_owned()));
        }
        if from == self.root || (from.is_dir() && dir.starts_with(&from)) {
            return Err(ProjectError::InvalidName(relative.display().to_string()));
        }
        let name = relative
            .file_name()
            .ok_or_else(|| ProjectError::InvalidName(relative.display().to_string()))?;
        let target = dest_dir.join(name);
        if target == relative {
            return Ok(target);
        }
        let to = self.checked_new(&target)?;
        fs::rename(from, to)?;
        Ok(target)
    }

    /// `dest_dir/<name>`, or `dest_dir/<stem> copy[ N].<ext>` when taken.
    fn free_name(&self, dest_dir: &Path, source: &Path) -> Result<PathBuf, ProjectError> {
        let name = source
            .file_name()
            .ok_or_else(|| ProjectError::InvalidName(source.display().to_string()))?
            .to_string_lossy()
            .into_owned();
        let direct = dest_dir.join(&name);
        if self.root.join(&direct).symlink_metadata().is_err() {
            return Ok(direct);
        }
        let (stem, ext) = match name.rfind('.').filter(|&i| i > 0) {
            Some(i) => (&name[..i], &name[i..]),
            None => (name.as_str(), ""),
        };
        for n in 1..1000 {
            let suffix = if n == 1 {
                " copy".to_owned()
            } else {
                format!(" copy {n}")
            };
            let candidate = dest_dir.join(format!("{stem}{suffix}{ext}"));
            if self.root.join(&candidate).symlink_metadata().is_err() {
                return Ok(candidate);
            }
        }
        Err(ProjectError::AlreadyExists(dest_dir.join(name)))
    }

    /// Deletes a file or a directory tree. The project root itself cannot be deleted.
    pub fn delete(&self, relative: &Path) -> Result<(), ProjectError> {
        // Delete a link itself, never its target (which may lie outside).
        let link = self.root.join(relative);
        if relative.components().count() > 0
            && link
                .symlink_metadata()
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            self.checked_follow(relative).or_else(|error| match error {
                // A dangling link can still be removed.
                ProjectError::Io(_) => Ok(link.clone()),
                other => Err(other),
            })?;
            fs::remove_file(link)?;
            return Ok(());
        }
        let path = self.checked(relative)?;
        if path == self.root {
            return Err(ProjectError::InvalidName(relative.display().to_string()));
        }
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Writes a file inside the project, recreating it (and missing parent
    /// directories) when it was deleted outside the IDE.
    pub fn write_text(&self, relative: &Path, text: &str) -> Result<(), ProjectError> {
        let path = if self.root.join(relative).symlink_metadata().is_ok() {
            self.checked(relative)?
        } else {
            let path = self.checked_new(relative)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            path
        };
        fs::write(path, text).map_err(ProjectError::from)
    }
}

/// Recursive copy that skips symlinks (they could point outside the project).
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    let metadata = from.symlink_metadata()?;
    if metadata.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        fs::copy(from, to)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn generated_jdtls_artefacts_are_hidden_in_build_projects_only() {
        let dir = std::env::temp_dir().join(format!("jt-artefacts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("bin/main")).unwrap();
        fs::create_dir_all(dir.join(".settings")).unwrap();
        fs::write(dir.join("bin/main/App.class"), "x").unwrap();
        fs::write(dir.join(".classpath"), "x").unwrap();
        fs::write(dir.join("Main.java"), "x").unwrap();
        let names = |p: &Project| -> Vec<String> {
            let mut names: Vec<String> = p
                .entries(100)
                .iter()
                .map(|e| e.relative_path.display().to_string())
                .collect();
            names.sort();
            names
        };
        let project = Project::open(&dir).unwrap();
        assert!(
            names(&project).contains(&".classpath".to_owned()),
            "no build file"
        );
        fs::write(dir.join("build.gradle"), "").unwrap();
        assert_eq!(names(&project), vec!["Main.java", "build.gradle"]);
        // A bin/ with user sources stays visible.
        fs::write(dir.join("bin/run.sh"), "x").unwrap();
        assert!(names(&project).contains(&"bin".to_owned()));
        let _ = fs::remove_dir_all(&dir);
    }

    use super::*;

    #[test]
    fn opening_a_missing_directory_names_the_path() {
        let missing = std::env::temp_dir().join("archipel-ide-definitely-missing-dir");
        let error = Project::open(&missing).unwrap_err().to_string();
        assert!(error.contains("no directory at"), "{error}");
        assert!(
            error.contains("archipel-ide-definitely-missing-dir"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn linked_directories_are_listed_lazily_and_readable_but_not_writable() {
        let base = std::env::temp_dir().join(format!("archipel-ide-links-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let outside = base.join("output-base/bin");
        fs::create_dir_all(outside.join("pkg")).unwrap();
        fs::write(outside.join("pkg/gen.txt"), "generated").unwrap();
        let root = base.join("ws");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("BUILD.bazel"), "").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("bazel-bin")).unwrap();
        let project = Project::open(&root).unwrap();
        let entries = project.entries(100);
        let link = entries
            .iter()
            .find(|e| e.relative_path == Path::new("bazel-bin"))
            .unwrap();
        assert!(link.is_dir && link.is_symlink, "{link:?}");
        assert!(
            !entries
                .iter()
                .any(|e| e.relative_path.starts_with("bazel-bin")
                    && e.relative_path != Path::new("bazel-bin")),
            "linked trees are not walked"
        );
        let children = project.children(Path::new("bazel-bin")).unwrap();
        assert_eq!(children[0].relative_path, Path::new("bazel-bin/pkg"));
        assert!(children[0].is_dir);
        assert_eq!(
            project.children(Path::new("bazel-bin/pkg")).unwrap()[0].relative_path,
            Path::new("bazel-bin/pkg/gen.txt")
        );
        assert_eq!(
            project
                .read_text(Path::new("bazel-bin/pkg/gen.txt"))
                .unwrap(),
            "generated"
        );
        assert!(
            project
                .read_text(Path::new("../output-base/bin/pkg/gen.txt"))
                .is_err()
        );
        assert!(matches!(
            project.write_text(Path::new("bazel-bin/pkg/gen.txt"), "x"),
            Err(ProjectError::OutsideRoot(_))
        ));
        project.delete(Path::new("bazel-bin")).unwrap();
        assert!(outside.join("pkg/gen.txt").exists(), "target survives");
        let _ = fs::remove_dir_all(base);
    }
    #[test]
    fn opens_lists_and_reads_project() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-project-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        let project = Project::open(&root).unwrap();
        assert!(
            project
                .entries(10)
                .iter()
                .any(|e| e.relative_path == Path::new("src/main.rs"))
        );
        assert_eq!(
            project.read_text(Path::new("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creates_renames_and_deletes_inside_the_root() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-project-ops-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let project = Project::open(&root).unwrap();
        project.create_file(Path::new("src/new.rs")).unwrap();
        assert!(root.join("src/new.rs").is_file());
        assert!(matches!(
            project.create_file(Path::new("src/new.rs")),
            Err(ProjectError::AlreadyExists(_))
        ));
        for bad in ["../escape.rs", "/abs.rs", "", "a/../../b"] {
            assert!(
                matches!(
                    project.create_file(Path::new(bad)),
                    Err(ProjectError::InvalidName(_))
                ),
                "{bad}"
            );
        }
        let renamed = project.rename(Path::new("src/new.rs"), "lib.rs").unwrap();
        assert_eq!(renamed, Path::new("src/lib.rs"));
        assert!(project.rename(&renamed, "x/y").is_err());
        project.create_dir(Path::new("empty")).unwrap();
        project.delete(Path::new("src")).unwrap();
        project.delete(Path::new("empty")).unwrap();
        assert!(!root.join("src").exists());
        assert!(project.delete(Path::new("")).is_err());
        assert!(project.delete(Path::new(".")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copy_and_move_into_directories() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-copy-move-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("a/sub")).unwrap();
        fs::create_dir_all(root.join("b")).unwrap();
        fs::write(root.join("a/sub/x.txt"), "x").unwrap();
        fs::write(root.join("a/f.txt"), "f").unwrap();
        let project = Project::open(&root).unwrap();
        let copied = project
            .copy_into(Path::new("a/f.txt"), Path::new("a"))
            .unwrap();
        assert_eq!(copied, Path::new("a/f copy.txt"));
        let copied = project.copy_into(Path::new("a"), Path::new("b")).unwrap();
        assert_eq!(copied, Path::new("b/a"));
        assert!(root.join("b/a/sub/x.txt").is_file());
        assert!(
            project
                .copy_into(Path::new("a"), Path::new("a/sub"))
                .is_err()
        );
        let moved = project
            .move_into(Path::new("a/f.txt"), Path::new("b"))
            .unwrap();
        assert_eq!(moved, Path::new("b/f.txt"));
        assert!(!root.join("a/f.txt").exists());
        assert!(
            project
                .move_into(Path::new("../x"), Path::new("b"))
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }
}
