//! Project tool window model: compact packages, directory marks (source/test/
//! resources/excluded roots), the file clipboard, the Find in Files scope and the
//! virtual `External Libraries` / `Scratches and Consoles` nodes.
//!
//! The UI draws [`TreeRow`]s; every user action is a typed command registered here.

use crate::{IdeState, spec, string_array_param, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_project::FileEntry;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// Project-relative directory holding scratch files.
pub const SCRATCH_DIR: &str = ".archipel/scratches";
pub const GROUP_LIBRARIES: &str = "external-libraries";
pub const GROUP_SCRATCHES_ROOT: &str = "scratches-and-consoles";
pub const GROUP_SCRATCHES: &str = "scratches";

/// "Mark Directory as" choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DirMark {
    Sources,
    Tests,
    Resources,
    TestResources,
    Excluded,
}

impl DirMark {
    /// In menu order.
    pub const ALL: [DirMark; 5] = [
        DirMark::Sources,
        DirMark::Tests,
        DirMark::Resources,
        DirMark::TestResources,
        DirMark::Excluded,
    ];

    pub fn parse(id: &str) -> Option<DirMark> {
        Some(match id {
            "sources" => DirMark::Sources,
            "tests" => DirMark::Tests,
            "resources" => DirMark::Resources,
            "testResources" => DirMark::TestResources,
            "excluded" => DirMark::Excluded,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            DirMark::Sources => "sources",
            DirMark::Tests => "tests",
            DirMark::Resources => "resources",
            DirMark::TestResources => "testResources",
            DirMark::Excluded => "excluded",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DirMark::Sources => "Sources Root",
            DirMark::Tests => "Test Sources Root",
            DirMark::Resources => "Resources Root",
            DirMark::TestResources => "Test Resources Root",
            DirMark::Excluded => "Excluded",
        }
    }
}

/// What a directory is for; drives folder colours, compaction and tinting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirRole {
    Plain,
    Source,
    Test,
    Resources,
    TestResources,
    Excluded,
    /// `.git`, `.idea`, `.archipel` (and the old `.jetthing`).
    Hidden,
    /// A directory inside a source root (a Java package).
    Package {
        test: bool,
    },
}

impl DirRole {
    pub fn is_test(self) -> bool {
        matches!(
            self,
            DirRole::Test | DirRole::TestResources | DirRole::Package { test: true }
        )
    }
}

/// `.jetthing` is the data directory of versions before the rename (see `archipel_ide_compat`).
const HIDDEN_DIRS: [&str; 4] = [".git", ".idea", ".archipel", ".jetthing"];
const EXCLUDED_DIRS: [&str; 7] = [
    "build",
    "target",
    "out",
    "node_modules",
    "dist",
    ".gradle",
    ".kotlin",
];

fn role_of_mark(mark: DirMark) -> DirRole {
    match mark {
        DirMark::Sources => DirRole::Source,
        DirMark::Tests => DirRole::Test,
        DirMark::Resources => DirRole::Resources,
        DirMark::TestResources => DirRole::TestResources,
        DirMark::Excluded => DirRole::Excluded,
    }
}

/// Classifies a project-relative directory. `marks` (user marks over the Gradle
/// source sets) win over the conventional layout.
pub fn classify(path: &Path, marks: &BTreeMap<PathBuf, DirMark>) -> DirRole {
    if let Some(mark) = marks.get(path) {
        return role_of_mark(*mark);
    }
    for ancestor in path.ancestors().skip(1) {
        match marks.get(ancestor) {
            Some(DirMark::Excluded) => return DirRole::Excluded,
            Some(DirMark::Sources) => return DirRole::Package { test: false },
            Some(DirMark::Tests) => return DirRole::Package { test: true },
            Some(_) => break,
            None => {}
        }
    }
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let Some(last) = parts.last() else {
        return DirRole::Plain;
    };
    if HIDDEN_DIRS.contains(&last.as_str()) {
        return DirRole::Hidden;
    }
    if parts.iter().any(|p| EXCLUDED_DIRS.contains(&p.as_str())) {
        return DirRole::Excluded;
    }
    // `src/<set>/<kind>[/package…]`
    if let Some(i) = (0..parts.len().saturating_sub(2)).find(|&i| parts[i] == "src") {
        let set = parts[i + 1].to_ascii_lowercase();
        let is_test = set.starts_with("test") || set.starts_with("androidtest");
        let depth = parts.len() - (i + 3);
        match parts[i + 2].as_str() {
            "java" | "kotlin" if depth > 0 => return DirRole::Package { test: is_test },
            "java" | "kotlin" if is_test => return DirRole::Test,
            "java" | "kotlin" => return DirRole::Source,
            "resources" | "res" if depth == 0 && is_test => return DirRole::TestResources,
            "resources" | "res" if depth == 0 => return DirRole::Resources,
            _ => {}
        }
    }
    DirRole::Plain
}

/// Most usages the Safe Delete dialog lists.
pub const MAX_USAGES: usize = 200;

/// One place that still mentions a file about to be deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Usage {
    pub path: PathBuf,
    pub line: usize,
    pub text: String,
}

/// Files copied or cut in the Project view, waiting for Paste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileClipboard {
    pub paths: Vec<PathBuf>,
    pub cut: bool,
}

/// Cached result of [`IdeState::tree_rows`] and the key it was built for.
pub(crate) struct TreeCache {
    key: u64,
    rows: Arc<Vec<TreeRow>>,
}

/// Project view state that is not part of the file list.
#[derive(Debug, Clone)]
pub struct ProjectView {
    /// "Compact Middle Packages": `com` + `example` shown as `com.example`.
    pub compact_packages: bool,
    /// User marks (persisted in the workspace file).
    pub marks: BTreeMap<PathBuf, DirMark>,
    pub clipboard: Option<FileClipboard>,
    /// Directory that Find in Files is limited to.
    pub search_scope: Option<PathBuf>,
    /// Expanded virtual nodes ([`GROUP_LIBRARIES`], …).
    pub expanded_groups: BTreeSet<String>,
}

impl Default for ProjectView {
    fn default() -> Self {
        ProjectView {
            compact_packages: true,
            marks: BTreeMap::new(),
            clipboard: None,
            search_scope: None,
            expanded_groups: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Directory,
    File,
    /// A virtual node (`External Libraries`, `Scratches and Consoles`, …).
    Group,
    Jdk,
    Library,
}

/// One drawn row of the Project tree (below the root row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub depth: usize,
    /// Display name; `com.example` for a compacted package chain.
    pub label: String,
    /// Project-relative path (the deepest directory of a compacted chain);
    /// absolute for libraries and the JDK; `None` for groups.
    pub path: Option<PathBuf>,
    pub kind: RowKind,
    pub expanded: bool,
    /// Group id for [`RowKind::Group`] rows.
    pub group: Option<&'static str>,
    pub role: DirRole,
    /// Drawn greyed with a tinted row.
    pub excluded: bool,
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl IdeState {
    /// User marks over the source sets of the Gradle model.
    pub fn directory_marks(&self) -> BTreeMap<PathBuf, DirMark> {
        let mut marks = BTreeMap::new();
        if let (Some(model), Some(project)) = (&self.gradle_model, &self.project) {
            let relative = |dir: &Path| dir.strip_prefix(project.root()).ok().map(Path::to_owned);
            for module in &model.projects {
                for set in &module.source_sets {
                    let lower = set.name.to_ascii_lowercase();
                    let test = lower.starts_with("test") || lower.starts_with("androidtest");
                    for dir in set.java.iter().filter_map(|dir| relative(dir)) {
                        marks.insert(
                            dir,
                            if test {
                                DirMark::Tests
                            } else {
                                DirMark::Sources
                            },
                        );
                    }
                    for dir in set.resources.iter().filter_map(|dir| relative(dir)) {
                        let mark = if test {
                            DirMark::TestResources
                        } else {
                            DirMark::Resources
                        };
                        marks.insert(dir, mark);
                    }
                }
            }
        }
        marks.extend(self.project_view.marks.clone());
        marks
    }

    /// Replaces the project file list and invalidates the cached tree rows.
    pub(crate) fn set_files(&mut self, files: Vec<FileEntry>) {
        self.files = files;
        self.files_generation += 1;
    }

    /// Fingerprint of everything [`IdeState::tree_rows`] reads other than the
    /// (large) file list, which is covered by its generation counter.
    fn tree_key(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.files_generation, self.model_generation).hash(&mut hasher);
        self.files.len().hash(&mut hasher);
        self.project
            .as_ref()
            .map(|project| project.root())
            .hash(&mut hasher);
        self.expanded_dirs.hash(&mut hasher);
        self.project_view.compact_packages.hash(&mut hasher);
        self.project_view.expanded_groups.hash(&mut hasher);
        for (path, mark) in &self.project_view.marks {
            path.hash(&mut hasher);
            std::mem::discriminant(mark).hash(&mut hasher);
        }
        if self
            .project_view
            .expanded_groups
            .contains(GROUP_SCRATCHES_ROOT)
            && self.project_view.expanded_groups.contains(GROUP_SCRATCHES)
        {
            self.scratch_files().hash(&mut hasher);
        }
        hasher.finish()
    }

    /// [`IdeState::tree_rows`], rebuilt only when its inputs changed.
    pub fn cached_tree_rows(&mut self) -> Arc<Vec<TreeRow>> {
        let key = self.tree_key();
        if let Some(cache) = &self.tree_cache
            && cache.key == key
        {
            return cache.rows.clone();
        }
        let rows = Arc::new(self.tree_rows());
        self.tree_cache = Some(TreeCache {
            key,
            rows: rows.clone(),
        });
        rows
    }

    /// Rows of the Project tree below the root row: files and directories (with
    /// compacted package chains), then the virtual nodes.
    pub fn tree_rows(&self) -> Vec<TreeRow> {
        let marks = self.directory_marks();
        let mut rows = self.file_rows(&marks);
        if self.project.is_some() {
            self.library_rows(&mut rows);
            self.scratch_rows(&mut rows);
        }
        rows
    }

    fn file_rows(&self, marks: &BTreeMap<PathBuf, DirMark>) -> Vec<TreeRow> {
        let compact = self.project_view.compact_packages;
        // Child counts: (files, directories, last directory).
        let mut children: HashMap<&Path, (usize, usize, Option<&Path>)> = HashMap::new();
        for entry in &self.files {
            let parent = entry.relative_path.parent().unwrap_or(Path::new(""));
            let slot = children.entry(parent).or_default();
            if entry.is_dir {
                slot.1 += 1;
                slot.2 = Some(entry.relative_path.as_path());
            } else {
                slot.0 += 1;
            }
        }
        // A package whose only content is one package merges into it.
        let merge_child = |dir: &Path| -> Option<&Path> {
            if !compact || !matches!(classify(dir, marks), DirRole::Package { .. }) {
                return None;
            }
            match children.get(dir) {
                Some(&(0, 1, Some(child)))
                    if matches!(classify(child, marks), DirRole::Package { .. }) =>
                {
                    Some(child)
                }
                _ => None,
            }
        };
        let tail_of = |dir: &Path| -> PathBuf {
            let mut tail = dir;
            while let Some(next) = merge_child(tail) {
                tail = next;
            }
            tail.to_owned()
        };
        let expanded = |dir: &Path| self.expanded_dirs.contains(&tail_of(dir));
        let mut rows = Vec::new();
        for entry in &self.files {
            if !entry
                .relative_path
                .ancestors()
                .skip(1)
                .filter(|ancestor| !ancestor.as_os_str().is_empty())
                .all(&expanded)
            {
                continue;
            }
            let parent = entry.relative_path.parent().unwrap_or(Path::new(""));
            // Later members of a chain are shown by the chain's first directory.
            if entry.is_dir && merge_child(parent) == Some(entry.relative_path.as_path()) {
                continue;
            }
            let depth = entry
                .relative_path
                .ancestors()
                .skip(1)
                .filter(|ancestor| !ancestor.as_os_str().is_empty())
                .filter(|ancestor| {
                    let up = ancestor.parent().unwrap_or(Path::new(""));
                    merge_child(up) != Some(*ancestor)
                })
                .count();
            rows.push(self.file_row(entry, depth, marks, &tail_of, &expanded));
        }
        rows
    }

    fn file_row(
        &self,
        entry: &FileEntry,
        depth: usize,
        marks: &BTreeMap<PathBuf, DirMark>,
        tail_of: &dyn Fn(&Path) -> PathBuf,
        expanded: &dyn Fn(&Path) -> bool,
    ) -> TreeRow {
        let path = &entry.relative_path;
        if !entry.is_dir {
            let parent_role = classify(path.parent().unwrap_or(Path::new("")), marks);
            return TreeRow {
                depth,
                label: file_name(path),
                path: Some(path.clone()),
                kind: RowKind::File,
                expanded: false,
                group: None,
                role: parent_role,
                excluded: parent_role == DirRole::Excluded,
            };
        }
        let tail = tail_of(path);
        let mut label = file_name(path);
        if tail != *path {
            let rest = tail.strip_prefix(path).unwrap_or(&tail);
            for part in rest.components() {
                label.push('.');
                label.push_str(&part.as_os_str().to_string_lossy());
            }
        }
        let role = classify(&tail, marks);
        TreeRow {
            depth,
            label,
            path: Some(tail),
            kind: RowKind::Directory,
            expanded: expanded(path),
            group: None,
            role,
            excluded: role == DirRole::Excluded,
        }
    }

    fn group_row(&self, depth: usize, label: &str, group: &'static str) -> TreeRow {
        TreeRow {
            depth,
            label: label.to_owned(),
            path: None,
            kind: RowKind::Group,
            expanded: self.project_view.expanded_groups.contains(group),
            group: Some(group),
            role: DirRole::Plain,
            excluded: false,
        }
    }

    /// `External Libraries`: the JDK and the Gradle dependencies of the last sync.
    fn library_rows(&self, rows: &mut Vec<TreeRow>) {
        let Some(model) = &self.gradle_model else {
            return;
        };
        let group = self.group_row(0, "External Libraries", GROUP_LIBRARIES);
        let open = group.expanded;
        rows.push(group);
        if !open {
            return;
        }
        let leaf = |label: String, path: PathBuf, kind| TreeRow {
            depth: 1,
            label,
            path: Some(path),
            kind,
            expanded: false,
            group: None,
            role: DirRole::Plain,
            excluded: false,
        };
        if let Some(jdk) = &model.jdk {
            rows.push(leaf(
                format!("JDK {}", jdk.version),
                jdk.home.clone(),
                RowKind::Jdk,
            ));
        }
        let libraries: BTreeMap<&str, &Path> = model
            .projects
            .iter()
            .flat_map(|project| &project.libraries)
            .map(|library| (library.name.as_str(), library.path.as_path()))
            .collect();
        for (name, path) in libraries {
            rows.push(leaf(name.to_owned(), path.to_owned(), RowKind::Library));
        }
    }

    /// `Scratches and Consoles` with the project's scratch files.
    fn scratch_rows(&self, rows: &mut Vec<TreeRow>) {
        let root = self.group_row(0, "Scratches and Consoles", GROUP_SCRATCHES_ROOT);
        let open = root.expanded;
        rows.push(root);
        if !open {
            return;
        }
        let scratches = self.group_row(1, "Scratches", GROUP_SCRATCHES);
        let open = scratches.expanded;
        rows.push(scratches);
        if !open {
            return;
        }
        for path in self.scratch_files() {
            rows.push(TreeRow {
                depth: 2,
                label: file_name(&path),
                path: Some(path),
                kind: RowKind::File,
                expanded: false,
                group: None,
                role: DirRole::Plain,
                excluded: false,
            });
        }
    }

    /// Project-relative paths of the scratch files, sorted by name.
    pub fn scratch_files(&self) -> Vec<PathBuf> {
        let Some(project) = &self.project else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(archipel_ide_compat::project_write_path(
            project.root().join(SCRATCH_DIR),
        )) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .map(|entry| Path::new(SCRATCH_DIR).join(entry.file_name()))
            .collect();
        files.sort();
        files
    }

    /// Creates `scratch.txt` (or the next free `scratch_N.txt`) and opens it.
    pub fn new_scratch_file(&mut self) -> Result<PathBuf, String> {
        let existing = self.scratch_files();
        let path = (0..)
            .map(|n| {
                let name = if n == 0 {
                    "scratch.txt".to_owned()
                } else {
                    format!("scratch_{n}.txt")
                };
                Path::new(SCRATCH_DIR).join(name)
            })
            .find(|candidate| !existing.contains(candidate))
            .expect("unbounded range");
        let project = self.project.as_ref().ok_or("no project is open")?;
        project
            .create_file(&path)
            .map_err(|error| error.to_string())?;
        self.project_view.expanded_groups.extend(
            [GROUP_SCRATCHES_ROOT, GROUP_SCRATCHES]
                .into_iter()
                .map(str::to_owned),
        );
        self.open_file(&path)?;
        self.status = format!("Created scratch {}", path.display());
        Ok(path)
    }

    /// Cut or copy `paths` for a later paste.
    pub fn set_file_clipboard(&mut self, paths: Vec<PathBuf>, cut: bool) {
        self.status = format!(
            "{} {} item{}",
            if cut { "Cut" } else { "Copied" },
            paths.len(),
            if paths.len() == 1 { "" } else { "s" }
        );
        self.project_view.clipboard = Some(FileClipboard { paths, cut });
    }

    /// Pastes the file clipboard into the directory `dir` (project-relative).
    /// Returns the new paths.
    pub fn paste_files(&mut self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let clipboard = self
            .project_view
            .clipboard
            .clone()
            .ok_or("nothing to paste: copy or cut a file first")?;
        let mut pasted = Vec::new();
        for path in &clipboard.paths {
            let project = self.project.as_ref().ok_or("no project is open")?;
            if clipboard.cut {
                let moved = project
                    .move_into(path, dir)
                    .map_err(|error| error.to_string())?;
                self.remap_paths(path, &moved);
                pasted.push(moved);
            } else {
                pasted.push(
                    project
                        .copy_into(path, dir)
                        .map_err(|error| error.to_string())?,
                );
            }
        }
        if clipboard.cut {
            self.project_view.clipboard = None;
        }
        for ancestor in dir.ancestors() {
            if !ancestor.as_os_str().is_empty() {
                self.expanded_dirs.insert(ancestor.to_owned());
            }
        }
        self.refresh_files();
        self.status = format!(
            "{} {} item{} into {}",
            if clipboard.cut { "Moved" } else { "Pasted" },
            pasted.len(),
            if pasted.len() == 1 { "" } else { "s" },
            if dir.as_os_str().is_empty() {
                "the project root".to_owned()
            } else {
                dir.display().to_string()
            }
        );
        Ok(pasted)
    }

    /// Marks (or, with `None`, unmarks) a directory.
    pub fn mark_directory(&mut self, path: &Path, mark: Option<DirMark>) -> Result<(), String> {
        if path.as_os_str().is_empty() {
            return Err("the project root cannot be marked".into());
        }
        if !self
            .files
            .iter()
            .any(|entry| entry.is_dir && entry.relative_path == path)
        {
            return Err(format!("{} is not a project directory", path.display()));
        }
        match mark {
            Some(mark) => {
                self.project_view.marks.insert(path.to_owned(), mark);
                self.status = format!("Marked {} as {}", path.display(), mark.label());
            }
            None => {
                self.project_view.marks.remove(path);
                self.status = format!("Unmarked {}", path.display());
            }
        }
        Ok(())
    }

    /// Text usages of the name of `path` (a file's stem, a directory's name) in
    /// the rest of the project, for the Safe Delete dialog. At most
    /// [`MAX_USAGES`]; files above 1 MB are skipped.
    pub fn safe_delete_usages(&self, path: &Path) -> Result<Vec<Usage>, String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        if path.as_os_str().is_empty() {
            return Err("the project root cannot be deleted".into());
        }
        let name = if project.root().join(path).is_dir() {
            file_name(path)
        } else {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        if name.is_empty() {
            return Ok(Vec::new());
        }
        let options = archipel_ide_search::SearchOptions {
            match_case: true,
            whole_words: true,
            regex: false,
            file_mask: String::new(),
        };
        let matcher = archipel_ide_search::Matcher::new(&name, &options)
            .map_err(|error| error.to_string())?;
        let mut usages = Vec::new();
        'files: for entry in self.files.iter().filter(|entry| !entry.is_dir) {
            if entry.relative_path.starts_with(path) {
                continue;
            }
            let absolute = project.root().join(&entry.relative_path);
            if std::fs::metadata(&absolute).is_ok_and(|meta| meta.len() > 1_000_000) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&absolute) else {
                continue;
            };
            for (index, line) in text.lines().enumerate() {
                if !matcher.occurrences(line).is_empty() {
                    usages.push(Usage {
                        path: entry.relative_path.clone(),
                        line: index + 1,
                        text: line.trim().chars().take(120).collect(),
                    });
                    if usages.len() >= MAX_USAGES {
                        break 'files;
                    }
                }
            }
        }
        Ok(usages)
    }

    /// Deletes `path` unless it is still used elsewhere and `force` is false.
    pub fn safe_delete(&mut self, path: &Path, force: bool) -> Result<(), String> {
        if !force {
            let usages = self.safe_delete_usages(path)?;
            if !usages.is_empty() {
                return Err(format!(
                    "{} is used in {} place{}; review the usages or delete anyway",
                    path.display(),
                    usages.len(),
                    if usages.len() == 1 { "" } else { "s" }
                ));
            }
        }
        self.delete_path(path)
    }

    /// Opens the system file manager at `path` (project-relative; its folder for
    /// a file).
    pub fn reveal_in_file_manager(&mut self, path: &Path) -> Result<(), String> {
        let absolute = self.absolute_dir(path, true)?;
        let (program, args): (&str, Vec<std::ffi::OsString>) = if cfg!(target_os = "macos") {
            ("open", vec![absolute.clone().into_os_string()])
        } else if cfg!(target_os = "windows") {
            ("explorer", vec![absolute.clone().into_os_string()])
        } else {
            ("xdg-open", vec![absolute.clone().into_os_string()])
        };
        std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|error| format!("could not run {program}: {error}"))?;
        self.status = format!("Opened {} in the file manager", absolute.display());
        Ok(())
    }

    /// Absolute directory of `path` (the path itself, or its folder when it is a file).
    pub(crate) fn absolute_dir(
        &self,
        path: &Path,
        parent_of_file: bool,
    ) -> Result<PathBuf, String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let absolute = project.root().join(path);
        if !absolute.exists() {
            return Err(format!("{} does not exist", path.display()));
        }
        if parent_of_file && absolute.is_file() {
            return Ok(absolute
                .parent()
                .map(Path::to_owned)
                .unwrap_or_else(|| project.root().to_owned()));
        }
        Ok(absolute)
    }
}

fn paths_param(params: &Value) -> Result<Vec<PathBuf>, CommandError> {
    let paths = string_array_param(params, "paths")?;
    if paths.is_empty() {
        return Err(CommandError::InvalidParams(
            "expected at least one path".into(),
        ));
    }
    Ok(paths.into_iter().map(PathBuf::from).collect())
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    for (id, label, cut) in [
        ("project.copyPaths", "Copy Files", false),
        ("project.cutPaths", "Cut Files", true),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Project", None, r#"{"paths":"string[]"}"#),
                |state: &IdeState| state.project.is_some(),
                move |state: &mut IdeState, params| {
                    state.set_file_clipboard(paths_param(&params)?, cut);
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "project.paste",
                "Paste Files",
                "Project",
                None,
                r#"{"dir":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some() && state.project_view.clipboard.is_some(),
            |state: &mut IdeState, params| {
                let dir = PathBuf::from(string_param(&params, "dir")?);
                let pasted = state.paste_files(&dir).map_err(CommandError::Failed)?;
                Ok(json!({"paths": pasted}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.markDirectory",
                "Mark Directory as",
                "Project",
                None,
                r#"{"path":"string","mark":"sources|tests|resources|testResources|excluded|none"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                let id = string_param(&params, "mark")?;
                let mark = if id == "none" {
                    None
                } else {
                    Some(DirMark::parse(&id).ok_or_else(|| {
                        CommandError::InvalidParams(format!("unknown directory mark `{id}`"))
                    })?)
                };
                state
                    .mark_directory(&path, mark)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.toggleCompactPackages",
                "Compact Middle Packages",
                "Project",
                None,
                "{}",
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                let view = &mut state.project_view;
                view.compact_packages = !view.compact_packages;
                Ok(json!({"compactPackages": view.compact_packages}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.toggleGroup",
                "Expand/Collapse Project View Node",
                "Project",
                None,
                r#"{"id":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let id = string_param(&params, "id")?;
                let groups = &mut state.project_view.expanded_groups;
                if !groups.remove(&id) {
                    groups.insert(id);
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.newScratch",
                "New Scratch File",
                "Project",
                None,
                "{}",
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                let path = state.new_scratch_file().map_err(CommandError::Failed)?;
                Ok(json!({"path": path}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.safeDeleteUsages",
                "Find Usages Before Delete",
                "Project",
                None,
                r#"{"path":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                let usages = state
                    .safe_delete_usages(&path)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"usages": usages}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.safeDelete",
                "Safe Delete",
                "Project",
                Some("⌥⌦"),
                r#"{"path":"string","force":"bool?"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                let force = params
                    .get("force")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                state
                    .safe_delete(&path, force)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.revealInFiles",
                "Show in File Manager",
                "Project",
                None,
                r#"{"path":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                state
                    .reveal_in_file_manager(&path)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.openInTerminal",
                "Open in Terminal",
                "Project",
                None,
                r#"{"path":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                let dir = state
                    .absolute_dir(&path, true)
                    .map_err(CommandError::Failed)?;
                state
                    .open_terminal_in(Some(&dir))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn project(name: &str, files: &[&str]) -> (PathBuf, IdeState) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-pv-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for file in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x").unwrap();
        }
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        (root, state)
    }

    fn labels(state: &IdeState) -> Vec<(usize, String)> {
        state
            .tree_rows()
            .into_iter()
            .map(|row| (row.depth, row.label))
            .collect()
    }

    #[test]
    fn middle_packages_compact_and_toggle() {
        let (root, mut state) = project(
            "compact",
            &[
                "src/main/java/com/example/App.java",
                "src/main/java/com/example/util/Util.java",
                "src/main/java/org/only/deep/Deep.java",
            ],
        );
        for dir in [
            "src",
            "src/main",
            "src/main/java",
            "src/main/java/com",
            "src/main/java/org",
            "src/main/java/org/only",
        ] {
            state.expanded_dirs.insert(dir.into());
        }
        // `com` has a single child (`example`), which merges; `org/only/deep` merges too.
        let rows = labels(&state);
        assert!(rows.contains(&(3, "com.example".into())), "{rows:?}");
        assert!(rows.contains(&(3, "org.only.deep".into())), "{rows:?}");
        assert!(!rows.iter().any(|(_, label)| label == "example"));
        // A compacted node is collapsed until its deepest directory is expanded.
        assert!(!rows.iter().any(|(_, label)| label == "App.java"));
        state
            .expanded_dirs
            .insert("src/main/java/com/example".into());
        let rows = labels(&state);
        assert!(rows.contains(&(4, "App.java".into())), "{rows:?}");
        assert!(rows.contains(&(4, "util".into())), "{rows:?}");
        let commands = built_in_commands();
        commands
            .execute("project.toggleCompactPackages", &mut state, Value::Null)
            .unwrap();
        let rows = labels(&state);
        assert!(rows.contains(&(3, "com".into())), "{rows:?}");
        assert!(rows.contains(&(4, "example".into())), "{rows:?}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn build_output_is_excluded_and_collapsed() {
        let (root, mut state) = project(
            "excluded",
            &[
                "build/classes/A.class",
                ".gradle/x/y",
                "src/App.java",
                "target/debug/a",
            ],
        );
        let rows = state.tree_rows();
        for name in ["build", ".gradle", "target"] {
            let row = rows.iter().find(|row| row.label == name).unwrap();
            assert!(row.excluded && !row.expanded, "{name}");
        }
        assert!(!rows.iter().any(|row| row.label == "A.class"));
        // `target` is listed but walked only when expanded.
        state.toggle_directory(Path::new("target")).unwrap();
        let rows = state.tree_rows();
        assert!(rows.iter().any(|row| row.label == "debug" && row.excluded));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tree_row_cache_reuses_rows_until_an_input_changes() {
        let (root, mut state) = project("tree-cache", &["a/f.txt", "b/g.txt"]);
        let first = state.cached_tree_rows();
        assert!(Arc::ptr_eq(&first, &state.cached_tree_rows()));
        assert_eq!(*first, state.tree_rows());
        // Expanding a directory changes the rows.
        state.toggle_directory(Path::new("a")).unwrap();
        let expanded = state.cached_tree_rows();
        assert!(!Arc::ptr_eq(&first, &expanded));
        assert_eq!(*expanded, state.tree_rows());
        assert!(Arc::ptr_eq(&expanded, &state.cached_tree_rows()));
        // A new file list bumps the generation even when the length is equal.
        let mut files = state.files.clone();
        files.reverse();
        state.set_files(files);
        let reloaded = state.cached_tree_rows();
        assert!(!Arc::ptr_eq(&expanded, &reloaded));
        assert_eq!(*reloaded, state.tree_rows());
        // Marks and compaction are part of the key.
        state.project_view.compact_packages = !state.project_view.compact_packages;
        assert!(!Arc::ptr_eq(&reloaded, &state.cached_tree_rows()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn marks_override_layout_and_persist() {
        let (root, mut state) = project("marks", &["gen/p/A.java", "src/B.java"]);
        let commands = built_in_commands();
        commands
            .execute(
                "project.markDirectory",
                &mut state,
                json!({"path": "gen", "mark": "sources"}),
            )
            .unwrap();
        let marks = state.directory_marks();
        assert_eq!(classify(Path::new("gen"), &marks), DirRole::Source);
        assert_eq!(
            classify(Path::new("gen/p"), &marks),
            DirRole::Package { test: false }
        );
        assert!(
            commands
                .execute(
                    "project.markDirectory",
                    &mut state,
                    json!({"path": "missing", "mark": "tests"}),
                )
                .is_err()
        );
        state.save_workspace().unwrap();
        let mut reopened = IdeState::default();
        reopened.open_project(&root).unwrap();
        assert_eq!(
            reopened.project_view.marks.get(Path::new("gen")),
            Some(&DirMark::Sources)
        );
        commands
            .execute(
                "project.markDirectory",
                &mut state,
                json!({"path": "gen", "mark": "none"}),
            )
            .unwrap();
        assert!(state.project_view.marks.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cut_copy_paste_moves_and_copies_files() {
        let (root, mut state) = project("paste", &["a/f.txt", "b/keep.txt"]);
        let commands = built_in_commands();
        commands
            .execute(
                "project.copyPaths",
                &mut state,
                json!({"paths": ["a/f.txt"]}),
            )
            .unwrap();
        commands
            .execute("project.paste", &mut state, json!({"dir": "b"}))
            .unwrap();
        assert!(root.join("a/f.txt").exists() && root.join("b/f.txt").exists());
        // Moving onto an existing name fails instead of overwriting.
        commands
            .execute(
                "project.cutPaths",
                &mut state,
                json!({"paths": ["a/f.txt"]}),
            )
            .unwrap();
        assert!(
            commands
                .execute("project.paste", &mut state, json!({"dir": "b"}))
                .is_err()
        );
        assert!(root.join("a/f.txt").exists());
        commands
            .execute(
                "project.cutPaths",
                &mut state,
                json!({"paths": ["b/keep.txt"]}),
            )
            .unwrap();
        commands
            .execute("project.paste", &mut state, json!({"dir": "a"}))
            .unwrap();
        assert!(!root.join("b/keep.txt").exists() && root.join("a/keep.txt").exists());
        assert!(
            commands
                .execute("project.paste", &mut state, json!({"dir": "a"}))
                .is_err(),
            "a cut clipboard is spent"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn virtual_nodes_list_libraries_and_scratches() {
        let (root, mut state) = project("virtual", &["App.java"]);
        let groups = |state: &IdeState| -> Vec<String> {
            state.tree_rows().into_iter().map(|row| row.label).collect()
        };
        // No Gradle sync yet: only Scratches and Consoles.
        assert_eq!(groups(&state), ["App.java", "Scratches and Consoles"]);
        let model = archipel_ide_gradle::GradleModel::parse(
            r#"{"gradleVersion":"8.5","rootDir":"/x","jdk":{"home":"/jdk","version":"21.0.4"},
                "projects":[{"path":":","name":"x","dir":"/x","libraries":[
                {"name":"org.a:b:1","path":"/m/b.jar"},{"name":"org.a:b:1","path":"/m/b.jar"}]}]}"#,
        )
        .unwrap();
        state.gradle_model = Some(model);
        let commands = built_in_commands();
        commands
            .execute(
                "project.toggleGroup",
                &mut state,
                json!({"id": GROUP_LIBRARIES}),
            )
            .unwrap();
        assert_eq!(
            groups(&state),
            [
                "App.java",
                "External Libraries",
                "JDK 21.0.4",
                "org.a:b:1",
                "Scratches and Consoles"
            ]
        );
        commands
            .execute("project.newScratch", &mut state, Value::Null)
            .unwrap();
        assert!(groups(&state).contains(&"scratch.txt".to_owned()));
        commands
            .execute("project.newScratch", &mut state, Value::Null)
            .unwrap();
        assert!(groups(&state).contains(&"scratch_1.txt".to_owned()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn safe_delete_reports_usages_until_forced() {
        let (root, mut state) =
            project("safe", &["src/Util.java", "src/App.java", "src/Other.java"]);
        std::fs::write(root.join("src/App.java"), "class App { Util u; }\n").unwrap();
        std::fs::write(root.join("src/Util.java"), "class Util {}\n").unwrap();
        std::fs::write(root.join("src/Other.java"), "class Utility {}\n").unwrap();
        state.open_project(&root).unwrap();
        let commands = built_in_commands();
        let result = commands
            .execute(
                "project.safeDeleteUsages",
                &mut state,
                json!({"path": "src/Util.java"}),
            )
            .unwrap();
        assert_eq!(result["usages"].as_array().unwrap().len(), 1, "{result}");
        assert_eq!(result["usages"][0]["path"], "src/App.java");
        assert!(
            commands
                .execute(
                    "project.safeDelete",
                    &mut state,
                    json!({"path": "src/Util.java"})
                )
                .is_err()
        );
        assert!(root.join("src/Util.java").exists());
        // Unused files delete without --force.
        commands
            .execute(
                "project.safeDelete",
                &mut state,
                json!({"path": "src/Other.java"}),
            )
            .unwrap();
        commands
            .execute(
                "project.safeDelete",
                &mut state,
                json!({"path": "src/Util.java", "force": true}),
            )
            .unwrap();
        assert!(!root.join("src/Util.java").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
