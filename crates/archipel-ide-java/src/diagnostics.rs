//! The native Java highlighting pass: unresolved-reference errors with their import fixes
//! (JAVA-ERR-007/011/012, JAVA-IMP-003/040) and the unused-import inspection (JAVA-IMP-070..072).

use crate::analyze::ProblemKind;
use crate::api::JavaProject;
use crate::env::{CurrentFile, Env};
use crate::import_edit::{self, ImportSettings};
use crate::import_fix::{self, ClassRefInfo, ImportClassFix, ImportStaticFix, Place, SearchInput};
use crate::index::{JavaIndex, JavaIndexState};
use crate::scope::ResolveScope;
use archipel_ide_analysis::daemon::{Pass, PassCx, PassId, PassOutput};
use archipel_ide_analysis::highlight::FixDescriptor;
use archipel_ide_analysis::inspect::ProblemDescriptor;
use archipel_ide_analysis::intention::{FixCx, QuickFix};
use archipel_ide_analysis::{
    Cancelled, FileId, HighlightInfo, HighlightKind, ModCommand, Severity, TextEdit, ToolId,
};
use archipel_ide_projectmodel::ProjectModel;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tree_sitter::{Node, Tree};

pub const PASS_ID: PassId = PassId("java-resolve");
pub const TOOL: &str = "java-resolve";

/// Settings of the native Java pass.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaSettings {
    /// Run the native resolve errors, import fixes and unused-import inspection.
    pub native: bool,
    pub unused_imports: bool,
    /// Offer class names from the native indexes in completion (JAVA-CMP-018).
    pub completion: bool,
    pub imports: ImportSettings,
}

impl Default for JavaSettings {
    fn default() -> Self {
        Self {
            native: true,
            unused_imports: true,
            completion: true,
            imports: ImportSettings::default(),
        }
    }
}

#[derive(Default)]
struct State {
    project: Option<JavaProject>,
    root: PathBuf,
    settings: JavaSettings,
    index: Option<JavaIndex>,
    model: Option<Arc<ProjectModel>>,
    /// The synthetic model of a folder without an imported project (ad-hoc module).
    adhoc: Option<Arc<ProjectModel>>,
    notes: Vec<String>,
    loading: bool,
    cache_dir: Option<PathBuf>,
    docs: std::collections::HashMap<PathBuf, DocState>,
    complete: bool,
    chooser: Option<Chooser>,
}

/// The open chooser popup (`Method to Import`, `Class to Import`).
#[derive(Debug, Clone)]
pub struct Chooser {
    pub title: String,
    pub rows: Vec<archipel_ide_analysis::ChoiceRow>,
}

struct DocState {
    rev: u64,
    seen: std::time::Instant,
    indexed: bool,
}

/// What the status bar and the control socket show about the Java indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaStatus {
    pub state: JavaIndexState,
    pub model_loading: bool,
    pub native: bool,
    /// Whether the classpath is the complete truth (import failed, jars missing, no libraries).
    pub classpath: crate::classpath::Completeness,
}

/// Quiet time after the last edit before an open buffer is reindexed.
const REINDEX_DELAY: std::time::Duration = std::time::Duration::from_millis(400);

/// The Java language service of a project: indexes + model, shared with the daemon pass.
#[derive(Default)]
pub struct JavaService {
    state: RwLock<State>,
}

impl JavaService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn set_project(&self, root: &Path, project: Option<JavaProject>) {
        let mut s = self.state.write();
        s.root = root.to_path_buf();
        s.project = project;
    }

    pub fn project(&self) -> Option<JavaProject> {
        self.state.read().project.clone()
    }

    pub fn root(&self) -> PathBuf {
        self.state.read().root.clone()
    }

    pub fn settings(&self) -> JavaSettings {
        self.state.read().settings.clone()
    }

    pub fn set_settings(&self, settings: JavaSettings) {
        self.state.write().settings = settings;
    }

    /// Brings the indexes in line with the project model (JAVA-STUB-018, PLAT-SCOPE-090): a new or
    /// changed model schedules an incremental sync; `loading` is the import in progress
    /// (JAVA-ERR-003). Returns true when the completeness of answers changed (the daemon must
    /// run again).
    pub fn sync(
        &self,
        root: &Path,
        model: Option<Arc<ProjectModel>>,
        loading: bool,
        notes: Vec<String>,
        cache_dir: Option<PathBuf>,
    ) -> bool {
        let mut s = self.state.write();
        let root_changed = s.root != root;
        if root_changed {
            s.root = root.to_path_buf();
            s.index = None;
            s.model = None;
            s.adhoc = None;
            s.docs.clear();
        }
        // No imported model (no build file, import not finished or failed): the ad-hoc module,
        // so completion and errors work in any folder with the JDK and the folder's sources.
        let model = match model {
            Some(m) => Some(m),
            None => {
                if s.adhoc.is_none() {
                    s.adhoc = crate::classpath::ad_hoc_model(root).map(Arc::new);
                }
                s.adhoc.clone()
            }
        };
        if s.index.is_none() {
            let dir = cache_dir.as_ref().map(|d| d.join("java"));
            if let Some(d) = &dir {
                let _ = std::fs::create_dir_all(d);
            }
            let index = JavaIndex::open(dir.as_deref());
            // Stubs of a library are shared by every project that uses the same jar.
            index.set_shared_cache(
                cache_dir
                    .as_ref()
                    .and_then(|d| d.parent())
                    .map(|p| p.join("java-libs")),
            );
            s.index = Some(index);
            s.cache_dir = cache_dir;
        }
        let index = s.index.clone().expect("created above");
        let model_changed = match (&s.model, &model) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        if model_changed {
            s.model = model.clone();
            if let Some(m) = &model {
                index.sync(m.clone());
            }
        }
        if model_changed || s.loading != loading || s.notes != notes || s.project.is_none() {
            s.loading = loading;
            s.notes = notes.clone();
            s.project = s
                .model
                .clone()
                .map(|m| JavaProject::new(index.clone(), m, loading).with_import_notes(notes));
        }
        let complete = s.project.as_ref().is_some_and(JavaProject::is_complete);
        let flipped = complete != s.complete || model_changed;
        s.complete = complete;
        flipped
    }

    pub fn status(&self) -> JavaStatus {
        let s = self.state.read();
        JavaStatus {
            state: s
                .index
                .as_ref()
                .map_or(JavaIndexState::Absent, JavaIndex::state),
            model_loading: s.loading,
            native: s.settings.native,
            classpath: s
                .project
                .as_ref()
                .map(JavaProject::completeness)
                .unwrap_or_default(),
        }
    }

    pub fn open_chooser(&self, title: String, rows: Vec<archipel_ide_analysis::ChoiceRow>) {
        self.state.write().chooser = Some(Chooser { title, rows });
    }

    pub fn chooser(&self) -> Option<Chooser> {
        self.state.read().chooser.clone()
    }

    /// Chooses row `index`: closes the chooser and returns what applies.
    pub fn choose(&self, index: usize) -> Option<ModCommand> {
        let chooser = self.state.write().chooser.take()?;
        chooser.rows.into_iter().nth(index).map(|r| r.command)
    }

    pub fn close_chooser(&self) {
        self.state.write().chooser = None;
    }

    pub fn index(&self) -> Option<JavaIndex> {
        self.state.read().index.clone()
    }

    /// An open buffer changed (or is seen for the first time): reindex it once typing paused
    /// (the overlay of unsaved text, JAVA-STUB-018). `text` is only called when needed.
    pub fn document_seen(&self, path: &Path, rev: u64, text: impl FnOnce() -> Arc<str>) {
        let mut s = self.state.write();
        let Some(index) = s.index.clone() else { return };
        let Some(project) = s.project.clone() else {
            return;
        };
        let path = self.abs(&s.root, path);
        if !project.applies_to(&path) {
            return;
        }
        let now = std::time::Instant::now();
        let entry = s.docs.entry(path.clone()).or_insert(DocState {
            rev,
            seen: now,
            indexed: false,
        });
        if entry.rev != rev {
            *entry = DocState {
                rev,
                seen: now,
                indexed: false,
            };
        }
        if entry.indexed || now.duration_since(entry.seen) < REINDEX_DELAY && entry.seen != now {
            return;
        }
        if !index.is_ready() {
            return;
        }
        entry.indexed = true;
        let module = project.files.module_for_file(&path).map(|m| {
            (
                m.name.clone(),
                project.files.is_in_test_source_content(&path),
            )
        });
        drop(s);
        let text = text();
        index.update_source(&path, &text, module.as_ref().map(|(m, t)| (m.as_str(), *t)));
    }

    /// Files changed on disk (watcher): sources are reindexed from disk, deleted ones forgotten.
    pub fn paths_changed(&self, paths: &[PathBuf]) {
        let s = self.state.read();
        let (Some(index), Some(project)) = (s.index.clone(), s.project.clone()) else {
            return;
        };
        let root = s.root.clone();
        drop(s);
        if !index.is_ready() {
            return;
        }
        for p in paths {
            let abs = self.abs(&root, p);
            if abs.extension().and_then(|e| e.to_str()) != Some("java")
                || !project.files.is_in_source_content(&abs)
            {
                continue;
            }
            match std::fs::read(&abs) {
                Ok(bytes) => {
                    let module = project.files.module_for_file(&abs).map(|m| {
                        (
                            m.name.clone(),
                            project.files.is_in_test_source_content(&abs),
                        )
                    });
                    index.update_source(
                        &abs,
                        &String::from_utf8_lossy(&bytes),
                        module.as_ref().map(|(m, t)| (m.as_str(), *t)),
                    );
                }
                Err(_) => index.remove_source(&abs),
            }
        }
    }

    fn abs(&self, root: &Path, p: &Path) -> PathBuf {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        }
    }

    pub fn absolute(&self, p: &Path) -> PathBuf {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.state.read().root.join(p)
        }
    }
}

fn place_for(
    path: &Path,
    text: &str,
    tree: &Tree,
    current: &CurrentFile,
    at: usize,
    scope: &ResolveScope,
) -> Place {
    let top = {
        // The top-level class containing `at`.
        let root = tree.root_node();
        let mut found = None;
        let mut c = root.walk();
        for n in root.children(&mut c) {
            if crate::source::is_class_kind(n.kind()) && n.start_byte() <= at && at <= n.end_byte()
            {
                found = n
                    .child_by_field_name("name")
                    .map(|x| text[x.byte_range()].to_string());
            }
        }
        found.and_then(|name| {
            current
                .stub
                .classes
                .iter()
                .find(|c| c.name == name)
                .map(|c| c.qname.clone())
        })
    };
    Place {
        path: path.to_path_buf(),
        package: current.stub.package.clone(),
        top_class: top,
        module: scope.module.clone(),
        test: scope.test,
        imports: current.stub.imports.clone(),
    }
}

fn fix_descriptor(fix: Arc<dyn QuickFix>) -> FixDescriptor {
    FixDescriptor {
        family: fix.family().into(),
        text: fix.name().into(),
        action: Some(fix),
    }
}

/// Removes the given import statements (all unused imports of the file, JAVA-IMP-071).
struct RemoveUnusedImports {
    ranges: Vec<std::ops::Range<usize>>,
}

impl QuickFix for RemoveUnusedImports {
    fn family(&self) -> &str {
        "Remove unused imports"
    }
    fn name(&self) -> String {
        "Remove unused imports".into()
    }
    fn fix(&self, _d: &ProblemDescriptor, cx: &FixCx<'_>) -> Result<ModCommand, Cancelled> {
        let text = cx.syntax.text();
        let mut out = text.to_string();
        let (nodes, _) = import_edit::parse_imports(text);
        let mut targets: Vec<_> = nodes
            .into_iter()
            .filter(|n| self.ranges.iter().any(|r| r.start == n.range.start))
            .collect();
        targets.sort_by_key(|n| std::cmp::Reverse(n.range.start));
        for n in targets {
            out = import_edit::delete_statement_public(&out, &n);
        }
        Ok(match import_edit::minimal_edit(text, &out) {
            Some(e) => ModCommand::UpdateFile {
                file: cx.file,
                edits: vec![e],
                expected_rev: cx.syntax.rev(),
            },
            None => ModCommand::Nothing,
        })
    }
    fn batch_ok(&self) -> bool {
        false
    }
}

/// What one analysis of a file produced, with the concrete fixes (for tests and previews).
#[derive(Default)]
pub struct Collected {
    pub infos: Vec<HighlightInfo>,
    /// `(problem range, fix)`
    pub static_fixes: Vec<(std::ops::Range<u32>, Arc<ImportStaticFix>)>,
    pub class_fixes: Vec<(std::ops::Range<u32>, Arc<ImportClassFix>)>,
}

/// Everything the pass reports for one file.
pub fn collect(
    project: &JavaProject,
    settings: &JavaSettings,
    path: &Path,
    text: &str,
    tree: &Tree,
) -> Vec<HighlightInfo> {
    collect_detailed(project, settings, path, text, tree).infos
}

pub fn collect_detailed(
    project: &JavaProject,
    settings: &JavaSettings,
    path: &Path,
    text: &str,
    tree: &Tree,
) -> Collected {
    let Some((analysis, current, scope)) = project.analyze_tree(path, text, tree) else {
        return Collected::default();
    };
    let mut extra = Collected::default();
    let env = Env::new(&project.index, &scope, Some(&current));
    let tool = ToolId::native(TOOL);
    let mut out: Vec<HighlightInfo> = Vec::new();
    let root = tree.root_node();
    for p in &analysis.problems {
        let (severity, kind) = match p.kind {
            ProblemKind::Pending => (
                Severity::INFORMATION,
                HighlightKind::TextAttributes("PENDING_REF"),
            ),
            _ => (Severity::ERROR, HighlightKind::Unresolved),
        };
        let mut info = HighlightInfo::new(p.range.clone(), severity, kind, tool.clone())
            .message(p.message.clone());
        info.source = Some("Java".into());
        if project.is_complete() {
            match &p.kind {
                ProblemKind::Method {
                    name,
                    args,
                    qualified: false,
                    ..
                } => {
                    let place =
                        place_for(path, text, tree, &current, p.range.start as usize, &scope);
                    let input = SearchInput {
                        name,
                        args: Some(args),
                        expected: &[],
                    };
                    if let Ok(cands) = import_fix::static_candidates(
                        project,
                        &env,
                        &place,
                        &input,
                        &settings.imports,
                    ) && !cands.is_empty()
                    {
                        let fix = Arc::new(ImportStaticFix {
                            candidates: cands,
                            settings: settings.imports.clone(),
                            constant: false,
                            always_on_demand: Vec::new(),
                        });
                        extra.static_fixes.push((p.range.clone(), fix.clone()));
                        info.fixes.push(fix_descriptor(fix));
                    }
                }
                ProblemKind::Symbol {
                    name,
                    class_ref,
                    annotation,
                } => {
                    let node = root
                        .descendant_for_byte_range(p.range.start as usize, p.range.end as usize);
                    if let Some(node) = node {
                        let place =
                            place_for(path, text, tree, &current, p.range.start as usize, &scope);
                        add_symbol_fixes(
                            project,
                            &env,
                            settings,
                            &place,
                            &current,
                            node,
                            text,
                            name,
                            *class_ref,
                            *annotation,
                            &mut info,
                            &mut extra,
                        );
                    }
                }
                _ => {}
            }
        }
        out.push(info);
    }
    if settings.unused_imports && project.is_complete() {
        let used = import_edit::unused_imports(text, &analysis.usage, &current.stub.package, &env);
        let ranges: Vec<_> = used.iter().map(|n| n.range.clone()).collect();
        for n in &used {
            let mut info = HighlightInfo::new(
                n.range.start as u32..n.range.end as u32,
                Severity::WARNING,
                HighlightKind::Unused,
                ToolId::native("java-unused-import"),
            )
            .message("Unused import statement");
            info.source = Some("Java".into());
            info.fixes
                .push(fix_descriptor(Arc::new(RemoveUnusedImports {
                    ranges: ranges.clone(),
                })));
            out.push(info);
        }
    }
    extra.infos = out;
    extra
}

#[allow(clippy::too_many_arguments)]
fn add_symbol_fixes(
    project: &JavaProject,
    env: &Env<'_>,
    settings: &JavaSettings,
    place: &Place,
    current: &CurrentFile,
    node: Node<'_>,
    text: &str,
    name: &str,
    class_ref: bool,
    annotation: bool,
    info: &mut HighlightInfo,
    extra: &mut Collected,
) {
    let parent = node.parent();
    let mut cref = ClassRefInfo {
        annotation,
        ..ClassRefInfo::default()
    };
    let mut eligible_class = class_ref;
    let mut eligible_const = false;
    if node.kind() == "identifier"
        && let Some(p) = parent
    {
        match p.kind() {
            "field_access"
                if p.child_by_field_name("object")
                    .is_some_and(|o| o.id() == node.id()) =>
            {
                if let Some(f) = p.child_by_field_name("field") {
                    cref.qualified_member = Some((text[f.byte_range()].to_string(), true));
                    eligible_class = true;
                }
            }
            "method_invocation"
                if p.child_by_field_name("object")
                    .is_some_and(|o| o.id() == node.id()) =>
            {
                if let Some(f) = p.child_by_field_name("name") {
                    cref.qualified_member = Some((text[f.byte_range()].to_string(), true));
                    eligible_class = true;
                }
            }
            "method_reference" => eligible_class = true,
            "expression_statement" => eligible_class = true,
            "marker_annotation" | "annotation" => {
                cref.annotation = true;
                eligible_class = true;
            }
            _ => {
                eligible_const = true;
            }
        }
        if !matches!(
            p.kind(),
            "field_access"
                | "method_invocation"
                | "method_reference"
                | "expression_statement"
                | "marker_annotation"
                | "annotation"
        ) {
            eligible_class = false;
        }
        if matches!(p.kind(), "field_access" | "method_invocation") && !eligible_class {
            eligible_const = true;
        }
    }
    // Qualified type names: only the head can be imported.
    if node.kind() == "type_identifier"
        && let Some(p) = parent
    {
        if p.kind() == "generic_type" {
            cref.type_args = true;
        }
        if p.kind() == "scoped_type_identifier"
            && p.child_by_field_name("scope")
                .is_some_and(|s| s.id() != node.id())
        {
            eligible_class = false;
        }
        if p.kind() == "scoped_type_identifier" {
            eligible_class = false;
        }
    }
    if let Some(p) = parent
        && p.kind() == "generic_type"
    {
        cref.type_args = true;
    }
    cref.unresolved_import_names = current
        .stub
        .imports
        .iter()
        .filter(|i| !i.is_static && !i.on_demand)
        .filter(|i| env.find_class(&i.name).ok().flatten().is_none())
        .filter_map(|i| i.name.rsplit('.').next().map(str::to_string))
        .collect();
    if eligible_const
        && !class_ref
        && let Ok(cands) = import_fix::static_candidates(
            project,
            env,
            place,
            &SearchInput {
                name,
                args: None,
                expected: &[],
            },
            &settings.imports,
        )
        && !cands.is_empty()
    {
        let fix = Arc::new(ImportStaticFix {
            candidates: cands,
            settings: settings.imports.clone(),
            constant: true,
            always_on_demand: Vec::new(),
        });
        extra.static_fixes.push((info.range.clone(), fix.clone()));
        info.fixes.push(fix_descriptor(fix));
    }
    if eligible_class
        && let Ok(cands) =
            import_fix::class_candidates(project, env, place, name, &cref, &settings.imports)
        && !cands.is_empty()
    {
        let fix = Arc::new(ImportClassFix {
            candidates: cands,
            settings: settings.imports.clone(),
            package: current.stub.package.clone(),
        });
        extra.class_fixes.push((info.range.clone(), fix.clone()));
        info.fixes.insert(0, fix_descriptor(fix));
    }
}

/// The daemon pass (`java-resolve`).
pub struct JavaResolvePass {
    pub service: Arc<JavaService>,
}

impl Pass for JavaResolvePass {
    fn id(&self) -> PassId {
        PASS_ID
    }
    fn after(&self) -> &[PassId] {
        &[PassId("syntax")]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        false
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        let empty = |infos| PassOutput {
            file: cx.file,
            rev: cx.syntax.rev(),
            tool: ToolId::native(TOOL),
            range: 0..u32::MAX,
            infos,
        };
        if cx.syntax.lang().id != "java" {
            return Ok(empty(Vec::new()));
        }
        let settings = self.service.settings();
        let Some(project) = self.service.project() else {
            return Ok(empty(Vec::new()));
        };
        let Some(rel) = cx.snap.path(cx.file) else {
            return Ok(empty(Vec::new()));
        };
        let path = self.service.absolute(rel);
        if !settings.native || !project.applies_to(&path) {
            return Ok(empty(Vec::new()));
        }
        cx.cancel.check()?;
        let infos = collect(
            &project,
            &settings,
            &path,
            cx.syntax.text(),
            cx.syntax.tree(),
        );
        cx.cancel.check()?;
        // The unused-import highlights live under their own tool id so each slice replaces atomically.
        Ok(empty(infos))
    }
}

/// A fix text for tests: applies the first candidate directly (headless mode, 5.4.2).
pub fn first_choice(command: ModCommand) -> ModCommand {
    match command {
        ModCommand::ChooseFrom { rows, .. } => rows
            .into_iter()
            .next()
            .map(|r| r.command)
            .unwrap_or(ModCommand::Nothing),
        other => other,
    }
}

/// Applies the edits of `command` for `file` to `text` (tests, previews).
pub fn apply_command(command: &ModCommand, file: FileId, text: &str) -> Option<String> {
    let edits: Vec<TextEdit> = command.edits_for(file).into_iter().cloned().collect();
    archipel_ide_analysis::modcommand::apply_edits(text, &edits).ok()
}
